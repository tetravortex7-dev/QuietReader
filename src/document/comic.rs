use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use crate::document::{BookChapter, BookMetadata, Document, TocItem};

/// Декодированное RGBA-изображение страницы (не зависит от GUI-фреймворка)
#[derive(Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Специфичный интерфейс для документов типа комикс/манга
pub trait ComicDocumentTrait: Send + Sync {
    /// Общее количество страниц
    fn page_count(&self) -> usize;
    /// Имя или относительный путь страницы по индексу
    fn page_name(&self, index: usize) -> &str;
    /// Информация о странице
    #[allow(dead_code)]
    fn page_info(&self, index: usize) -> Option<&ComicPageInfo>;
    /// Декодирование изображения страницы
    fn load_page_image(&self, index: usize) -> Result<DecodedImage, String>;
    /// Является ли документ мангой (порядок чтения справа-налево)
    fn is_manga(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComicArchiveKind {
    Zip,
    Rar,
    SevenZ,
    Folder,
}

#[derive(Debug, Clone)]
pub struct ComicPageInfo {
    /// Относительный путь внутри архива или папки
    pub archive_path: String,
    /// Имя файла для отображения
    pub display_name: String,
    /// Название главы/подпапки (если есть)
    pub chapter_title: Option<String>,
}

/// Бэкенд для чтения комиксов и манги (CBZ, CBR, CB7, папки с изображениями)
pub struct ComicDocument {
    metadata: BookMetadata,
    path: PathBuf,
    kind: ComicArchiveKind,
    pages: Vec<ComicPageInfo>,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    is_manga: bool,
}

impl ComicDocument {
    /// Открывает комикс или мангу по пути к файлу или папке
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(format!("Файл или каталог не найден: {}", path_ref.display()));
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
                    // Пробуем определить по заголовкам файла
                    if let Ok(mut f) = File::open(path_ref) {
                        let mut magic = [0u8; 8];
                        if let Ok(n) = f.read(&mut magic) {
                            if n >= 4 && &magic[0..4] == b"PK\x03\x04" {
                                ComicArchiveKind::Zip
                            } else if n >= 7 && &magic[0..7] == b"Rar!\x1a\x07\x00" {
                                ComicArchiveKind::Rar
                            } else if n >= 6 && &magic[0..6] == b"7z\xbc\xaf\x27\x1c" {
                                ComicArchiveKind::SevenZ
                            } else {
                                return Err(format!("Неподдерживаемый формат комикса: {}", path_ref.display()));
                            }
                        } else {
                            return Err(format!("Не удалось прочитать файл: {}", path_ref.display()));
                        }
                    } else {
                        return Err(format!("Не удалось открыть файл: {}", path_ref.display()));
                    }
                }
            }
        };

        // Сканируем страницы архива или каталога
        let (mut pages, comic_info_xml) = match kind {
            ComicArchiveKind::Folder => scan_folder(path_ref)?,
            ComicArchiveKind::Zip => scan_zip(path_ref)?,
            ComicArchiveKind::Rar => scan_rar(path_ref)?,
            ComicArchiveKind::SevenZ => scan_sevenz(path_ref)?,
        };

        if pages.is_empty() {
            return Err("В архиве/папке не найдено ни одного поддерживаемого изображения (JPEG, PNG, WEBP, GIF)".to_string());
        }

        // Ограничение на максимальное количество страниц (защита от чрезмерного потребления ресурсов)
        const MAX_PAGES_LIMIT: usize = 5000;
        if pages.len() > MAX_PAGES_LIMIT {
            pages.truncate(MAX_PAGES_LIMIT);
        }

        // Естественная сортировка страниц
        pages.sort_by(|a, b| natural_cmp(&a.archive_path, &b.archive_path));

        // Извлечение метаданных
        let default_title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Комикс")
            .to_string();

        let mut title = default_title;
        let mut author = "Неизвестный автор".to_string();
        let mut is_manga = false;

        if let Some(xml_bytes) = comic_info_xml {
            if let Ok(info) = parse_comic_info(&xml_bytes) {
                if let Some(t) = info.title {
                    if !t.trim().is_empty() {
                        title = t;
                    }
                }
                if let Some(w) = info.writer {
                    if !w.trim().is_empty() {
                        author = w;
                    }
                }
                if let Some(m) = info.is_manga {
                    is_manga = m;
                }
            }
        }

        // Формирование структуры глав и оглавления
        let (chapters, toc) = build_comic_chapters(&pages, &title);

        Ok(Self {
            metadata: BookMetadata { title, author },
            path: path_ref.to_path_buf(),
            kind,
            pages,
            chapters,
            toc,
            is_manga,
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

    fn as_comic(&self) -> Option<&dyn ComicDocumentTrait> {
        Some(self)
    }
}

impl ComicDocumentTrait for ComicDocument {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_name(&self, index: usize) -> &str {
        if let Some(p) = self.pages.get(index) {
            &p.display_name
        } else {
            ""
        }
    }

    fn page_info(&self, index: usize) -> Option<&ComicPageInfo> {
        self.pages.get(index)
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, String> {
        let page = self
            .pages
            .get(index)
            .ok_or_else(|| format!("Страница {} выходит за пределы документа", index + 1))?;

        let bytes = match self.kind {
            ComicArchiveKind::Folder => {
                let full_path = self.path.join(&page.archive_path);
                std::fs::read(&full_path)
                    .map_err(|e| format!("Не удалось прочитать файл {}: {e}", full_path.display()))?
            }
            ComicArchiveKind::Zip => {
                let file = File::open(&self.path)
                    .map_err(|e| format!("Не удалось открыть архив {}: {e}", self.path.display()))?;
                let mut zip = ZipArchive::new(file)
                    .map_err(|e| format!("Ошибка открытия ZIP-архива: {e}"))?;
                let mut entry = zip
                    .by_name(&page.archive_path)
                    .map_err(|e| format!("Файл {} не найден в архиве: {e}", page.archive_path))?;

                // Защита от декомпрессионных бомб (макс. 50 МБ на изображение)
                const MAX_PAGE_BYTES: u64 = 50 * 1024 * 1024;
                if entry.size() > MAX_PAGE_BYTES {
                    return Err(format!("Файл страницы превышает допустимый лимит 50 МБ"));
                }

                let mut bytes = Vec::with_capacity(entry.size() as usize);
                entry
                    .read_to_end(&mut bytes)
                    .map_err(|e| format!("Ошибка извлечения страницы: {e}"))?;
                bytes
            }
            ComicArchiveKind::Rar => {
                let file = File::open(&self.path)
                    .map_err(|e| format!("Не удалось открыть архив {}: {e}", self.path.display()))?;
                let mut archive = unrar_rs::RarArchive::open(file)
                    .map_err(|e| format!("Ошибка открытия RAR-архива: {e}"))?;
                let mut bytes = Vec::new();
                archive
                    .by_name(&page.archive_path)
                    .map_err(|e| format!("Файл {} не найден в RAR-архиве: {e}", page.archive_path))?
                    .copy_to(&mut bytes)
                    .map_err(|e| format!("Ошибка извлечения страницы из RAR: {e}"))?;
                bytes
            }
            ComicArchiveKind::SevenZ => {
                let mut found_bytes = None;
                let target_path = &page.archive_path;
                sevenz_rust::decompress_file_with_extract_fn(&self.path, "", |entry, reader, _| {
                    if entry.name() == target_path {
                        let mut buf = Vec::new();
                        let _ = std::io::copy(reader, &mut buf);
                        found_bytes = Some(buf);
                        Ok(false) // Прерываем чтение, страница найдена
                    } else {
                        Ok(true)
                    }
                })
                .map_err(|e| format!("Ошибка чтения 7z архива: {e}"))?;

                found_bytes.ok_or_else(|| format!("Файл {} не найден в 7z-архиве", page.archive_path))?
            }
        };

        decode_image_bytes(&bytes)
    }

    fn is_manga(&self) -> bool {
        self.is_manga
    }
}

/// Декодирует байты изображения (JPEG, PNG, WEBP, GIF и др.) в RGBA буфер
fn decode_image_bytes(bytes: &[u8]) -> Result<DecodedImage, String> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Не удалось декодировать изображение: {e}"))?;
    let rgba = img.to_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

/// Проверяет, является ли файл поддерживаемым форматом изображения
pub fn is_supported_image(name: &str) -> bool {
    // Игнорируем скрытые и системные файлы
    let filename = Path::new(name).file_name().and_then(|f| f.to_str()).unwrap_or("");
    if filename.starts_with('.') || filename.eq_ignore_ascii_case("thumbs.db") || name.contains("__MACOSX") {
        return false;
    }

    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp")
}

/// Проверяет безопасность относительного пути (защита от Zip Slip)
fn is_safe_path(p: &str) -> bool {
    let path = Path::new(p);
    !path.is_absolute()
        && !path.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Сканирует локальную папку с изображениями
fn scan_folder(folder: &Path) -> Result<(Vec<ComicPageInfo>, Option<Vec<u8>>), String> {
    let mut pages = Vec::new();
    let mut comic_info = None;

    fn visit_dir(
        root: &Path,
        current: &Path,
        pages: &mut Vec<ComicPageInfo>,
        comic_info: &mut Option<Vec<u8>>,
    ) {
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dir(root, &path, pages, comic_info);
                } else if path.is_file() {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let rel_str = rel.to_string_lossy().replace('\\', "/");

                    if rel.file_name().map(|f| f.to_string_lossy().eq_ignore_ascii_case("comicinfo.xml")).unwrap_or(false) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            *comic_info = Some(bytes);
                        }
                    } else if is_supported_image(&rel_str) && is_safe_path(&rel_str) {
                        let display_name = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| rel_str.clone());
                        let chapter_title = rel.parent().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().to_string());
                        pages.push(ComicPageInfo {
                            archive_path: rel_str,
                            display_name,
                            chapter_title,
                        });
                    }
                }
            }
        }
    }

    visit_dir(folder, folder, &mut pages, &mut comic_info);
    Ok((pages, comic_info))
}

/// Сканирует ZIP-архив (.cbz)
fn scan_zip(path: &Path) -> Result<(Vec<ComicPageInfo>, Option<Vec<u8>>), String> {
    let file = File::open(path).map_err(|e| format!("Не удалось открыть ZIP-архив: {e}"))?;
    let mut zip = ZipArchive::new(file).map_err(|e| format!("Ошибка парсинга ZIP: {e}"))?;

    let mut pages = Vec::new();
    let mut comic_info = None;

    for i in 0..zip.len() {
        if let Ok(mut entry) = zip.by_index(i) {
            let name = entry.name().to_string();
            if !is_safe_path(&name) {
                continue;
            }

            if name.ends_with('/') || name.ends_with('\\') {
                continue;
            }

            let path_obj = Path::new(&name);
            if path_obj.file_name().map(|f| f.to_string_lossy().eq_ignore_ascii_case("comicinfo.xml")).unwrap_or(false) {
                let mut bytes = Vec::new();
                if entry.read_to_end(&mut bytes).is_ok() {
                    comic_info = Some(bytes);
                }
            } else if is_supported_image(&name) {
                let display_name = path_obj.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| name.clone());
                let chapter_title = path_obj.parent().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().to_string());
                pages.push(ComicPageInfo {
                    archive_path: name,
                    display_name,
                    chapter_title,
                });
            }
        }
    }

    Ok((pages, comic_info))
}

/// Сканирует RAR-архив (.cbr)
fn scan_rar(path: &Path) -> Result<(Vec<ComicPageInfo>, Option<Vec<u8>>), String> {
    let file = File::open(path).map_err(|e| format!("Не удалось открыть RAR-архив: {e}"))?;
    let mut archive = unrar_rs::RarArchive::open(file).map_err(|e| format!("Ошибка парсинга RAR: {e}"))?;

    let mut pages = Vec::new();
    let mut comic_info_entry_name = None;

    let entry_names: Vec<String> = archive.entries().map(|e| e.name.clone()).collect();

    for raw_name in entry_names {
        let name = raw_name.replace('\\', "/");
        if !is_safe_path(&name) {
            continue;
        }

        let path_obj = Path::new(&name);
        if path_obj.file_name().map(|f| f.to_string_lossy().eq_ignore_ascii_case("comicinfo.xml")).unwrap_or(false) {
            comic_info_entry_name = Some(raw_name);
        } else if is_supported_image(&name) {
            let display_name = path_obj.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| name.clone());
            let chapter_title = path_obj.parent().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().to_string());
            pages.push(ComicPageInfo {
                archive_path: raw_name,
                display_name,
                chapter_title,
            });
        }
    }

    let mut comic_info = None;
    if let Some(ci_name) = comic_info_entry_name {
        if let Ok(handle) = archive.by_name(&ci_name) {
            let mut bytes = Vec::new();
            if handle.copy_to(&mut bytes).is_ok() {
                comic_info = Some(bytes);
            }
        }
    }

    Ok((pages, comic_info))
}

/// Сканирует 7z-архив (.cb7)
fn scan_sevenz(path: &Path) -> Result<(Vec<ComicPageInfo>, Option<Vec<u8>>), String> {
    let mut pages = Vec::new();
    let mut comic_info = None;

    sevenz_rust::decompress_file_with_extract_fn(path, "", |entry, reader, _| {
        let name = entry.name().replace('\\', "/");
        if !is_safe_path(&name) {
            return Ok(true);
        }

        let path_obj = Path::new(&name);
        if path_obj.file_name().map(|f| f.to_string_lossy().eq_ignore_ascii_case("comicinfo.xml")).unwrap_or(false) {
            let mut bytes = Vec::new();
            let _ = std::io::copy(reader, &mut bytes);
            comic_info = Some(bytes);
        } else if is_supported_image(&name) {
            let display_name = path_obj.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| name.clone());
            let chapter_title = path_obj.parent().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().to_string());
            pages.push(ComicPageInfo {
                archive_path: entry.name().to_string(),
                display_name,
                chapter_title,
            });
        }
        Ok(true)
    })
    .map_err(|e| format!("Ошибка чтения 7z архива: {e}"))?;

    Ok((pages, comic_info))
}

/// Формирует разделы и оглавление на основе страниц и подкаталогов
fn build_comic_chapters(pages: &[ComicPageInfo], default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut chapters = Vec::new();
    let mut toc = Vec::new();

    // Проверяем, сгруппированы ли страницы по папкам-главам
    let mut groups: Vec<(String, usize, usize)> = Vec::new(); // (Название главы, индекс начала, количество)

    for (idx, page) in pages.iter().enumerate() {
        let chapter_name = page.chapter_title.clone().unwrap_or_else(|| default_title.to_string());
        if let Some(last) = groups.last_mut() {
            if last.0 == chapter_name {
                last.2 += 1;
                continue;
            }
        }
        groups.push((chapter_name, idx, 1));
    }

    if groups.is_empty() {
        chapters.push(BookChapter {
            id: None,
            title: default_title.to_string(),
            paragraphs: vec![format!("Всего страниц: {}", pages.len())],
        });
        toc.push(TocItem {
            id: None,
            title: default_title.to_string(),
            chapter_index: 0,
            children: Vec::new(),
        });
    } else {
        for (i, (group_title, start_page, count)) in groups.into_iter().enumerate() {
            chapters.push(BookChapter {
                id: Some(format!("ch_{start_page}")),
                title: group_title.clone(),
                paragraphs: vec![format!("Страницы {}–{} (всего {})", start_page + 1, start_page + count, count)],
            });
            toc.push(TocItem {
                id: Some(format!("ch_{start_page}")),
                title: group_title,
                chapter_index: i,
                children: Vec::new(),
            });
        }
    }

    (chapters, toc)
}

struct ParsedComicInfo {
    title: Option<String>,
    writer: Option<String>,
    is_manga: Option<bool>,
}

/// Быстрый парсер стандартного ComicInfo.xml
fn parse_comic_info(bytes: &[u8]) -> Result<ParsedComicInfo, String> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);

    let mut title = None;
    let mut series = None;
    let mut number = None;
    let mut writer = None;
    let mut manga_tag = None;

    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                current_tag = e.name().as_ref().to_ascii_lowercase();
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                match current_tag.as_str() {
                    "title" => title = Some(text),
                    "series" => series = Some(text),
                    "number" => number = Some(text),
                    "writer" => writer = Some(text),
                    "manga" => manga_tag = Some(text),
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::End(_)) => {
                current_tag.clear();
            }
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
        let m_lower = m.to_ascii_lowercase();
        m_lower.contains("yes") || m_lower.contains("righttoleft")
    });

    Ok(ParsedComicInfo {
        title: full_title,
        writer,
        is_manga,
    })
}

/// Естественное сравнение строк (Natural Sort Order)
/// Гарантирует правильный порядок: page1, page2, page10 (а не page1, page10, page2)
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
                    // Извлекаем числа целиком
                    let mut num_a: u64 = 0;
                    while let Some(&d) = a_chars.peek() {
                        if let Some(digit) = d.to_digit(10) {
                            num_a = num_a.saturating_mul(10).saturating_add(digit as u64);
                            a_chars.next();
                        } else {
                            break;
                        }
                    }
                    let mut num_b: u64 = 0;
                    while let Some(&d) = b_chars.peek() {
                        if let Some(digit) = d.to_digit(10) {
                            num_b = num_b.saturating_mul(10).saturating_add(digit as u64);
                            b_chars.next();
                        } else {
                            break;
                        }
                    }
                    match num_a.cmp(&num_b) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else if ca.is_ascii_digit() && !cb.is_ascii_digit() {
                    return std::cmp::Ordering::Less;
                } else if !ca.is_ascii_digit() && cb.is_ascii_digit() {
                    return std::cmp::Ordering::Greater;
                } else {
                    let a_lower = ca.to_lowercase().next().unwrap_or(ca);
                    let b_lower = cb.to_lowercase().next().unwrap_or(cb);
                    match a_lower.cmp(&b_lower) {
                        std::cmp::Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}
