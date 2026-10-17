use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, DocumentViewKind, HasToc,
    PageContent, PageInfo, PageTextSpan, TocItem,
};

pub struct DjvuDocument {
    metadata: BookMetadata,
    _path: PathBuf,
    doc: djvu_rs::DjVuDocument,
    page_count: usize,
    page_names: Vec<String>,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl DjvuDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref).map_err(|e| {
            DocumentError::Io(format!(
                "Не удалось открыть DjVu файл {}: {e}",
                path_ref.display()
            ))
        })?;

        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        const MAX_DJVU_SIZE: u64 = 500 * 1024 * 1024;
        if file_len > MAX_DJVU_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер DjVu файла превышает максимально допустимый лимит 500 МБ".to_string(),
            ));
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать DjVu файл: {e}")))?;

        Self::from_bytes(bytes, path_ref)
    }

    pub fn from_bytes(bytes: Vec<u8>, path: &Path) -> Result<Self, DocumentError> {
        let offset = if bytes.len() >= 8 && bytes.starts_with(b"AT&TFORM") {
            0
        } else if let Some(pos) = bytes[..bytes.len().min(1024)]
            .windows(8)
            .position(|w| w == b"AT&TFORM")
        {
            pos
        } else if bytes.len() >= 4 && bytes.starts_with(b"FORM") {
            0
        } else {
            return Err(DocumentError::Corrupted(
                "Файл не является валидным DjVu документом (отсутствует заголовок AT&TFORM)"
                    .to_string(),
            ));
        };

        let data = if offset > 0 {
            &bytes[offset..]
        } else {
            &bytes[..]
        };
        if data.len() < 12 {
            return Err(DocumentError::Corrupted(
                "Файл DjVu слишком короткий для чтения заголовка".to_string(),
            ));
        }

        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        let base_dir_buf = base_dir.to_path_buf();
        let safe_resolver = move |name: &str| -> Result<Vec<u8>, djvu_rs::DocError> {
            let clean = name.strip_prefix("file://").unwrap_or(name);
            let p = Path::new(clean);
            if p.is_absolute() || !crate::document::is_safe_archive_path(clean) {
                return Err(djvu_rs::DocError::IndirectResolve(clean.to_string()));
            }
            let target = base_dir_buf.join(p);
            if let (Ok(canon_base), Ok(canon_target)) =
                (base_dir_buf.canonicalize(), target.canonicalize())
            {
                if !canon_target.starts_with(&canon_base) {
                    return Err(djvu_rs::DocError::IndirectResolve(clean.to_string()));
                }
            } else {
                return Err(djvu_rs::DocError::IndirectResolve(clean.to_string()));
            }
            let metadata = std::fs::metadata(&target)
                .map_err(|_| djvu_rs::DocError::IndirectResolve(clean.to_string()))?;
            if !metadata.is_file() || metadata.len() > crate::document::MAX_SINGLE_ENTRY_BYTES {
                return Err(djvu_rs::DocError::IndirectResolve(clean.to_string()));
            }
            std::fs::read(&target)
                .map_err(|_| djvu_rs::DocError::IndirectResolve(clean.to_string()))
        };

        let mut limits = djvu_rs::resource_limits::ResourceLimits::default();
        limits.max_file_bytes = Some(500 * 1024 * 1024);
        limits.max_pages = Some(10_000);
        limits.max_components = Some(10_000);
        limits.max_page_pixels = Some(25_000_000);
        limits.max_total_pixels = Some(500_000_000);
        limits.max_decoded_bytes = Some(150 * 1024 * 1024);
        limits.max_render_pixels = Some(25_000_000);
        let opts = djvu_rs::resource_limits::ParseOptions {
            limits: Some(limits),
        };

        let parse_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            djvu_rs::DjVuDocument::parse_with_resolver_and_options(data, Some(safe_resolver), &opts)
        }));

        let doc = match parse_result {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => {
                return Err(DocumentError::Corrupted(format!(
                    "Ошибка парсинга DjVu документа: {e:?}"
                )));
            }
            Err(_) => {
                return Err(DocumentError::Corrupted(
                    "Аварийный сбой при разборе DjVu структуры".to_string(),
                ));
            }
        };

        let page_count = doc.page_count();
        if page_count == 0 {
            return Err(DocumentError::Corrupted(
                "DjVu документ не содержит ни одной страницы".to_string(),
            ));
        }

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ DjVu")
            .to_string();

        let meta_opt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| doc.metadata()))
            .ok()
            .and_then(|r| r.ok())
            .flatten();

        let (title, author) = if let Some(meta) = meta_opt {
            let t = meta
                .title
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(default_title);
            let a = meta
                .author
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Неизвестный автор".to_string());
            (t, a)
        } else {
            (default_title, "Неизвестный автор".to_string())
        };

        let mut page_names = Vec::with_capacity(page_count);
        let mut page_id_map = HashMap::new();
        if let Ok(dir) = doc.component_directory() {
            let mut page_idx = 0;
            for entry in dir {
                if entry.kind == 'P' {
                    let clean_id = entry.id.trim().to_string();
                    let lower = clean_id.to_lowercase();
                    page_id_map.insert(lower.clone(), page_idx);
                    if let Some(stripped) = lower
                        .strip_suffix(".djvu")
                        .or_else(|| lower.strip_suffix(".djv"))
                    {
                        page_id_map.insert(stripped.to_string(), page_idx);
                    }
                    let dec = percent_decode_djvu(&lower);
                    page_id_map.insert(dec.clone(), page_idx);
                    if let Some(stripped) = dec
                        .strip_suffix(".djvu")
                        .or_else(|| dec.strip_suffix(".djv"))
                    {
                        page_id_map.insert(stripped.to_string(), page_idx);
                    }
                    page_names.push(clean_id);
                    page_idx += 1;
                }
            }
        }

        let bookmarks = doc.bookmarks();
        let toc = convert_djvu_bookmarks(bookmarks, &page_id_map, page_count, 0);

        let chapters = if toc.is_empty() {
            (0..page_count)
                .map(|i| BookChapter {
                    id: Some(format!("page_{}", i + 1)),
                    title: format!("Страница {}", i + 1),
                    paragraphs: vec![format!("Страница {}", i + 1)],
                })
                .collect()
        } else {
            let mut list = Vec::with_capacity(toc.len());
            for (i, item) in toc.iter().enumerate() {
                list.push(BookChapter {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    paragraphs: vec![format!(
                        "Раздел {} (страница {})",
                        i + 1,
                        item.chapter_index + 1
                    )],
                });
            }
            list
        };

        Ok(Self {
            metadata: BookMetadata { title, author },
            _path: path.to_path_buf(),
            doc,
            page_count,
            page_names,
            chapters,
            toc,
        })
    }
}

pub(crate) fn percent_decode_djvu(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b_val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            result.push(b_val);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub(crate) fn resolve_djvu_bookmark_page(
    url: &str,
    page_id_map: &HashMap<String, usize>,
    total_pages: usize,
) -> usize {
    if total_pages == 0 {
        return 0;
    }
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return 0;
    }

    let decoded = percent_decode_djvu(trimmed);
    let s = decoded.trim();

    let target = s.strip_prefix('#').unwrap_or(s);
    let clean = target.split(['?', '&']).next().unwrap_or(target).trim();
    if clean.is_empty() {
        return 0;
    }

    let clean_lower = clean.to_lowercase();
    if let Some(&idx) = page_id_map.get(&clean_lower) {
        return idx.min(total_pages.saturating_sub(1));
    }
    if let Some(stripped) = clean_lower
        .strip_suffix(".djvu")
        .or_else(|| clean_lower.strip_suffix(".djv"))
    {
        if let Some(&idx) = page_id_map.get(stripped) {
            return idx.min(total_pages.saturating_sub(1));
        }
    }
    if let Some(&idx) = page_id_map
        .get(&format!("{clean_lower}.djvu"))
        .or_else(|| page_id_map.get(&format!("{clean_lower}.djv")))
    {
        return idx.min(total_pages.saturating_sub(1));
    }

    let num_candidate = clean_lower.strip_prefix('+').unwrap_or(&clean_lower);
    if let Ok(p) = num_candidate.parse::<usize>() {
        if p > 0 {
            return (p - 1).min(total_pages.saturating_sub(1));
        }
        return 0;
    }

    let prefixes = ["page", "pg", "p"];
    for prefix in prefixes {
        if let Some(rest) = clean_lower.strip_prefix(prefix) {
            let digits_candidate = rest.trim_start_matches(['_', '-', '+', '=']);
            if let Ok(p) = digits_candidate.parse::<usize>() {
                if p > 0 {
                    return (p - 1).min(total_pages.saturating_sub(1));
                }
                return 0;
            }
        }
    }

    0
}

fn convert_djvu_bookmarks(
    bookmarks: &[djvu_rs::DjVuBookmark],
    page_id_map: &HashMap<String, usize>,
    total_pages: usize,
    depth: usize,
) -> Vec<TocItem> {
    if depth >= 32 {
        return Vec::new();
    }
    bookmarks
        .iter()
        .map(|bm| {
            let chapter_index = resolve_djvu_bookmark_page(&bm.url, page_id_map, total_pages);
            TocItem {
                id: Some(bm.url.clone()),
                title: if bm.title.trim().is_empty() {
                    format!("Страница {}", chapter_index + 1)
                } else {
                    bm.title.clone()
                },
                chapter_index,
                children: convert_djvu_bookmarks(&bm.children, page_id_map, total_pages, depth + 1),
            }
        })
        .collect()
}

impl HasToc for DjvuDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

impl Document for DjvuDocument {
    fn metadata(&self) -> &BookMetadata {
        &self.metadata
    }

    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }

    fn toc(&self) -> &[TocItem] {
        &self.toc
    }

    fn format_name(&self) -> &'static str {
        "DjVu"
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn view_kind(&self) -> DocumentViewKind {
        DocumentViewKind::Pdf
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        Some(self)
    }
}

impl PageContent for DjvuDocument {
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_name(&self, index: usize) -> &str {
        self.page_names.get(index).map(|s| s.as_str()).unwrap_or("")
    }

    fn page_info(&self, _index: usize) -> Option<&PageInfo> {
        None
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError> {
        self.load_page_image_scaled(index, 2.0)
    }

    fn load_page_image_scaled(
        &self,
        index: usize,
        scale: f32,
    ) -> Result<DecodedImage, DocumentError> {
        if index >= self.page_count {
            return Err(DocumentError::Corrupted(format!(
                "Страница {} выходит за пределы документа",
                index + 1
            )));
        }

        let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let page = self.doc.page(index).map_err(|e| {
                DocumentError::Corrupted(format!("Ошибка получения страницы {}: {e:?}", index + 1))
            })?;

            let page_w = page.width() as f32;
            let page_h = page.height() as f32;
            if page_w <= 0.0 || page_h <= 0.0 {
                return Err(DocumentError::Corrupted(format!(
                    "Страница {} имеет некорректные размеры (0x0)",
                    index + 1
                )));
            }

            let mut render_scale = scale.clamp(0.1, 4.0);
            if page_w * render_scale > 4096.0 {
                render_scale = render_scale.min(4096.0 / page_w);
            }
            if page_h * render_scale > 4096.0 {
                render_scale = render_scale.min(4096.0 / page_h);
            }
            let total_pixels = (page_w * render_scale) * (page_h * render_scale);
            if total_pixels > 25_000_000.0 {
                render_scale = render_scale.min((25_000_000.0 / (page_w * page_h)).sqrt());
            }
            let render_scale = render_scale.clamp(0.05, 4.0);

            let width = ((page_w * render_scale).round() as u32).clamp(1, 4096);
            let height = ((page_h * render_scale).round() as u32).clamp(1, 4096);

            let opts = djvu_rs::djvu_render::RenderOptions {
                width,
                height,
                permissive: true,
                ..Default::default()
            };

            let pixmap = djvu_rs::djvu_render::render_pixmap(page, &opts).map_err(|e| {
                DocumentError::Corrupted(format!(
                    "Ошибка отрисовки страницы {} DjVu: {e:?}",
                    index + 1
                ))
            })?;

            if pixmap.width == 0 || pixmap.height == 0 {
                return Err(DocumentError::Corrupted(format!(
                    "Страница {} имеет нулевой размер",
                    index + 1
                )));
            }

            let expected_len = (pixmap.width as usize)
                .saturating_mul(pixmap.height as usize)
                .saturating_mul(4);
            if pixmap.data.len() != expected_len {
                return Err(DocumentError::Corrupted(format!(
                    "Некорректный размер буфера страницы {}: ожидается {}, получено {}",
                    index + 1,
                    expected_len,
                    pixmap.data.len()
                )));
            }

            Ok(DecodedImage {
                width: pixmap.width,
                height: pixmap.height,
                rgba: pixmap.data,
            })
        }));

        self.doc.enforce_cache_budget(128 * 1024 * 1024, &[index]);

        match render_result {
            Ok(res) => res,
            Err(_) => Err(DocumentError::Corrupted(format!(
                "Аварийный сбой декодера при отрисовке страницы {}",
                index + 1
            ))),
        }
    }

    fn extract_page_text(&self, index: usize) -> Option<String> {
        if index >= self.page_count {
            return None;
        }
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let page = self.doc.page(index).ok()?;
            page.text().ok().flatten()
        }));
        res.ok().flatten()
    }

    fn extract_page_text_spans(&self, index: usize) -> Option<Vec<PageTextSpan>> {
        if index >= self.page_count {
            return None;
        }
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let page = self.doc.page(index).ok()?;
            let layer = page.text_layer().ok().flatten()?;
            let page_w = page.width() as f32;
            let page_h = page.height() as f32;
            if page_w <= 0.0 || page_h <= 0.0 {
                return None;
            }

            let mut spans = Vec::new();
            fn collect_spans(
                zone: &djvu_rs::text::TextZone,
                page_w: f32,
                page_h: f32,
                depth: usize,
                spans: &mut Vec<PageTextSpan>,
            ) {
                if depth >= 32 {
                    return;
                }
                if matches!(
                    zone.kind,
                    djvu_rs::text::TextZoneKind::Word | djvu_rs::text::TextZoneKind::Character
                ) || (zone.children.is_empty() && !zone.text.trim().is_empty())
                {
                    let text = zone.text.trim();
                    if !text.is_empty() {
                        let min_x = (zone.rect.x as f32 / page_w).clamp(0.0, 1.0);
                        let min_y = (zone.rect.y as f32 / page_h).clamp(0.0, 1.0);
                        let max_x =
                            ((zone.rect.x + zone.rect.width) as f32 / page_w).clamp(0.0, 1.0);
                        let max_y =
                            ((zone.rect.y + zone.rect.height) as f32 / page_h).clamp(0.0, 1.0);
                        if max_x > min_x && max_y > min_y {
                            spans.push(PageTextSpan {
                                text: text.to_string(),
                                bbox: [min_x, min_y, max_x, max_y],
                            });
                        }
                    }
                } else {
                    for child in &zone.children {
                        collect_spans(child, page_w, page_h, depth + 1, spans);
                    }
                }
            }

            for zone in &layer.zones {
                collect_spans(zone, page_w, page_h, 0, &mut spans);
            }

            if spans.is_empty() { None } else { Some(spans) }
        }));
        res.ok().flatten()
    }

    fn is_manga(&self) -> bool {
        false
    }
}
