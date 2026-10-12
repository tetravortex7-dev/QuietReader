use std::collections::HashMap;
use std::path::Path;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use crate::document::{BookChapter, BookMetadata, Document, TocItem};

pub struct Fb2Document {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
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
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let text = decode_text(bytes);
        Self::parse_xml(&text)
    }

    fn parse_xml(xml: &str) -> Result<Self, String> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut book_title = String::new();
        let mut author_first = String::new();
        let mut author_middle = String::new();
        let mut author_last = String::new();

        // Metadata flags
        let mut in_title_info = false;
        let mut in_author = false;
        let mut in_book_title = false;
        let mut in_first_name = false;
        let mut in_middle_name = false;
        let mut in_last_name = false;

        // Content tree building
        let mut bodies: Vec<BodyNode> = Vec::new();
        let mut current_body: Option<BodyNode> = None;
        let mut section_stack: Vec<SectionNode> = Vec::new();

        // Explicit TOC links from <body name="toc">
        let mut explicit_toc_links: Vec<TocLink> = Vec::new();
        let mut current_link_href: Option<String> = None;
        let mut current_link_text = String::new();

        // Tag tracking
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

                        "title" => {
                            // Only consider title if direct child of section or body, not inside poem/cite/epigraph
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
                            // Link inside body name="toc"
                            if let Some(ref body) = current_body {
                                if body.name.as_deref().unwrap_or("").eq_ignore_ascii_case("toc") {
                                    for attr in e.attributes().flatten() {
                                        let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                                        if key == "href" {
                                            current_link_href = Some(attr.value.to_string());
                                            current_link_text.clear();
                                        }
                                    }
                                }
                            }
                        }

                        _ => {}
                    }

                    tag_stack.push(tag_name);
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
                            if let (Some(href), false) = (current_link_href.take(), current_link_text.trim().is_empty()) {
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
                                    title_paragraphs.push(current_title_paragraph.trim().to_string());
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
                            // If any sections were unclosed, pop them
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

                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("XML parsing error: {e}")),
                _ => {}
            }
            buf.clear();
        }

        // Close any remaining body if XML didn't have </body
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
        let author = if author_parts.is_empty() {
            "Неизвестный автор".to_string()
        } else {
            author_parts.join(" ")
        };

        let title = if book_title.trim().is_empty() {
            "Без названия".to_string()
        } else {
            book_title.trim().to_string()
        };

        // Flatten document bodies and sections into chapters and TOC tree
        let (chapters, mut toc) = build_chapters_and_toc(bodies);

        // If auto-generated TOC is empty, but we have explicit TOC links from <body name="toc">:
        if toc.is_empty() && !explicit_toc_links.is_empty() {
            let mut id_map: HashMap<String, usize> = HashMap::new();
            for (idx, ch) in chapters.iter().enumerate() {
                if let Some(ref id) = ch.id {
                    id_map.insert(id.clone(), idx);
                }
            }

            for link in explicit_toc_links {
                let target_id = link.href.strip_prefix('#').unwrap_or(&link.href);
                if let Some(&chapter_idx) = id_map.get(target_id) {
                    toc.push(TocItem {
                        id: Some(target_id.to_string()),
                        title: link.title,
                        chapter_index: chapter_idx,
                        children: Vec::new(),
                    });
                }
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
}

fn build_chapters_and_toc(bodies: Vec<BodyNode>) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut ctx = FlattenContext {
        chapters: Vec::new(),
    };
    let mut root_toc: Vec<TocItem> = Vec::new();

    for body in bodies {
        // Skip body name="toc" from text rendering
        if body.name.as_deref().unwrap_or("").eq_ignore_ascii_case("toc") {
            continue;
        }

        let body_has_direct_text = body.direct_paragraphs.iter().any(|p| !p.trim().is_empty());
        let body_has_title = body.title.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false);

        let mut body_start_chapter: Option<usize> = None;

        // If body has direct text, create a preamble chapter for it
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
            let (sec_first_ch, sec_tocs) = flatten_section(section, &mut ctx);
            if body_start_chapter.is_none() {
                body_start_chapter = sec_first_ch;
            }
            body_sections_toc.extend(sec_tocs);
        }

        if body_has_title {
            let title = body.title.unwrap();
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

    if ctx.chapters.is_empty() {
        ctx.chapters.push(BookChapter {
            id: None,
            title: String::new(),
            paragraphs: vec!["[Документ не содержит текста]".to_string()],
        });
    }

    (ctx.chapters, root_toc)
}

fn flatten_section(
    mut section: SectionNode,
    ctx: &mut FlattenContext,
) -> (Option<usize>, Vec<TocItem>) {
    let has_own_title = section.title.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false);

    // If section has title, but no direct paragraphs, check if its leading subsections are untitled leaf sections.
    // In FB2, chapter intro/preamble is commonly put in an untitled subsection before titled sub-sessions/sections.
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

    let has_own_paragraphs = section.paragraphs.iter().any(|p| !p.trim().is_empty());
    let mut own_chapter_idx: Option<usize> = None;

    // Create a chapter for this section if it has paragraphs,
    // or if it is a leaf section with a title.
    // Container sections without direct paragraphs must NOT create empty chapters.
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
        let (sub_first_ch, sub_tocs) = flatten_section(sub, ctx);
        if first_child_chapter.is_none() {
            first_child_chapter = sub_first_ch;
        }
        child_tocs.extend(sub_tocs);
    }

    let effective_chapter_idx = own_chapter_idx.or(first_child_chapter);

    if has_own_title {
        let title = section.title.unwrap();
        let chapter_index = effective_chapter_idx.unwrap_or(0);
        let toc_item = TocItem {
            id: section.id,
            title,
            chapter_index,
            children: child_tocs,
        };
        (effective_chapter_idx, vec![toc_item])
    } else {
        // Section has NO title: do not create an empty TOC item,
        // but promote child TOC items to the parent level!
        (effective_chapter_idx, child_tocs)
    }
}

fn local_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
}

fn decode_text(bytes: &[u8]) -> String {
    // Check BOM first
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }

    // Inspect first 300 bytes for <?xml ... encoding="..."?>
    let probe_len = bytes.len().min(300);
    let probe_str = String::from_utf8_lossy(&bytes[..probe_len]);

    if let Some(enc_label) = find_xml_encoding(&probe_str) {
        if let Some(encoding) = encoding_rs::Encoding::for_label(enc_label.as_bytes()) {
            let (decoded, _, _) = encoding.decode(bytes);
            return decoded.into_owned();
        }
    }

    // Default to UTF-8
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

