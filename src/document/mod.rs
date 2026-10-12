pub mod comic;
pub mod epub;
pub mod fb2;
pub mod pdf;
pub mod registry;
pub mod txt;

pub use comic::ComicDocument;
pub use epub::EpubDocument;
pub use fb2::Fb2Document;
pub use pdf::PdfDocument;
#[allow(unused_imports)]
pub use registry::{DocumentFormat, FORMAT_REGISTRY};
pub use txt::TxtDocument;

use std::path::Path;

/// Типизированная ошибка работы с документами (без привязки к UI)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    Io(String),
    Unsupported(String),
    Corrupted(String),
    LimitExceeded(String),
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg)
            | Self::Unsupported(msg)
            | Self::Corrupted(msg)
            | Self::LimitExceeded(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for DocumentError {}

impl From<std::io::Error> for DocumentError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Декодированное RGBA-изображение страницы (не зависит от GUI-фреймворка)
#[derive(Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Информация об отдельной странице для постраничных документов
#[derive(Debug, Clone)]
pub struct PageInfo {
    /// Относительный путь внутри архива или папки
    pub archive_path: String,
    /// Имя файла или страницы для отображения
    pub display_name: String,
    /// Название главы/раздела (если есть)
    pub chapter_title: Option<String>,
}

/// Опциональный интерфейс для постраничных и фиксированных документов (PDF, комиксы, манга)
pub trait PageContent: Send + Sync {
    /// Общее количество страниц
    fn page_count(&self) -> usize;
    /// Имя или заголовок страницы по индексу
    fn page_name(&self, index: usize) -> &str;
    /// Детальная информация о странице
    #[allow(dead_code)]
    fn page_info(&self, index: usize) -> Option<&PageInfo> {
        let _ = index;
        None
    }
    /// Декодирование или растеризация изображения страницы
    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError>;
    /// Является ли документ мангой (направление чтения справа-налево по умолчанию)
    fn is_manga(&self) -> bool {
        false
    }
}

/// Опциональный интерфейс для потоковых текстовых документов
#[allow(dead_code)]
pub trait TextContent: Send + Sync {
    fn chapters(&self) -> &[BookChapter];
}

/// Опциональный интерфейс для документов с оглавлением
#[allow(dead_code)]
pub trait HasToc: Send + Sync {
    fn toc(&self) -> &[TocItem];
}

/// Открывает документ любого поддерживаемого формата через центральный реестр
pub fn open_document<P: AsRef<Path>>(path: P) -> Result<Box<dyn Document>, DocumentError> {
    registry::open_with_registry(path.as_ref())
}

#[derive(Debug, Clone)]
pub struct BookMetadata {
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct BookChapter {
    pub id: Option<String>,
    pub title: String,
    pub paragraphs: Vec<String>,
}

impl BookChapter {
    /// Проверяет, пуст ли раздел (нет заголовка и нет непустых абзацев текста)
    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty() && self.paragraphs.iter().all(|p| p.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocItem {
    pub id: Option<String>,
    pub title: String,
    pub chapter_index: usize,
    pub children: Vec<TocItem>,
}

/// Базовый контракт документа с поддержкой запроса опциональных возможностей (ISP)
pub trait Document: Send + Sync {
    fn metadata(&self) -> &BookMetadata;
    fn chapters(&self) -> &[BookChapter];
    fn toc(&self) -> &[TocItem] {
        &[]
    }
    /// Запрос возможности постраничного отображения (PageContent)
    fn page_content(&self) -> Option<&dyn PageContent> {
        None
    }
    /// Запрос возможности потокового текста (TextContent)
    #[allow(dead_code)]
    fn text_content(&self) -> Option<&dyn TextContent> {
        None
    }
    /// Запрос возможности оглавления (HasToc)
    #[allow(dead_code)]
    fn toc_capability(&self) -> Option<&dyn HasToc> {
        None
    }
}

pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;
pub const MAX_SINGLE_ENTRY_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_TOTAL_UNPACK_BYTES: u64 = 500 * 1024 * 1024;

/// Проверяет безопасность пути внутри архива (защита от Path Traversal / Zip Slip)
pub fn is_safe_archive_path(p: &str) -> bool {
    if p.is_empty() || p.starts_with('/') || p.starts_with('\\') || p.contains(':') {
        return false;
    }
    let normalized = p.replace('\\', "/");
    for seg in normalized.split('/') {
        if seg == ".." {
            return false;
        }
    }
    let path = Path::new(&normalized);
    !path.is_absolute()
        && !path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

/// Безопасное чтение из потока с ограничением на размер одной записи и общий объём распаковки
pub fn read_bounded<R: std::io::Read>(
    reader: R,
    max_entry_bytes: u64,
    total_unpacked: &mut u64,
    max_total_bytes: u64,
) -> Result<Vec<u8>, DocumentError> {
    use std::io::Read;
    let mut buf = Vec::new();
    let n = reader
        .take(max_entry_bytes.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|e| DocumentError::Io(format!("Ошибка чтения из архива: {e}")))?
        as u64;
    if n > max_entry_bytes {
        return Err(DocumentError::LimitExceeded(format!(
            "Превышен лимит размера файла в архиве ({max_entry_bytes} байт)"
        )));
    }
    *total_unpacked = total_unpacked.saturating_add(n);
    if *total_unpacked > max_total_bytes {
        return Err(DocumentError::LimitExceeded(format!(
            "Превышен общий лимит распаковки архива ({max_total_bytes} байт)"
        )));
    }
    Ok(buf)
}
