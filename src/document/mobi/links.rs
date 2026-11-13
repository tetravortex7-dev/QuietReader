use super::decode::{
    decode_mobi_content, is_cp1251_mojibake, is_cp1252_mojibake, resolve_mobi_encoding,
};
use std::collections::HashMap;

pub(crate) struct RawMobiContext<'a> {
    clean_bytes: &'a [u8],
    bom_len: usize,
    header_codepage: u32,
    effective_enc: &'static encoding_rs::Encoding,
    cp1251_mojibake: bool,
}

impl<'a> RawMobiContext<'a> {
    pub(crate) fn new(raw_bytes: &'a [u8], html: &str, header_codepage: u32) -> Self {
        let bom_len = if raw_bytes.starts_with(b"\xEF\xBB\xBF") {
            3
        } else {
            0
        };
        let clean_bytes = &raw_bytes[bom_len..];
        let detected = resolve_mobi_encoding(clean_bytes, header_codepage);
        let cp1251_mojibake =
            detected != encoding_rs::UTF_8 && is_cp1251_mojibake(&detected.decode(clean_bytes).0);
        let effective_enc = if detected != encoding_rs::UTF_8
            && !cp1251_mojibake
            && is_cp1252_mojibake(&detected.decode(clean_bytes).0)
        {
            encoding_rs::WINDOWS_1251
        } else if detected == encoding_rs::UTF_8 && html.len() == clean_bytes.len() {
            encoding_rs::UTF_8
        } else {
            detected
        };
        Self {
            clean_bytes,
            bom_len,
            header_codepage,
            effective_enc,
            cp1251_mojibake,
        }
    }

    fn resolve_offset(&self, html: &str, raw_offset: usize) -> usize {
        let adj = raw_offset
            .saturating_sub(self.bom_len)
            .min(self.clean_bytes.len());
        let byte_offset = if self.cp1251_mojibake {
            decode_mobi_content(&self.clean_bytes[..adj], self.header_codepage).len()
        } else if self.effective_enc == encoding_rs::UTF_8 {
            adj
        } else {
            decoded_utf8_len(self.effective_enc, &self.clean_bytes[..adj])
        };
        html.floor_char_boundary(byte_offset.min(html.len()))
    }
}

fn decoded_utf8_len(enc: &'static encoding_rs::Encoding, input: &[u8]) -> usize {
    if input.is_empty() {
        return 0;
    }
    let mut decoder = enc.new_decoder_without_bom_handling();
    let mut buf = [0u8; 4096];
    let mut total_read = 0usize;
    let mut total_written = 0usize;
    loop {
        let (result, read, written, _) =
            decoder.decode_to_utf8(&input[total_read..], &mut buf, true);
        total_read += read;
        total_written += written;
        if matches!(result, encoding_rs::CoderResult::InputEmpty) || (read == 0 && written == 0) {
            break;
        }
    }
    total_written
}

#[cfg(all(test, feature = "enable_tests"))]
pub fn preprocess_links(html: &str) -> (String, HashMap<String, String>) {
    preprocess_links_internal(html, None)
}

pub(crate) fn preprocess_links_with_raw(
    html: &str,
    raw_bytes: &[u8],
    header_codepage: u32,
) -> (String, HashMap<String, String>) {
    let ctx = RawMobiContext::new(raw_bytes, html, header_codepage);
    preprocess_links_internal(html, Some(&ctx))
}

fn preprocess_links_internal(
    html: &str,
    raw_ctx: Option<&RawMobiContext<'_>>,
) -> (String, HashMap<String, String>) {
    let lower = html.to_ascii_lowercase();
    let mut output = String::with_capacity(html.len());
    let mut notes = HashMap::new();
    let mut last = 0;
    let mut search = 0;

    while let Some((tag_start, tag_end)) = find_anchor_start(&lower, search) {
        let tag = &html[tag_start..=tag_end];
        if tag.trim_end().ends_with("/>") {
            search = tag_end + 1;
            continue;
        }
        let Some(close_rel) = lower[tag_end + 1..].find("</a>") else {
            search = tag_end + 1;
            continue;
        };
        let close_start = tag_end + 1 + close_rel;
        if let Some((inner_start, inner_end)) =
            find_anchor_start(&lower[..close_start], tag_end + 1)
            && !lower[inner_start..=inner_end].trim_end().ends_with("/>")
        {
            search = tag_end + 1;
            continue;
        }
        let close_end = close_start + 4;
        let label = &html[tag_end + 1..close_start];

        if let Some((id, note)) = note_reference(tag, label, html, &lower, raw_ctx) {
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

fn find_anchor_start(lower: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = lower.as_bytes();
    let mut cursor = lower.floor_char_boundary(from.min(lower.len()));
    while let Some(relative) = lower[cursor..].find("<a") {
        let tag_start = cursor + relative;
        let valid_boundary = matches!(
            bytes.get(tag_start + 2),
            Some(&b) if b.is_ascii_whitespace() || b == b'>' || b == b'/'
        );
        if !valid_boundary {
            cursor = tag_start + 2;
            continue;
        }
        let tag_end_rel = lower[tag_start + 2..].find('>')?;
        return Some((tag_start, tag_start + 2 + tag_end_rel));
    }
    None
}

fn note_reference(
    tag: &str,
    label: &str,
    html: &str,
    lower_html: &str,
    raw_ctx: Option<&RawMobiContext<'_>>,
) -> Option<(String, String)> {
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
        let note = extract_note_at(html, offset, raw_ctx)?;
        return Some((format!("mobi_pos_{offset}"), note));
    }

    if let Some(ref href) = href
        && let Some((fid, offset, fragment)) = parse_kindle_pos_fid(href)
    {
        if let Some(ref frag) = fragment
            && let Some(note) = extract_note_by_id(html, lower_html, frag)
        {
            let decoded_frag = crate::document::epub::percent_decode_str(frag);
            return Some((format!("mobi_kf8_{fid}_{offset}_{decoded_frag}"), note));
        }
        if offset > 0
            && let Some(note) = extract_note_at(html, offset, raw_ctx)
        {
            return Some((format!("mobi_kf8_{fid}_{offset}"), note));
        }
    }

    if let Some(href) = href
        && let Some(fragment) = href.strip_prefix('#')
        && !fragment.is_empty()
    {
        let note = extract_note_by_id(html, lower_html, fragment)?;
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

pub fn parse_kindle_pos_fid(href: &str) -> Option<(usize, usize, Option<String>)> {
    let lower = href.to_ascii_lowercase();
    let prefix = "kindle:pos:fid:";
    let start = lower.find(prefix)? + prefix.len();
    let rest = &href[start..];
    let lower_rest = &lower[start..];

    let off_pattern = ":off:";
    let off_pos = lower_rest.find(off_pattern)?;
    let fid_str = &rest[..off_pos];
    let fid = usize::from_str_radix(fid_str.trim(), 16)
        .or_else(|_| fid_str.trim().parse::<usize>())
        .unwrap_or(0);

    let after_off = &rest[off_pos + off_pattern.len()..];
    let (offset_and_query, fragment) = match after_off.split_once('#') {
        Some((left, right)) => (left, Some(right.to_string())),
        None => (after_off, None),
    };

    let offset_str = offset_and_query
        .split('?')
        .next()
        .unwrap_or(offset_and_query)
        .trim();
    let offset = offset_str.parse::<usize>().unwrap_or(0);

    Some((fid, offset, fragment))
}

fn extract_note_at(
    html: &str,
    raw_offset: usize,
    raw_ctx: Option<&RawMobiContext<'_>>,
) -> Option<String> {
    let offset = match raw_ctx {
        Some(ctx) => ctx.resolve_offset(html, raw_offset),
        None => html.floor_char_boundary(raw_offset.min(html.len())),
    };
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

fn extract_note_by_id(html: &str, lower: &str, id: &str) -> Option<String> {
    extract_note_by_attr(html, lower, "id", id)
        .or_else(|| extract_note_by_attr(html, lower, "name", id))
}

fn extract_note_by_attr(html: &str, lower: &str, attr_name: &str, id: &str) -> Option<String> {
    let decoded = crate::document::epub::percent_decode_str(id);
    let mut search = 0;
    while let Some((_, after_val, value)) =
        find_attr_in_slice(html, lower, attr_name, search, false)
    {
        if value == id || value == decoded {
            let tag_end = html[after_val..].find('>')? + after_val;
            let slice = &html[tag_end + 1..];
            let end = block_end(slice);
            let text = strip_tags(&slice[..end]);
            return if text.is_empty() { None } else { Some(text) };
        }
        search = after_val;
    }
    None
}

fn block_end(slice: &str) -> usize {
    let max_end = slice.floor_char_boundary(slice.len().min(4000));
    let probe_end = slice.floor_char_boundary(slice.len().min(4016));
    let lower = slice[..probe_end].to_ascii_lowercase();
    let mut end = max_end;
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
    let plain = strip_tags(text)
        .replace("&dagger;", "†")
        .replace("&Dagger;", "‡")
        .replace("&ast;", "*");
    let trimmed = plain.trim();
    let inner = trimmed
        .trim_matches(|c: char| {
            matches!(c, '[' | ']' | '(' | ')' | '.' | ',' | ';' | ':' | '!' | '?')
        })
        .trim();
    if inner.is_empty() || inner.chars().count() > 8 {
        return false;
    }
    inner.chars().all(|c| c.is_ascii_digit())
        || matches!(inner, "*" | "**" | "***" | "†" | "‡" | "††" | "‡‡")
        || (inner.chars().count() <= 4 && inner.chars().all(|c| c.is_ascii_alphabetic()))
}

fn find_attr_in_slice<'a>(
    haystack: &'a str,
    lower_haystack: &str,
    name: &str,
    start_from: usize,
    allow_unquoted: bool,
) -> Option<(usize, usize, &'a str)> {
    let bytes = lower_haystack.as_bytes();
    let mut cursor = haystack.floor_char_boundary(start_from.min(haystack.len()));

    while let Some(rel) = lower_haystack[cursor..].find(name) {
        let key_start = cursor + rel;
        let key_end = key_start + name.len();
        if key_start > 0
            && !bytes[key_start - 1].is_ascii_whitespace()
            && bytes[key_start - 1] != b'"'
            && bytes[key_start - 1] != b'\''
        {
            cursor = key_end;
            continue;
        }

        let mut pos = key_end;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if bytes.get(pos) != Some(&b'=') {
            cursor = key_end;
            continue;
        }
        pos += 1;
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= bytes.len() {
            return None;
        }

        let quote_byte = bytes[pos];
        if quote_byte == b'"' || quote_byte == b'\'' {
            let quote = quote_byte as char;
            let value_start = pos + 1;
            let Some(end_rel) = haystack[value_start..].find(quote) else {
                cursor = value_start;
                continue;
            };
            let value_end = value_start + end_rel;
            return Some((key_start, value_end + 1, &haystack[value_start..value_end]));
        }

        if allow_unquoted {
            let rest = &haystack[pos..];
            let value = rest
                .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .next()
                .unwrap_or("");
            if !value.is_empty() {
                return Some((key_start, pos + value.len(), value));
            }
        }

        let Some(ch) = haystack[pos..].chars().next() else {
            return None;
        };
        cursor = pos + ch.len_utf8();
    }
    None
}

fn tag_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    find_attr_in_slice(tag, &lower, name, 0, true).map(|(_, _, value)| value.to_string())
}
