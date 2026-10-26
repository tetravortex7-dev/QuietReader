pub mod links;
pub mod outline;
pub mod text;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, DocumentViewKind,
    PageContent, PageInfo, PageTextSpan, TocItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfPermissions {
    pub can_print: bool,
    pub can_copy: bool,
    pub can_modify: bool,
    pub can_annotate: bool,
}

impl Default for PdfPermissions {
    fn default() -> Self {
        Self {
            can_print: true,
            can_copy: true,
            can_modify: true,
            can_annotate: true,
        }
    }
}

pub struct PdfDocument {
    metadata: BookMetadata,
    _path: PathBuf,
    pdf: hayro::hayro_syntax::Pdf,
    page_count: usize,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    links: std::collections::HashMap<usize, Vec<crate::document::PageLink>>,
    permissions: PdfPermissions,
}

impl PdfDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| {
            DocumentError::Io(format!(
                "Не удалось открыть PDF файл {}: {e}",
                path_ref.display()
            ))
        })?;

        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        const MAX_PDF_SIZE: u64 = 300 * 1024 * 1024;
        if file_len > MAX_PDF_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер PDF файла превышает максимально допустимый лимит 300 МБ".to_string(),
            ));
        }
        if file_len == 0 {
            return Err(DocumentError::Corrupted(
                "Файл не является валидным PDF документом (файл пуст)".to_string(),
            ));
        }

        let mut bytes = Vec::new();
        let read = file
            .take(MAX_PDF_SIZE.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| {
                DocumentError::Io(format!(
                    "Не удалось прочитать PDF файл {}: {e}",
                    path_ref.display()
                ))
            })?;
        if read as u64 > MAX_PDF_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер PDF файла превышает максимально допустимый лимит 300 МБ".to_string(),
            ));
        }

        let data = hayro::hayro_syntax::PdfData::from(bytes);
        Self::from_pdf_data(data, path_ref)
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: Vec<u8>, path: &Path) -> Result<Self, DocumentError> {
        Self::from_pdf_data(bytes.into(), path)
    }

    fn from_pdf_data(
        data: hayro::hayro_syntax::PdfData,
        path: &Path,
    ) -> Result<Self, DocumentError> {
        let bytes = data.as_ref();
        if bytes.len() < 4 || !bytes.starts_with(b"%PDF") {
            if let Some(pos) = bytes[..bytes.len().min(1024)]
                .windows(4)
                .position(|w| w == b"%PDF")
            {
                let _ = pos;
            } else {
                return Err(DocumentError::Corrupted(
                    "Файл не является валидным PDF документом (отсутствует заголовок %PDF)"
                        .to_string(),
                ));
            }
        }

        let permissions = extract_pdf_permissions(bytes);

        let pdf = match hayro::hayro_syntax::Pdf::new(data.clone()) {
            Ok(pdf) => pdf,
            Err(_) => hayro::hayro_syntax::Pdf::new_with_password(data, "").map_err(|e| {
                DocumentError::Corrupted(format!("Ошибка парсинга PDF документа: {e:?}"))
            })?,
        };

        let page_count = pdf.pages().len();
        if page_count == 0 {
            return Err(DocumentError::Corrupted(
                "PDF документ не содержит ни одной страницы".to_string(),
            ));
        }

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ PDF")
            .to_string();

        let meta = pdf.metadata();
        let title = meta
            .title
            .as_ref()
            .map(|t| outline::decode_pdf_string(t))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(default_title);

        let author = meta
            .author
            .as_ref()
            .map(|a| outline::decode_pdf_string(a))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Неизвестный автор".to_string());

        let toc = outline::extract_pdf_toc(&pdf, page_count);
        let links = links::extract_pdf_links(&pdf, page_count);

        let mut chapters = Vec::with_capacity(toc.len());
        for (i, item) in toc.iter().enumerate() {
            chapters.push(BookChapter {
                id: item.id.clone(),
                title: item.title.clone(),
                paragraphs: vec![format!(
                    "Раздел {} (страница {})",
                    i + 1,
                    item.chapter_index + 1
                )],
            });
        }

        Ok(Self {
            metadata: BookMetadata { title, author },
            _path: path.to_path_buf(),
            pdf,
            page_count,
            chapters,
            toc,
            links,
            permissions,
        })
    }

    #[allow(dead_code)]
    pub fn permissions(&self) -> PdfPermissions {
        self.permissions
    }
}

impl Document for PdfDocument {
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
        "PDF"
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn view_kind(&self) -> DocumentViewKind {
        DocumentViewKind::Pdf
    }

    fn can_copy(&self) -> bool {
        self.permissions.can_copy
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }
}

impl PageContent for PdfDocument {
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_name(&self, _index: usize) -> &str {
        ""
    }

    fn page_info(&self, _index: usize) -> Option<&PageInfo> {
        None
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError> {
        self.load_page_image_scaled(index, 2.5)
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

        let pages = self.pdf.pages();
        let page = pages.get(index).ok_or_else(|| {
            DocumentError::Corrupted(format!("Страница {} не найдена", index + 1))
        })?;

        let interp_settings = hayro::hayro_interpret::InterpreterSettings::default();
        let render_settings = hayro::RenderSettings::default();

        let (page_w, page_h) = page.render_dimensions();
        let page_w = page_w.max(1.0);
        let page_h = page_h.max(1.0);

        let mut render_scale = scale.clamp(0.5, 4.0);
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
        let render_scale = render_scale.max(0.1);

        let pixmap_settings = hayro::PixmapSettings {
            x_scale: render_scale,
            y_scale: render_scale,
            bg_color: hayro::vello_cpu::color::palette::css::WHITE,
        };

        let cache = hayro::RenderCache::new();

        let pixmap = hayro::render(
            page,
            &cache,
            &interp_settings,
            &render_settings,
            &pixmap_settings,
        );

        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;

        if width == 0 || height == 0 {
            return Err(DocumentError::Corrupted(format!(
                "Страница {} имеет нулевой размер",
                index + 1
            )));
        }

        let raw_pixels = pixmap.data();
        let total_bytes = (width * height * 4) as usize;
        let mut rgba = vec![0u8; total_bytes];

        for (i, p) in raw_pixels.iter().enumerate() {
            let offset = i * 4;
            if p.a == 255 {
                rgba[offset] = p.r;
                rgba[offset + 1] = p.g;
                rgba[offset + 2] = p.b;
                rgba[offset + 3] = 255;
            } else if p.a == 0 {
                continue;
            } else {
                let inv_a = 255.0 / p.a as f32;
                rgba[offset] = (p.r as f32 * inv_a).min(255.0) as u8;
                rgba[offset + 1] = (p.g as f32 * inv_a).min(255.0) as u8;
                rgba[offset + 2] = (p.b as f32 * inv_a).min(255.0) as u8;
                rgba[offset + 3] = p.a;
            }
        }

        Ok(DecodedImage {
            width,
            height,
            rgba,
        })
    }

    fn extract_page_text(&self, index: usize) -> Option<String> {
        if index >= self.page_count {
            return None;
        }

        let pages = self.pdf.pages();
        let page = pages.get(index)?;

        text::extract_page_text(page)
    }

    fn page_links(&self, index: usize) -> &[crate::document::PageLink] {
        self.links.get(&index).map(Vec::as_slice).unwrap_or(&[])
    }

    fn extract_page_text_spans(&self, index: usize) -> Option<Vec<PageTextSpan>> {
        if index >= self.page_count {
            return None;
        }

        let pages = self.pdf.pages();
        let page = pages.get(index)?;

        text::extract_page_spans(page)
    }

    fn is_manga(&self) -> bool {
        false
    }
}

pub fn extract_pdf_permissions(data: &[u8]) -> PdfPermissions {
    let mut pos = 0;
    while let Some(idx) = find_subsequence(&data[pos..], b"/Filter") {
        let filter_pos = pos + idx;
        let window_start = filter_pos.saturating_sub(64);
        let window_end = (filter_pos + 512).min(data.len());
        let slice = &data[window_start..window_end];

        if let Some(p_idx) = find_p_entry(slice) {
            let p_slice = &slice[p_idx..];
            if let Some(p_val) = parse_pdf_integer(p_slice) {
                let p = if p_val < 0 {
                    u32::from_be_bytes((p_val as i32).to_be_bytes())
                } else {
                    p_val as u32
                };
                return PdfPermissions {
                    can_print: (p & 0x0004) != 0,
                    can_modify: (p & 0x0008) != 0,
                    can_copy: (p & 0x0010) != 0,
                    can_annotate: (p & 0x0020) != 0,
                };
            }
        }
        pos = filter_pos + 7;
    }
    PdfPermissions::default()
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_p_entry(slice: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 2 < slice.len() {
        if slice[i] == b'/' && slice[i + 1] == b'P' {
            let next = slice[i + 2];
            if next.is_ascii_whitespace() || next == b'-' || next == b'+' {
                return Some(i + 2);
            }
        }
        i += 1;
    }
    None
}

fn parse_pdf_integer(slice: &[u8]) -> Option<i64> {
    let mut i = 0;
    while i < slice.len() && slice[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= slice.len() {
        return None;
    }
    let mut is_negative = false;
    if slice[i] == b'-' {
        is_negative = true;
        i += 1;
    } else if slice[i] == b'+' {
        i += 1;
    }
    let start = i;
    while i < slice.len() && slice[i].is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return None;
    }
    let s = std::str::from_utf8(&slice[start..i]).ok()?;
    let val: i64 = s.parse().ok()?;
    Some(if is_negative { -val } else { val })
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
