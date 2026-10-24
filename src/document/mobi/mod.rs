use crate::document::{BookChapter, BookMetadata, Document, DocumentError, TextContent, TocItem};
use mobi::Mobi;
use std::collections::HashMap;
use std::path::Path;

pub mod decode;
mod links;
use decode::*;

const MAX_MOBI_FILE_SIZE: u64 = 150 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct MobiDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    images: HashMap<String, Vec<u8>>,
    footnotes: HashMap<String, String>,
    note_locations: HashMap<String, crate::document::NoteLocation>,
    format_name: &'static str,
}

impl MobiDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать MOBI/AZW файл: {e}")))?;
        if meta.len() > MAX_MOBI_FILE_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер файла MOBI/AZW превышает допустимый лимит (150 МБ)".to_string(),
            ));
        }
        let ext = path_ref
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let m = Mobi::from_path(path_ref)
            .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения MOBI/AZW файла: {e}")))?;

        let (raw_bytes, is_kf8) = extract_raw_mobi_bytes(&m)?;

        let format_name = match ext.as_str() {
            "azw3" => "AZW3",
            "azw" => "AZW",
            "prc" => "PRC",
            "mobi" => "MOBI",
            _ => {
                if is_kf8 {
                    "AZW3"
                } else {
                    "MOBI"
                }
            }
        };

        let header_codepage = match m.metadata.text_encoding() {
            mobi::headers::TextEncoding::UTF8 => 65001,
            mobi::headers::TextEncoding::CP1252 => 1252,
            mobi::headers::TextEncoding::Unknown(cp) => cp,
        };
        let content = decode_mobi_content(&raw_bytes, header_codepage);
        let detected_enc = resolve_mobi_encoding(&raw_bytes, header_codepage);

        let title_bytes = m
            .metadata
            .exth_record(mobi::headers::ExthRecord::Title)
            .and_then(|v| v.first())
            .map(|v| v.as_slice())
            .unwrap_or(&m.metadata.name);

        let mut title = decode_metadata_string(title_bytes, Some(detected_enc), header_codepage);
        if title.is_empty() {
            let m_title = m.title();
            if !m_title.is_empty() && !m_title.contains('\u{FFFD}') {
                if is_cp1252_mojibake(&m_title) {
                    title = repair_cp1252_mojibake(&m_title);
                } else {
                    title = m_title.trim().to_string();
                }
            }
        }
        if title.is_empty() {
            title = path_ref
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Документ MOBI".to_string());
        }

        let author_bytes = m
            .metadata
            .exth_record(mobi::headers::ExthRecord::Author)
            .and_then(|v| v.first())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let mut author = decode_metadata_string(author_bytes, Some(detected_enc), header_codepage);
        if author.is_empty() {
            if let Some(m_author) = m.author() {
                if !m_author.contains('\u{FFFD}') {
                    if is_cp1252_mojibake(&m_author) {
                        author = repair_cp1252_mojibake(&m_author);
                    } else {
                        author = m_author.trim().to_string();
                    }
                }
            }
        }

        let metadata = BookMetadata { title, author };

        let (chapters, toc, footnotes) = parse_mobi_html_with_notes(&content, &metadata.title);
        let note_locations = crate::document::locate_notes_in_chapters(&chapters, &footnotes);
        if chapters.is_empty() {
            return Err(DocumentError::Corrupted(
                "Не удалось извлечь главы из книги".to_string(),
            ));
        }

        Ok(Self {
            metadata,
            chapters,
            toc,
            images: collect_mobi_images(&m),
            footnotes,
            note_locations,
            format_name,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > MAX_MOBI_FILE_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер данных MOBI/AZW превышает допустимый лимит (150 МБ)".to_string(),
            ));
        }
        let m = Mobi::new(bytes.to_vec())
            .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения MOBI/AZW данных: {e}")))?;

        let (raw_bytes, is_kf8) = extract_raw_mobi_bytes(&m)?;
        let format_name = if is_kf8 { "AZW3" } else { "MOBI" };

        let header_codepage = match m.metadata.text_encoding() {
            mobi::headers::TextEncoding::UTF8 => 65001,
            mobi::headers::TextEncoding::CP1252 => 1252,
            mobi::headers::TextEncoding::Unknown(cp) => cp,
        };
        let content = decode_mobi_content(&raw_bytes, header_codepage);
        let detected_enc = resolve_mobi_encoding(&raw_bytes, header_codepage);

        let title_bytes = m
            .metadata
            .exth_record(mobi::headers::ExthRecord::Title)
            .and_then(|v| v.first())
            .map(|v| v.as_slice())
            .unwrap_or(&m.metadata.name);

        let mut title = decode_metadata_string(title_bytes, Some(detected_enc), header_codepage);
        if title.is_empty() {
            let m_title = m.title();
            if !m_title.is_empty() && !m_title.contains('\u{FFFD}') {
                if is_cp1252_mojibake(&m_title) {
                    title = repair_cp1252_mojibake(&m_title);
                } else {
                    title = m_title.trim().to_string();
                }
            }
        }
        if title.is_empty() {
            title = "Документ MOBI".to_string();
        }

        let author_bytes = m
            .metadata
            .exth_record(mobi::headers::ExthRecord::Author)
            .and_then(|v| v.first())
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let mut author = decode_metadata_string(author_bytes, Some(detected_enc), header_codepage);
        if author.is_empty() {
            if let Some(m_author) = m.author() {
                if !m_author.contains('\u{FFFD}') {
                    if is_cp1252_mojibake(&m_author) {
                        author = repair_cp1252_mojibake(&m_author);
                    } else {
                        author = m_author.trim().to_string();
                    }
                }
            }
        }

        let metadata = BookMetadata { title, author };

        let (chapters, toc, footnotes) = parse_mobi_html_with_notes(&content, &metadata.title);
        let note_locations = crate::document::locate_notes_in_chapters(&chapters, &footnotes);
        if chapters.is_empty() {
            return Err(DocumentError::Corrupted(
                "Не удалось извлечь главы из книги".to_string(),
            ));
        }

        Ok(Self {
            metadata,
            chapters,
            toc,
            images: collect_mobi_images(&m),
            footnotes,
            note_locations,
            format_name,
        })
    }
}

pub fn clean_chapter_title(raw: &str) -> String {
    let sanitized = crate::document::epub::html::sanitize_xml_entities(raw);
    let mut cleaned = String::with_capacity(sanitized.len());
    let mut in_tag = false;
    for c in sanitized.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            cleaned.push(c);
        }
    }
    let unescaped =
        quick_xml::escape::unescape(&cleaned).unwrap_or(std::borrow::Cow::Borrowed(&cleaned));
    let mut words = unescaped.split_whitespace();
    let mut res = String::new();
    if let Some(first) = words.next() {
        res.push_str(first);
        for w in words {
            res.push(' ');
            res.push_str(w);
        }
    }
    res
}

pub fn split_by_pagebreaks(html: &str) -> Vec<&str> {
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

pub fn split_by_headings(html: &str) -> Vec<&str> {
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

pub fn split_by_html_documents(html: &str) -> Vec<&str> {
    let lower = html.to_ascii_lowercase();
    let tag = "</html>";
    let count = lower.match_indices(tag).count();
    if count < 2 {
        return Vec::new();
    }
    let mut sections = Vec::new();
    let mut last = 0;
    for (idx, _) in lower.match_indices(tag) {
        let end = idx + tag.len();
        let chunk = html[last..end].trim();
        if !chunk.is_empty() {
            sections.push(chunk);
        }
        last = end;
    }
    if last < html.len() {
        let tail = html[last..].trim();
        if !tail.is_empty() {
            sections.push(tail);
        }
    }
    sections
}

pub fn is_likely_chapter_heading(text: &str) -> bool {
    let cleaned = clean_chapter_title(text);
    if cleaned.is_empty() || cleaned.chars().count() > 70 {
        return false;
    }
    let lower = cleaned.to_lowercase();
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
        "сеанс",
        "приложение",
        "appendix",
    ];
    keywords.iter().any(|&k| lower.starts_with(k))
}

pub fn parse_mobi_html_with_notes(
    raw_html: &str,
    default_title: &str,
) -> (Vec<BookChapter>, Vec<TocItem>, HashMap<String, String>) {
    let (html, notes) = links::preprocess_links(raw_html);
    let (chapters, toc) = parse_mobi_html(&html, default_title);
    (chapters, toc, notes)
}

pub fn parse_mobi_html(raw_html: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut raw_sections = split_by_pagebreaks(raw_html);
    if raw_sections.len() <= 1 {
        let html_docs = split_by_html_documents(raw_html);
        if html_docs.len() > 1 {
            raw_sections = html_docs;
        } else {
            let heading_sections = split_by_headings(raw_html);
            if heading_sections.len() > 1 {
                raw_sections = heading_sections;
            }
        }
    }

    if raw_sections.is_empty() {
        raw_sections.push(raw_html);
    }

    let mut parsed_sections: Vec<(String, Vec<String>)> = Vec::new();
    for raw_sec in raw_sections {
        let sanitized = crate::document::epub::html::sanitize_xml_entities(raw_sec);
        let (extracted_title, paragraphs) =
            crate::document::epub::html::extract_html_lenient(&sanitized);
        if !paragraphs.is_empty() || !extracted_title.trim().is_empty() {
            parsed_sections.push((extracted_title, paragraphs));
        }
    }

    let mut merged_sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut i = 0;
    while i < parsed_sections.len() {
        let (title, mut paras) = parsed_sections[i].clone();
        if i + 1 < parsed_sections.len() {
            let is_title_page =
                (paras.len() <= 3 && !paras.is_empty() && is_likely_chapter_heading(&paras[0]))
                    || (!title.trim().is_empty() && paras.is_empty());
            let next_has_heading = if let Some(first_p) = parsed_sections[i + 1].1.first() {
                is_likely_chapter_heading(first_p)
            } else {
                !parsed_sections[i + 1].0.trim().is_empty()
            };
            if is_title_page && !next_has_heading {
                paras.extend(parsed_sections[i + 1].1.clone());
                let final_title = if !title.trim().is_empty() {
                    title
                } else {
                    parsed_sections[i + 1].0.clone()
                };
                merged_sections.push((final_title, paras));
                i += 2;
                continue;
            }
        }
        merged_sections.push((title, paras));
        i += 1;
    }

    let mut chapters = Vec::new();
    let mut toc = Vec::new();

    for (extracted_title, mut paragraphs) in merged_sections {
        if paragraphs.is_empty() {
            continue;
        }

        let raw_title = if !extracted_title.trim().is_empty() {
            extracted_title.trim().to_string()
        } else if paragraphs.len() > 1 && is_likely_chapter_heading(&paragraphs[0]) {
            paragraphs.remove(0).trim().to_string()
        } else if paragraphs.len() > 1
            && clean_chapter_title(&paragraphs[0]).chars().count() <= 50
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

        let mut chapter_title = clean_chapter_title(&raw_title);
        if chapter_title.is_empty() {
            chapter_title = format!("Глава {}", chapters.len() + 1);
        }

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
            let raw_t = if !extracted_title.trim().is_empty() {
                extracted_title.trim()
            } else {
                default_title
            };
            let mut title = clean_chapter_title(raw_t);
            if title.is_empty() {
                title = default_title.to_string();
            }
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

fn collect_mobi_images(m: &Mobi) -> HashMap<String, Vec<u8>> {
    let mut images = HashMap::new();
    let mut total = 0usize;
    for (index, record) in m.image_records().iter().enumerate() {
        if image::guess_format(record.content).is_err()
            || record.content.len() > crate::document::MAX_SINGLE_ENTRY_BYTES as usize
        {
            continue;
        }
        let Some(new_total) = total.checked_add(record.content.len()) else {
            break;
        };
        if new_total > crate::document::MAX_TOTAL_UNPACK_BYTES as usize {
            break;
        }
        total = new_total;
        images.insert(format!("mobi_rec_{}", index + 1), record.content.to_vec());
    }
    images
}

impl TextContent for MobiDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
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

    fn image_bytes(&self, id: &str) -> Option<Vec<u8>> {
        self.images.get(id).cloned()
    }

    fn footnote(&self, id: &str) -> Option<&str> {
        self.footnotes.get(id).map(String::as_str)
    }

    fn note_location(&self, id: &str) -> Option<crate::document::NoteLocation> {
        self.note_locations.get(id).copied()
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
