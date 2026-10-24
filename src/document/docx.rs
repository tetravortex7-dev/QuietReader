use crate::document::{BookChapter, BookMetadata, Document, DocumentError, TextContent, TocItem};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct DocxDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    images: std::collections::HashMap<String, Vec<u8>>,
    footnotes: std::collections::HashMap<String, String>,
    note_locations: std::collections::HashMap<String, crate::document::NoteLocation>,
    header_text: Option<String>,
    footer_text: Option<String>,
}

impl TextContent for DocxDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl Document for DocxDocument {
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
        "DOCX"
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
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

    fn header_text(&self) -> Option<&str> {
        self.header_text.as_deref()
    }

    fn footer_text(&self) -> Option<&str> {
        self.footer_text.as_deref()
    }
}

impl DocxDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path = path.as_ref();
        let meta = std::fs::metadata(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать файл DOCX: {e}")))?;
        if meta.len() > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Размер файла DOCX превышает допустимый лимит".to_string(),
            ));
        }

        let file = File::open(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось открыть файл DOCX: {e}")))?;
        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ Word");

        Self::from_reader(file, default_title)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Размер данных DOCX превышает лимит".to_string(),
            ));
        }
        let cursor = std::io::Cursor::new(bytes);
        Self::from_reader(cursor, default_title)
    }

    fn from_reader<R: Read + std::io::Seek>(
        reader: R,
        default_title: &str,
    ) -> Result<Self, DocumentError> {
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| DocumentError::Corrupted(format!("Поврежденный архив DOCX: {e}")))?;

        if archive.len() > crate::document::MAX_ARCHIVE_ENTRIES {
            return Err(DocumentError::LimitExceeded(
                "Слишком много файлов в архиве DOCX".to_string(),
            ));
        }

        let mut total_unpacked = 0u64;
        let mut doc_xml_bytes: Option<Vec<u8>> = None;
        let mut core_xml_bytes: Option<Vec<u8>> = None;
        let mut rels_xml_bytes: Option<Vec<u8>> = None;
        let mut footnotes_xml_bytes: Option<Vec<u8>> = None;
        let mut endnotes_xml_bytes: Option<Vec<u8>> = None;
        let mut media: Vec<(String, Vec<u8>)> = Vec::new();
        let mut header_parts: Vec<String> = Vec::new();
        let mut footer_parts: Vec<String> = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| DocumentError::Corrupted(format!("Ошибка архива DOCX: {e}")))?;
            let name = file
                .name()
                .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения ZIP: {e}")))?
                .to_string();
            if !crate::document::is_safe_archive_path(&name) {
                return Err(DocumentError::Corrupted(format!(
                    "Небезопасный путь в архиве DOCX: {name}"
                )));
            }

            if name == "word/document.xml" {
                doc_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            } else if name == "docProps/core.xml" {
                core_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            } else if name == "word/_rels/document.xml.rels" {
                rels_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            } else if name == "word/footnotes.xml" {
                footnotes_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            } else if name == "word/endnotes.xml" {
                endnotes_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            } else if name.starts_with("word/header")
                && name.ends_with(".xml")
                && let Ok(bytes) = crate::document::read_bounded(
                    &mut file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )
            {
                let text = extract_docx_story_text(&bytes);
                if !text.is_empty() {
                    header_parts.push(text);
                }
            } else if name.starts_with("word/footer")
                && name.ends_with(".xml")
                && let Ok(bytes) = crate::document::read_bounded(
                    &mut file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )
            {
                let text = extract_docx_story_text(&bytes);
                if !text.is_empty() {
                    footer_parts.push(text);
                }
            } else if name.starts_with("word/media/")
                && media.len() < 2_000
                && let Ok(bytes) = crate::document::read_bounded(
                    &mut file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )
            {
                media.push((name.clone(), bytes));
            }
        }

        let doc_xml = doc_xml_bytes.ok_or_else(|| {
            DocumentError::Corrupted("В архиве DOCX отсутствует word/document.xml".to_string())
        })?;

        let doc_xml_str = String::from_utf8_lossy(&doc_xml);
        let metadata = core_xml_bytes
            .as_deref()
            .map(Self::parse_core_properties)
            .unwrap_or_else(|| BookMetadata {
                title: default_title.to_string(),
                author: "Неизвестный автор".to_string(),
            });

        let images = build_docx_images(rels_xml_bytes.as_deref(), &media);
        let mut footnotes = footnotes_xml_bytes
            .as_deref()
            .map(|xml| parse_docx_notes(xml, "footnote", "docx_fn_"))
            .unwrap_or_default();
        if let Some(xml) = endnotes_xml_bytes.as_deref() {
            footnotes.extend(parse_docx_notes(xml, "endnote", "docx_en_"));
        }
        let header_text = join_story_parts(header_parts);
        let footer_text = join_story_parts(footer_parts);
        let (mut chapters, toc) = Self::parse_document_xml(&doc_xml_str, &metadata.title, &images)?;
        let note_locations = crate::document::build_notes_appendix(&mut chapters, &footnotes);

        Ok(Self {
            metadata,
            chapters,
            toc,
            images,
            footnotes,
            note_locations,
            header_text,
            footer_text,
        })
    }

    fn parse_core_properties(xml: &[u8]) -> BookMetadata {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let xml_str = String::from_utf8_lossy(xml);
        let mut reader = Reader::from_str(&xml_str);
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut author = String::new();
        let mut current_tag = String::new();
        let mut buf = Vec::new();

        while let Ok(event) = reader.read_event_into(&mut buf) {
            match event {
                Event::Start(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    current_tag = tag;
                }
                Event::Text(ref e) => {
                    let raw_str = e.as_ref();
                    let txt = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());
                    if current_tag == "title" && title.is_empty() {
                        title = txt.trim().to_string();
                    } else if (current_tag == "creator" || current_tag == "author")
                        && author.is_empty()
                    {
                        author = txt.trim().to_string();
                    }
                }
                Event::End(_) => {
                    current_tag.clear();
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        BookMetadata {
            title: if title.is_empty() {
                "Документ Word".to_string()
            } else {
                title
            },
            author: if author.is_empty() {
                "Неизвестный автор".to_string()
            } else {
                author
            },
        }
    }

    fn parse_document_xml(
        xml: &str,
        default_title: &str,
        images: &HashMap<String, Vec<u8>>,
    ) -> Result<(Vec<BookChapter>, Vec<TocItem>), DocumentError> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut chapters: Vec<BookChapter> = Vec::new();
        let mut current_chapter_title = String::new();
        let mut current_chapter_paras: Vec<String> = Vec::new();

        let mut in_p = false;
        let mut in_t = false;
        let mut current_p_text = String::new();
        let mut is_heading = false;
        let mut in_table = 0usize;
        let mut in_row = false;
        let mut in_cell = false;
        let mut cell_text = String::new();
        let mut row_cells: Vec<String> = Vec::new();

        let mut buf = Vec::new();

        while let Ok(event) = reader.read_event_into(&mut buf) {
            match event {
                Event::Start(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag == "tbl" {
                        in_table = in_table.saturating_add(1);
                    } else if tag == "tr" && in_table > 0 {
                        in_row = true;
                        row_cells.clear();
                    } else if tag == "tc" && in_row {
                        in_cell = true;
                        cell_text.clear();
                    } else if tag == "blip" {
                        let mut relationship = String::new();
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("embed") {
                                relationship = attr.value.to_string();
                            }
                        }
                        if !relationship.is_empty()
                            && images.contains_key(&relationship)
                            && !in_cell
                        {
                            let text = current_p_text.trim();
                            if !text.is_empty() {
                                current_chapter_paras.push(text.to_string());
                                current_p_text.clear();
                            }
                            current_chapter_paras.push(crate::document::image_token(&relationship));
                        }
                    } else if let Some(note_id) = docx_note_reference(e) {
                        current_p_text.push_str(&crate::document::footnote_token(&note_id));
                    } else if tag == "p" {
                        in_p = true;
                        current_p_text.clear();
                        is_heading = false;
                    } else if in_p && tag == "pstyle" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let val = attr.value.to_ascii_lowercase();
                                if val.contains("heading")
                                    || val.contains("title")
                                    || val.contains("заголовок")
                                {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && tag == "outlinelvl" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let lvl = attr.value.trim();
                                if lvl == "0" || lvl == "1" {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && tag == "t" {
                        in_t = true;
                    }
                }
                Event::Empty(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag == "blip" {
                        let mut relationship = String::new();
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("embed") {
                                relationship = attr.value.to_string();
                            }
                        }
                        if !relationship.is_empty()
                            && images.contains_key(&relationship)
                            && !in_cell
                        {
                            let text = current_p_text.trim();
                            if !text.is_empty() {
                                current_chapter_paras.push(text.to_string());
                                current_p_text.clear();
                            }
                            current_chapter_paras.push(crate::document::image_token(&relationship));
                        }
                    } else if let Some(note_id) = docx_note_reference(e) {
                        current_p_text.push_str(&crate::document::footnote_token(&note_id));
                    } else if in_p && tag == "pstyle" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let val = attr.value.to_ascii_lowercase();
                                if val.contains("heading")
                                    || val.contains("title")
                                    || val.contains("заголовок")
                                {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && tag == "outlinelvl" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let lvl = attr.value.trim();
                                if lvl == "0" || lvl == "1" {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && (tag == "br" || tag == "cr") {
                        current_p_text.push('\n');
                    } else if in_p && tag == "tab" {
                        current_p_text.push('\t');
                    }
                }
                Event::Text(ref e) => {
                    if in_t {
                        let raw = e.as_ref();
                        let txt = quick_xml::escape::unescape(raw)
                            .map(|c| c.into_owned())
                            .unwrap_or_else(|_| raw.to_string());
                        current_p_text.push_str(&txt);
                    }
                }
                Event::End(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag == "tc" && in_cell {
                        let text = cell_text.trim();
                        row_cells.push(text.to_string());
                        cell_text.clear();
                        in_cell = false;
                    } else if tag == "tr" && in_row {
                        let row = row_cells.join(" | ");
                        if !row.trim().is_empty() {
                            current_chapter_paras.push(row);
                        }
                        row_cells.clear();
                        in_row = false;
                    } else if tag == "tbl" {
                        in_table = in_table.saturating_sub(1);
                    } else if tag == "t" {
                        in_t = false;
                    } else if tag == "p" {
                        in_p = false;
                        let text = current_p_text.trim();
                        if !text.is_empty() {
                            if in_cell {
                                if !cell_text.is_empty() {
                                    cell_text.push(' ');
                                }
                                cell_text.push_str(text);
                            } else if is_heading {
                                if !current_chapter_paras.is_empty()
                                    || !current_chapter_title.is_empty()
                                {
                                    let title = if current_chapter_title.is_empty() {
                                        if chapters.is_empty() {
                                            default_title.to_string()
                                        } else {
                                            format!("Раздел {}", chapters.len() + 1)
                                        }
                                    } else {
                                        std::mem::take(&mut current_chapter_title)
                                    };
                                    chapters.push(BookChapter {
                                        id: Some(format!("chap_{}", chapters.len())),
                                        title,
                                        paragraphs: std::mem::take(&mut current_chapter_paras),
                                    });
                                }
                                current_chapter_title = text.to_string();
                            } else {
                                current_chapter_paras.push(text.to_string());
                            }
                        }
                        current_p_text.clear();
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        if !current_chapter_paras.is_empty() || !current_chapter_title.is_empty() {
            let title = if current_chapter_title.is_empty() {
                if chapters.is_empty() {
                    default_title.to_string()
                } else {
                    format!("Раздел {}", chapters.len() + 1)
                }
            } else {
                current_chapter_title
            };
            chapters.push(BookChapter {
                id: Some(format!("chap_{}", chapters.len())),
                title,
                paragraphs: current_chapter_paras,
            });
        }

        if chapters.is_empty() {
            chapters.push(BookChapter {
                id: Some("chap_0".to_string()),
                title: default_title.to_string(),
                paragraphs: vec!["Документ не содержит текста.".to_string()],
            });
        }

        let mut toc = Vec::new();
        if chapters.len() > 1 {
            for (idx, chap) in chapters.iter().enumerate() {
                toc.push(TocItem {
                    id: chap.id.clone(),
                    title: chap.title.clone(),
                    chapter_index: idx,
                    children: Vec::new(),
                });
            }
        }

        Ok((chapters, toc))
    }
}

fn parse_docx_relationships(xml: &str) -> HashMap<String, String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut relationships = HashMap::new();
    let mut buf = Vec::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        let element = match event {
            Event::Start(ref e) | Event::Empty(ref e) => e,
            Event::Eof => break,
            _ => {
                buf.clear();
                continue;
            }
        };
        let tag = local_name(element.name().as_ref()).to_ascii_lowercase();
        if tag == "relationship" {
            let mut id = String::new();
            let mut target = String::new();
            for attr in element.attributes().flatten() {
                match local_name(attr.key.as_ref()).to_ascii_lowercase().as_str() {
                    "id" => id = attr.value.to_string(),
                    "target" => target = attr.value.to_string(),
                    _ => {}
                }
            }
            if !id.is_empty() && !target.is_empty() && !target.starts_with("http") {
                relationships.insert(id, target);
            }
        }
        buf.clear();
    }
    relationships
}

fn build_docx_images(
    relationships: Option<&[u8]>,
    media: &[(String, Vec<u8>)],
) -> HashMap<String, Vec<u8>> {
    let Some(relationships) = relationships else {
        return HashMap::new();
    };
    let xml = String::from_utf8_lossy(relationships);
    let mut images = HashMap::new();
    for (id, target) in parse_docx_relationships(&xml) {
        let target = target.trim_start_matches('/');
        let resolved = if target.starts_with("word/") {
            target.to_string()
        } else {
            crate::document::epub::resolve_path("word", target)
        };
        let decoded = crate::document::epub::percent_decode_str(&resolved);
        let found = media
            .iter()
            .find(|(name, _)| name == &resolved || name == &decoded);
        if let Some((_, bytes)) = found
            && image::guess_format(bytes).is_ok()
        {
            images.insert(id, bytes.clone());
        }
    }
    images
}

fn extract_docx_story_text(xml: &[u8]) -> String {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let xml = String::from_utf8_lossy(xml);
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(false);
    let mut text = String::new();
    let mut in_text = false;
    let mut buf = Vec::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(ref e) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "t" {
                    in_text = true;
                } else if matches!(tag.as_str(), "p" | "br") {
                    text.push(' ');
                }
            }
            Event::Empty(ref e) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if matches!(tag.as_str(), "br" | "tab") {
                    text.push(' ');
                }
            }
            Event::Text(ref e) if in_text => {
                let raw = e.as_ref();
                let value = quick_xml::escape::unescape(raw)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                text.push_str(&value);
            }
            Event::End(ref e) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "t" {
                    in_text = false;
                } else if tag == "p" {
                    text.push(' ');
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn join_story_parts(parts: Vec<String>) -> Option<String> {
    let mut unique = Vec::new();
    for part in parts {
        if !part.is_empty() && !unique.iter().any(|existing| existing == &part) {
            unique.push(part);
        }
    }
    if unique.is_empty() {
        None
    } else {
        Some(unique.join(" • "))
    }
}

fn docx_note_reference(e: &quick_xml::events::BytesStart<'_>) -> Option<String> {
    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
    let prefix = match tag.as_str() {
        "footnotereference" => "docx_fn_",
        "endnotereference" => "docx_en_",
        _ => return None,
    };
    let mut id = String::new();
    for attr in e.attributes().flatten() {
        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("id") {
            id = attr.value.to_string();
        }
    }
    if id.is_empty() {
        None
    } else {
        Some(format!("{prefix}{id}"))
    }
}

fn parse_docx_notes(xml: &[u8], tag_name: &str, prefix: &str) -> HashMap<String, String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let xml = String::from_utf8_lossy(xml);
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(false);
    let mut notes = HashMap::new();
    let mut current: Option<String> = None;
    let mut text = String::new();
    let mut buf = Vec::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(ref e) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == tag_name {
                    let mut id = String::new();
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("id") {
                            id = attr.value.to_string();
                        }
                    }
                    if !id.is_empty() {
                        current = Some(format!("{prefix}{id}"));
                        text.clear();
                    }
                } else if current.is_some() && (tag == "p" || tag == "br") {
                    text.push(' ');
                }
            }
            Event::Empty(ref e) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if current.is_some() && tag == "br" {
                    text.push(' ');
                }
            }
            Event::Text(ref e) if current.is_some() => {
                let raw = e.as_ref();
                let value = quick_xml::escape::unescape(raw)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                text.push_str(&value);
            }
            Event::End(ref e) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == tag_name
                    && let Some(id) = current.take()
                {
                    let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !cleaned.is_empty() {
                        notes.insert(id, cleaned);
                    }
                    text.clear();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    notes
}

fn local_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
