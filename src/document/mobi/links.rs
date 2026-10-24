use std::collections::HashMap;

pub fn preprocess_links(html: &str) -> (String, HashMap<String, String>) {
    let lower = html.to_ascii_lowercase();
    let mut output = String::with_capacity(html.len());
    let mut notes = HashMap::new();
    let mut last = 0;
    let mut search = 0;

    while let Some(relative) = lower[search..].find("<a") {
        let tag_start = search + relative;
        let Some(tag_end_rel) = lower[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_rel;
        let tag = &html[tag_start..=tag_end];
        let Some(close_rel) = lower[tag_end + 1..].find("</a>") else {
            search = tag_end + 1;
            continue;
        };
        let close_start = tag_end + 1 + close_rel;
        let close_end = close_start + 4;
        let label = &html[tag_end + 1..close_start];

        if let Some((id, note)) = note_reference(tag, label, html) {
            output.push_str(&html[last..tag_start]);
            output.push_str(&crate::document::footnote_token(&id));
            last = close_end;
            notes.insert(id, note);
        }
        search = close_end;
    }

    output.push_str(&html[last..]);
    (output, notes)
}

fn note_reference(tag: &str, label: &str, html: &str) -> Option<(String, String)> {
    let filepos =
        tag_attribute(tag, "filepos").and_then(|value| value.trim().parse::<usize>().ok());
    let href = tag_attribute(tag, "href");
    let link_type = tag_attribute(tag, "type").unwrap_or_default();
    let marker = looks_like_note_marker(label);
    let explicit_note = link_type.to_ascii_lowercase().contains("note");

    if !marker && !explicit_note {
        return None;
    }

    if let Some(offset) = filepos {
        let note = extract_note_at(html, offset)?;
        return Some((format!("mobi_pos_{offset}"), note));
    }

    if let Some(href) = href
        && let Some(fragment) = href.strip_prefix('#')
        && !fragment.is_empty()
    {
        let note = extract_note_by_id(html, fragment)?;
        return Some((
            format!(
                "mobi_hash_{}",
                crate::document::epub::percent_decode_str(fragment)
            ),
            note,
        ));
    }

    None
}

fn extract_note_at(html: &str, offset: usize) -> Option<String> {
    let offset = offset.min(html.len());
    let mut start = offset;
    let before = &html[..offset];
    let inside_tag = before.rfind('<') > before.rfind('>');
    if inside_tag && let Some(relative) = html[offset..].find('>') {
        start = offset + relative + 1;
    }
    let slice = &html[start..];
    let end = block_end(slice);
    let text = strip_tags(&slice[..end]);
    if text.is_empty() { None } else { Some(text) }
}

fn extract_note_by_id(html: &str, id: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let decoded = crate::document::epub::percent_decode_str(id);
    let mut search = 0;
    while let Some(relative) = lower[search..].find("id=") {
        let index = search + relative;
        let rest = &html[index + 3..];
        let rest = rest.trim_start();
        let Some(quote) = rest.chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            search = index + 3;
            continue;
        }
        let Some(end) = rest[1..].find(quote) else {
            break;
        };
        let value = &rest[1..1 + end];
        if value == id || value == decoded {
            let tag_end = html[index..].find('>')? + index;
            let slice = &html[tag_end + 1..];
            let end = block_end(slice);
            let text = strip_tags(&slice[..end]);
            return if text.is_empty() { None } else { Some(text) };
        }
        search = index + 3 + end;
    }
    None
}

fn block_end(slice: &str) -> usize {
    let lower = slice.to_ascii_lowercase();
    let mut end = slice.len().min(4000);
    for marker in ["</p>", "</div>", "</li>", "<p ", "<div ", "<li "] {
        if let Some(position) = lower.find(marker)
            && position > 0
        {
            end = end.min(position);
        }
    }
    end
}

fn strip_tags(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut inside = false;
    for c in text.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => output.push(c),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn looks_like_note_marker(text: &str) -> bool {
    let trimmed = text.trim();
    let inner = trimmed.trim_matches(|c: char| {
        matches!(
            c,
            '[' | ']' | '(' | ')' | '.' | ',' | ';' | ':' | '!' | '?' | '*'
        )
    });
    if inner.is_empty() || inner.chars().count() > 8 {
        return false;
    }
    inner.chars().all(|c| c.is_ascii_digit())
        || matches!(inner, "*" | "**" | "†" | "‡" | "††" | "‡‡")
        || (inner.chars().count() <= 4 && inner.chars().all(|c| c.is_ascii_alphabetic()))
}

fn tag_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let pattern = format!("{name}=");
    let index = lower.find(&pattern)?;
    let rest = tag[index + pattern.len()..].trim_start();
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
