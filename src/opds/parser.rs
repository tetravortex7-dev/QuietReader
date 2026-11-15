use super::types::{OpdsAcquisitionLink, OpdsEntry, OpdsFeed, OpdsLink};
use quick_xml::events::Event;
use quick_xml::events::attributes::Attribute;
use quick_xml::reader::Reader;
use url::Url;

pub fn resolve_relative_url(base: &str, relative: &str) -> String {
    let rel = relative.trim();
    if rel.is_empty() {
        return base.trim().to_string();
    }

    const LCURL_TOKEN: &str = "__QUIETREADER_OPDS_LCURL__";
    const RCURL_TOKEN: &str = "__QUIETREADER_OPDS_RCURL__";

    let safe_rel = rel.replace('{', LCURL_TOKEN).replace('}', RCURL_TOKEN);
    if let Ok(abs) = Url::parse(&safe_rel) {
        return abs
            .as_str()
            .replace(LCURL_TOKEN, "{")
            .replace(RCURL_TOKEN, "}");
    }

    let base_trimmed = base.trim();
    let base_url = Url::parse(base_trimmed).ok().or_else(|| {
        if !base_trimmed.contains("://") && !base_trimmed.is_empty() {
            Url::parse(&format!("http://{base_trimmed}")).ok()
        } else {
            None
        }
    });

    if let Some(base_parsed) = base_url {
        if let Ok(joined) = base_parsed.join(&safe_rel) {
            return joined
                .as_str()
                .replace(LCURL_TOKEN, "{")
                .replace(RCURL_TOKEN, "}");
        }
    }

    rel.to_string()
}

pub fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

pub fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in chunks.by_ref() {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(CHARSET[((n >> 18) & 63) as usize] as char);
        out.push(CHARSET[((n >> 12) & 63) as usize] as char);
        out.push(CHARSET[((n >> 6) & 63) as usize] as char);
        out.push(CHARSET[(n & 63) as usize] as char);
    }
    let rem = chunks.remainder();
    if rem.len() == 1 {
        let n = (rem[0] as u32) << 16;
        out.push(CHARSET[((n >> 18) & 63) as usize] as char);
        out.push(CHARSET[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem.len() == 2 {
        let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
        out.push(CHARSET[((n >> 18) & 63) as usize] as char);
        out.push(CHARSET[((n >> 12) & 63) as usize] as char);
        out.push(CHARSET[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

pub fn sanitize_filename(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = clean.trim().trim_matches('.');
    if trimmed.is_empty() {
        "book".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn detect_format_and_ext(mime: &str, href: &str) -> Option<(&'static str, &'static str)> {
    let m = mime.to_ascii_lowercase();
    let href_path = href.split(['?', '#']).next().unwrap_or(href);
    let h = href_path.to_ascii_lowercase();
    const FORMATS: &[(&[&str], &[&str], &str, &str)] = &[
        (&["epub"], &[".epub"], "EPUB", "epub"),
        (&["fictionbook", "fb2"], &[".fb2", ".fb2.zip"], "FB2", "fb2"),
        (&["x-mobi8", "azw3"], &[".azw3"], "AZW3", "azw3"),
        (&["mobipocket", "mobi"], &[".mobi", ".prc"], "MOBI", "mobi"),
        (&["pdf"], &[".pdf"], "PDF", "pdf"),
        (&["djvu"], &[".djvu", ".djv"], "DJVU", "djvu"),
        (&["cbz"], &[".cbz"], "CBZ", "cbz"),
        (&["cbr"], &[".cbr"], "CBR", "cbr"),
        (&["text/plain"], &[".txt"], "TXT", "txt"),
    ];
    for (mimes, exts, badge, ext) in FORMATS {
        if mimes.iter().any(|k| m.contains(k)) || exts.iter().any(|e| h.ends_with(e)) {
            if *ext == "fb2" && h.ends_with(".fb2.zip") {
                return Some(("FB2.ZIP", "fb2.zip"));
            }
            return Some((badge, ext));
        }
    }
    None
}

fn decode_html_entity(entity: &str) -> Option<String> {
    match entity {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" | "#39" => Some("'".to_string()),
        "nbsp" => Some(" ".to_string()),
        "mdash" => Some("—".to_string()),
        "ndash" => Some("–".to_string()),
        s if s.starts_with("#x") || s.starts_with("#X") => u32::from_str_radix(&s[2..], 16)
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string()),
        s if s.starts_with('#') => s[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string()),
        _ => None,
    }
}

fn is_block_break_tag(tag_raw: &str) -> bool {
    let trimmed = tag_raw.trim().trim_start_matches('/');
    let name = trimmed.split_whitespace().next().unwrap_or("");
    name.eq_ignore_ascii_case("p")
        || name.eq_ignore_ascii_case("div")
        || name.eq_ignore_ascii_case("br")
        || name.eq_ignore_ascii_case("hr")
        || name.eq_ignore_ascii_case("li")
        || name.eq_ignore_ascii_case("blockquote")
        || name.eq_ignore_ascii_case("h1")
        || name.eq_ignore_ascii_case("h2")
        || name.eq_ignore_ascii_case("h3")
        || name.eq_ignore_ascii_case("h4")
}

pub fn strip_html_tags(input: &str) -> String {
    let mut no_tags = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();
    for c in input.chars() {
        match c {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                if in_tag && is_block_break_tag(&tag_buf) && !no_tags.is_empty() {
                    if !no_tags.ends_with(char::is_whitespace) {
                        no_tags.push(' ');
                    }
                }
                in_tag = false;
                tag_buf.clear();
            }
            _ if in_tag => tag_buf.push(c),
            _ => no_tags.push(c),
        }
    }

    let mut out = String::with_capacity(no_tags.len());
    let mut chars = no_tags.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch == '&' {
            if let Some(semi_rel) = no_tags[idx + 1..].find(';') {
                if semi_rel <= 12 {
                    let entity = &no_tags[idx + 1..idx + 1 + semi_rel];
                    if let Some(decoded) = decode_html_entity(entity) {
                        out.push_str(&decoded);
                        for _ in 0..=semi_rel {
                            chars.next();
                        }
                        continue;
                    }
                }
            }
        }
        out.push(ch);
    }
    out.trim().to_string()
}

fn decode_attr_value(attr: &Attribute<'_>) -> String {
    attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
        .map(|v| v.into_owned())
        .unwrap_or_else(|_| attr.value.to_string())
}

fn pop_matching_tag(stack: &mut Vec<String>, closing: &str) {
    if let Some(pos) = stack
        .iter()
        .rposition(|tag| tag.eq_ignore_ascii_case(closing))
    {
        stack.truncate(pos);
    }
}

fn in_entry_element(stack: &[String], target: &str) -> bool {
    let Some(entry_pos) = stack.iter().rposition(|t| t.eq_ignore_ascii_case("entry")) else {
        return false;
    };
    stack[entry_pos + 1..]
        .iter()
        .any(|t| t.eq_ignore_ascii_case(target))
}

pub fn parse_opensearch_template(xml_str: &str, base_url: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                if name.as_ref().eq_ignore_ascii_case("url") {
                    let mut is_atom = false;
                    let mut template = None;
                    for attr in e.attributes().flatten() {
                        let key = attr.key.local_name();
                        if key.as_ref().eq_ignore_ascii_case("type") {
                            let val = decode_attr_value(&attr).to_ascii_lowercase();
                            if val.contains("atom+xml") || val.contains("opds") {
                                is_atom = true;
                            }
                        } else if key.as_ref().eq_ignore_ascii_case("template") {
                            template = Some(decode_attr_value(&attr));
                        }
                    }
                    if is_atom && let Some(tmpl) = template {
                        return Some(resolve_relative_url(base_url, &tmpl));
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

pub fn parse_opds_feed(xml_str: &str, base_url: &str) -> Result<OpdsFeed, String> {
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(false);

    let mut feed_title = String::new();
    let mut feed_id = String::new();
    let mut feed_icon_raw = String::new();
    let mut feed_links = Vec::new();
    let mut entries = Vec::new();

    let mut next_url = None;
    let mut prev_url = None;
    let mut search_template = None;
    let mut search_descriptor_url = None;

    let mut in_feed = false;
    let mut in_entry = false;
    let mut in_author = false;
    let mut tag_stack: Vec<String> = Vec::new();

    let mut cur_entry_id = String::new();
    let mut cur_entry_title = String::new();
    let mut cur_author_name = String::new();
    let mut cur_entry_authors: Vec<String> = Vec::new();
    let mut cur_entry_summary_raw = String::new();
    let mut cur_entry_updated_raw = String::new();
    let mut cur_entry_published_raw = String::new();
    let mut cur_entry_categories = Vec::new();
    let mut cur_entry_thumb = None;
    let mut cur_entry_acquisitions: Vec<OpdsAcquisitionLink> = Vec::new();
    let mut cur_entry_nav = None;

    let mut buf = Vec::new();

    macro_rules! append_active {
        ($txt:expr) => {
            if in_entry {
                if in_author
                    && tag_stack
                        .last()
                        .is_some_and(|t| t.eq_ignore_ascii_case("name"))
                {
                    cur_author_name.push_str($txt);
                } else if in_entry_element(&tag_stack, "summary")
                    || in_entry_element(&tag_stack, "content")
                {
                    cur_entry_summary_raw.push_str($txt);
                } else if !in_author && in_entry_element(&tag_stack, "title") {
                    cur_entry_title.push_str($txt);
                } else if !in_author
                    && tag_stack
                        .last()
                        .is_some_and(|t| t.eq_ignore_ascii_case("id"))
                {
                    cur_entry_id.push_str($txt.trim());
                } else if tag_stack
                    .last()
                    .is_some_and(|t| t.eq_ignore_ascii_case("updated"))
                {
                    cur_entry_updated_raw.push_str($txt.trim());
                } else if tag_stack
                    .last()
                    .is_some_and(|t| t.eq_ignore_ascii_case("published"))
                {
                    cur_entry_published_raw.push_str($txt.trim());
                }
            } else if in_feed {
                if !in_author
                    && tag_stack
                        .last()
                        .is_some_and(|t| t.eq_ignore_ascii_case("title"))
                {
                    feed_title.push_str($txt);
                } else if !in_author
                    && tag_stack
                        .last()
                        .is_some_and(|t| t.eq_ignore_ascii_case("id"))
                {
                    feed_id.push_str($txt.trim());
                } else if tag_stack.last().is_some_and(|t| {
                    t.eq_ignore_ascii_case("icon") || t.eq_ignore_ascii_case("logo")
                }) {
                    feed_icon_raw.push_str($txt.trim());
                }
            }
        };
    }

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.local_name().as_ref().to_string();
                if in_entry
                    && (in_entry_element(&tag_stack, "summary")
                        || in_entry_element(&tag_stack, "content"))
                    && is_block_break_tag(&name)
                    && !cur_entry_summary_raw.is_empty()
                    && !cur_entry_summary_raw.ends_with(char::is_whitespace)
                {
                    cur_entry_summary_raw.push(' ');
                }

                if name.eq_ignore_ascii_case("feed") {
                    in_feed = true;
                } else if name.eq_ignore_ascii_case("entry") {
                    in_entry = true;
                    cur_entry_id.clear();
                    cur_entry_title.clear();
                    cur_author_name.clear();
                    cur_entry_authors.clear();
                    cur_entry_summary_raw.clear();
                    cur_entry_updated_raw.clear();
                    cur_entry_published_raw.clear();
                    cur_entry_categories.clear();
                    cur_entry_thumb = None;
                    cur_entry_acquisitions.clear();
                    cur_entry_nav = None;
                } else if name.eq_ignore_ascii_case("author") {
                    in_author = true;
                    cur_author_name.clear();
                }
                tag_stack.push(name);
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                let name_str = name.as_ref();
                if name_str.eq_ignore_ascii_case("link") {
                    let mut href = String::new();
                    let mut rel = String::new();
                    let mut mime_type = String::new();
                    let mut title = String::new();
                    let mut template = None;
                    let mut length = None;

                    for attr in e.attributes().flatten() {
                        let k = attr.key.local_name();
                        let v = decode_attr_value(&attr);
                        if k.as_ref().eq_ignore_ascii_case("href") {
                            href = v;
                        } else if k.as_ref().eq_ignore_ascii_case("rel") {
                            rel = v;
                        } else if k.as_ref().eq_ignore_ascii_case("type") {
                            mime_type = v;
                        } else if k.as_ref().eq_ignore_ascii_case("title") {
                            title = v;
                        } else if k.as_ref().eq_ignore_ascii_case("template") {
                            template = Some(v);
                        } else if k.as_ref().eq_ignore_ascii_case("length") {
                            length = v.trim().parse::<u64>().ok().filter(|&n| n > 0);
                        }
                    }

                    let resolved_href = resolve_relative_url(base_url, &href);

                    if in_entry {
                        let rel_lower = rel.to_ascii_lowercase();
                        let type_lower = mime_type.to_ascii_lowercase();

                        if rel_lower.contains("image")
                            || rel_lower.contains("thumbnail")
                            || rel_lower.contains("cover")
                        {
                            if cur_entry_thumb.is_none() || rel_lower.contains("thumbnail") {
                                cur_entry_thumb = Some(resolved_href.clone());
                            }
                        } else if rel_lower.contains("acquisition")
                            || (rel_lower.is_empty()
                                && detect_format_and_ext(&mime_type, &href).is_some())
                        {
                            if let Some((badge, ext)) = detect_format_and_ext(&mime_type, &href) {
                                cur_entry_acquisitions.push(OpdsAcquisitionLink {
                                    href: resolved_href.clone(),
                                    mime_type: mime_type.clone(),
                                    title: if title.is_empty() {
                                        badge.to_string()
                                    } else {
                                        title.clone()
                                    },
                                    format_label: badge.to_string(),
                                    extension: ext.to_string(),
                                    length,
                                });
                            }
                        } else if rel_lower.contains("subsection")
                            || type_lower.contains("atom+xml")
                            || (rel_lower == "alternate" && type_lower.contains("opds"))
                        {
                            if cur_entry_nav.is_none() {
                                cur_entry_nav = Some(resolved_href.clone());
                            }
                        }
                    } else if in_feed {
                        let rel_lower = rel.to_ascii_lowercase();
                        let type_lower = mime_type.to_ascii_lowercase();

                        if rel_lower == "next" {
                            next_url = Some(resolved_href.clone());
                        } else if rel_lower == "prev" || rel_lower == "previous" {
                            prev_url = Some(resolved_href.clone());
                        } else if rel_lower == "search" {
                            if let Some(tmpl) = template {
                                search_template = Some(resolve_relative_url(base_url, &tmpl));
                            } else if href.contains("{searchTerms}") {
                                search_template = Some(resolved_href.clone());
                            } else if type_lower.contains("opensearchdescription") {
                                search_descriptor_url = Some(resolved_href.clone());
                            }
                        }

                        feed_links.push(OpdsLink {
                            rel,
                            href: resolved_href,
                            link_type: if mime_type.is_empty() {
                                None
                            } else {
                                Some(mime_type)
                            },
                            title: if title.is_empty() { None } else { Some(title) },
                        });
                    }
                } else if name_str.eq_ignore_ascii_case("category") && in_entry {
                    for attr in e.attributes().flatten() {
                        let k = attr.key.local_name();
                        if k.as_ref().eq_ignore_ascii_case("label")
                            || k.as_ref().eq_ignore_ascii_case("term")
                        {
                            let val = decode_attr_value(&attr).trim().to_string();
                            if !val.is_empty() && !cur_entry_categories.contains(&val) {
                                cur_entry_categories.push(val);
                                break;
                            }
                        }
                    }
                } else if in_entry
                    && (in_entry_element(&tag_stack, "summary")
                        || in_entry_element(&tag_stack, "content"))
                    && is_block_break_tag(name_str)
                    && !cur_entry_summary_raw.is_empty()
                    && !cur_entry_summary_raw.ends_with(char::is_whitespace)
                {
                    cur_entry_summary_raw.push(' ');
                }
            }
            Ok(Event::Text(ref e)) => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                append_active!(&text);
            }
            Ok(Event::CData(ref e)) => {
                let text = e.as_ref();
                append_active!(text);
            }
            Ok(Event::GeneralRef(ref e)) => {
                let code = e.as_ref();
                let repl_char = match code {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    s if s.starts_with("#x") || s.starts_with("#X") => {
                        u32::from_str_radix(&s[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                    }
                    s if s.starts_with('#') => s[1..].parse::<u32>().ok().and_then(char::from_u32),
                    _ => None,
                };
                if let Some(ch) = repl_char {
                    let mut tmp = [0u8; 4];
                    let text = ch.encode_utf8(&mut tmp);
                    append_active!(text);
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.local_name();
                let name_str = name.as_ref();
                if in_entry
                    && (in_entry_element(&tag_stack, "summary")
                        || in_entry_element(&tag_stack, "content"))
                    && is_block_break_tag(name_str)
                    && !cur_entry_summary_raw.is_empty()
                    && !cur_entry_summary_raw.ends_with(char::is_whitespace)
                {
                    cur_entry_summary_raw.push(' ');
                }

                if name_str.eq_ignore_ascii_case("entry") {
                    let final_title = cur_entry_title.trim().to_string();
                    let final_id = if cur_entry_id.is_empty() {
                        final_title.clone()
                    } else {
                        cur_entry_id.clone()
                    };
                    let final_summary = if !cur_entry_summary_raw.is_empty() {
                        let stripped = strip_html_tags(cur_entry_summary_raw.trim());
                        if !stripped.is_empty() {
                            Some(stripped)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let final_updated = if !cur_entry_updated_raw.trim().is_empty() {
                        Some(cur_entry_updated_raw.trim().to_string())
                    } else if !cur_entry_published_raw.trim().is_empty() {
                        Some(cur_entry_published_raw.trim().to_string())
                    } else {
                        None
                    };

                    entries.push(OpdsEntry {
                        id: final_id,
                        title: final_title,
                        authors: cur_entry_authors.clone(),
                        summary: final_summary,
                        updated: final_updated,
                        categories: cur_entry_categories.clone(),
                        thumbnail_url: cur_entry_thumb.take(),
                        acquisition_links: cur_entry_acquisitions.clone(),
                        navigation_link: cur_entry_nav.take(),
                    });
                    in_entry = false;
                } else if name_str.eq_ignore_ascii_case("author") {
                    let clean_auth = cur_author_name.trim().to_string();
                    if !clean_auth.is_empty() && !cur_entry_authors.contains(&clean_auth) {
                        cur_entry_authors.push(clean_auth);
                    }
                    cur_author_name.clear();
                    in_author = false;
                }
                pop_matching_tag(&mut tag_stack, name_str);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML error: {e}")),
            _ => {}
        }
        buf.clear();
    }

    let final_feed_title = feed_title.trim().to_string();
    let feed_title = if final_feed_title.is_empty() {
        "OPDS Catalog".to_string()
    } else {
        final_feed_title
    };

    let feed_icon = if !feed_icon_raw.is_empty() {
        Some(resolve_relative_url(base_url, &feed_icon_raw))
    } else {
        None
    };

    Ok(OpdsFeed {
        title: feed_title,
        id: feed_id,
        icon: feed_icon,
        links: feed_links,
        entries,
        next_url,
        prev_url,
        search_template,
        search_descriptor_url,
    })
}
