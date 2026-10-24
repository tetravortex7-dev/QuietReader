use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, MAX_ARCHIVE_ENTRIES,
    MAX_SINGLE_ENTRY_BYTES, MAX_TOTAL_UNPACK_BYTES, TextContent, TocItem, is_safe_archive_path,
    read_bounded,
};
use base64::Engine;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

mod footnotes;
mod lenient;
mod structure;

use lenient::{extract_fb2_lenient, normalize_whitespace};
use structure::build_chapters_and_toc;

pub struct Fb2Document {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    footnotes: HashMap<String, String>,
    note_locations: HashMap<String, crate::document::NoteLocation>,
    images: HashMap<String, Vec<u8>>,
}

impl TextContent for Fb2Document {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl Document for Fb2Document {
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
        "FB2"
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
    }

    fn footnote(&self, id: &str) -> Option<&str> {
        self.footnotes.get(id).map(String::as_str)
    }

    fn note_location(&self, id: &str) -> Option<crate::document::NoteLocation> {
        self.note_locations.get(id).copied()
    }

    fn image_bytes(&self, id: &str) -> Option<Vec<u8>> {
        self.images.get(id).cloned()
    }
}

#[derive(Default, Debug)]
struct SectionNode {
    id: Option<String>,
    title: Option<String>,
    paragraphs: Vec<String>,
    subsections: Vec<SectionNode>,
}

#[derive(Default, Debug)]
struct BodyNode {
    name: Option<String>,
    title: Option<String>,
    direct_paragraphs: Vec<String>,
    sections: Vec<SectionNode>,
}

#[derive(Debug)]
struct TocLink {
    href: String,
    title: String,
}

impl Fb2Document {
    pub const MAX_FB2_SIZE: u64 = 150 * 1024 * 1024;

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref)
            .map_err(|e| DocumentError::Io(format!("Failed to read file metadata: {e}")))?;
        if meta.len() > Self::MAX_FB2_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер файла FB2 превышает максимально допустимый лимит (150 МБ)".to_string(),
            ));
        }
        let bytes = std::fs::read(path_ref)
            .map_err(|e| DocumentError::Io(format!("Failed to read file: {e}")))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > Self::MAX_FB2_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер данных FB2 превышает максимально допустимый лимит (150 МБ)".to_string(),
            ));
        }
        let text = decode_text(bytes);
        Self::parse_xml(&text)
    }

    pub fn from_zip_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)
            .map_err(|e| DocumentError::Io(format!("Не удалось открыть ZIP: {e}")))?;
        let mut zip = ZipArchive::new(file)
            .map_err(|e| DocumentError::Corrupted(format!("Ошибка парсинга ZIP: {e}")))?;

        if zip.len() > MAX_ARCHIVE_ENTRIES {
            return Err(DocumentError::LimitExceeded(
                "В архиве превышен лимит количества файлов (10 000)".to_string(),
            ));
        }

        let mut total_unpacked = 0u64;
        for i in 0..zip.len() {
            let entry = zip
                .by_index(i)
                .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения ZIP: {e}")))?;
            let name = entry
                .name()
                .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения ZIP: {e}")))?
                .to_string();
            if !is_safe_archive_path(&name) {
                return Err(DocumentError::Corrupted(format!(
                    "Обнаружен небезопасный путь в архиве: {name}"
                )));
            }
            if entry.size() > MAX_SINGLE_ENTRY_BYTES {
                return Err(DocumentError::LimitExceeded(format!(
                    "Файл {name} превышает лимит 50 МБ"
                )));
            }
            if !name.to_ascii_lowercase().ends_with(".fb2") {
                continue;
            }
            let bytes = read_bounded(
                entry,
                MAX_SINGLE_ENTRY_BYTES,
                &mut total_unpacked,
                MAX_TOTAL_UNPACK_BYTES,
            )?;
            return Self::from_bytes(&bytes);
        }

        Err(DocumentError::Unsupported(
            "В ZIP-архиве не найден файл FB2".to_string(),
        ))
    }

    fn parse_xml(xml: &str) -> Result<Self, DocumentError> {
        if !xml.to_ascii_lowercase().contains("<fictionbook") {
            return Err(DocumentError::Corrupted(
                "Отсутствует корневой тег <FictionBook> в FB2 документе".to_string(),
            ));
        }
        let sanitized = crate::document::epub::sanitize_xml_entities(xml);
        let mut reader = Reader::from_str(&sanitized);
        reader.config_mut().trim_text(false);
        reader.config_mut().check_end_names = false;

        let mut book_title = String::new();
        let mut author_first = String::new();
        let mut author_middle = String::new();
        let mut author_last = String::new();

        let mut in_title_info = false;
        let mut in_author = false;
        let mut in_book_title = false;
        let mut in_first_name = false;
        let mut in_middle_name = false;
        let mut in_last_name = false;

        let mut bodies: Vec<BodyNode> = Vec::new();
        let mut current_body: Option<BodyNode> = None;
        let mut section_stack: Vec<SectionNode> = Vec::new();

        let mut explicit_toc_links: Vec<TocLink> = Vec::new();
        let mut current_link_href: Option<String> = None;
        let mut current_link_text = String::new();
        let mut current_note_link: Option<String> = None;
        let mut current_plain_note: Option<(String, usize)> = None;

        let mut tag_stack: Vec<String> = Vec::new();
        let mut in_title = false;
        let mut title_paragraphs: Vec<String> = Vec::new();
        let mut current_title_paragraph = String::new();

        let mut in_paragraph = false;
        let mut current_paragraph = String::new();
        let mut images: HashMap<String, Vec<u8>> = HashMap::new();
        let mut image_total = 0usize;
        let mut in_binary = false;
        let mut binary_id = String::new();
        let mut binary_data = String::new();
        let mut in_table = false;
        let mut in_row = false;
        let mut in_cell = false;
        let mut cell_text = String::new();
        let mut row_cells: Vec<String> = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();

                    match tag_name.as_str() {
                        "binary" => {
                            let mut id = String::new();
                            for attr in e.attributes().flatten() {
                                if local_name(attr.key.as_ref()).eq_ignore_ascii_case("id") {
                                    id = attr.value.to_string();
                                }
                            }
                            if !id.is_empty() {
                                in_binary = true;
                                binary_id = id;
                                binary_data.clear();
                            }
                        }
                        "image" => {
                            let mut href = String::new();
                            for attr in e.attributes().flatten() {
                                if local_name(attr.key.as_ref()).eq_ignore_ascii_case("href") {
                                    href = attr.value.to_string();
                                }
                            }
                            let id = href.trim_start_matches('#').trim();
                            if !id.is_empty() && !in_title && current_body.is_some() {
                                if in_paragraph {
                                    let paragraph = normalize_whitespace(&current_paragraph);
                                    if !paragraph.is_empty() {
                                        push_fb2_paragraph(
                                            &mut section_stack,
                                            &mut current_body,
                                            paragraph,
                                        );
                                    }
                                    current_paragraph.clear();
                                }
                                push_fb2_paragraph(
                                    &mut section_stack,
                                    &mut current_body,
                                    crate::document::image_token(id),
                                );
                            }
                        }
                        "title-info" => in_title_info = true,
                        "author" if in_title_info => in_author = true,
                        "book-title" if in_title_info => in_book_title = true,
                        "first-name" if in_author => in_first_name = true,
                        "middle-name" if in_author => in_middle_name = true,
                        "last-name" if in_author => in_last_name = true,

                        "body" => {
                            let mut body_name = None;
                            for attr in e.attributes().flatten() {
                                let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                                if key == "name" {
                                    body_name = Some(attr.value.to_string());
                                }
                            }
                            current_body = Some(BodyNode {
                                name: body_name,
                                title: None,
                                direct_paragraphs: Vec::new(),
                                sections: Vec::new(),
                            });
                            section_stack.clear();
                        }

                        "section" if current_body.is_some() => {
                            if section_stack.len() < 32 {
                                let mut sec_id = None;
                                for attr in e.attributes().flatten() {
                                    let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                                    if key == "id" {
                                        sec_id = Some(attr.value.to_string());
                                    }
                                }
                                section_stack.push(SectionNode {
                                    id: sec_id,
                                    title: None,
                                    paragraphs: Vec::new(),
                                    subsections: Vec::new(),
                                });
                            }
                        }

                        "title" => {
                            let parent_tag = tag_stack.last().map(|s| s.as_str()).unwrap_or("");
                            if parent_tag == "section" || parent_tag == "body" {
                                in_title = true;
                                title_paragraphs.clear();
                                current_title_paragraph.clear();
                            }
                        }

                        "table" => {
                            in_table = true;
                            in_row = false;
                            in_cell = false;
                            in_paragraph = false;
                            current_paragraph.clear();
                        }

                        "tr" if in_table => {
                            in_row = true;
                            row_cells.clear();
                        }

                        "td" | "th" if in_row => {
                            in_cell = true;
                            cell_text.clear();
                        }

                        "p" | "v" | "subtitle" | "text-author" if in_cell => {}

                        "p" | "v" | "subtitle" | "text-author" => {
                            if in_title {
                                current_title_paragraph.clear();
                            } else if current_body.is_some() {
                                in_paragraph = true;
                                current_paragraph.clear();
                            }
                        }

                        "strong" | "b" => {
                            if in_paragraph && !in_title && current_note_link.is_none() {
                                current_paragraph.push_str("<b>");
                            }
                        }

                        "emphasis" | "i" | "em" => {
                            if in_paragraph && !in_title && current_note_link.is_none() {
                                current_paragraph.push_str("<i>");
                            }
                        }

                        "a" => {
                            let mut href = None;
                            let mut link_type = String::new();
                            for attr in e.attributes().flatten() {
                                let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                                match key.as_str() {
                                    "href" => href = Some(attr.value.to_string()),
                                    "type" => link_type = attr.value.to_string(),
                                    _ => {}
                                }
                            }
                            let in_toc = current_body
                                .as_ref()
                                .and_then(|body| body.name.as_deref())
                                .map(|name| name.eq_ignore_ascii_case("toc"))
                                .unwrap_or(false);
                            if in_toc {
                                if let Some(href) = href {
                                    current_link_href = Some(href);
                                    current_link_text.clear();
                                }
                            } else if link_type.to_ascii_lowercase().contains("note")
                                && let Some(href) = href
                            {
                                let id = href.strip_prefix('#').unwrap_or(&href);
                                if !id.is_empty() {
                                    current_note_link =
                                        Some(crate::document::epub::percent_decode_str(id));
                                }
                            } else if let Some(href) = href
                                && let Some(fragment) = href.strip_prefix('#')
                                && !fragment.is_empty()
                            {
                                current_plain_note = Some((
                                    crate::document::epub::percent_decode_str(fragment),
                                    current_paragraph.len(),
                                ));
                            }
                        }

                        _ => {}
                    }

                    if tag_stack.len() < 64 {
                        tag_stack.push(tag_name);
                    }
                }

                Ok(Event::End(ref e)) => {
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();
                    tag_stack.pop();

                    match tag_name.as_str() {
                        "binary" => {
                            if in_binary {
                                if let Some(bytes) = decode_fb2_base64(&binary_data)
                                    && bytes.len()
                                        <= crate::document::MAX_SINGLE_ENTRY_BYTES as usize
                                    && image_total.saturating_add(bytes.len())
                                        <= crate::document::MAX_TOTAL_UNPACK_BYTES as usize
                                {
                                    image_total += bytes.len();
                                    images.insert(std::mem::take(&mut binary_id), bytes);
                                }
                                in_binary = false;
                                binary_data.clear();
                            }
                        }
                        "title-info" => in_title_info = false,
                        "author" => in_author = false,
                        "book-title" => in_book_title = false,
                        "first-name" => in_first_name = false,
                        "middle-name" => in_middle_name = false,
                        "last-name" => in_last_name = false,

                        "a" => {
                            if let Some(note_id) = current_note_link.take() {
                                if in_paragraph && !in_title {
                                    current_paragraph
                                        .push_str(&crate::document::footnote_token(&note_id));
                                }
                            } else if let (Some(href), false) = (
                                current_link_href.take(),
                                current_link_text.trim().is_empty(),
                            ) {
                                explicit_toc_links.push(TocLink {
                                    href,
                                    title: normalize_whitespace(&current_link_text),
                                });
                            } else if let Some((fragment, start)) = current_plain_note.take() {
                                if in_paragraph && !in_title && start <= current_paragraph.len() {
                                    let label = crate::document::strip_formatting_tags(
                                        &current_paragraph[start..],
                                    );
                                    if crate::document::epub::html::looks_like_footnote_link(
                                        &fragment, &label,
                                    ) {
                                        current_paragraph.truncate(start);
                                        current_paragraph
                                            .push_str(&crate::document::footnote_token(&fragment));
                                    }
                                }
                            }
                        }

                        "td" | "th" if in_cell => {
                            row_cells.push(cell_text.trim().to_string());
                            cell_text.clear();
                            in_cell = false;
                        }

                        "tr" if in_row => {
                            let row = row_cells.join(" | ");
                            if !row.trim().is_empty() {
                                push_fb2_paragraph(&mut section_stack, &mut current_body, row);
                            }
                            row_cells.clear();
                            in_row = false;
                        }

                        "table" => {
                            in_table = false;
                            in_row = false;
                            in_cell = false;
                        }

                        "p" | "v" | "subtitle" | "text-author" if in_cell => {}

                        "p" | "v" | "subtitle" | "text-author" => {
                            if in_title {
                                let p_clean = normalize_whitespace(&current_title_paragraph);
                                if !p_clean.is_empty() {
                                    title_paragraphs.push(p_clean);
                                }
                                current_title_paragraph.clear();
                            } else if in_paragraph {
                                in_paragraph = false;
                                let p_clean = normalize_whitespace(&current_paragraph);
                                if !p_clean.is_empty() {
                                    if let Some(sec) = section_stack.last_mut() {
                                        sec.paragraphs.push(p_clean);
                                    } else if let Some(ref mut body) = current_body {
                                        body.direct_paragraphs.push(p_clean);
                                    }
                                }
                                current_paragraph.clear();
                            }
                        }

                        "strong" | "b" => {
                            if in_paragraph && !in_title && current_note_link.is_none() {
                                current_paragraph.push_str("</b>");
                            }
                        }

                        "emphasis" | "i" | "em" => {
                            if in_paragraph && !in_title && current_note_link.is_none() {
                                current_paragraph.push_str("</i>");
                            }
                        }

                        "title" => {
                            if in_title {
                                in_title = false;
                                if !current_title_paragraph.trim().is_empty() {
                                    title_paragraphs
                                        .push(current_title_paragraph.trim().to_string());
                                    current_title_paragraph.clear();
                                }
                                let title_text = title_paragraphs.join(" — ");
                                if !title_text.is_empty() {
                                    if let Some(sec) = section_stack.last_mut() {
                                        sec.title = Some(title_text);
                                    } else if let Some(ref mut body) = current_body {
                                        body.title = Some(title_text);
                                    }
                                }
                            }
                        }

                        "section" => {
                            if let Some(completed_sec) = section_stack.pop() {
                                if let Some(parent_sec) = section_stack.last_mut() {
                                    parent_sec.subsections.push(completed_sec);
                                } else if let Some(ref mut body) = current_body {
                                    body.sections.push(completed_sec);
                                }
                            }
                        }

                        "body" => {
                            while let Some(sec) = section_stack.pop() {
                                if let Some(parent_sec) = section_stack.last_mut() {
                                    parent_sec.subsections.push(sec);
                                } else if let Some(ref mut body) = current_body {
                                    body.sections.push(sec);
                                }
                            }
                            if let Some(body) = current_body.take() {
                                bodies.push(body);
                            }
                        }

                        _ => {}
                    }
                }

                Ok(Event::Empty(ref e)) => {
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag_name == "empty-line" && current_body.is_some() && !in_title {
                        push_fb2_paragraph(&mut section_stack, &mut current_body, String::new());
                    } else if tag_name == "image" && current_body.is_some() && !in_title {
                        let mut href = String::new();
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("href") {
                                href = attr.value.to_string();
                            }
                        }
                        let id = href.trim_start_matches('#').trim();
                        if !id.is_empty() {
                            if in_paragraph {
                                let paragraph = normalize_whitespace(&current_paragraph);
                                if !paragraph.is_empty() {
                                    push_fb2_paragraph(
                                        &mut section_stack,
                                        &mut current_body,
                                        paragraph,
                                    );
                                }
                                current_paragraph.clear();
                            }
                            push_fb2_paragraph(
                                &mut section_stack,
                                &mut current_body,
                                crate::document::image_token(id),
                            );
                        }
                    }
                }

                Ok(Event::Text(ref e)) => {
                    let raw_str = e.as_ref();
                    let text = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());

                    if in_binary {
                        binary_data.push_str(&text);
                    } else if in_cell {
                        cell_text.push_str(&text);
                    } else if in_book_title {
                        book_title.push_str(&text);
                    } else if in_first_name {
                        author_first.push_str(&text);
                    } else if in_middle_name {
                        author_middle.push_str(&text);
                    } else if in_last_name {
                        author_last.push_str(&text);
                    } else if in_title {
                        current_title_paragraph.push_str(&text);
                    } else if current_note_link.is_some() {
                    } else if current_link_href.is_some() {
                        current_link_text.push_str(&text);
                    } else if in_paragraph {
                        current_paragraph.push_str(&text);
                    }
                }

                Ok(Event::GeneralRef(ref e)) => {
                    let repl = match e.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => "",
                    };
                    if !repl.is_empty() {
                        if in_first_name {
                            author_first.push_str(repl);
                        } else if in_middle_name {
                            author_middle.push_str(repl);
                        } else if in_last_name {
                            author_last.push_str(repl);
                        } else if in_title {
                            current_title_paragraph.push_str(repl);
                        } else if current_note_link.is_some() {
                        } else if current_link_href.is_some() {
                            current_link_text.push_str(repl);
                        } else if in_paragraph {
                            current_paragraph.push_str(repl);
                        }
                    }
                }

                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        if let Some(body) = current_body.take() {
            bodies.push(body);
        }

        let mut author_parts = Vec::new();
        if !author_first.trim().is_empty() {
            author_parts.push(author_first.trim());
        }
        if !author_middle.trim().is_empty() {
            author_parts.push(author_middle.trim());
        }
        if !author_last.trim().is_empty() {
            author_parts.push(author_last.trim());
        }
        let mut author = if author_parts.is_empty() {
            "Неизвестный автор".to_string()
        } else {
            author_parts.join(" ")
        };

        let mut title = if book_title.trim().is_empty() {
            "Без названия".to_string()
        } else {
            book_title.trim().to_string()
        };

        let (mut chapters, mut toc, section_id_to_chapter, footnotes, mut note_locations) =
            build_chapters_and_toc(bodies);

        if chapters.is_empty() || chapters.iter().all(|c| c.is_empty()) {
            let (fb_title, fb_author, fb_chapters) = extract_fb2_lenient(xml);
            if !fb_chapters.is_empty() && fb_chapters.iter().any(|c| !c.is_empty()) {
                if title == "Без названия" && !fb_title.is_empty() {
                    title = fb_title;
                }
                if author == "Неизвестный автор" && !fb_author.is_empty() {
                    author = fb_author;
                }
                chapters = fb_chapters;
                note_locations.clear();
                toc = chapters
                    .iter()
                    .enumerate()
                    .map(|(i, c)| TocItem {
                        id: c.id.clone(),
                        title: if c.title.is_empty() {
                            format!("Раздел {}", i + 1)
                        } else {
                            c.title.clone()
                        },
                        chapter_index: i,
                        children: Vec::new(),
                    })
                    .collect();
            } else {
                return Err(DocumentError::Corrupted(
                    "Не удалось извлечь содержимое из повреждённого FB2 документа".to_string(),
                ));
            }
        }

        if !explicit_toc_links.is_empty() {
            let mut custom_toc = Vec::new();
            for link in explicit_toc_links {
                let target_id = link.href.strip_prefix('#').unwrap_or(&link.href);
                let decoded_id = crate::document::epub::percent_decode_str(target_id);
                let chapter_idx = section_id_to_chapter
                    .get(target_id)
                    .or_else(|| section_id_to_chapter.get(&decoded_id))
                    .copied();

                if let Some(ch_idx) = chapter_idx {
                    custom_toc.push(TocItem {
                        id: Some(target_id.to_string()),
                        title: link.title,
                        chapter_index: ch_idx,
                        children: Vec::new(),
                    });
                }
            }
            if !custom_toc.is_empty() {
                toc = custom_toc;
            }
        }

        Ok(Fb2Document {
            metadata: BookMetadata { title, author },
            chapters,
            toc,
            footnotes,
            note_locations,
            images,
        })
    }
}

fn push_fb2_paragraph(
    section_stack: &mut [SectionNode],
    body: &mut Option<BodyNode>,
    paragraph: String,
) {
    if let Some(section) = section_stack.last_mut() {
        section.paragraphs.push(paragraph);
    } else if let Some(body) = body.as_mut() {
        body.direct_paragraphs.push(paragraph);
    }
}

fn decode_fb2_base64(text: &str) -> Option<Vec<u8>> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return None;
    }
    base64::engine::general_purpose::STANDARD
        .decode(compact)
        .ok()
}

fn local_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
}

pub fn zip_contains_fb2(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let Ok(mut zip) = ZipArchive::new(file) else {
        return false;
    };
    if zip.len() > MAX_ARCHIVE_ENTRIES {
        return false;
    }
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i)
            && !entry.is_dir()
            && let Ok(name) = entry.name()
            && name.to_ascii_lowercase().ends_with(".fb2")
        {
            return true;
        }
    }
    false
}

fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }

    let probe_len = bytes.len().min(300);
    let probe_str = String::from_utf8_lossy(&bytes[..probe_len]);

    if let Some(enc_label) = find_xml_encoding(&probe_str)
        && let Some(encoding) = encoding_rs::Encoding::for_label(enc_label.as_bytes())
    {
        let (decoded, _, _) = encoding.decode(bytes);
        return decoded.into_owned();
    }

    let (decoded, _, _) = encoding_rs::UTF_8.decode(bytes);
    decoded.into_owned()
}

fn find_xml_encoding(header: &str) -> Option<&str> {
    let lower = header.to_lowercase();
    let enc_idx = lower.find("encoding=")?;
    let remainder = &header[enc_idx + 9..];
    let quote = remainder.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &remainder[1..];
    let end_idx = rest.find(quote)?;
    Some(&rest[..end_idx])
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
