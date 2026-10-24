use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use zip::ZipArchive;

use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, DocumentViewKind,
    PageContent, PageInfo, TocItem,
};

static SEVENZ_CACHE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct SevenZCache {
    dir: PathBuf,
    files: HashMap<String, PathBuf>,
}

impl SevenZCache {
    fn new() -> Result<Self, DocumentError> {
        let parent = std::env::temp_dir().join("QuietReader").join("comic-cache");
        std::fs::create_dir_all(&parent)
            .map_err(|e| DocumentError::Io(format!("Не удалось создать каталог кэша CB7: {e}")))?;
        let id = SEVENZ_CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = parent.join(format!("{}-{id}", std::process::id()));
        std::fs::create_dir(&dir)
            .map_err(|e| DocumentError::Io(format!("Не удалось создать кэш CB7: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)) {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(DocumentError::Io(format!(
                    "Не удалось ограничить доступ к кэшу CB7: {e}"
                )));
            }
        }
        Ok(Self {
            dir,
            files: HashMap::new(),
        })
    }
}

impl Drop for SevenZCache {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComicArchiveKind {
    Zip,
    Rar,
    SevenZ,
    Folder,
}

pub struct ComicDocument {
    metadata: BookMetadata,
    path: PathBuf,
    kind: ComicArchiveKind,
    pages: Vec<PageInfo>,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    is_manga: bool,
    sevenz_cache: Option<SevenZCache>,
}

impl ComicDocument {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(DocumentError::Io(format!(
                "Файл или каталог не найден: {}",
                path_ref.display()
            )));
        }

        let kind = if path_ref.is_dir() {
            ComicArchiveKind::Folder
        } else {
            let ext = path_ref
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            match ext.as_str() {
                "cbz" | "zip" => ComicArchiveKind::Zip,
                "cbr" | "rar" => ComicArchiveKind::Rar,
                "cb7" | "7z" => ComicArchiveKind::SevenZ,
                _ => {
                    let mut magic = [0u8; 8];
                    let n = File::open(path_ref)
                        .and_then(|mut f| f.read(&mut magic))
                        .map_err(|e| DocumentError::Io(format!("{}: {e}", path_ref.display())))?;
                    if n >= 4 && &magic[0..4] == b"PK\x03\x04" {
                        ComicArchiveKind::Zip
                    } else if (n >= 7 && &magic[0..7] == b"Rar!\x1a\x07\x00")
                        || (n >= 8 && &magic[0..8] == b"Rar!\x1a\x07\x01\x00")
                    {
                        ComicArchiveKind::Rar
                    } else if n >= 6 && &magic[0..6] == b"7z\xbc\xaf\x27\x1c" {
                        ComicArchiveKind::SevenZ
                    } else {
                        return Err(DocumentError::Unsupported(format!(
                            "Неподдерживаемый формат комикса: {}",
                            path_ref.display()
                        )));
                    }
                }
            }
        };

        let (mut pages, comic_info_xml, sevenz_cache) = match kind {
            ComicArchiveKind::Folder => {
                let (pages, info) = scan_folder(path_ref)?;
                (pages, info, None)
            }
            ComicArchiveKind::Zip => {
                let (pages, info) = scan_zip(path_ref)?;
                (pages, info, None)
            }
            ComicArchiveKind::Rar => {
                let (pages, info) = scan_rar(path_ref)?;
                (pages, info, None)
            }
            ComicArchiveKind::SevenZ => scan_sevenz(path_ref)?,
        };

        if pages.is_empty() {
            return Err(DocumentError::Corrupted(
                "В архиве/папке не найдено ни одного поддерживаемого изображения (JPEG, PNG, WEBP, GIF)"
                    .to_string(),
            ));
        }

        const MAX_PAGES_LIMIT: usize = 5000;
        if pages.len() > MAX_PAGES_LIMIT {
            pages.truncate(MAX_PAGES_LIMIT);
        }

        pages.sort_by(|a, b| natural_cmp(&a.archive_path, &b.archive_path));

        let default_title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Комикс")
            .to_string();

        let mut title = default_title;
        let mut author = "Неизвестный автор".to_string();
        let mut is_manga = false;

        let path_lower = path_ref.to_string_lossy().to_lowercase();
        if path_lower.contains("manga") || path_lower.contains("манга") {
            is_manga = true;
        }

        if let Some(xml_bytes) = comic_info_xml
            && let Ok(info) = parse_comic_info(&xml_bytes)
        {
            if let Some(t) = info.title
                && !t.trim().is_empty()
            {
                title = t;
            }
            if let Some(w) = info.writer
                && !w.trim().is_empty()
            {
                author = w;
            }
            if let Some(m) = info.is_manga {
                is_manga = m;
            }
        }

        let (chapters, toc) = build_comic_chapters(&pages, &title);

        Ok(Self {
            metadata: BookMetadata { title, author },
            path: path_ref.to_path_buf(),
            kind,
            pages,
            chapters,
            toc,
            is_manga,
            sevenz_cache,
        })
    }
}

impl Document for ComicDocument {
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
        "Comic"
    }

    fn has_custom_toc(&self) -> bool {
        self.toc.len() > 1
    }

    fn view_kind(&self) -> DocumentViewKind {
        DocumentViewKind::Comic
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }
}

impl PageContent for ComicDocument {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_name(&self, index: usize) -> &str {
        self.pages
            .get(index)
            .map(|p| p.display_name.as_str())
            .unwrap_or("")
    }

    fn page_info(&self, index: usize) -> Option<&PageInfo> {
        self.pages.get(index)
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError> {
        let page = self.pages.get(index).ok_or_else(|| {
            DocumentError::Corrupted(format!("Страница {} выходит за пределы", index + 1))
        })?;
        let max_single = crate::document::MAX_SINGLE_ENTRY_BYTES;
        let max_total = crate::document::MAX_TOTAL_UNPACK_BYTES;

        let bytes = match self.kind {
            ComicArchiveKind::Folder => {
                if !is_safe_path(&page.archive_path) {
                    return Err(DocumentError::Corrupted("Небезопасный путь".to_string()));
                }
                let full = self.path.join(&page.archive_path);
                let meta = std::fs::metadata(&full)?;
                if meta.len() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        "Превышен лимит 50 МБ".to_string(),
                    ));
                }
                std::fs::read(&full)?
            }
            ComicArchiveKind::Zip => {
                let mut zip = ZipArchive::new(File::open(&self.path)?)
                    .map_err(|e| DocumentError::Corrupted(format!("Ошибка ZIP: {e}")))?;
                let entry = zip
                    .by_name(&page.archive_path)
                    .map_err(|e| DocumentError::Corrupted(format!("Не найден в ZIP: {e}")))?;
                if entry.size() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        "Превышен лимит 50 МБ".to_string(),
                    ));
                }
                let mut total = 0u64;
                crate::document::read_bounded(entry, max_single, &mut total, max_total)?
            }
            ComicArchiveKind::Rar => {
                let mut archive = unrar_rs::RarArchive::open(File::open(&self.path)?)
                    .map_err(|e| DocumentError::Corrupted(format!("Ошибка RAR: {e}")))?;
                let handle = archive
                    .by_name(&page.archive_path)
                    .map_err(|e| DocumentError::Corrupted(format!("Не найден в RAR: {e}")))?;
                let mut total = 0u64;
                crate::document::read_bounded(handle, max_single, &mut total, max_total)?
            }
            ComicArchiveKind::SevenZ => {
                let cache = self
                    .sevenz_cache
                    .as_ref()
                    .ok_or_else(|| DocumentError::Corrupted("Кэш CB7 недоступен".to_string()))?;
                let cached = cache
                    .files
                    .get(&page.archive_path)
                    .ok_or_else(|| DocumentError::Corrupted("Не найден в 7z".to_string()))?;
                let meta = std::fs::metadata(cached)?;
                if meta.len() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        "Превышен лимит 50 МБ".to_string(),
                    ));
                }
                std::fs::read(cached)?
            }
        };

        decode_image_bytes(&bytes)
    }

    fn is_manga(&self) -> bool {
        self.is_manga
    }
}

fn decode_image_bytes(bytes: &[u8]) -> Result<DecodedImage, DocumentError> {
    let dim_reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DocumentError::Corrupted(format!("Ошибка формата изображения: {e}")))?;
    if let Ok((w, h)) = dim_reader.into_dimensions() {
        if w == 0 || h == 0 {
            return Err(DocumentError::Corrupted(
                "Изображение имеет нулевой размер".to_string(),
            ));
        }
        if w > 8192 || h > 16384 || (w as u64) * (h as u64) > 25_000_000 {
            return Err(DocumentError::LimitExceeded(format!(
                "Размеры изображения ({w}x{h}) превышают допустимый лимит"
            )));
        }
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DocumentError::Corrupted(format!("Ошибка формата изображения: {e}")))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(150 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().map_err(|e| {
        DocumentError::Corrupted(format!("Не удалось декодировать изображение: {e}"))
    })?;
    let img = if img.width() > 4096 || img.height() > 4096 {
        img.thumbnail(4096, 4096)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

pub fn is_supported_image(name: &str) -> bool {
    let filename = Path::new(name)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    if filename.starts_with('.')
        || filename.eq_ignore_ascii_case("thumbs.db")
        || name.contains("__MACOSX")
    {
        return false;
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp"
    )
}

pub fn zip_contains_supported_image(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let Ok(mut zip) = ZipArchive::new(file) else {
        return false;
    };
    if zip.len() > crate::document::MAX_ARCHIVE_ENTRIES {
        return false;
    }
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i)
            && !entry.is_dir()
            && let Ok(name) = entry.name()
            && crate::document::is_safe_archive_path(&name)
            && is_supported_image(&name)
        {
            return true;
        }
    }
    false
}

fn is_safe_path(p: &str) -> bool {
    crate::document::is_safe_archive_path(p)
}

fn is_comic_info_xml(p: &Path) -> bool {
    p.file_name()
        .map(|f| f.to_string_lossy().eq_ignore_ascii_case("comicinfo.xml"))
        .unwrap_or(false)
}

fn make_page_info(archive_path: String, path_obj: &Path) -> PageInfo {
    let display_name = path_obj
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| archive_path.clone());
    let chapter_title = path_obj
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().to_string());
    PageInfo {
        archive_path,
        display_name,
        chapter_title,
    }
}

fn scan_folder(folder: &Path) -> Result<(Vec<PageInfo>, Option<Vec<u8>>), DocumentError> {
    let mut pages = Vec::new();
    let mut comic_info = None;

    fn visit_dir(
        root: &Path,
        current: &Path,
        depth: usize,
        pages: &mut Vec<PageInfo>,
        comic_info: &mut Option<Vec<u8>>,
    ) {
        if depth > crate::document::MAX_FOLDER_DEPTH {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                if pages.len() >= crate::document::MAX_ARCHIVE_ENTRIES {
                    break;
                }
                let path = entry.path();
                let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    visit_dir(root, &path, depth + 1, pages, comic_info);
                } else if metadata.is_file() {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let rel_str = rel.to_string_lossy().replace('\\', "/");

                    if is_comic_info_xml(rel) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            *comic_info = Some(bytes);
                        }
                    } else if is_supported_image(&rel_str) && is_safe_path(&rel_str) {
                        pages.push(make_page_info(rel_str, rel));
                    }
                }
            }
        }
    }

    visit_dir(folder, folder, 0, &mut pages, &mut comic_info);
    Ok((pages, comic_info))
}

fn scan_zip(path: &Path) -> Result<(Vec<PageInfo>, Option<Vec<u8>>), DocumentError> {
    let file =
        File::open(path).map_err(|e| DocumentError::Io(format!("Не удалось открыть ZIP: {e}")))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| DocumentError::Corrupted(format!("Ошибка парсинга ZIP: {e}")))?;

    if zip.len() > crate::document::MAX_ARCHIVE_ENTRIES {
        return Err(DocumentError::LimitExceeded(
            "В архиве превышен лимит количества файлов (10 000)".to_string(),
        ));
    }

    let mut pages = Vec::new();
    let mut comic_info = None;
    let mut total_unpacked: u64 = 0;

    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            let name = entry
                .name()
                .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения ZIP: {e}")))?
                .to_string();
            if !is_safe_path(&name) {
                return Err(DocumentError::Corrupted(format!(
                    "Обнаружен небезопасный путь в архиве: {name}"
                )));
            }
            if entry.size() > crate::document::MAX_SINGLE_ENTRY_BYTES {
                return Err(DocumentError::LimitExceeded(format!(
                    "Файл {name} превышает лимит 50 МБ"
                )));
            }
            total_unpacked = total_unpacked.saturating_add(entry.size());
            if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
                return Err(DocumentError::LimitExceeded(
                    "Превышен общий лимит распаковки архива (500 МБ)".to_string(),
                ));
            }
            if name.ends_with('/') || name.ends_with('\\') {
                continue;
            }
            let path_obj = Path::new(&name);
            if is_comic_info_xml(path_obj) {
                let mut total_xml = 0u64;
                if let Ok(bytes) = crate::document::read_bounded(
                    entry,
                    5 * 1024 * 1024,
                    &mut total_xml,
                    5 * 1024 * 1024,
                ) {
                    comic_info = Some(bytes);
                }
            } else if is_supported_image(&name) {
                pages.push(make_page_info(name.clone(), path_obj));
            }
        }
    }

    Ok((pages, comic_info))
}

fn scan_rar(path: &Path) -> Result<(Vec<PageInfo>, Option<Vec<u8>>), DocumentError> {
    let file =
        File::open(path).map_err(|e| DocumentError::Io(format!("Не удалось открыть RAR: {e}")))?;
    let mut archive = unrar_rs::RarArchive::open(file)
        .map_err(|e| DocumentError::Corrupted(format!("Ошибка парсинга RAR: {e}")))?;

    let mut pages = Vec::new();
    let mut comic_info_entry_name = None;

    let entry_names: Vec<String> = archive
        .entries()
        .take(crate::document::MAX_ARCHIVE_ENTRIES + 1)
        .map(|e| e.name.clone())
        .collect();
    if entry_names.len() > crate::document::MAX_ARCHIVE_ENTRIES {
        return Err(DocumentError::LimitExceeded(
            "В RAR-архиве превышен лимит количества файлов (10 000)".to_string(),
        ));
    }

    for raw_name in entry_names {
        let name = raw_name.replace('\\', "/");
        if !is_safe_path(&name) {
            return Err(DocumentError::Corrupted(format!(
                "Обнаружен небезопасный путь в RAR-архиве: {name}"
            )));
        }

        let path_obj = Path::new(&name);
        if is_comic_info_xml(path_obj) {
            comic_info_entry_name = Some(raw_name);
        } else if is_supported_image(&name) {
            pages.push(make_page_info(raw_name, path_obj));
        }
    }

    let mut comic_info = None;
    if let Some(ci_name) = comic_info_entry_name
        && let Ok(handle) = archive.by_name(&ci_name)
    {
        let limit = 5 * 1024 * 1024;
        let mut total = 0u64;
        if let Ok(bytes) = crate::document::read_bounded(handle, limit, &mut total, limit) {
            comic_info = Some(bytes);
        }
    }

    Ok((pages, comic_info))
}

fn scan_sevenz(
    path: &Path,
) -> Result<(Vec<PageInfo>, Option<Vec<u8>>, Option<SevenZCache>), DocumentError> {
    let mut pages = Vec::new();
    let mut comic_info = None;
    let mut entry_count = 0usize;
    let mut total_unpacked = 0u64;
    let mut sec_err: Option<DocumentError> = None;
    let mut cache = SevenZCache::new()?;

    sevenz_rust::decompress_file_with_extract_fn(path, "", |entry, reader, _| {
        if sec_err.is_some() {
            return Ok(false);
        }
        entry_count += 1;
        if entry_count > crate::document::MAX_ARCHIVE_ENTRIES {
            sec_err = Some(DocumentError::LimitExceeded(
                "В 7z-архиве превышен лимит количества файлов (10 000)".to_string(),
            ));
            return Ok(false);
        }
        let name = entry.name().replace('\\', "/");
        if !is_safe_path(&name) {
            sec_err = Some(DocumentError::Corrupted(format!(
                "Обнаружен небезопасный путь в 7z-архиве: {name}"
            )));
            return Ok(false);
        }
        if entry.size() > crate::document::MAX_SINGLE_ENTRY_BYTES {
            sec_err = Some(DocumentError::LimitExceeded(format!(
                "Файл {name} превышает лимит 50 МБ"
            )));
            return Ok(false);
        }
        total_unpacked = total_unpacked.saturating_add(entry.size());
        if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
            sec_err = Some(DocumentError::LimitExceeded(
                "Превышен общий лимит распаковки 7z-архива (500 МБ)".to_string(),
            ));
            return Ok(false);
        }

        let path_obj = Path::new(&name);
        if is_comic_info_xml(path_obj) {
            let mut total_xml = 0u64;
            let limit = 5 * 1024 * 1024;
            if let Ok(b) = crate::document::read_bounded(reader, limit, &mut total_xml, limit) {
                comic_info = Some(b);
            }
        } else if is_supported_image(&name) {
            let mut total_page = 0u64;
            let bytes = match crate::document::read_bounded(
                reader,
                crate::document::MAX_SINGLE_ENTRY_BYTES,
                &mut total_page,
                crate::document::MAX_SINGLE_ENTRY_BYTES,
            ) {
                Ok(bytes) => bytes,
                Err(e) => {
                    sec_err = Some(e);
                    return Ok(false);
                }
            };
            let cache_name = format!("page_{}.img", cache.files.len());
            let cache_path = cache.dir.join(cache_name);
            if let Err(e) = std::fs::write(&cache_path, bytes) {
                sec_err = Some(DocumentError::Io(format!(
                    "Не удалось сохранить страницу CB7: {e}"
                )));
                return Ok(false);
            }
            cache.files.insert(entry.name().to_string(), cache_path);
            pages.push(make_page_info(entry.name().to_string(), path_obj));
        }
        Ok(true)
    })
    .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения 7z архива: {e}")))?;

    if let Some(err) = sec_err {
        return Err(err);
    }
    Ok((pages, comic_info, Some(cache)))
}

fn build_comic_chapters(
    pages: &[PageInfo],
    default_title: &str,
) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut groups: Vec<(String, usize, usize)> = Vec::new();
    for (idx, page) in pages.iter().enumerate() {
        let name = page
            .chapter_title
            .clone()
            .unwrap_or_else(|| default_title.to_string());
        if let Some(last) = groups.last_mut()
            && last.0 == name
        {
            last.2 += 1;
        } else {
            groups.push((name, idx, 1));
        }
    }

    if groups.is_empty() {
        let ch = BookChapter {
            id: None,
            title: default_title.to_string(),
            paragraphs: vec![format!("Всего страниц: {}", pages.len())],
        };
        let t = TocItem {
            id: None,
            title: default_title.to_string(),
            chapter_index: 0,
            children: Vec::new(),
        };
        return (vec![ch], vec![t]);
    }

    let mut chapters = Vec::with_capacity(groups.len());
    let mut toc = Vec::with_capacity(groups.len());
    for (group_title, start, count) in groups {
        let id = Some(format!("ch_{start}"));
        let desc = format!("Страницы {}–{} (всего {count})", start + 1, start + count);
        chapters.push(BookChapter {
            id: id.clone(),
            title: group_title.clone(),
            paragraphs: vec![desc],
        });
        toc.push(TocItem {
            id,
            title: group_title,
            chapter_index: start,
            children: Vec::new(),
        });
    }
    (chapters, toc)
}

struct ParsedComicInfo {
    title: Option<String>,
    writer: Option<String>,
    is_manga: Option<bool>,
}

fn parse_comic_info(bytes: &[u8]) -> Result<ParsedComicInfo, String> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let (mut title, mut series, mut number, mut writer, mut manga_tag) =
        (None, None, None, None, None);
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                current_tag = e.name().as_ref().to_ascii_lowercase();
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                let raw = e.as_ref();
                let t = quick_xml::escape::unescape(raw)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                match current_tag.as_str() {
                    "title" => title = Some(t),
                    "series" => series = Some(t),
                    "number" => number = Some(t),
                    "writer" => writer = Some(t),
                    "manga" => manga_tag = Some(t),
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::End(_)) => current_tag.clear(),
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(format!("Ошибка парсинга ComicInfo.xml: {e}")),
            _ => {}
        }
        buf.clear();
    }

    let full_title = match (series, number, title) {
        (Some(s), Some(n), Some(t)) if !t.is_empty() => Some(format!("{s} #{n}: {t}")),
        (Some(s), Some(n), _) => Some(format!("{s} #{n}")),
        (Some(s), None, Some(t)) if !t.is_empty() => Some(format!("{s}: {t}")),
        (Some(s), None, None) => Some(s),
        (None, _, Some(t)) => Some(t),
        _ => None,
    };
    let is_manga = manga_tag.map(|m| {
        let ml = m.to_ascii_lowercase();
        ml.contains("yes") || ml.contains("righttoleft")
    });
    Ok(ParsedComicInfo {
        title: full_title,
        writer,
        is_manga,
    })
}

fn consume_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u64 {
    let mut num: u64 = 0;
    while let Some(&d) = chars.peek() {
        if let Some(digit) = d.to_digit(10) {
            num = num.saturating_mul(10).saturating_add(digit as u64);
            chars.next();
        } else {
            break;
        }
    }
    num
}

pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let cmp = consume_number(&mut a_chars).cmp(&consume_number(&mut b_chars));
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                } else if ca.is_ascii_digit() {
                    return std::cmp::Ordering::Less;
                } else if cb.is_ascii_digit() {
                    return std::cmp::Ordering::Greater;
                } else {
                    let al = ca.to_lowercase().next().unwrap_or(ca);
                    let bl = cb.to_lowercase().next().unwrap_or(cb);
                    let cmp = al.cmp(&bl);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
        }
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
