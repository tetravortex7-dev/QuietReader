use crate::document::epub::html::parse_html_chapter_full;
use crate::document::txt::decode_text_bytes_with;
use crate::document::{BookChapter, BookMetadata, Document, DocumentError, TextContent, TocItem};
use std::path::Path;

pub struct HtmlDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for HtmlDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl Document for HtmlDocument {
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
        "HTML"
    }

    fn has_custom_toc(&self) -> bool {
        self.toc.len() > 1
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
    }
}

struct Heading {
    start: usize,
    content_start: usize,
    title: String,
}

impl HtmlDocument {
    pub const MAX_HTML_SIZE: u64 = 100 * 1024 * 1024;

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        Self::from_file_with_encoding(path, None)
    }

    pub fn from_file_with_encoding<P: AsRef<Path>>(
        path: P,
        encoding: Option<&str>,
    ) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref).map_err(|e| {
            DocumentError::Io(format!("Не удалось прочитать метаданные HTML-файла: {e}"))
        })?;
        if meta.len() > Self::MAX_HTML_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер HTML-файла превышает допустимый лимит (100 МБ)".to_string(),
            ));
        }
        let bytes = std::fs::read(path_ref)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать HTML-файл: {e}")))?;
        let default_title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("HTML-документ")
            .to_string();
        Self::from_bytes(&bytes, &default_title, encoding)
    }

    pub fn from_bytes(
        bytes: &[u8],
        default_title: &str,
        encoding: Option<&str>,
    ) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > Self::MAX_HTML_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер HTML-данных превышает допустимый лимит (100 МБ)".to_string(),
            ));
        }
        let text = decode_text_bytes_with(bytes, encoding);
        if text.trim().is_empty() {
            return Err(DocumentError::Corrupted(
                "HTML-документ не содержит текста".to_string(),
            ));
        }

        let headings = find_headings(&text);
        let document_title = find_title(&text)
            .or_else(|| headings.first().map(|heading| heading.title.clone()))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| default_title.to_string());
        let author = find_author(&text);

        let mut chapters = Vec::new();
        let mut toc = Vec::new();

        if headings.is_empty() {
            let parsed = parse_html_chapter_full(&text);
            if !parsed.paragraphs.is_empty() {
                let title = if parsed.title.trim().is_empty() {
                    document_title.clone()
                } else {
                    parsed.title.trim().to_string()
                };
                push_chapter(&mut chapters, &mut toc, title, parsed.paragraphs);
            }
        } else {
            let front = &text[..headings[0].start];
            let parsed_front = parse_html_chapter_full(front);
            if !parsed_front.paragraphs.is_empty() {
                let title = if parsed_front.title.trim().is_empty() {
                    document_title.clone()
                } else {
                    parsed_front.title.trim().to_string()
                };
                push_chapter(&mut chapters, &mut toc, title, parsed_front.paragraphs);
            }
            for (index, heading) in headings.iter().enumerate() {
                let end = headings
                    .get(index + 1)
                    .map(|next| next.start)
                    .unwrap_or(text.len());
                if heading.content_start >= end {
                    continue;
                }
                let chunk = &text[heading.start..end];
                let parsed = parse_html_chapter_full(chunk);
                if parsed.paragraphs.is_empty() {
                    continue;
                }
                push_chapter(
                    &mut chapters,
                    &mut toc,
                    heading.title.clone(),
                    parsed.paragraphs,
                );
            }
        }

        if chapters.is_empty() {
            return Err(DocumentError::Corrupted(
                "В HTML-документе не найдено читаемого текста".to_string(),
            ));
        }

        Ok(Self {
            metadata: BookMetadata {
                title: document_title,
                author,
            },
            chapters,
            toc,
        })
    }
}

fn push_chapter(
    chapters: &mut Vec<BookChapter>,
    toc: &mut Vec<TocItem>,
    title: String,
    paragraphs: Vec<String>,
) {
    let index = chapters.len();
    let id = Some(format!("html_{index}"));
    chapters.push(BookChapter {
        id: id.clone(),
        title: title.clone(),
        paragraphs,
    });
    toc.push(TocItem {
        id,
        title,
        chapter_index: index,
        children: Vec::new(),
    });
}

fn find_headings(html: &str) -> Vec<Heading> {
    let lower = html.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut headings = Vec::new();
    let mut position = 0usize;

    while position + 3 < bytes.len() {
        if bytes[position] != b'<' {
            position += 1;
            continue;
        }
        let open_len =
            if lower[position..].starts_with("<h1") || lower[position..].starts_with("<h2") {
                3
            } else {
                position += 1;
                continue;
            };
        let boundary = bytes.get(position + open_len).copied();
        if !matches!(
            boundary,
            Some(b'>') | Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'\t') | Some(b'/')
        ) {
            position += 1;
            continue;
        }
        let Some(open_end) = lower[position..].find('>') else {
            break;
        };
        let content_start = position + open_end + 1;
        let close = if &lower[position..position + 3] == "<h1" {
            "</h1>"
        } else {
            "</h2>"
        };
        let Some(close_offset) = lower[content_start..].find(close) else {
            break;
        };
        let content_end = content_start + close_offset;
        let end = content_end + close.len();
        let title = strip_tags(&html[content_start..content_end]);
        if !title.is_empty() {
            headings.push(Heading {
                start: position,
                content_start,
                title,
            });
        }
        position = end;
    }

    headings
}

fn find_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<title")?;
    let open_end = lower[start..].find('>')? + start + 1;
    let end = lower[open_end..].find("</title>")? + open_end;
    let title = strip_tags(&html[open_end..end]);
    if title.is_empty() { None } else { Some(title) }
}

fn find_author(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut position = 0usize;
    while let Some(found) = lower[position..].find("<meta") {
        let start = position + found;
        let Some(open_end) = lower[start..].find('>') else {
            break;
        };
        let tag = &lower[start..start + open_end];
        let raw_tag = &html[start..start + open_end];
        if tag.contains("name=\"author\"")
            || tag.contains("name='author'")
            || tag.contains("name=author")
        {
            if let Some(content) = attribute_value(raw_tag, tag, "content") {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        position = start + open_end + 1;
    }
    String::new()
}

fn attribute_value(raw: &str, lower: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=");
    let index = lower.find(&pattern)?;
    let rest = raw[index + pattern.len()..].trim_start();
    let quote = rest.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = rest[1..].find(quote)?;
        Some(rest[1..1 + end].to_string())
    } else {
        rest.split(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .next()
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }
}

fn strip_tags(fragment: &str) -> String {
    let mut text = String::with_capacity(fragment.len());
    let mut depth = 0usize;
    let mut chars = fragment.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            '&' => {
                let mut entity = String::new();
                while let Some(&next) = chars.peek() {
                    if next == ';' || entity.len() > 10 {
                        break;
                    }
                    entity.push(next);
                    chars.next();
                }
                if chars.peek() == Some(&';') {
                    chars.next();
                }
                text.push_str(&decode_entity(&entity));
            }
            _ => text.push(ch),
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entity(entity: &str) -> String {
    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
        && let Ok(code) = u32::from_str_radix(hex, 16)
        && let Some(ch) = char::from_u32(code)
    {
        return ch.to_string();
    }
    if let Some(decimal) = entity.strip_prefix('#')
        && let Ok(code) = decimal.parse::<u32>()
        && let Some(ch) = char::from_u32(code)
    {
        return ch.to_string();
    }
    match entity {
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" => " ".to_string(),
        "mdash" => "—".to_string(),
        "ndash" => "–".to_string(),
        "hellip" => "…".to_string(),
        "laquo" => "«".to_string(),
        "raquo" => "»".to_string(),
        "copy" => "©".to_string(),
        _ => String::new(),
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
