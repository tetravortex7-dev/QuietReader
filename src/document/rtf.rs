use crate::document::{BookChapter, BookMetadata, Document, DocumentError, TextContent, TocItem};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct RtfDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    footnotes: std::collections::HashMap<String, String>,
    note_locations: std::collections::HashMap<String, crate::document::NoteLocation>,
    header_text: Option<String>,
    footer_text: Option<String>,
}

impl TextContent for RtfDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl Document for RtfDocument {
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
        "RTF"
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

    fn header_text(&self) -> Option<&str> {
        self.header_text.as_deref()
    }

    fn footer_text(&self) -> Option<&str> {
        self.footer_text.as_deref()
    }
}

impl RtfDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path = path.as_ref();
        let meta = std::fs::metadata(path)
            .map_err(|e| DocumentError::Io(crate::document::errors::io_read("RTF", &e)))?;
        if meta.len() > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::file_too_large("RTF"),
            ));
        }

        let mut file = File::open(path)
            .map_err(|e| DocumentError::Io(crate::document::errors::io_open("RTF", &e)))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(crate::document::errors::io_read("RTF", &e)))?;

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(crate::document::errors::tokens::TITLE_RTF);

        Self::from_bytes(&bytes, default_title)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str) -> Result<Self, DocumentError> {
        if bytes.len() > crate::document::MAX_TOTAL_UNPACK_BYTES as usize {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::data_too_large("RTF"),
            ));
        }
        validate_rtf_header(bytes)?;

        let doc_encoding = detect_rtf_encoding(bytes);
        let (prepared, footnotes) = preprocess_notes(bytes, b"\\footnote", "rtf_fn_", doc_encoding);
        let (prepared, endnotes) =
            preprocess_notes(&prepared, b"\\endnote", "rtf_en_", doc_encoding);
        let mut notes = footnotes;
        notes.extend(endnotes);
        let header_text = extract_rtf_groups(bytes, b"\\header", doc_encoding);
        let footer_text = extract_rtf_groups(bytes, b"\\footer", doc_encoding);
        let (mut chapters, toc, metadata) = parse_rtf(&prepared, default_title)?;
        let note_locations = crate::document::build_notes_appendix(&mut chapters, &notes);

        Ok(Self {
            metadata,
            chapters,
            toc,
            footnotes: notes,
            note_locations,
            header_text,
            footer_text,
        })
    }
}

fn validate_rtf_header(bytes: &[u8]) -> Result<(), DocumentError> {
    let without_bom = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut start = 0;
    while start < without_bom.len() && matches!(without_bom[start], b' ' | b'\t' | b'\r' | b'\n') {
        start += 1;
    }
    let trimmed = &without_bom[start..];
    if trimmed.len() < 5 {
        return Err(DocumentError::Corrupted(
            crate::document::errors::file_too_short("RTF"),
        ));
    }
    if !trimmed.starts_with(b"{\\rtf") {
        return Err(DocumentError::Corrupted(
            crate::document::errors::invalid_header("RTF", "{\\rtf}"),
        ));
    }
    Ok(())
}

fn encoding_from_codepage(cpg: i32) -> &'static encoding_rs::Encoding {
    match cpg {
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        874 => encoding_rs::WINDOWS_874,
        932 => encoding_rs::SHIFT_JIS,
        936 | 54936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        20866 => encoding_rs::KOI8_R,
        21866 => encoding_rs::KOI8_U,
        28595 => encoding_rs::ISO_8859_5,
        65001 => encoding_rs::UTF_8,
        _ => encoding_rs::WINDOWS_1252,
    }
}

fn read_rtf_control_word<'a>(bytes: &'a [u8], i: &mut usize) -> (&'a str, Option<i32>) {
    let len = bytes.len();
    let start = *i;
    while *i < len && bytes[*i].is_ascii_alphabetic() {
        *i += 1;
    }
    let word = std::str::from_utf8(&bytes[start..*i]).unwrap_or("");
    let p_start = *i;
    let mut has_param = false;
    if *i < len && bytes[*i] == b'-' {
        *i += 1;
    }
    while *i < len && bytes[*i].is_ascii_digit() {
        has_param = true;
        *i += 1;
    }
    let param = if has_param {
        std::str::from_utf8(&bytes[p_start..*i])
            .ok()
            .and_then(|s| s.parse::<i32>().ok())
    } else {
        None
    };
    if *i < len && bytes[*i] == b' ' {
        *i += 1;
    }
    (word, param)
}

fn detect_rtf_encoding(bytes: &[u8]) -> &'static encoding_rs::Encoding {
    let limit = bytes.len().min(4096);
    let header = &bytes[..limit];
    let mut enc = encoding_rs::WINDOWS_1252;
    let mut i = 0;
    while i < header.len() {
        if header[i] == b'\\' {
            i += 1;
            if i >= header.len() {
                break;
            }
            if header[i].is_ascii_alphabetic() {
                let (word, param) = read_rtf_control_word(header, &mut i);
                if word == "mac" {
                    enc = encoding_rs::MACINTOSH;
                } else if word == "ansicpg"
                    && let Some(cpg) = param
                {
                    return encoding_from_codepage(cpg);
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    enc
}

fn skip_rtf_unicode_fallback(bytes: &[u8], i: &mut usize, skip_count: usize) {
    let len = bytes.len();
    let mut skipped = 0;
    while skipped < skip_count && *i < len {
        let b = bytes[*i];
        if b == b'\r' || b == b'\n' {
            *i += 1;
            continue;
        }
        if b == b'{' || b == b'}' {
            break;
        }
        if b == b'\\' {
            if *i + 3 < len && bytes[*i + 1] == b'\'' {
                *i += 4;
                skipped += 1;
            } else if *i + 1 < len && matches!(bytes[*i + 1], b'\\' | b'{' | b'}' | b'~' | b'_') {
                *i += 2;
                skipped += 1;
            } else {
                break;
            }
        } else {
            *i += 1;
            skipped += 1;
        }
    }
}

fn decode_rtf_unicode_param(v: i32, pending_surrogate: &mut Option<u16>) -> Option<char> {
    let code = if v < 0 { (v + 65536) as u32 } else { v as u32 };
    if (0xD800..=0xDBFF).contains(&code) {
        *pending_surrogate = Some(code as u16);
        return None;
    }
    if (0xDC00..=0xDFFF).contains(&code) {
        if let Some(hi) = pending_surrogate.take() {
            let full = 0x10000 + ((((hi as u32) - 0xD800) << 10) | (code - 0xDC00));
            return char::from_u32(full);
        }
        return None;
    }
    *pending_surrogate = None;
    char::from_u32(code)
}

#[derive(Clone, Copy)]
enum Destination {
    Normal,
    Skip,
    Title,
    Author,
}

#[derive(Clone, Copy)]
struct RtfState {
    dest: Destination,
    uc: usize,
}

fn current_dest(stack: &[RtfState]) -> Destination {
    stack.last().map(|s| s.dest).unwrap_or(Destination::Normal)
}

fn push_dest_char(
    dest: Destination,
    ch: char,
    cur_p: &mut String,
    tit: &mut String,
    aut: &mut String,
) {
    match dest {
        Destination::Normal => cur_p.push(ch),
        Destination::Title => tit.push(ch),
        Destination::Author => aut.push(ch),
        Destination::Skip => {}
    }
}

fn parse_rtf(
    bytes: &[u8],
    default_title: &str,
) -> Result<(Vec<BookChapter>, Vec<TocItem>, BookMetadata), DocumentError> {
    validate_rtf_header(bytes)?;

    let mut encoding: &'static encoding_rs::Encoding = encoding_rs::WINDOWS_1252;
    let mut title = String::new();
    let mut author = String::new();

    let mut chapters: Vec<BookChapter> = Vec::new();
    let mut current_paras: Vec<String> = Vec::new();
    let mut current_para = String::new();
    let mut hex_buffer: Vec<u8> = Vec::new();
    let mut pending_surrogate: Option<u16> = None;

    let mut state_stack = vec![RtfState {
        dest: Destination::Normal,
        uc: 1,
    }];

    let flush_hex = |hex_buf: &mut Vec<u8>,
                     enc: &'static encoding_rs::Encoding,
                     cur_p: &mut String,
                     dest: Destination,
                     tit: &mut String,
                     aut: &mut String| {
        if hex_buf.is_empty() {
            return;
        }
        let (decoded, _, _) = enc.decode(hex_buf);
        match dest {
            Destination::Normal => cur_p.push_str(&decoded),
            Destination::Title => tit.push_str(&decoded),
            Destination::Author => aut.push_str(&decoded),
            Destination::Skip => {}
        }
        hex_buf.clear();
    };

    let flush_paragraph = |cur_p: &mut String, paras: &mut Vec<String>| {
        let trimmed = cur_p.trim();
        if !trimmed.is_empty() {
            paras.push(trimmed.to_string());
        }
        cur_p.clear();
    };

    let mut i = 0;
    let len = bytes.len();

    while i < len {
        let b = bytes[i];
        let dest = current_dest(&state_stack);
        if b == b'{' {
            flush_hex(
                &mut hex_buffer,
                encoding,
                &mut current_para,
                dest,
                &mut title,
                &mut author,
            );
            let parent = state_stack.last().copied().unwrap_or(RtfState {
                dest: Destination::Normal,
                uc: 1,
            });
            state_stack.push(parent);
            i += 1;
        } else if b == b'}' {
            flush_hex(
                &mut hex_buffer,
                encoding,
                &mut current_para,
                dest,
                &mut title,
                &mut author,
            );
            if state_stack.len() > 1 {
                state_stack.pop();
            }
            i += 1;
        } else if b == b'\\' {
            flush_hex(
                &mut hex_buffer,
                encoding,
                &mut current_para,
                dest,
                &mut title,
                &mut author,
            );
            i += 1;
            if i >= len {
                break;
            }
            let next = bytes[i];
            if next == b'\\' || next == b'{' || next == b'}' {
                push_dest_char(
                    dest,
                    next as char,
                    &mut current_para,
                    &mut title,
                    &mut author,
                );
                i += 1;
            } else if next == b'\'' {
                i += 1;
                if i + 1 < len {
                    let h1 = bytes[i];
                    let h2 = bytes[i + 1];
                    i += 2;
                    if !matches!(dest, Destination::Skip)
                        && let (Some(d1), Some(d2)) = (hex_digit(h1), hex_digit(h2))
                    {
                        hex_buffer.push((d1 << 4) | d2);
                    }
                }
            } else if next == b'*' {
                if let Some(top) = state_stack.last_mut() {
                    top.dest = Destination::Skip;
                }
                i += 1;
            } else if next == b'~' {
                push_dest_char(dest, '\u{00A0}', &mut current_para, &mut title, &mut author);
                i += 1;
            } else if next == b'_' {
                push_dest_char(dest, '-', &mut current_para, &mut title, &mut author);
                i += 1;
            } else if next == b'\r' || next == b'\n' {
                if matches!(dest, Destination::Normal) {
                    flush_paragraph(&mut current_para, &mut current_paras);
                }
                i += 1;
            } else if next.is_ascii_alphabetic() {
                let (word, param_val) = read_rtf_control_word(bytes, &mut i);
                match word {
                    "ansi" => encoding = encoding_rs::WINDOWS_1252,
                    "mac" => encoding = encoding_rs::MACINTOSH,
                    "ansicpg" => {
                        if let Some(cpg) = param_val {
                            encoding = encoding_from_codepage(cpg);
                        }
                    }
                    "bin" => {
                        if let Some(v) = param_val
                            && v > 0
                        {
                            i = i.saturating_add(v as usize).min(len);
                        }
                    }
                    "fonttbl" | "colortbl" | "stylesheet" | "pict" | "object" | "header"
                    | "footer" | "headerl" | "headerr" | "headerf" | "footerl" | "footerr"
                    | "footerf" | "footnote" | "endnote" | "operator" | "company" | "creatim"
                    | "revtim" | "printim" | "buptim" | "doccomm" | "fldinst" | "listtable"
                    | "listoverridetable" | "revtbl" | "rsidtbl" | "xmlnstbl" | "themedata"
                    | "colorschememapping" | "latentstyles" | "datastore" | "pgdsctbl" => {
                        if let Some(top) = state_stack.last_mut() {
                            top.dest = Destination::Skip;
                        }
                    }
                    "title" => {
                        if let Some(top) = state_stack.last_mut() {
                            top.dest = Destination::Title;
                        }
                    }
                    "author" => {
                        if let Some(top) = state_stack.last_mut() {
                            top.dest = Destination::Author;
                        }
                    }
                    "uc" => {
                        if let Some(v) = param_val
                            && let Some(top) = state_stack.last_mut()
                        {
                            top.uc = v.max(0) as usize;
                        }
                    }
                    "u" => {
                        if let Some(v) = param_val {
                            if let Some(ch) = decode_rtf_unicode_param(v, &mut pending_surrogate) {
                                push_dest_char(
                                    dest,
                                    ch,
                                    &mut current_para,
                                    &mut title,
                                    &mut author,
                                );
                            }
                            let skip_count = state_stack.last().map(|s| s.uc).unwrap_or(1);
                            skip_rtf_unicode_fallback(bytes, &mut i, skip_count);
                        }
                    }
                    "par" | "line" => {
                        if matches!(dest, Destination::Normal) {
                            flush_paragraph(&mut current_para, &mut current_paras);
                        }
                    }
                    "page" | "sect" => {
                        if matches!(dest, Destination::Normal) {
                            flush_paragraph(&mut current_para, &mut current_paras);
                            if !current_paras.is_empty() {
                                chapters.push(BookChapter {
                                    id: Some(format!("chap_{}", chapters.len())),
                                    title: crate::document::errors::tokens::section(
                                        chapters.len() + 1,
                                    ),
                                    paragraphs: std::mem::take(&mut current_paras),
                                });
                            }
                        }
                    }
                    "tab" => {
                        if matches!(dest, Destination::Normal) {
                            current_para.push('\t');
                        }
                    }
                    _ => {}
                }
            } else {
                i += 1;
            }
        } else if b == b'\r' || b == b'\n' {
            i += 1;
        } else {
            if !matches!(dest, Destination::Skip) {
                if b >= 0x80 {
                    hex_buffer.push(b);
                } else {
                    flush_hex(
                        &mut hex_buffer,
                        encoding,
                        &mut current_para,
                        dest,
                        &mut title,
                        &mut author,
                    );
                    push_dest_char(dest, b as char, &mut current_para, &mut title, &mut author);
                }
            }
            i += 1;
        }
    }

    flush_hex(
        &mut hex_buffer,
        encoding,
        &mut current_para,
        state_stack
            .last()
            .map(|s| s.dest)
            .unwrap_or(Destination::Normal),
        &mut title,
        &mut author,
    );
    flush_paragraph(&mut current_para, &mut current_paras);

    if !current_paras.is_empty() {
        let chap_idx = chapters.len();
        let chap_title = if chap_idx == 0 {
            default_title.to_string()
        } else {
            crate::document::errors::tokens::section(chap_idx + 1)
        };
        chapters.push(BookChapter {
            id: Some(format!("chap_{chap_idx}")),
            title: chap_title,
            paragraphs: current_paras,
        });
    }

    if chapters.is_empty() {
        chapters.push(BookChapter {
            id: Some("chap_0".to_string()),
            title: default_title.to_string(),
            paragraphs: vec![crate::document::errors::tokens::NO_TEXT.to_string()],
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

    let metadata = BookMetadata {
        title: if title.trim().is_empty() {
            default_title.to_string()
        } else {
            title.trim().to_string()
        },
        author: if author.trim().is_empty() {
            crate::document::errors::tokens::UNKNOWN_AUTHOR.to_string()
        } else {
            author.trim().to_string()
        },
    };

    Ok((chapters, toc, metadata))
}

fn preprocess_notes(
    bytes: &[u8],
    needle: &[u8],
    prefix: &str,
    encoding: &'static encoding_rs::Encoding,
) -> (Vec<u8>, std::collections::HashMap<String, String>) {
    let mut output = Vec::with_capacity(bytes.len());
    let mut notes = std::collections::HashMap::new();
    let mut last = 0;
    let mut search = 0;
    let mut counter = 0usize;

    while let Some(relative) = find_rtf_control_word(&bytes[search..], needle) {
        let position = search + relative;
        let Some(group_start) = find_group_start(bytes, position) else {
            search = position + needle.len();
            continue;
        };
        let Some(group_end) = find_group_end(bytes, group_start) else {
            break;
        };
        let mut content_start = position + needle.len();
        if content_start < group_end && bytes[content_start] == b' ' {
            content_start += 1;
        }
        let text = rtf_group_text(&bytes[content_start..group_end], encoding);
        if !text.is_empty() {
            let id = format!("{prefix}{counter}");
            counter += 1;
            output.extend_from_slice(&bytes[last..group_start]);
            let token = crate::document::footnote_token(&id);
            let escaped = token.replace('{', "\\{").replace('}', "\\}");
            output.extend_from_slice(escaped.as_bytes());
            notes.insert(id, text);
            last = group_end + 1;
        }
        search = group_end + 1;
    }

    output.extend_from_slice(&bytes[last..]);
    (output, notes)
}

fn extract_rtf_groups(
    bytes: &[u8],
    needle: &[u8],
    encoding: &'static encoding_rs::Encoding,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut search = 0;
    while let Some(relative) = find_rtf_control_word(&bytes[search..], needle) {
        let position = search + relative;
        let Some(start) = find_group_start(bytes, position) else {
            search = position + needle.len();
            continue;
        };
        let Some(end) = find_group_end(bytes, start) else {
            break;
        };
        let mut text_start = position + needle.len();
        if text_start < end && bytes[text_start] == b' ' {
            text_start += 1;
        }
        let text = rtf_group_text(&bytes[text_start..end], encoding);
        if !text.is_empty() && !parts.iter().any(|part| part == &text) {
            parts.push(text);
        }
        search = end + 1;
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" • "))
    }
}

fn find_rtf_control_word(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let mut offset = 0;
    while let Some(rel) = find_bytes(&haystack[offset..], needle) {
        let pos = offset + rel;
        let after = pos + needle.len();
        let word_boundary = after >= haystack.len() || !haystack[after].is_ascii_alphabetic();
        if word_boundary && !is_escaped(haystack, pos) {
            return Some(pos);
        }
        offset = pos + 1;
    }
    None
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut count = 0;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        count += 1;
        cursor -= 1;
    }
    count % 2 == 1
}

fn find_group_start(bytes: &[u8], position: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = position;
    while index > 0 {
        index -= 1;
        if is_escaped(bytes, index) {
            continue;
        }
        match bytes[index] {
            b'}' => depth += 1,
            b'{' => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

fn find_group_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = start;
    while index < bytes.len() {
        if !is_escaped(bytes, index) {
            match bytes[index] {
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        index += 1;
    }
    None
}

fn flush_group_hex(
    hex_buffer: &mut Vec<u8>,
    encoding: &'static encoding_rs::Encoding,
    output: &mut String,
) {
    if hex_buffer.is_empty() {
        return;
    }
    let (decoded, _, _) = encoding.decode(hex_buffer);
    output.push_str(&decoded);
    hex_buffer.clear();
}

fn rtf_group_text(bytes: &[u8], encoding: &'static encoding_rs::Encoding) -> String {
    let mut output = String::new();
    let mut hex_buffer: Vec<u8> = Vec::new();
    let mut stack: Vec<(usize, bool)> = vec![(1, false)];
    let mut pending_surrogate: Option<u16> = None;
    let mut index = 0;
    let len = bytes.len();

    while index < len {
        let is_skip = stack.last().is_some_and(|(_, skip)| *skip);
        match bytes[index] {
            b'{' => {
                flush_group_hex(&mut hex_buffer, encoding, &mut output);
                let top = stack.last().copied().unwrap_or((1, false));
                stack.push(top);
                index += 1;
            }
            b'}' => {
                flush_group_hex(&mut hex_buffer, encoding, &mut output);
                if stack.len() > 1 {
                    stack.pop();
                }
                index += 1;
            }
            b'\\' => {
                flush_group_hex(&mut hex_buffer, encoding, &mut output);
                index += 1;
                if index >= len {
                    break;
                }
                match bytes[index] {
                    b'\\' | b'{' | b'}' => {
                        if !is_skip {
                            output.push(bytes[index] as char);
                        }
                        index += 1;
                    }
                    b'*' => {
                        if let Some(top) = stack.last_mut() {
                            top.1 = true;
                        }
                        index += 1;
                    }
                    b'~' => {
                        if !is_skip {
                            output.push('\u{00A0}');
                        }
                        index += 1;
                    }
                    b'_' => {
                        if !is_skip {
                            output.push('-');
                        }
                        index += 1;
                    }
                    b'\r' | b'\n' => {
                        if !is_skip && !output.ends_with(' ') {
                            output.push(' ');
                        }
                        index += 1;
                    }
                    b'\'' => {
                        index += 1;
                        if index + 1 < len {
                            let high = hex_digit(bytes[index]);
                            let low = hex_digit(bytes[index + 1]);
                            if !is_skip && let (Some(high), Some(low)) = (high, low) {
                                hex_buffer.push((high << 4) | low);
                            }
                            index += 2;
                        }
                    }
                    byte if byte.is_ascii_alphabetic() => {
                        let (word, param) = read_rtf_control_word(bytes, &mut index);
                        match word {
                            "bin" => {
                                if let Some(v) = param
                                    && v > 0
                                {
                                    index = index.saturating_add(v as usize).min(len);
                                }
                            }
                            "fonttbl" | "colortbl" | "stylesheet" | "pict" | "object"
                            | "fldinst" | "bkmkstart" | "bkmkend" => {
                                if let Some(top) = stack.last_mut() {
                                    top.1 = true;
                                }
                            }
                            "uc" => {
                                if let Some(v) = param
                                    && let Some(top) = stack.last_mut()
                                {
                                    top.0 = v.max(0) as usize;
                                }
                            }
                            "u" => {
                                if let Some(v) = param {
                                    if !is_skip
                                        && let Some(ch) =
                                            decode_rtf_unicode_param(v, &mut pending_surrogate)
                                    {
                                        output.push(ch);
                                    }
                                    let skip_count = stack.last().map(|s| s.0).unwrap_or(1);
                                    skip_rtf_unicode_fallback(bytes, &mut index, skip_count);
                                }
                            }
                            "par" | "line" | "tab" => {
                                if !is_skip && !output.ends_with(' ') {
                                    output.push(' ');
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        index += 1;
                    }
                }
            }
            b'\r' | b'\n' => {
                index += 1;
            }
            byte => {
                if !is_skip {
                    if byte >= 0x80 {
                        hex_buffer.push(byte);
                    } else {
                        flush_group_hex(&mut hex_buffer, encoding, &mut output);
                        output.push(byte as char);
                    }
                }
                index += 1;
            }
        }
    }

    flush_group_hex(&mut hex_buffer, encoding, &mut output);
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
