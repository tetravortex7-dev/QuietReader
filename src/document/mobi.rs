use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use mobi::headers::Encryption;
use mobi::Mobi;
use std::path::Path;

pub struct MobiDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    format_name: &'static str,
}

impl MobiDocument {
    pub const MAX_MOBI_SIZE: u64 = 150 * 1024 * 1024;

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let p = path.as_ref();
        let meta = std::fs::metadata(p)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать файл книги: {e}")))?;
        if meta.len() > Self::MAX_MOBI_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер файла книги превышает лимит (150 МБ)".to_string(),
            ));
        }

        let m = Mobi::from_path(p).map_err(|e| DocumentError::Corrupted(e.to_string()))?;

        if m.encryption() != Encryption::No {
            return Err(DocumentError::Unsupported(
                "Файл защищён DRM шифрованием Amazon".to_string(),
            ));
        }

        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        let format_name = match ext.as_str() {
            "azw3" => "AZW3",
            "azw" => "AZW",
            "prc" => "PRC",
            _ => "MOBI",
        };

        let raw_title = m.title();
        let title = if raw_title.trim().is_empty() {
            p.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Kindle Document")
                .to_string()
        } else {
            raw_title.trim().to_string()
        };
        let author = m.author().unwrap_or_default().trim().to_string();
        let metadata = BookMetadata { title, author };

        let content = if let Some(kf8_text) = try_extract_kf8_boundary_text(&m) {
            kf8_text
        } else {
            m.content_as_string()
                .unwrap_or_else(|_| m.content_as_string_lossy())
        };

        if content.trim().is_empty() {
            return Err(DocumentError::Corrupted(
                "Текстовое содержимое книги пусто или повреждено".to_string(),
            ));
        }

        let (chapters, toc) = parse_mobi_html(&content, &metadata.title);
        if chapters.is_empty() {
            return Err(DocumentError::Corrupted(
                "Не удалось извлечь главы из книги".to_string(),
            ));
        }

        Ok(Self {
            metadata,
            chapters,
            toc,
            format_name,
        })
    }
}

pub fn decompress_palmdoc(data: &[u8]) -> Vec<u8> {
    let init_cap = data.len().saturating_mul(2).min(16 * 1024 * 1024);
    let mut out = Vec::with_capacity(init_cap);
    let mut i = 0;

    while i < data.len() {
        let byte = data[i];
        i += 1;

        match byte {
            0x00 | 0x09..=0x7F => {
                out.push(byte);
            }
            0x01..=0x08 => {
                let count = byte as usize;
                let end = (i + count).min(data.len());
                out.extend_from_slice(&data[i..end]);
                i += count;
            }
            0x80..=0xBF => {
                if i < data.len() {
                    let next = data[i];
                    i += 1;
                    let b1 = byte as usize;
                    let b2 = next as usize;
                    let distance = ((b1 & 0x3F) << 5) | (b2 >> 3);
                    let length = (b2 & 0x07) + 3;

                    if distance > 0 && distance <= out.len() {
                        let start = out.len() - distance;
                        for j in 0..length {
                            let val = out[start + (j % distance)];
                            out.push(val);
                        }
                    } else {
                        out.push(byte);
                        out.push(next);
                    }
                } else {
                    out.push(byte);
                }
            }
            0xC0..=0xFF => {
                out.push(b' ');
                out.push(byte ^ 0x80);
            }
        }
    }

    out
}

fn try_extract_kf8_boundary_text(m: &Mobi) -> Option<String> {
    let raw_records = m.raw_records();
    let records = raw_records.records();

    let boundary_idx = records
        .iter()
        .position(|r| r.content == b"BOUNDARY" || r.content.starts_with(b"BOUNDARY"))?;

    let header_idx = boundary_idx + 1;
    if header_idx >= records.len() {
        return None;
    }

    let header_rec = records[header_idx].content;
    if header_rec.len() < 16 {
        return None;
    }

    let compression = u16::from_be_bytes([header_rec[0], header_rec[1]]);
    let text_count = u16::from_be_bytes([header_rec[8], header_rec[9]]) as usize;
    if text_count == 0 {
        return None;
    }

    let start_rec = header_idx + 1;
    let end_rec = (start_rec + text_count).min(records.len());
    let mut decompressed = Vec::new();

    for r in &records[start_rec..end_rec] {
        if compression == 2 {
            decompressed.extend_from_slice(&decompress_palmdoc(r.content));
        } else if compression == 1 {
            decompressed.extend_from_slice(r.content);
        }
    }

    if decompressed.is_empty() {
        return None;
    }

    match String::from_utf8(decompressed) {
        Ok(s) => Some(s),
        Err(e) => {
            let bytes = e.into_bytes();
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
            Some(cow.into_owned())
        }
    }
}

fn split_by_pagebreaks(html: &str) -> Vec<&str> {
    let mut sections = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut last_pos = 0;
    let mut search_pos = 0;

    while search_pos < lower.len() {
        let tag_start = if let Some(p1) = lower[search_pos..].find("<mbp:pagebreak") {
            Some(search_pos + p1)
        } else if let Some(p2) = lower[search_pos..].find("<pagebreak") {
            Some(search_pos + p2)
        } else {
            None
        };

        if let Some(start) = tag_start {
            if let Some(end_rel) = lower[start..].find('>') {
                let end = start + end_rel + 1;
                if start > last_pos {
                    let chunk = html[last_pos..start].trim();
                    if !chunk.is_empty() {
                        sections.push(chunk);
                    }
                }
                last_pos = end;
                search_pos = end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if last_pos < html.len() {
        let tail = html[last_pos..].trim();
        if !tail.is_empty() {
            sections.push(tail);
        }
    }

    sections
}

fn split_by_headings(html: &str) -> Vec<&str> {
    let mut sections = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut indices = Vec::new();

    let mut pos = 0;
    while pos < lower.len() {
        let h1 = lower[pos..].find("<h1").map(|i| pos + i);
        let h2 = lower[pos..].find("<h2").map(|i| pos + i);
        let h3 = lower[pos..].find("<h3").map(|i| pos + i);

        let next = [h1, h2, h3].into_iter().flatten().min();
        if let Some(abs_idx) = next {
            indices.push(abs_idx);
            pos = abs_idx + 3;
        } else {
            break;
        }
    }

    if indices.len() >= 2 {
        let mut last = 0;
        for &idx in &indices {
            if idx > last {
                let chunk = html[last..idx].trim();
                if !chunk.is_empty() {
                    sections.push(chunk);
                }
            }
            last = idx;
        }
        if last < html.len() {
            let tail = html[last..].trim();
            if !tail.is_empty() {
                sections.push(tail);
            }
        }
    }

    sections
}

fn is_likely_chapter_heading(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 70 {
        return false;
    }
    let lower = trimmed.to_lowercase();
    let keywords = [
        "глава",
        "chapter",
        "часть",
        "part",
        "пролог",
        "prologue",
        "эпилог",
        "epilogue",
        "раздел",
        "section",
        "акт",
        "act",
        "введение",
        "introduction",
        "заключение",
        "conclusion",
    ];
    keywords.iter().any(|&k| lower.starts_with(k))
}

fn parse_mobi_html(raw_html: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut raw_sections = split_by_pagebreaks(raw_html);
    if raw_sections.len() <= 1 {
        let heading_sections = split_by_headings(raw_html);
        if heading_sections.len() > 1 {
            raw_sections = heading_sections;
        }
    }

    let mut chapters = Vec::new();
    let mut toc = Vec::new();

    if raw_sections.is_empty() {
        raw_sections.push(raw_html);
    }

    for raw_sec in raw_sections {
        let sanitized = crate::document::epub::html::sanitize_xml_entities(raw_sec);
        let (extracted_title, mut paragraphs) =
            crate::document::epub::html::extract_html_lenient(&sanitized);
        if paragraphs.is_empty() {
            continue;
        }

        let chapter_title = if !extracted_title.trim().is_empty() {
            extracted_title.trim().to_string()
        } else if paragraphs.len() > 1 && is_likely_chapter_heading(&paragraphs[0]) {
            paragraphs.remove(0).trim().to_string()
        } else if paragraphs.len() > 1
            && paragraphs[0].chars().count() <= 50
            && !paragraphs[0].ends_with('.')
            && !paragraphs[0].ends_with(';')
        {
            paragraphs.remove(0).trim().to_string()
        } else if let Some(first) = paragraphs.first() {
            if is_likely_chapter_heading(first) {
                first.clone()
            } else {
                format!("Глава {}", chapters.len() + 1)
            }
        } else {
            format!("Глава {}", chapters.len() + 1)
        };

        if paragraphs.is_empty() {
            continue;
        }

        if paragraphs.len() > 150 {
            let chunks: Vec<_> = paragraphs.chunks(100).map(|c| c.to_vec()).collect();
            for (sub_idx, chunk) in chunks.into_iter().enumerate() {
                let sub_title = if sub_idx == 0 {
                    chapter_title.clone()
                } else {
                    format!("{chapter_title} (часть {})", sub_idx + 1)
                };
                let ch_idx = chapters.len();
                chapters.push(BookChapter {
                    id: None,
                    title: sub_title.clone(),
                    paragraphs: chunk,
                });
                toc.push(TocItem {
                    id: None,
                    title: sub_title,
                    chapter_index: ch_idx,
                    children: Vec::new(),
                });
            }
        } else {
            let ch_idx = chapters.len();
            chapters.push(BookChapter {
                id: None,
                title: chapter_title.clone(),
                paragraphs,
            });
            toc.push(TocItem {
                id: None,
                title: chapter_title,
                chapter_index: ch_idx,
                children: Vec::new(),
            });
        }
    }

    if chapters.is_empty() {
        let sanitized = crate::document::epub::html::sanitize_xml_entities(raw_html);
        let (extracted_title, paragraphs) =
            crate::document::epub::html::extract_html_lenient(&sanitized);
        if !paragraphs.is_empty() {
            let title = if !extracted_title.trim().is_empty() {
                extracted_title.trim().to_string()
            } else {
                default_title.to_string()
            };
            chapters.push(BookChapter {
                id: None,
                title: title.clone(),
                paragraphs,
            });
            toc.push(TocItem {
                id: None,
                title,
                chapter_index: 0,
                children: Vec::new(),
            });
        }
    }

    (chapters, toc)
}

impl TextContent for MobiDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for MobiDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

impl Document for MobiDocument {
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
        self.format_name
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
    }

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompress_palmdoc() {
        let compressed = [
            b'H', b'e', b'l', b'l', b'o', 0x20, b'W', b'o', b'r', b'l', b'd',
            b'a' ^ 0x80,
        ];
        let decomp = decompress_palmdoc(&compressed);
        assert_eq!(String::from_utf8(decomp).unwrap(), "Hello World a");
    }

    #[test]
    fn test_split_by_pagebreaks() {
        let html = "Start<mbp:pagebreak/>Middle<MBP:PAGEBREAK>End";
        let parts = split_by_pagebreaks(html);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "Start");
        assert_eq!(parts[1], "Middle");
        assert_eq!(parts[2], "End");

        let html_no_break = "Just some text without breaks";
        let parts_none = split_by_pagebreaks(html_no_break);
        assert_eq!(parts_none.len(), 1);
        assert_eq!(parts_none[0], "Just some text without breaks");
    }

    #[test]
    fn test_split_by_headings() {
        let html = "Intro<h1>Chapter 1</h1>Text 1<h2>Chapter 2</h2>Text 2";
        let parts = split_by_headings(html);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "Intro");
        assert!(parts[1].starts_with("<h1>Chapter 1</h1>"));
        assert!(parts[2].starts_with("<h2>Chapter 2</h2>"));

        let single_h1 = "Only one <h1>Title</h1> in document";
        let parts_single = split_by_headings(single_h1);
        assert!(parts_single.is_empty());
    }

    #[test]
    fn test_is_likely_chapter_heading() {
        assert!(is_likely_chapter_heading("Глава 1: Начало"));
        assert!(is_likely_chapter_heading("Chapter 42"));
        assert!(is_likely_chapter_heading("Часть II"));
        assert!(is_likely_chapter_heading("Пролог"));
        assert!(is_likely_chapter_heading("Epilogue"));
        assert!(is_likely_chapter_heading("Introduction"));

        assert!(!is_likely_chapter_heading(""));
        assert!(!is_likely_chapter_heading(
            "Это обычное длинное предложение, которое не является заголовком главы в книге."
        ));
    }

    #[test]
    fn test_parse_mobi_html_pagebreaks_and_formatting() {
        let html = r#"
            <p>Title Page</p>
            <mbp:pagebreak/>
            <h1>Chapter 1</h1>
            <p>First paragraph with <b>bold</b> and <i>italic</i> words &mdash; nice!</p>
            <mbp:pagebreak/>
            <h1>Chapter 2</h1>
            <p>Second chapter paragraph.</p>
        "#;
        let (chapters, toc) = parse_mobi_html(html, "Default Title");
        assert_eq!(chapters.len(), 3);
        assert_eq!(toc.len(), 3);

        assert_eq!(chapters[0].paragraphs, vec!["Title Page"]);
        assert!(chapters[1].paragraphs.iter().any(|p| p.contains("<b>bold</b>")));
        assert!(chapters[1].paragraphs.iter().any(|p| p.contains("<i>italic</i>")));
        assert!(chapters[1].paragraphs.iter().any(|p| p.contains('—')));
        assert_eq!(toc[1].chapter_index, 1);
        assert_eq!(toc[2].chapter_index, 2);
    }

    #[test]
    fn test_parse_mobi_html_large_chapter_chunking() {
        let mut html = String::from("<h1>Long Chapter</h1>");
        for i in 0..220 {
            html.push_str(&format!("<p>Paragraph {i} with some content.</p>"));
        }
        let (chapters, toc) = parse_mobi_html(&html, "Book");
        assert!(chapters.len() >= 2);
        assert_eq!(chapters.len(), toc.len());
        assert_eq!(toc[0].title, "Long Chapter");
        assert!(toc[1].title.contains("Long Chapter (часть 2)"));
    }

    #[test]
    fn test_mobi_corrupted_file_handling() {
        let temp_dir = std::env::temp_dir();
        let invalid_path = temp_dir.join("non_existent_book_12345.mobi");
        let res = MobiDocument::from_file(&invalid_path);
        assert!(matches!(res, Err(DocumentError::Io(_))));

        let garbage_path = temp_dir.join("corrupted_book_12345.mobi");
        let _ = std::fs::write(&garbage_path, b"not a mobi file garbage bytes");
        let res_garbage = MobiDocument::from_file(&garbage_path);
        let _ = std::fs::remove_file(&garbage_path);
        assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));
    }
}

