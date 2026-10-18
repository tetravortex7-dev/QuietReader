use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::local_name;

pub fn sanitize_xml_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if c == '&' {
            let rest = &input[idx + 1..];
            if let Some(semi_pos) = rest.find(';')
                && semi_pos <= 10
            {
                let entity_name = &rest[..semi_pos];
                if matches!(entity_name, "amp" | "lt" | "gt" | "quot" | "apos") {
                    out.push('&');
                    out.push_str(entity_name);
                    out.push(';');
                    for _ in 0..=semi_pos {
                        chars.next();
                    }
                    continue;
                }
                if entity_name.starts_with('#')
                    && (entity_name[1..].chars().all(|ch| ch.is_ascii_digit())
                        || (entity_name.starts_with("#x") || entity_name.starts_with("#X"))
                            && entity_name[2..].chars().all(|ch| ch.is_ascii_hexdigit()))
                {
                    out.push('&');
                    out.push_str(entity_name);
                    out.push(';');
                    for _ in 0..=semi_pos {
                        chars.next();
                    }
                    continue;
                }
                let repl = match entity_name {
                    "nbsp" => " ",
                    "mdash" => "—",
                    "ndash" => "–",
                    "hellip" => "…",
                    "laquo" => "«",
                    "raquo" => "»",
                    "ldquo" => "“",
                    "rdquo" => "”",
                    "lsquo" => "‘",
                    "rsquo" => "’",
                    "copy" => "©",
                    "trade" => "™",
                    "reg" => "®",
                    "bull" => "•",
                    "times" => "×",
                    "divide" => "÷",
                    "minus" => "−",
                    "deg" => "°",
                    "euro" => "€",
                    "pound" => "£",
                    "yen" => "¥",
                    "cent" => "¢",
                    "sect" => "§",
                    "para" => "¶",
                    _ => "",
                };
                if !repl.is_empty() {
                    out.push_str(repl);
                    for _ in 0..=semi_pos {
                        chars.next();
                    }
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

pub fn parse_html_chapter(html: &str) -> (String, Vec<String>, Vec<String>) {
    let sanitized = sanitize_xml_entities(html);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;

    let mut title = String::new();
    let mut paragraphs = Vec::new();
    let mut anchors = Vec::new();
    let mut current_p = String::new();
    let (mut in_title_tag, mut in_heading_tag, mut skip_content) = (false, false, false);
    let mut buf = Vec::new();
    let mut had_error = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                for attr in e.attributes().flatten() {
                    let k = local_name(attr.key.as_ref()).to_ascii_lowercase();
                    if k == "id" || k == "name" {
                        anchors.push(attr.value.to_string());
                    }
                }
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "script" | "style" | "head" => skip_content = true,
                    "title" => {
                        skip_content = false;
                        in_title_tag = true;
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_p(&mut current_p, &mut paragraphs);
                        in_heading_tag = true;
                    }
                    "p" | "div" | "blockquote" | "li" => flush_p(&mut current_p, &mut paragraphs),
                    "br" => current_p.push('\n'),
                    "b" | "strong" => {
                        if !skip_content && !in_title_tag && !in_heading_tag {
                            current_p.push_str("<b>");
                        }
                    }
                    "i" | "em" => {
                        if !skip_content && !in_title_tag && !in_heading_tag {
                            current_p.push_str("<i>");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                for attr in e.attributes().flatten() {
                    let k = local_name(attr.key.as_ref()).to_ascii_lowercase();
                    if k == "id" || k == "name" {
                        anchors.push(attr.value.to_string());
                    }
                }
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "br" => current_p.push('\n'),
                    "hr" => {
                        flush_p(&mut current_p, &mut paragraphs);
                        paragraphs.push("* * *".to_string());
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if !skip_content => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                if !text.is_empty() {
                    if in_title_tag && title.is_empty() {
                        title.push_str(text.trim());
                    } else {
                        current_p.push_str(&text);
                    }
                }
            }
            Ok(Event::GeneralRef(ref e)) if !skip_content => {
                let repl = match e.as_ref() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    _ => "",
                };
                if !repl.is_empty() {
                    current_p.push_str(repl);
                }
            }
            Ok(Event::End(ref e)) => {
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "script" | "style" | "head" => skip_content = false,
                    "title" => in_title_tag = false,
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        if in_heading_tag && title.is_empty() && !current_p.trim().is_empty() {
                            title = current_p.trim().to_string();
                        }
                        flush_p(&mut current_p, &mut paragraphs);
                        in_heading_tag = false;
                    }
                    "p" | "div" | "blockquote" | "li" => flush_p(&mut current_p, &mut paragraphs),
                    "b" | "strong" => {
                        if !skip_content && !in_title_tag && !in_heading_tag {
                            current_p.push_str("</b>");
                        }
                    }
                    "i" | "em" => {
                        if !skip_content && !in_title_tag && !in_heading_tag {
                            current_p.push_str("</i>");
                        }
                    }
                    _ => {}
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
    (title, paragraphs, anchors)
}
