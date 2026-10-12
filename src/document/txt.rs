use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use std::path::Path;

/// Бэкенд для потоковых текстовых форматов: TXT, Markdown (MD), LOG
pub struct TxtDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for TxtDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for TxtDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

impl Document for TxtDocument {
    fn metadata(&self) -> &BookMetadata {
        &self.metadata
    }

    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }

    fn toc(&self) -> &[TocItem] {
        &self.toc
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
    }

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        Some(self)
    }
}

impl TxtDocument {
    pub const MAX_TXT_SIZE: u64 = 100 * 1024 * 1024;

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref).map_err(|e| {
            DocumentError::Io(format!(
                "Не удалось прочитать метаданные текстового файла: {e}"
            ))
        })?;
        if meta.len() > Self::MAX_TXT_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер текстового файла превышает допустимый лимит (100 МБ)".to_string(),
            ));
        }

        let bytes = std::fs::read(path_ref)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать текстовый файл: {e}")))?;

        let default_title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Текстовый документ")
            .to_string();

        let ext = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        Self::from_bytes(&bytes, &default_title, &ext)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str, ext: &str) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > Self::MAX_TXT_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер текстовых данных превышает допустимый лимит (100 МБ)".to_string(),
            ));
        }

        // Проверка на бинарный мусор (нулевые байты в первых 4 КБ, если это не UTF-16)
        let sample = &bytes[..bytes.len().min(4096)];
        let is_utf16 = bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
        if !is_utf16 && sample.contains(&0) {
            return Err(DocumentError::Corrupted(
                "Файл содержит бинарные данные и не является текстовым документом".to_string(),
            ));
        }

        let text = decode_text_bytes(bytes);
        let is_markdown = ext == "md" || ext == "markdown";
        let is_log = ext == "log";

        let (chapters, toc) = if is_markdown {
            parse_markdown_chapters(&text, default_title)
        } else if is_log {
            parse_log_chapters(&text, default_title)
        } else {
            parse_plain_text_chapters(&text, default_title)
        };

        let title = toc
            .first()
            .map(|t| t.title.clone())
            .filter(|t| is_markdown && !t.is_empty())
            .unwrap_or_else(|| default_title.to_string());

        Ok(Self {
            metadata: BookMetadata {
                title,
                author: String::new(),
            },
            chapters,
            toc,
        })
    }
}

fn decode_text_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, _) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
        return cow.into_owned();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
        return cow.into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let (cow, _, _) = encoding_rs::WINDOWS_1251.decode(bytes);
    cow.into_owned()
}

fn parse_markdown_chapters(text: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    let mut current_title = String::new();
    let mut current_paragraphs = Vec::new();
    let mut current_p = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed
            .strip_prefix("# ")
            .or_else(|| trimmed.strip_prefix("## "))
            .or_else(|| trimmed.strip_prefix("### "))
        {
            if !current_p.trim().is_empty() {
                current_paragraphs.push(current_p.trim().to_string());
                current_p.clear();
            }
            if !current_title.is_empty() || !current_paragraphs.is_empty() {
                let idx = chapters.len();
                let ch_title = if current_title.is_empty() {
                    default_title.to_string()
                } else {
                    current_title.clone()
                };
                chapters.push(BookChapter {
                    id: Some(format!("md_{idx}")),
                    title: ch_title.clone(),
                    paragraphs: std::mem::take(&mut current_paragraphs),
                });
                toc.push(TocItem {
                    id: Some(format!("md_{idx}")),
                    title: ch_title,
                    chapter_index: idx,
                    children: Vec::new(),
                });
            }
            current_title = heading.trim().to_string();
        } else if trimmed.is_empty() {
            if !current_p.trim().is_empty() {
                current_paragraphs.push(current_p.trim().to_string());
                current_p.clear();
            }
        } else {
            if !current_p.is_empty() {
                current_p.push(' ');
            }
            current_p.push_str(trimmed);
        }
    }

    if !current_p.trim().is_empty() {
        current_paragraphs.push(current_p.trim().to_string());
    }
    if !current_title.is_empty() || !current_paragraphs.is_empty() || chapters.is_empty() {
        let idx = chapters.len();
        let ch_title = if current_title.is_empty() {
            default_title.to_string()
        } else {
            current_title
        };
        chapters.push(BookChapter {
            id: Some(format!("md_{idx}")),
            title: ch_title.clone(),
            paragraphs: current_paragraphs,
        });
        toc.push(TocItem {
            id: Some(format!("md_{idx}")),
            title: ch_title,
            chapter_index: idx,
            children: Vec::new(),
        });
    }

    (chapters, toc)
}

fn parse_log_chapters(text: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    const LINES_PER_CHUNK: usize = 300;
    let lines: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();

    if lines.is_empty() {
        return (
            vec![BookChapter {
                id: Some("log_0".to_string()),
                title: default_title.to_string(),
                paragraphs: vec![String::new()],
            }],
            vec![TocItem {
                id: Some("log_0".to_string()),
                title: default_title.to_string(),
                chapter_index: 0,
                children: Vec::new(),
            }],
        );
    }

    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    for (idx, chunk) in lines.chunks(LINES_PER_CHUNK).enumerate() {
        let start_line = idx * LINES_PER_CHUNK + 1;
        let end_line = start_line + chunk.len() - 1;
        let title = if lines.len() <= LINES_PER_CHUNK {
            default_title.to_string()
        } else {
            format!("{default_title} (строки {start_line}–{end_line})")
        };
        chapters.push(BookChapter {
            id: Some(format!("log_{idx}")),
            title: title.clone(),
            paragraphs: chunk.to_vec(),
        });
        toc.push(TocItem {
            id: Some(format!("log_{idx}")),
            title,
            chapter_index: idx,
            children: Vec::new(),
        });
    }
    (chapters, toc)
}

fn parse_plain_text_chapters(text: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut paragraphs = Vec::new();
    let mut current_p = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_p.is_empty() {
                paragraphs.push(std::mem::take(&mut current_p));
            }
        } else {
            if !current_p.is_empty() {
                current_p.push(' ');
            }
            current_p.push_str(trimmed);
        }
    }
    if !current_p.is_empty() {
        paragraphs.push(current_p);
    }

    const PARAGRAPHS_PER_CHAPTER: usize = 250;
    if paragraphs.is_empty() {
        paragraphs.push(String::new());
    }

    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    for (idx, chunk) in paragraphs.chunks(PARAGRAPHS_PER_CHAPTER).enumerate() {
        let title = if paragraphs.len() <= PARAGRAPHS_PER_CHAPTER {
            default_title.to_string()
        } else {
            format!("Раздел {}", idx + 1)
        };
        chapters.push(BookChapter {
            id: Some(format!("txt_{idx}")),
            title: title.clone(),
            paragraphs: chunk.to_vec(),
        });
        toc.push(TocItem {
            id: Some(format!("txt_{idx}")),
            title,
            chapter_index: idx,
            children: Vec::new(),
        });
    }
    (chapters, toc)
}
