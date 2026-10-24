pub mod comic;
pub mod cover;
pub mod djvu;
pub mod doc;
pub mod docx;
pub mod epub;
pub mod fb2;
pub mod html;
pub mod mobi;
pub mod pdf;
pub mod registry;
pub mod rtf;
pub mod txt;

pub use comic::ComicDocument;
pub use djvu::DjvuDocument;
pub use doc::DocDocument;
pub use docx::DocxDocument;
pub use epub::EpubDocument;
pub use fb2::Fb2Document;
pub use html::HtmlDocument;
pub use mobi::MobiDocument;
pub use pdf::PdfDocument;
#[allow(unused_imports)]
pub use registry::{DocumentFormat, FORMAT_REGISTRY};
pub use rtf::RtfDocument;
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

#[derive(Debug, Clone, Copy)]
pub struct PageLink {
    pub bbox: [f32; 4],
    pub target_page: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteLocation {
    pub chapter_index: usize,
    pub paragraph_index: usize,
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
    fn page_links(&self, _index: usize) -> &[PageLink] {
        &[]
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
pub fn open_document<P: AsRef<Path>>(path: P) -> Result<Box<dyn Document>, DocumentError> {
    registry::open_with_registry(path.as_ref())
}

pub fn open_document_with_encoding<P: AsRef<Path>>(
    path: P,
    encoding: Option<&str>,
) -> Result<Box<dyn Document>, DocumentError> {
    registry::open_with_registry_and_encoding(path.as_ref(), encoding)
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

pub const NOTES_CHAPTER_ID: &str = "__quietreader_notes__";
pub const NOTES_CHAPTER_TITLE: &str = "Примечания";

pub fn push_notes_chapter(
    chapters: &mut Vec<BookChapter>,
    title: String,
    notes: &[(String, String)],
) -> std::collections::HashMap<String, NoteLocation> {
    if notes.is_empty() {
        return std::collections::HashMap::new();
    }
    let chapter_index = chapters.len();
    let mut locations = std::collections::HashMap::new();
    let mut paragraphs = Vec::with_capacity(notes.len());
    for (index, (id, text)) in notes.iter().enumerate() {
        paragraphs.push(format!("[{}] {}", index + 1, text));
        locations.insert(
            id.clone(),
            NoteLocation {
                chapter_index,
                paragraph_index: index,
            },
        );
    }
    chapters.push(BookChapter {
        id: Some(NOTES_CHAPTER_ID.to_string()),
        title,
        paragraphs,
    });
    locations
}

pub fn build_notes_appendix(
    chapters: &mut Vec<BookChapter>,
    notes: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, NoteLocation> {
    if notes.is_empty() {
        return std::collections::HashMap::new();
    }
    let mut ordered: Vec<String> = Vec::new();
    for chapter in chapters.iter() {
        for text in std::iter::once(chapter.title.as_str())
            .chain(chapter.paragraphs.iter().map(String::as_str))
        {
            for (id, _, _) in footnote_tokens(text) {
                if notes.contains_key(id) && !ordered.iter().any(|existing| existing == id) {
                    ordered.push(id.to_string());
                }
            }
        }
    }
    let mut remaining: Vec<String> = notes
        .keys()
        .filter(|id| !ordered.contains(id))
        .cloned()
        .collect();
    remaining.sort_by_key(|id| note_sort_key(id));
    ordered.extend(remaining);
    let ordered_notes: Vec<(String, String)> = ordered
        .into_iter()
        .filter_map(|id| notes.get(&id).map(|text| (id, text.clone())))
        .collect();
    push_notes_chapter(chapters, NOTES_CHAPTER_TITLE.to_string(), &ordered_notes)
}

pub fn locate_notes_in_chapters(
    chapters: &[BookChapter],
    notes: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, NoteLocation> {
    let mut locations = std::collections::HashMap::new();
    for (id, note) in notes {
        let normalized = normalize_match_text(note);
        let normalized = normalized
            .trim_start_matches(|c: char| {
                c.is_ascii_digit() || matches!(c, '[' | ']' | '(' | ')' | '.' | ':')
            })
            .trim_start();
        if normalized.chars().count() < 6 {
            continue;
        }
        let needle: String = normalized.chars().take(48).collect();
        for (chapter_index, chapter) in chapters.iter().enumerate() {
            if let Some(paragraph_index) = chapter
                .paragraphs
                .iter()
                .position(|paragraph| normalize_match_text(paragraph).contains(&needle))
            {
                locations.insert(
                    id.clone(),
                    NoteLocation {
                        chapter_index,
                        paragraph_index,
                    },
                );
                break;
            }
        }
    }
    locations
}

fn note_sort_key(id: &str) -> (String, u64) {
    let split = id.rfind('_').map(|index| index + 1).unwrap_or(0);
    let (prefix, suffix) = id.split_at(split);
    (prefix.to_string(), suffix.parse().unwrap_or(u64::MAX))
}

fn normalize_match_text(text: &str) -> String {
    strip_formatting_tags(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub const FOOTNOTE_TOKEN_PREFIX: &str = "{{note:";
pub const FOOTNOTE_TOKEN_SUFFIX: &str = "}}";
pub const IMAGE_TOKEN_PREFIX: &str = "{{image:";
pub const IMAGE_TOKEN_SUFFIX: &str = "}}";

pub fn footnote_token(id: &str) -> String {
    format!("{FOOTNOTE_TOKEN_PREFIX}{id}{FOOTNOTE_TOKEN_SUFFIX}")
}

pub fn footnote_tokens(text: &str) -> Vec<(&str, usize, usize)> {
    let mut tokens = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find(FOOTNOTE_TOKEN_PREFIX) {
        let start = search_from + relative_start;
        let id_start = start + FOOTNOTE_TOKEN_PREFIX.len();
        let Some(relative_end) = text[id_start..].find(FOOTNOTE_TOKEN_SUFFIX) else {
            break;
        };
        let id_end = id_start + relative_end;
        let end = id_end + FOOTNOTE_TOKEN_SUFFIX.len();
        tokens.push((&text[id_start..id_end], start, end));
        search_from = end;
    }
    tokens
}

pub fn image_token(id: &str) -> String {
    format!("{IMAGE_TOKEN_PREFIX}{id}{IMAGE_TOKEN_SUFFIX}")
}

pub fn image_token_id(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    let id = trimmed
        .strip_prefix(IMAGE_TOKEN_PREFIX)?
        .strip_suffix(IMAGE_TOKEN_SUFFIX)?
        .trim();
    if id.is_empty() { None } else { Some(id) }
}

pub fn image_tokens(text: &str) -> Vec<(&str, usize, usize)> {
    let mut tokens = Vec::new();
    let mut search_from = 0;
    while let Some(relative_start) = text[search_from..].find(IMAGE_TOKEN_PREFIX) {
        let start = search_from + relative_start;
        let id_start = start + IMAGE_TOKEN_PREFIX.len();
        let Some(relative_end) = text[id_start..].find(IMAGE_TOKEN_SUFFIX) else {
            break;
        };
        let id_end = id_start + relative_end;
        let end = id_end + IMAGE_TOKEN_SUFFIX.len();
        tokens.push((&text[id_start..id_end], start, end));
        search_from = end;
    }
    tokens
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocItem {
    pub id: Option<String>,
    pub title: String,
    pub chapter_index: usize,
    pub children: Vec<TocItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedSpan<'a> {
    pub text: &'a str,
    pub bold: bool,
    pub italic: bool,
}

pub fn strip_formatting_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len() {
        if b[i] == b'{'
            && (s[i..].starts_with(FOOTNOTE_TOKEN_PREFIX) || s[i..].starts_with(IMAGE_TOKEN_PREFIX))
            && let Some(end) = s[i..].find("}}")
        {
            i += end + 2;
            continue;
        }
        if b[i] == b'<' {
            if s[i..].starts_with("<b>") || s[i..].starts_with("<i>") {
                i += 3;
                continue;
            } else if s[i..].starts_with("</b>") || s[i..].starts_with("</i>") {
                i += 4;
                continue;
            }
        }
        let Some(ch) = s[i..].chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

pub fn parse_formatted_spans(text: &str) -> Vec<FormattedSpan<'_>> {
    let mut spans = Vec::new();
    let mut bold_depth: usize = 0;
    let mut italic_depth: usize = 0;
    let mut current_start = 0;
    let mut i = 0;
    let bytes = text.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'<' {
            let tag_info = if text[i..].starts_with("<b>") {
                Some((3, true, true))
            } else if text[i..].starts_with("</b>") {
                Some((4, true, false))
            } else if text[i..].starts_with("<i>") {
                Some((3, false, true))
            } else if text[i..].starts_with("</i>") {
                Some((4, false, false))
            } else {
                None
            };

            if let Some((len, is_bold, is_open)) = tag_info {
                if i > current_start {
                    spans.push(FormattedSpan {
                        text: &text[current_start..i],
                        bold: bold_depth > 0,
                        italic: italic_depth > 0,
                    });
                }
                if is_bold {
                    if is_open {
                        bold_depth = bold_depth.saturating_add(1);
                    } else {
                        bold_depth = bold_depth.saturating_sub(1);
                    }
                } else {
                    if is_open {
                        italic_depth = italic_depth.saturating_add(1);
                    } else {
                        italic_depth = italic_depth.saturating_sub(1);
                    }
                }
                i += len;
                current_start = i;
                continue;
            }
        }
        let ch_len = text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        i += ch_len;
    }

    if current_start < text.len() {
        spans.push(FormattedSpan {
            text: &text[current_start..],
            bold: bold_depth > 0,
            italic: italic_depth > 0,
        });
    }

    spans
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
        "Document"
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
    fn footnote(&self, _id: &str) -> Option<&str> {
        None
    }
    fn note_location(&self, _id: &str) -> Option<NoteLocation> {
        None
    }
    fn image_bytes(&self, _id: &str) -> Option<Vec<u8>> {
        None
    }
    fn header_text(&self) -> Option<&str> {
        None
    }
    fn footer_text(&self) -> Option<&str> {
        None
    }
}

pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;
pub const MAX_SINGLE_ENTRY_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_TOTAL_UNPACK_BYTES: u64 = 500 * 1024 * 1024;
pub const MAX_FOLDER_DEPTH: usize = 32;

pub fn is_safe_archive_path(p: &str) -> bool {
    let trimmed = p.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || trimmed.contains(':')
    {
        return false;
    }
    let decoded = epub::percent_decode_str(trimmed);
    let dec_trimmed = decoded.trim();
    if dec_trimmed.is_empty()
        || dec_trimmed.starts_with('/')
        || dec_trimmed.starts_with('\\')
        || dec_trimmed.contains(':')
    {
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

#[cfg(all(test, feature = "enable_tests"))]
mod test;
