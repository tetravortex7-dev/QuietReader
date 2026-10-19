use super::types::{OpdsAcquisitionLink, OpdsEntry, OpdsFeed, OpdsLink};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

pub fn resolve_relative_url(base: &str, relative: &str) -> String {
    let rel = relative.trim();
    if rel.is_empty() {
        return base.to_string();
    }
    if rel.starts_with("http://") || rel.starts_with("https://") {
        return rel.to_string();
    }
    if rel.starts_with("//") {
        let scheme = if base.starts_with("https:") {
            "https:"
        } else {
            "http:"
        };
        return format!("{scheme}{rel}");
    }

    let (scheme, rest) = if let Some((s, r)) = base.split_once("://") {
        (s, r)
    } else {
        ("http", base)
    };

    let (host_port, path_query) = if let Some((hp, pq)) = rest.split_once('/') {
        (hp, format!("/{pq}"))
    } else {
        (rest, "/".to_string())
    };

    if rel.starts_with('/') {
        return format!("{scheme}://{host_port}{rel}");
    }

    let path_only = path_query.split('?').next().unwrap_or("/");
    let dir = if let Some(idx) = path_only.rfind('/') {
        &path_only[..=idx]
    } else {
        "/"
    };

    let combined = format!("{dir}{rel}");
    let mut segments = Vec::new();
    for seg in combined.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        } else if seg == ".." {
            segments.pop();
        } else {
            segments.push(seg);
        }
    }

    let normalized_path = segments.join("/");
    format!("{scheme}://{host_port}/{normalized_path}")
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
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
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
    let h = href.to_ascii_lowercase();
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
            return Some((badge, ext));
        }
    }
    None
}

pub fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
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
                            let val = attr.value.as_ref();
                            if val.contains("atom+xml") || val.contains("opds") {
                                is_atom = true;
                            }
                        } else if key.as_ref().eq_ignore_ascii_case("template") {
                            template = Some(attr.value.to_string());
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
    let mut current_tag = String::new();

    let mut cur_entry_id = String::new();
    let mut cur_entry_title = String::new();
    let mut cur_author_name = String::new();
    let mut cur_entry_authors: Vec<String> = Vec::new();
    let mut cur_entry_summary_raw = String::new();
    let mut cur_entry_summary = None;
    let mut cur_entry_updated = None;
    let mut cur_entry_categories = Vec::new();
    let mut cur_entry_thumb = None;
    let mut cur_entry_acquisitions: Vec<OpdsAcquisitionLink> = Vec::new();
    let mut cur_entry_nav = None;

    let mut buf = Vec::new();

    macro_rules! append_active {
        ($txt:expr) => {
            if in_entry {
                if current_tag.eq_ignore_ascii_case("title") {
                    cur_entry_title.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("id") {
                    cur_entry_id.push_str($txt.trim());
                } else if in_author && current_tag.eq_ignore_ascii_case("name") {
                    cur_author_name.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("summary")
                    || current_tag.eq_ignore_ascii_case("content")
                {
                    cur_entry_summary_raw.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("updated")
                    || current_tag.eq_ignore_ascii_case("published")
                {
                    if cur_entry_updated.is_none() {
                        cur_entry_updated = Some($txt.trim().to_string());
                    }
                }
            } else if in_feed {
                if current_tag.eq_ignore_ascii_case("title") {
                    feed_title.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("id") {
                    feed_id.push_str($txt.trim());
                } else if current_tag.eq_ignore_ascii_case("icon")
                    || current_tag.eq_ignore_ascii_case("logo")
                {
                    feed_icon_raw.push_str($txt.trim());
                }
            }
        };
    }

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.local_name().as_ref().to_string();
                if name.eq_ignore_ascii_case("feed") {
                    in_feed = true;
                } else if name.eq_ignore_ascii_case("entry") {
                    in_entry = true;
                    cur_entry_id.clear();
                    cur_entry_title.clear();
                    cur_author_name.clear();
                    cur_entry_authors.clear();
                    cur_entry_summary_raw.clear();
                    cur_entry_summary = None;
                    cur_entry_updated = None;
                    cur_entry_categories.clear();
                    cur_entry_thumb = None;
                    cur_entry_acquisitions.clear();
                    cur_entry_nav = None;
                } else if name.eq_ignore_ascii_case("author") {
                    in_author = true;
                    cur_author_name.clear();
                }
                current_tag = name;
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                if name.as_ref().eq_ignore_ascii_case("link") {
                    let mut href = String::new();
                    let mut rel = String::new();
                    let mut mime_type = String::new();
                    let mut title = String::new();
                    let mut template = None;

                    for attr in e.attributes().flatten() {
                        let k = attr.key.local_name();
                        let v = attr.value.to_string();
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
                                });
                            }
                        } else if rel_lower.contains("subsection")
                            || type_lower.contains("atom+xml")
                            || rel_lower == "alternate" && type_lower.contains("opds")
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
                } else if name.as_ref().eq_ignore_ascii_case("category") && in_entry {
                    for attr in e.attributes().flatten() {
                        let k = attr.key.local_name();
                        if k.as_ref().eq_ignore_ascii_case("label")
                            || k.as_ref().eq_ignore_ascii_case("term")
                        {
                            let val = attr.value.trim().to_string();
                            if !val.is_empty() && !cur_entry_categories.contains(&val) {
                                cur_entry_categories.push(val);
                                break;
                            }
                        }
                    }
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
                if name.as_ref().eq_ignore_ascii_case("entry") {
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
                        cur_entry_summary.take()
                    };

                    entries.push(OpdsEntry {
                        id: final_id,
                        title: final_title,
                        authors: cur_entry_authors.clone(),
                        summary: final_summary,
                        updated: cur_entry_updated.take(),
                        categories: cur_entry_categories.clone(),
                        thumbnail_url: cur_entry_thumb.take(),
                        acquisition_links: cur_entry_acquisitions.clone(),
                        navigation_link: cur_entry_nav.take(),
                    });
                    in_entry = false;
                } else if name.as_ref().eq_ignore_ascii_case("author") {
                    let clean_auth = cur_author_name.trim().to_string();
                    if !clean_auth.is_empty() && !cur_entry_authors.contains(&clean_auth) {
                        cur_entry_authors.push(clean_auth);
                    }
                    cur_author_name.clear();
                    in_author = false;
                }
                current_tag.clear();
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
