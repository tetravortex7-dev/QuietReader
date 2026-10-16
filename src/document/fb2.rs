use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::path::Path;

pub struct Fb2Document {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for Fb2Document {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for Fb2Document {
    fn toc(&self) -> &[TocItem] {
        &self.toc
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

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        Some(self)
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

        let mut tag_stack: Vec<String> = Vec::new();
        let mut in_title = false;
        let mut title_paragraphs: Vec<String> = Vec::new();
        let mut current_title_paragraph = String::new();

        let mut in_paragraph = false;
        let mut current_paragraph = String::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();

                    match tag_name.as_str() {
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

                        "p" | "v" | "subtitle" | "text-author" => {
                            if in_title {
                                current_title_paragraph.clear();
                            } else if current_body.is_some() {
                                in_paragraph = true;
                                current_paragraph.clear();
                            }
                        }

                        "a" => {
                            if let Some(ref body) = current_body
                                && body
                                    .name
                                    .as_deref()
                                    .unwrap_or("")
                                    .eq_ignore_ascii_case("toc")
                            {
                                for attr in e.attributes().flatten() {
                                    let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                                    if key == "href" {
                                        current_link_href = Some(attr.value.to_string());
                                        current_link_text.clear();
                                    }
                                }
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
                        "title-info" => in_title_info = false,
                        "author" => in_author = false,
                        "book-title" => in_book_title = false,
                        "first-name" => in_first_name = false,
                        "middle-name" => in_middle_name = false,
                        "last-name" => in_last_name = false,

                        "a" => {
                            if let (Some(href), false) = (
                                current_link_href.take(),
                                current_link_text.trim().is_empty(),
                            ) {
                                explicit_toc_links.push(TocLink {
                                    href,
                                    title: normalize_whitespace(&current_link_text),
                                });
                            }
                        }

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
                        if let Some(sec) = section_stack.last_mut() {
                            sec.paragraphs.push(String::new());
                        } else if let Some(ref mut body) = current_body {
                            body.direct_paragraphs.push(String::new());
                        }
                    }
                }

                Ok(Event::Text(ref e)) => {
                    let raw_str = e.as_ref();
                    let text = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());

                    if in_book_title {
                        book_title.push_str(&text);
                    } else if in_first_name {
                        author_first.push_str(&text);
                    } else if in_middle_name {
                        author_middle.push_str(&text);
                    } else if in_last_name {
                        author_last.push_str(&text);
                    } else if in_title {
                        current_title_paragraph.push_str(&text);
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

        let (mut chapters, mut toc, section_id_to_chapter) = build_chapters_and_toc(bodies);

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
                toc = chapters
                    .iter()
                    .enumerate()
                    .map(|(i, c)| TocItem {
                        id: c.id.clone(),
                        title: if c.title.is_empty() { format!("Раздел {}", i + 1) } else { c.title.clone() },
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
        })
    }
}

struct FlattenContext {
    chapters: Vec<BookChapter>,
    section_id_to_chapter: HashMap<String, usize>,
}

fn build_chapters_and_toc(bodies: Vec<BodyNode>) -> (Vec<BookChapter>, Vec<TocItem>, HashMap<String, usize>) {
    let mut ctx = FlattenContext {
        chapters: Vec::new(),
        section_id_to_chapter: HashMap::new(),
    };
    let mut root_toc: Vec<TocItem> = Vec::new();

    for body in bodies {
        if body
            .name
            .as_deref()
            .unwrap_or("")
            .eq_ignore_ascii_case("toc")
        {
            continue;
        }

        let body_has_direct_text = body.direct_paragraphs.iter().any(|p| !p.trim().is_empty());
        let body_has_title = body
            .title
            .as_ref()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false);

        let mut body_start_chapter: Option<usize> = None;

        if body_has_direct_text || (body.sections.is_empty() && body_has_title) {
            let idx = ctx.chapters.len();
            body_start_chapter = Some(idx);
            ctx.chapters.push(BookChapter {
                id: None,
                title: body.title.clone().unwrap_or_default(),
                paragraphs: body.direct_paragraphs,
            });
        }

        let mut body_sections_toc: Vec<TocItem> = Vec::new();

        for section in body.sections {
            let (sec_first_ch, sec_tocs) = flatten_section(section, &mut ctx, 0);
            if body_start_chapter.is_none() {
                body_start_chapter = sec_first_ch;
            }
            body_sections_toc.extend(sec_tocs);
        }

        if body_has_title {
            let title = body.title.unwrap_or_default();
            let chapter_index = body_start_chapter.unwrap_or(0);
            root_toc.push(TocItem {
                id: None,
                title,
                chapter_index,
                children: body_sections_toc,
            });
        } else {
            root_toc.extend(body_sections_toc);
        }
    }

    (ctx.chapters, root_toc, ctx.section_id_to_chapter)
}

fn flatten_section(
    mut section: SectionNode,
    ctx: &mut FlattenContext,
    depth: usize,
) -> (Option<usize>, Vec<TocItem>) {
    let has_own_title = section
        .title
        .as_ref()
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false);

    if has_own_title && section.paragraphs.is_empty() && !section.subsections.is_empty() {
        while !section.subsections.is_empty() {
            let is_untitled_leaf = section.subsections[0]
                .title
                .as_ref()
                .map(|t| t.trim().is_empty())
                .unwrap_or(true)
                && section.subsections[0].subsections.is_empty();

            if is_untitled_leaf {
                let first_sub = section.subsections.remove(0);
                if section.id.is_none() {
                    section.id = first_sub.id;
                }
                section.paragraphs.extend(first_sub.paragraphs);
            } else {
                break;
            }
        }
    }

    if depth >= 32 {
        let mut overflow_paragraphs = Vec::new();
        for sub in section.subsections.drain(..) {
            overflow_paragraphs.extend(sub.paragraphs);
            for s in sub.subsections {
                overflow_paragraphs.extend(s.paragraphs);
            }
        }
        section.paragraphs.extend(overflow_paragraphs);
    }

    let has_own_paragraphs = section.paragraphs.iter().any(|p| !p.trim().is_empty());
    let mut own_chapter_idx: Option<usize> = None;

    let should_create_chapter = if has_own_paragraphs {
        true
    } else if section.subsections.is_empty() {
        has_own_title
    } else {
        false
    };

    if should_create_chapter {
        let idx = ctx.chapters.len();
        own_chapter_idx = Some(idx);
        ctx.chapters.push(BookChapter {
            id: section.id.clone(),
            title: section.title.clone().unwrap_or_default(),
            paragraphs: section.paragraphs,
        });
    }

    let mut child_tocs: Vec<TocItem> = Vec::new();
    let mut first_child_chapter: Option<usize> = None;

    for sub in section.subsections {
        let (sub_first_ch, sub_tocs) = flatten_section(sub, ctx, depth + 1);
        if first_child_chapter.is_none() {
            first_child_chapter = sub_first_ch;
        }
        child_tocs.extend(sub_tocs);
    }

    let effective_chapter_idx = own_chapter_idx.or(first_child_chapter);

    if let (Some(id), Some(ch_idx)) = (&section.id, effective_chapter_idx) {
        ctx.section_id_to_chapter.entry(id.clone()).or_insert(ch_idx);
        ctx.section_id_to_chapter
            .entry(crate::document::epub::percent_decode_str(id))
            .or_insert(ch_idx);
    }

    if has_own_title {
        let title = section.title.unwrap_or_default();
        let chapter_index = effective_chapter_idx.unwrap_or(0);
        let toc_item = TocItem {
            id: section.id,
            title,
            chapter_index,
            children: child_tocs,
        };
        (effective_chapter_idx, vec![toc_item])
    } else {
        (effective_chapter_idx, child_tocs)
    }
}

pub fn extract_fb2_lenient(xml: &str) -> (String, String, Vec<BookChapter>) {
    let mut title = String::new();
    let mut author = String::new();
    let mut chapters = Vec::new();
    let mut current_paragraphs = Vec::new();
    let mut current_title = String::new();
    let mut in_title = false;
    let mut in_p = false;
    let mut current_p = String::new();

    let mut i = 0;
    let bytes = xml.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rest = &xml[i + 1..];
            if let Some(end_bracket) = rest.find('>') {
                let tag_raw = rest[..end_bracket].trim();
                let is_closing = tag_raw.starts_with('/');
                let tag_name = tag_raw
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('/')
                    .to_ascii_lowercase();

                match tag_name.as_str() {
                    "book-title" => {
                        if is_closing && title.is_empty() {
                            title = current_p.trim().to_string();
                            current_p.clear();
                        }
                    }
                    "author" => {
                        if is_closing && author.is_empty() {
                            author = current_p.trim().to_string();
                            current_p.clear();
                        }
                    }
                    "title" => {
                        in_title = !is_closing;
                        if is_closing {
                            current_title = current_p.trim().to_string();
                            current_p.clear();
                        }
                    }
                    "section" if is_closing => {
                        if !current_paragraphs.is_empty() || !current_title.is_empty() {
                            chapters.push(BookChapter {
                                id: None,
                                title: std::mem::take(&mut current_title),
                                paragraphs: std::mem::take(&mut current_paragraphs),
                            });
                        }
                    }
                    "p" | "v" | "subtitle" => {
                        in_p = !is_closing;
                        if is_closing {
                            let p_clean = normalize_whitespace(&current_p);
                            if !p_clean.is_empty() {
                                if in_title {
                                    current_title = p_clean;
                                } else {
                                    current_paragraphs.push(p_clean);
                                }
                            }
                            current_p.clear();
                        }
                    }
                    _ => {}
                }
                i += 1 + end_bracket + 1;
                continue;
            }
        }

        if in_p || title.is_empty() || author.is_empty() {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            if let Ok(text_slice) = std::str::from_utf8(&bytes[start..i]) {
                let unescaped = quick_xml::escape::unescape(text_slice)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| text_slice.to_string());
                current_p.push_str(&unescaped);
            }
        } else {
            i += 1;
        }
    }

    if !current_paragraphs.is_empty() || !current_title.is_empty() {
        chapters.push(BookChapter {
            id: None,
            title: current_title,
            paragraphs: current_paragraphs,
        });
    }

    (title, author, chapters)
}

fn local_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
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

fn normalize_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !last_was_space && !result.is_empty() {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }
    result.trim().to_string()
}
