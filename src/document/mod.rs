pub mod comic;
pub mod djvu;
pub mod epub;
pub mod fb2;
pub mod pdf;
pub mod registry;
pub mod txt;

pub use comic::ComicDocument;
pub use djvu::DjvuDocument;
pub use epub::EpubDocument;
pub use fb2::Fb2Document;
pub use pdf::PdfDocument;
#[allow(unused_imports)]
pub use registry::{DocumentFormat, FORMAT_REGISTRY};
pub use txt::TxtDocument;

use std::path::Path;

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

#[derive(Clone)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub archive_path: String,
    pub display_name: String,
    pub chapter_title: Option<String>,
}

pub trait PageContent: Send + Sync {
    fn page_count(&self) -> usize;
    fn page_name(&self, index: usize) -> &str;
    #[allow(dead_code)]
    fn page_info(&self, index: usize) -> Option<&PageInfo> {
        let _ = index;
        None
    }
    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError>;
    fn load_page_image_scaled(
        &self,
        index: usize,
        _scale: f32,
    ) -> Result<DecodedImage, DocumentError> {
        self.load_page_image(index)
    }
    fn extract_page_text(&self, _index: usize) -> Option<String> {
        None
    }
    fn extract_page_text_spans(&self, _index: usize) -> Option<Vec<PageTextSpan>> {
        None
    }
    fn is_manga(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
pub struct PageTextSpan {
    pub text: String,
    pub bbox: [f32; 4],
}

#[allow(dead_code)]
pub trait TextContent: Send + Sync {
    fn chapters(&self) -> &[BookChapter];
}

#[allow(dead_code)]
pub trait HasToc: Send + Sync {
    fn toc(&self) -> &[TocItem];
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentViewKind {
    Text,
    Pdf,
    Comic,
}

pub trait Document: Send + Sync {
    fn metadata(&self) -> &BookMetadata;
    fn chapters(&self) -> &[BookChapter];
    fn toc(&self) -> &[TocItem] {
        &[]
    }
    fn format_name(&self) -> &'static str {
        "Документ"
    }
    fn has_custom_toc(&self) -> bool {
        !self.toc().is_empty()
    }
    fn view_kind(&self) -> DocumentViewKind {
        DocumentViewKind::Text
    }
    fn page_content(&self) -> Option<&dyn PageContent> {
        None
    }
    #[allow(dead_code)]
    fn text_content(&self) -> Option<&dyn TextContent> {
        None
    }
    #[allow(dead_code)]
    fn toc_capability(&self) -> Option<&dyn HasToc> {
        None
    }
}

pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;
pub const MAX_SINGLE_ENTRY_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_TOTAL_UNPACK_BYTES: u64 = 500 * 1024 * 1024;

pub fn is_safe_archive_path(p: &str) -> bool {
    let trimmed = p.trim();
    if trimmed.is_empty() || trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.contains(':') {
        return false;
    }
    let decoded = epub::percent_decode_str(trimmed);
    let dec_trimmed = decoded.trim();
    if dec_trimmed.is_empty() || dec_trimmed.starts_with('/') || dec_trimmed.starts_with('\\') || dec_trimmed.contains(':') {
        return false;
    }
    let normalized = dec_trimmed.replace('\\', "/");
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
    let Some(new_total) = total_unpacked.checked_add(n) else {
        return Err(DocumentError::LimitExceeded(format!(
            "Превышен общий лимит распаковки архива ({max_total_bytes} байт)"
        )));
    };
    if new_total > max_total_bytes {
        return Err(DocumentError::LimitExceeded(format!(
            "Превышен общий лимит распаковки архива ({max_total_bytes} байт)"
        )));
    }
    *total_unpacked = new_total;
    Ok(buf)
}
