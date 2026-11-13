use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;

use super::entities::{
    decode_attr_value, decode_numeric_entity, decode_xml_ref, named_html_entity,
};
use super::local_name;

fn is_void_html_tag(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn push_collapsed_html_text(dst: &mut String, text: &str) {
    let mut pending_space = false;
    for ch in text.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r') {
            if !dst.is_empty() && !dst.ends_with([' ', '\n']) {
                pending_space = true;
            }
        } else {
            if pending_space {
                dst.push(' ');
                pending_space = false;
            }
            dst.push(ch);
        }
    }
    if pending_space && !dst.is_empty() && !dst.ends_with([' ', '\n']) {
        dst.push(' ');
    }
}

fn consume_chars_until(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    end_byte_inclusive: usize,
) {
    while let Some(&(next_idx, _)) = chars.peek() {
        if next_idx <= end_byte_inclusive {
            chars.next();
        } else {
            break;
        }
    }
}

pub fn sanitize_xml_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if c == '<'
            && input[idx..].starts_with("<![CDATA[")
            && let Some(end_rel) = input[idx + 9..].find("]]>")
        {
            let cdata_end = idx + 9 + end_rel + 3;
            out.push_str(&input[idx..cdata_end]);
            consume_chars_until(&mut chars, cdata_end - 1);
            continue;
        }
        if c == '&' {
            let rest = &input[idx + 1..];
            if let Some(semi_pos) = rest.find(';')
                && semi_pos <= 12
            {
                let entity_name = &rest[..semi_pos];
                if matches!(entity_name, "amp" | "lt" | "gt" | "quot" | "apos")
                    || decode_numeric_entity(entity_name).is_some()
                {
                    out.push('&');
                    out.push_str(entity_name);
                    out.push(';');
                    consume_chars_until(&mut chars, idx + 1 + semi_pos);
                    continue;
                }
                if let Some(repl) = named_html_entity(entity_name) {
                    out.push_str(repl);
                    consume_chars_until(&mut chars, idx + 1 + semi_pos);
                    continue;
                }
            }
            out.push_str("&amp;");
        } else {
            out.push(c);
        }
    }
    out
}

pub fn flush_p(current_p: &mut String, paragraphs: &mut Vec<String>) {
    if !current_p.trim().is_empty() {
        paragraphs.push(current_p.trim().to_string());
        current_p.clear();
    }
}

pub fn extract_html_lenient(html: &str) -> (String, Vec<String>) {
    let mut title = String::new();
    let mut paragraphs = Vec::new();
    let mut current_p = String::new();
    let mut in_title = false;
    let mut skip = false;
    let mut i = 0;
    let bytes = html.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rest = &html[i + 1..];
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
                    "script" | "style" | "head" => skip = !is_closing,
                    "title" => {
                        in_title = !is_closing;
                        skip = false;
                    }
                    "p" | "div" | "blockquote" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_p(&mut current_p, &mut paragraphs)
                    }
                    "br" => current_p.push('\n'),
                    "hr" => {
                        flush_p(&mut current_p, &mut paragraphs);
                        paragraphs.push("* * *".to_string());
                    }
                    "img" | "image" => {
                        if !skip && !in_title {
                            let reference = tag_attribute(tag_raw, "recindex")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                                .filter(|index| *index > 0)
                                .map(|index| format!("mobi_rec_{index}"))
                                .or_else(|| {
                                    tag_attribute(tag_raw, "src")
                                        .or_else(|| tag_attribute(tag_raw, "href"))
                                        .filter(|value| {
                                            !value.is_empty()
                                                && !value.starts_with("data:")
                                                && !value.starts_with("http")
                                        })
                                });
                            if let Some(reference) = reference {
                                flush_p(&mut current_p, &mut paragraphs);
                                paragraphs.push(crate::document::image_token(&reference));
                            }
                        }
                    }
                    "b" | "strong" => {
                        if !skip && !in_title {
                            if is_closing {
                                current_p.push_str("</b>");
                            } else {
                                current_p.push_str("<b>");
                            }
                        }
                    }
                    "i" | "em" => {
                        if !skip && !in_title {
                            if is_closing {
                                current_p.push_str("</i>");
                            } else {
                                current_p.push_str("<i>");
                            }
                        }
                    }
                    _ => {}
                }
                i += 1 + end_bracket + 1;
                continue;
            }
        }

        if !skip {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            if let Ok(text_slice) = std::str::from_utf8(&bytes[start..i]) {
                let unescaped = quick_xml::escape::unescape(text_slice)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| text_slice.to_string());
                let has_leading = unescaped.starts_with(char::is_whitespace);
                let has_trailing = unescaped.ends_with(char::is_whitespace);
                let trimmed = unescaped.trim();
                if !trimmed.is_empty() {
                    if in_title && title.is_empty() {
                        title = trimmed.to_string();
                    } else {
                        if has_leading
                            && !current_p.is_empty()
                            && !current_p.ends_with(' ')
                            && !current_p.ends_with('\n')
                            && !current_p.ends_with("<b>")
                            && !current_p.ends_with("<i>")
                        {
                            current_p.push(' ');
                        }
                        current_p.push_str(trimmed);
                        if has_trailing && !current_p.ends_with(' ') {
                            current_p.push(' ');
                        }
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    flush_p(&mut current_p, &mut paragraphs);
    (title, paragraphs)
}

#[derive(Default)]
struct HtmlAttributes {
    id: Option<String>,
    href: Option<String>,
    src: Option<String>,
    recindex: Option<String>,
    kind: String,
    role: String,
    class: String,
}

struct LinkCandidate {
    file: String,
    fragment: String,
    start: usize,
}

pub struct NoteLink {
    pub id: String,
    pub file: String,
}

pub struct ParsedChapter {
    pub title: String,
    pub paragraphs: Vec<String>,
    pub anchors: Vec<String>,
    pub anchor_paragraphs: HashMap<String, usize>,
    pub notes: HashMap<String, String>,
    pub note_links: Vec<NoteLink>,
}

fn read_attributes(e: &quick_xml::events::BytesStart<'_>) -> HtmlAttributes {
    let mut attrs = HtmlAttributes::default();
    for attr in e.attributes().flatten() {
        let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
        let value = decode_attr_value(&attr);
        match key.as_str() {
            "id" | "name" => attrs.id = Some(value),
            "href" => attrs.href = Some(value),
            "src" => attrs.src = Some(value),
            "recindex" => attrs.recindex = Some(value),
            "type" => attrs.kind = value,
            "role" => attrs.role = value,
            "class" => attrs.class = value,
            _ => {}
        }
    }
    attrs
}

fn tag_attribute(tag_raw: &str, name: &str) -> Option<String> {
    let lower = tag_raw.to_ascii_lowercase();
    let pattern = format!("{name}=");
    let index = lower.find(&pattern)?;
    let rest = tag_raw[index + pattern.len()..].trim_start();
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

fn image_reference(attrs: &HtmlAttributes) -> Option<String> {
    if let Some(recindex) = &attrs.recindex
        && let Ok(index) = recindex.trim().parse::<usize>()
        && index > 0
    {
        return Some(format!("mobi_rec_{index}"));
    }
    if let Some(src) = attrs.src.as_ref().or(attrs.href.as_ref()) {
        let trimmed = src.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("data:") && !trimmed.starts_with("http") {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn is_noteref_link(attrs: &HtmlAttributes, tag: &str) -> bool {
    tag == "a"
        && (attrs.kind.to_ascii_lowercase().contains("noteref")
            || attrs.role.eq_ignore_ascii_case("doc-noteref")
            || attrs
                .class
                .split_whitespace()
                .any(|class| class.to_ascii_lowercase().contains("noteref")))
}

fn note_key(value: &str) -> String {
    value
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase()
}

fn is_note_container(attrs: &HtmlAttributes, tag: &str) -> bool {
    let Some(id) = &attrs.id else {
        return false;
    };
    if id.is_empty() {
        return false;
    }
    let kind = attrs.kind.to_ascii_lowercase();
    let class = attrs.class.to_ascii_lowercase();
    let id_lower = note_key(id);
    let is_footnote_kind = kind.contains("footnote") || kind.contains("endnote");
    let is_footnote_role = attrs.role.eq_ignore_ascii_case("doc-footnote");
    let is_footnote_class = class
        .split_whitespace()
        .any(|class| class.contains("footnote") || class.contains("endnote") || class == "note");
    let is_notes_container = matches!(
        id_lower.as_str(),
        "notes" | "footnotes" | "endnotes" | "comments" | "сноски" | "примечания" | "комментарии"
    );
    let is_note_id = !is_notes_container
        && (id_lower.starts_with("note")
            || id_lower.starts_with("footnote")
            || id_lower.starts_with("endnote")
            || id_lower.starts_with("comment")
            || id_lower.starts_with("fn")
            || id_lower.starts_with("ftn")
            || id_lower.starts_with("сноск")
            || id_lower.starts_with("примеч")
            || (id_lower.starts_with('n')
                && id_lower.len() > 1
                && id_lower[1..].chars().all(|c| c.is_ascii_digit())));
    is_footnote_kind
        || is_footnote_role
        || is_footnote_class
        || is_note_id
            && matches!(
                tag,
                "aside" | "div" | "p" | "li" | "section" | "article" | "blockquote"
            )
}

fn note_fragment_hint(fragment: &str) -> bool {
    let lower = note_key(fragment);
    lower.contains("note")
        || lower.contains("comment")
        || lower.contains("сноск")
        || lower.contains("примеч")
        || lower.starts_with("fn")
        || lower.starts_with("ftn")
}

fn looks_like_note_marker(text: &str) -> bool {
    let trimmed = text.trim();
    let inner = trimmed
        .trim_matches(|c| matches!(c, '[' | ']' | '(' | ')' | '.' | ',' | ';' | ':' | '!' | '?'));
    if inner.is_empty() {
        return false;
    }
    let marker = inner.trim();
    if marker.is_empty() {
        return false;
    }
    if marker.chars().all(|c| c.is_ascii_digit() || c.is_numeric()) {
        return true;
    }
    if marker.chars().count() <= 4
        && marker
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '.' || c == '*')
    {
        return true;
    }
    matches!(marker, "†" | "‡" | "††" | "‡‡")
}

pub(crate) fn looks_like_footnote_link(fragment: &str, text: &str) -> bool {
    if !looks_like_note_marker(text) {
        return false;
    }
    if note_fragment_hint(fragment) {
        return true;
    }
    let short_fragment = fragment.chars().count() <= 6
        && fragment.chars().last().is_some_and(|c| c.is_ascii_digit());
    short_fragment
}

fn is_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "p" | "div"
            | "blockquote"
            | "li"
            | "section"
            | "aside"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "br"
    )
}

fn normalize_note_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse_html_chapter_full(html: &str) -> ParsedChapter {
    let sanitized = sanitize_xml_entities(html);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;

    let mut title = String::new();
    let mut title_buf = String::new();
    let mut paragraphs = Vec::new();
    let mut anchors = Vec::new();
    let mut anchor_paragraphs = HashMap::new();
    let mut notes = HashMap::new();
    let mut note_links: Vec<NoteLink> = Vec::new();
    let mut current_p = String::new();
    let mut current_note_ref: Option<String> = None;
    let mut current_link: Option<LinkCandidate> = None;
    let mut note_id: Option<String> = None;
    let mut note_text = String::new();
    let mut note_depth = 0usize;
    let mut skip_depth = 0usize;
    let mut in_head = false;
    let (mut in_title_tag, mut in_heading_tag) = (false, false);
    let mut buf = Vec::new();
    let mut had_error = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                let is_void = is_void_html_tag(&tag);
                let attrs = read_attributes(e);
                if let Some(id) = &attrs.id {
                    anchors.push(id.clone());
                    anchor_paragraphs
                        .entry(id.clone())
                        .or_insert(paragraphs.len());
                    anchor_paragraphs
                        .entry(super::percent_decode_str(id))
                        .or_insert(paragraphs.len());
                }
                if matches!(tag.as_str(), "script" | "style") && !is_void {
                    skip_depth = skip_depth.saturating_add(1);
                } else if tag == "head" {
                    in_head = true;
                } else if tag == "body" {
                    in_head = false;
                }
                let noteref = is_noteref_link(&attrs, &tag);
                if tag == "a"
                    && note_id.is_none()
                    && skip_depth == 0
                    && !in_head
                    && let Some(href) = &attrs.href
                    && let Some(fragment) = href.split('#').nth(1)
                    && !fragment.is_empty()
                {
                    let fragment = super::percent_decode_str(fragment);
                    if noteref {
                        current_note_ref = Some(fragment.clone());
                    }
                    current_link = Some(LinkCandidate {
                        file: href.split('#').next().unwrap_or_default().to_string(),
                        fragment,
                        start: current_p.len(),
                    });
                }
                let starts_note = !noteref && !is_void && is_note_container(&attrs, &tag);
                if note_id.is_some() {
                    if !is_void {
                        note_depth = note_depth.saturating_add(1);
                    }
                    if skip_depth == 0 && is_block_tag(&tag) {
                        note_text.push(' ');
                    }
                } else if starts_note && skip_depth == 0 && !in_head {
                    note_id = attrs.id.clone();
                    note_text.clear();
                    note_depth = 1;
                }
                if note_id.is_some() {
                    if skip_depth == 0 && tag == "br" {
                        note_text.push(' ');
                    }
                } else if skip_depth == 0 {
                    match tag.as_str() {
                        "img" | "image" => {
                            if !in_head
                                && !in_title_tag
                                && !in_heading_tag
                                && let Some(reference) = image_reference(&attrs)
                            {
                                flush_p(&mut current_p, &mut paragraphs);
                                paragraphs.push(crate::document::image_token(&reference));
                            }
                        }
                        "title" => {
                            in_title_tag = true;
                            title_buf.clear();
                        }
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if !in_head => {
                            flush_p(&mut current_p, &mut paragraphs);
                            in_heading_tag = true;
                        }
                        "p" | "div" | "blockquote" | "li" if !in_head => {
                            flush_p(&mut current_p, &mut paragraphs);
                        }
                        "br" if !in_head => {
                            while current_p.ends_with(' ') {
                                current_p.pop();
                            }
                            current_p.push('\n');
                        }
                        "b" | "strong" => {
                            if !in_head
                                && !in_title_tag
                                && !in_heading_tag
                                && current_note_ref.is_none()
                            {
                                current_p.push_str("<b>");
                            }
                        }
                        "i" | "em" => {
                            if !in_head
                                && !in_title_tag
                                && !in_heading_tag
                                && current_note_ref.is_none()
                            {
                                current_p.push_str("<i>");
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                let attrs = read_attributes(e);
                if let Some(id) = &attrs.id {
                    anchors.push(id.clone());
                    anchor_paragraphs
                        .entry(id.clone())
                        .or_insert(paragraphs.len());
                    anchor_paragraphs
                        .entry(super::percent_decode_str(id))
                        .or_insert(paragraphs.len());
                }
                if note_id.is_some() {
                    if skip_depth == 0 && tag == "br" {
                        note_text.push(' ');
                    }
                } else if skip_depth == 0 && !in_head {
                    match tag.as_str() {
                        "br" => {
                            while current_p.ends_with(' ') {
                                current_p.pop();
                            }
                            current_p.push('\n');
                        }
                        "hr" => {
                            flush_p(&mut current_p, &mut paragraphs);
                            paragraphs.push("* * *".to_string());
                        }
                        "img" | "image" => {
                            if !in_title_tag
                                && !in_heading_tag
                                && let Some(reference) = image_reference(&attrs)
                            {
                                flush_p(&mut current_p, &mut paragraphs);
                                paragraphs.push(crate::document::image_token(&reference));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(ref e)) if skip_depth == 0 => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                if !text.is_empty() {
                    if in_title_tag {
                        title_buf.push_str(&text);
                    } else if in_head {
                    } else if note_id.is_some() {
                        note_text.push_str(&text);
                    } else if current_note_ref.is_none() {
                        push_collapsed_html_text(&mut current_p, &text);
                    }
                }
            }
            Ok(Event::CData(ref e)) if skip_depth == 0 => {
                let text = e.as_ref();
                if !text.is_empty() {
                    if in_title_tag {
                        title_buf.push_str(text);
                    } else if in_head {
                    } else if note_id.is_some() {
                        note_text.push_str(text);
                    } else if current_note_ref.is_none() {
                        push_collapsed_html_text(&mut current_p, text);
                    }
                }
            }
            Ok(Event::GeneralRef(ref e)) if skip_depth == 0 => {
                if let Some(repl) = decode_xml_ref(e.as_ref()) {
                    if in_title_tag {
                        title_buf.push_str(&repl);
                    } else if in_head {
                    } else if note_id.is_some() {
                        note_text.push_str(&repl);
                    } else if current_note_ref.is_none() {
                        push_collapsed_html_text(&mut current_p, &repl);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                let is_void = is_void_html_tag(&tag);
                if matches!(tag.as_str(), "script" | "style") && !is_void {
                    skip_depth = skip_depth.saturating_sub(1);
                } else if tag == "head" {
                    in_head = false;
                }
                if note_id.is_some() {
                    if !is_void {
                        if skip_depth == 0 && is_block_tag(&tag) {
                            note_text.push(' ');
                        }
                        note_depth = note_depth.saturating_sub(1);
                        if note_depth == 0 {
                            if let Some(id) = note_id.take() {
                                let cleaned = normalize_note_text(&note_text);
                                if !cleaned.is_empty() {
                                    notes.insert(id, cleaned);
                                }
                            }
                            note_text.clear();
                        }
                    }
                } else {
                    if tag == "a" {
                        let explicit_ref = current_note_ref.take();
                        let link = current_link.take();
                        if !in_title_tag && !in_heading_tag && skip_depth == 0 && !in_head {
                            if let Some(note_ref) = explicit_ref {
                                note_links.push(NoteLink {
                                    id: note_ref.clone(),
                                    file: link
                                        .as_ref()
                                        .map(|candidate| candidate.file.clone())
                                        .unwrap_or_default(),
                                });
                                current_p.push_str(&crate::document::footnote_token(&note_ref));
                            } else if let Some(link) = link {
                                let raw_link_text = current_p.get(link.start..).unwrap_or_default();
                                let link_text =
                                    crate::document::strip_formatting_tags(raw_link_text);
                                if looks_like_footnote_link(&link.fragment, &link_text) {
                                    note_links.push(NoteLink {
                                        id: link.fragment.clone(),
                                        file: link.file,
                                    });
                                    current_p.truncate(link.start);
                                    current_p
                                        .push_str(&crate::document::footnote_token(&link.fragment));
                                }
                            }
                        }
                    }
                    if skip_depth == 0 {
                        match tag.as_str() {
                            "title" => {
                                if in_title_tag && title.is_empty() {
                                    let collapsed =
                                        title_buf.split_whitespace().collect::<Vec<_>>().join(" ");
                                    if !collapsed.is_empty() {
                                        title = collapsed;
                                    }
                                }
                                in_title_tag = false;
                                title_buf.clear();
                            }
                            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                                if in_heading_tag
                                    && title.is_empty()
                                    && !current_p.trim().is_empty()
                                {
                                    title = current_p.trim().to_string();
                                }
                                flush_p(&mut current_p, &mut paragraphs);
                                in_heading_tag = false;
                            }
                            "p" | "div" | "blockquote" | "li" if !in_head => {
                                flush_p(&mut current_p, &mut paragraphs);
                            }
                            "b" | "strong" => {
                                if !in_head
                                    && !in_title_tag
                                    && !in_heading_tag
                                    && current_note_ref.is_none()
                                {
                                    current_p.push_str("</b>");
                                }
                            }
                            "i" | "em" => {
                                if !in_head
                                    && !in_title_tag
                                    && !in_heading_tag
                                    && current_note_ref.is_none()
                                {
                                    current_p.push_str("</i>");
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => {
                had_error = true;
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    if let Some(id) = note_id.take() {
        let cleaned = normalize_note_text(&note_text);
        if !cleaned.is_empty() {
            notes.insert(id, cleaned);
        }
    }
    flush_p(&mut current_p, &mut paragraphs);

    if had_error || paragraphs.is_empty() {
        let (fb_title, fb_paras) = extract_html_lenient(html);
        if !fb_paras.is_empty() {
            if paragraphs.is_empty() || paragraphs.len() < fb_paras.len() / 2 {
                paragraphs = fb_paras;
            }
            if title.is_empty() {
                title = fb_title;
            }
        }
    }
    for link in &note_links {
        if link.file.is_empty() && !notes.contains_key(&link.id) {
            if let Some(text) = extract_note_text_by_id(html, &link.id) {
                notes.insert(link.id.clone(), text);
            }
        }
    }

    ParsedChapter {
        title,
        paragraphs,
        anchors,
        anchor_paragraphs,
        notes,
        note_links,
    }
}

pub fn parse_html_chapter_with_notes(
    html: &str,
) -> (String, Vec<String>, Vec<String>, HashMap<String, String>) {
    let parsed = parse_html_chapter_full(html);
    (
        parsed.title,
        parsed.paragraphs,
        parsed.anchors,
        parsed.notes,
    )
}

#[allow(dead_code)]
pub fn parse_html_chapter(html: &str) -> (String, Vec<String>, Vec<String>) {
    let parsed = parse_html_chapter_full(html);
    (parsed.title, parsed.paragraphs, parsed.anchors)
}

pub fn extract_note_text_by_id(html: &str, target_id: &str) -> Option<String> {
    if target_id.is_empty() {
        return None;
    }
    let sanitized = sanitize_xml_entities(html);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;

    let mut active = false;
    let mut depth = 0usize;
    let mut skip_depth = 0usize;
    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                let is_void = is_void_html_tag(&tag);
                let attrs = read_attributes(e);
                if !active {
                    if !is_void && matches_id(&attrs, target_id) {
                        active = true;
                        depth = 1;
                    }
                } else {
                    if !is_void {
                        depth = depth.saturating_add(1);
                    }
                    if matches!(tag.as_str(), "script" | "style") && !is_void {
                        skip_depth = skip_depth.saturating_add(1);
                    } else if skip_depth == 0 && is_block_tag(&tag) {
                        flush_note_block(&mut current, &mut blocks);
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                if active && skip_depth == 0 {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if matches!(tag.as_str(), "br" | "hr") {
                        flush_note_block(&mut current, &mut blocks);
                    }
                }
            }
            Ok(Event::Text(ref e)) if active && skip_depth == 0 => {
                let raw = e.as_ref();
                let text = quick_xml::escape::unescape(raw)
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                current.push_str(&text);
            }
            Ok(Event::CData(ref e)) if active && skip_depth == 0 => {
                current.push_str(e.as_ref());
            }
            Ok(Event::GeneralRef(ref e)) if active && skip_depth == 0 => {
                if let Some(repl) = decode_xml_ref(e.as_ref()) {
                    current.push_str(&repl);
                }
            }
            Ok(Event::End(ref e)) => {
                if active {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    let is_void = is_void_html_tag(&tag);
                    if !is_void {
                        if skip_depth > 0 {
                            if matches!(tag.as_str(), "script" | "style") {
                                skip_depth -= 1;
                            }
                        } else if is_block_tag(&tag) {
                            flush_note_block(&mut current, &mut blocks);
                        }
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            flush_note_block(&mut current, &mut blocks);
                            break;
                        }
                    }
                }
            }
            Ok(Event::Eof) => {
                if active {
                    flush_note_block(&mut current, &mut blocks);
                }
                break;
            }
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if blocks.len() > 1 && is_note_number_block(&blocks[0]) {
        blocks.remove(0);
    }
    let text = blocks.join(" ");
    if text.is_empty() { None } else { Some(text) }
}

fn matches_id(attrs: &HtmlAttributes, target_id: &str) -> bool {
    attrs
        .id
        .as_deref()
        .is_some_and(|id| id == target_id || super::percent_decode_str(id) == target_id)
}

fn flush_note_block(current: &mut String, blocks: &mut Vec<String>) {
    let text = normalize_note_text(current);
    if !text.is_empty() {
        blocks.push(text);
    }
    current.clear();
}

fn is_note_number_block(text: &str) -> bool {
    let trimmed = text
        .trim()
        .trim_matches(|c: char| matches!(c, '[' | ']' | '(' | ')' | '.' | ',' | ';' | ':'));
    !trimmed.is_empty()
        && trimmed.chars().count() <= 4
        && trimmed.chars().all(|c| c.is_ascii_digit())
}
