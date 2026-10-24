use crate::document::{BookChapter, BookMetadata, Document, DocumentError, TextContent, TocItem};
use std::path::Path;

pub struct TxtDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for TxtDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl Document for TxtDocument {
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
        "TXT"
    }

    fn has_custom_toc(&self) -> bool {
        self.toc.iter().any(|item| {
            let id = item.id.as_deref().unwrap_or("");
            id.starts_with("md_") || id.starts_with("txt_head_") || id.starts_with("log_head_")
        }) && self.toc.len() > 1
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
    }
}

impl TxtDocument {
    pub const MAX_TXT_SIZE: u64 = 100 * 1024 * 1024;

    pub fn from_text(text: &str, default_title: &str) -> Self {
        Self::from_bytes(text.as_bytes(), default_title, "txt").unwrap_or_else(|_| Self {
            metadata: BookMetadata {
                title: default_title.to_string(),
                author: String::new(),
            },
            chapters: Vec::new(),
            toc: Vec::new(),
        })
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        Self::from_file_with_encoding(path, None)
    }

    pub fn from_file_with_encoding<P: AsRef<Path>>(
        path: P,
        encoding: Option<&str>,
    ) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref).map_err(|e| {
            DocumentError::Io(format!(
                "Не удалось прочитать метаданные текстового файла: {e}"
            ))
        })?;
        if meta.len() > Self::MAX_TXT_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер текстового файла превышает допустимый лимит (100 МБ)".to_string(),
            ));
        }

        let bytes = std::fs::read(path_ref)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать текстовый файл: {e}")))?;

        let default_title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Текстовый документ")
            .to_string();

        let ext = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        Self::from_bytes_with_encoding(&bytes, &default_title, &ext, encoding)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str, ext: &str) -> Result<Self, DocumentError> {
        Self::from_bytes_with_encoding(bytes, default_title, ext, None)
    }

    pub fn from_bytes_with_encoding(
        bytes: &[u8],
        default_title: &str,
        ext: &str,
        encoding: Option<&str>,
    ) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > Self::MAX_TXT_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер текстовых данных превышает допустимый лимит (100 МБ)".to_string(),
            ));
        }

        let sample = &bytes[..bytes.len().min(4096)];
        let forced_utf16 = encoding
            .and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
            .is_some_and(|enc| enc == encoding_rs::UTF_16LE || enc == encoding_rs::UTF_16BE);
        let is_utf16 =
            forced_utf16 || bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
        if !is_utf16 && sample.contains(&0) {
            return Err(DocumentError::Corrupted(
                "Файл содержит бинарные данные и не является текстовым документом".to_string(),
            ));
        }

        let text = decode_text_bytes_with(bytes, encoding);
        let is_markdown = ext == "md" || ext == "markdown";
        let is_log = ext == "log";

        let (chapters, toc) = if is_markdown {
            parse_markdown_chapters(&text, default_title)
        } else if is_log {
            parse_log_chapters(&text, default_title)
        } else {
            parse_plain_text_chapters(&text, default_title)
        };

        let title = toc
            .first()
            .map(|t| t.title.clone())
            .filter(|t| is_markdown && !t.is_empty())
            .unwrap_or_else(|| default_title.to_string());

        Ok(Self {
            metadata: BookMetadata {
                title,
                author: String::new(),
            },
            chapters,
            toc,
        })
    }
}

pub(crate) fn decode_text_bytes_with(bytes: &[u8], encoding: Option<&str>) -> String {
    let forced = encoding.and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()));
    if let Some(forced) = forced {
        let (decoded, _, _) = forced.decode(bytes);
        return decoded.into_owned();
    }
    decode_text_bytes(bytes)
}

fn decode_text_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, _) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
        return cow.into_owned();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
        return cow.into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    let sample = &bytes[..bytes.len().min(64 * 1024)];
    let mut best_encoding = encoding_rs::WINDOWS_1251;
    let mut best_score = i64::MIN;
    for encoding in [
        encoding_rs::WINDOWS_1251,
        encoding_rs::WINDOWS_1252,
        encoding_rs::KOI8_R,
        encoding_rs::IBM866,
    ] {
        let (decoded, _, _) = encoding.decode(sample);
        let score = score_decoded_text(&decoded);
        if score > best_score {
            best_score = score;
            best_encoding = encoding;
        }
    }
    let (decoded, _, _) = best_encoding.decode(bytes);
    decoded.into_owned()
}

fn score_decoded_text(text: &str) -> i64 {
    let mut score = 0i64;
    for c in text.chars() {
        if c == '\u{FFFD}' {
            score -= 1000;
        } else if c.is_control() && !c.is_whitespace() {
            score -= 200;
        } else if c.is_ascii_alphabetic() || c.is_ascii_digit() {
            score += 1;
        } else if is_cyrillic(c) {
            score += 3;
        } else if c.is_alphabetic() {
            score += 1;
        } else if c.is_ascii_punctuation() {
            score += 1;
        } else if matches!(c, '\u{2010}'..='\u{2027}' | '\u{2030}'..='\u{205E}') {
            score += 2;
        } else if !c.is_whitespace() {
            score -= 1;
        }
    }

    for word in text.split_whitespace() {
        let mut has_latin = false;
        let mut has_cyrillic = false;
        for c in word.chars() {
            if is_cyrillic(c) {
                has_cyrillic = true;
            } else if c.is_alphabetic() {
                has_latin = true;
            }
        }
        if has_latin && has_cyrillic {
            score -= 12;
        }
    }

    let lower = text.to_lowercase();
    for bigram in [
        "ст", "но", "то", "на", "ен", "ов", "ни", "ра", "во", "ко", "пр", "ро", "по", "го", "ос",
        "ли", "ре", "ал", "ер", "ть", "ся", "не", "ом", "от", "ка", "та", "те", "ла", "ло", "ан",
        "ор", "де", "ит", "за", "из", "ел", "ль", "ны", "бы", "ьн", "ей", "ри", "ив", "ве", "ет",
        "чт", "эт", "ак", "дл", "ил",
    ] {
        score += (lower.matches(bigram).count() as i64) * 3;
    }
    for trigram in [
        "про", "при", "что", "это", "как", "для", "или", "его", "она", "они", "ест",
    ] {
        score += (lower.matches(trigram).count() as i64) * 4;
    }
    score
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{052F}')
}

fn parse_markdown_chapters(text: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    let mut current_title = String::new();
    let mut current_paragraphs = Vec::new();
    let mut current_p = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            if !trimmed.is_empty() {
                current_paragraphs.push(trimmed.to_string());
            }
            continue;
        }
        if let Some(heading) = markdown_heading(trimmed) {
            flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
            if !current_title.is_empty() || !current_paragraphs.is_empty() {
                push_markdown_chapter(
                    &mut chapters,
                    &mut toc,
                    &current_title,
                    default_title,
                    std::mem::take(&mut current_paragraphs),
                );
            }
            current_title = render_markdown_inline(heading);
        } else if trimmed.is_empty() {
            flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
        } else if is_markdown_rule(trimmed) {
            flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
        } else if let Some(item) = markdown_list_item(trimmed) {
            flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
            current_paragraphs.push(item);
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
            current_paragraphs.push(format!("<i>{}</i>", render_markdown_inline(quote)));
        } else {
            let rendered = render_markdown_inline(trimmed);
            if !current_p.is_empty() {
                current_p.push(' ');
            }
            current_p.push_str(&rendered);
        }
    }

    flush_markdown_paragraph(&mut current_paragraphs, &mut current_p);
    if !current_title.is_empty() || !current_paragraphs.is_empty() || chapters.is_empty() {
        push_markdown_chapter(
            &mut chapters,
            &mut toc,
            &current_title,
            default_title,
            current_paragraphs,
        );
    }

    (chapters, toc)
}

fn flush_markdown_paragraph(paragraphs: &mut Vec<String>, current: &mut String) {
    if !current.trim().is_empty() {
        paragraphs.push(current.trim().to_string());
        current.clear();
    }
}

fn push_markdown_chapter(
    chapters: &mut Vec<BookChapter>,
    toc: &mut Vec<TocItem>,
    title: &str,
    default_title: &str,
    paragraphs: Vec<String>,
) {
    let idx = chapters.len();
    let chapter_title = if title.is_empty() {
        default_title.to_string()
    } else {
        title.to_string()
    };
    chapters.push(BookChapter {
        id: Some(format!("md_{idx}")),
        title: chapter_title.clone(),
        paragraphs,
    });
    toc.push(TocItem {
        id: Some(format!("md_{idx}")),
        title: chapter_title,
        chapter_index: idx,
        children: Vec::new(),
    });
}

fn markdown_heading(line: &str) -> Option<&str> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    line[hashes..].strip_prefix(' ').map(str::trim)
}

fn is_markdown_rule(line: &str) -> bool {
    let mut chars = line.chars().filter(|c| !c.is_whitespace());
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '-' | '*' | '_') {
        return false;
    }
    let mut count = 1;
    for c in chars {
        if c != first {
            return false;
        }
        count += 1;
    }
    count >= 3
}

fn markdown_list_item(line: &str) -> Option<String> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(format!("• {}", render_markdown_inline(rest)));
        }
    }
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && line[digits..].starts_with(". ") {
        let rest = &line[digits + 2..];
        return Some(format!(
            "{}. {}",
            &line[..digits],
            render_markdown_inline(rest)
        ));
    }
    None
}

fn render_markdown_inline(text: &str) -> String {
    render_markdown_inline_depth(text, 0)
}

fn render_markdown_inline_depth(text: &str, depth: usize) -> String {
    if depth >= 4 {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\'
            && let Some(next) = chars.get(i + 1)
        {
            out.push(*next);
            i += 2;
            continue;
        }
        if c == '!'
            && chars.get(i + 1) == Some(&'[')
            && let Some((label, end)) = parse_markdown_link(&chars, i + 1)
        {
            out.push_str(&render_markdown_inline_depth(&label, depth + 1));
            i = end;
            continue;
        }
        if c == '['
            && let Some((label, end)) = parse_markdown_link(&chars, i)
        {
            out.push_str(&render_markdown_inline_depth(&label, depth + 1));
            i = end;
            continue;
        }
        if c == '`'
            && let Some(end) = find_markdown_marker(&chars, i + 1, &['`'])
        {
            out.extend(chars[i + 1..end].iter());
            i = end + 1;
            continue;
        }
        if matches!(c, '*' | '_' | '~') {
            let double = chars.get(i + 1) == Some(&c);
            if double && c != '~' {
                if let Some(end) = find_markdown_double(&chars, i + 2, c) {
                    let inner: String = chars[i + 2..end].iter().collect();
                    let rendered = render_markdown_inline_depth(&inner, depth + 1);
                    out.push_str("<b>");
                    out.push_str(&rendered);
                    out.push_str("</b>");
                    i = end + 2;
                    continue;
                }
            } else if double && c == '~' {
                if let Some(end) = find_markdown_double(&chars, i + 2, c) {
                    let inner: String = chars[i + 2..end].iter().collect();
                    out.push_str(&render_markdown_inline_depth(&inner, depth + 1));
                    i = end + 2;
                    continue;
                }
            } else if c != '~' {
                let boundary_before = i == 0 || !chars[i - 1].is_alphanumeric();
                let boundary_after_space =
                    chars.get(i + 1).is_some_and(|next| !next.is_whitespace());
                if boundary_before
                    && boundary_after_space
                    && let Some(end) = find_markdown_marker(&chars, i + 1, &[c])
                {
                    let boundary_after =
                        end + 1 >= chars.len() || !chars[end + 1].is_alphanumeric();
                    if boundary_after {
                        let inner: String = chars[i + 1..end].iter().collect();
                        let rendered = render_markdown_inline_depth(&inner, depth + 1);
                        out.push_str("<i>");
                        out.push_str(&rendered);
                        out.push_str("</i>");
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn parse_markdown_link(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) != Some(&'[') {
        return None;
    }
    let close = chars
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, c)| **c == ']')
        .map(|(idx, _)| idx)?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars
        .iter()
        .enumerate()
        .skip(close + 2)
        .find(|(_, c)| **c == ')')
        .map(|(idx, _)| idx)?;
    let label: String = chars[start + 1..close].iter().collect();
    Some((label, end + 1))
}

fn find_markdown_marker(chars: &[char], start: usize, marker: &[char]) -> Option<usize> {
    let mut i = start;
    while i + marker.len() <= chars.len() {
        if chars[i..i + marker.len()] == marker[..] {
            if i > start {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_markdown_double(chars: &[char], start: usize, marker: char) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker && i > start {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_log_chapters(text: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    const LINES_PER_CHUNK: usize = 300;
    let lines: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();

    if lines.is_empty() {
        return (
            vec![BookChapter {
                id: Some("log_0".to_string()),
                title: default_title.to_string(),
                paragraphs: vec![String::new()],
            }],
            vec![TocItem {
                id: Some("log_0".to_string()),
                title: default_title.to_string(),
                chapter_index: 0,
                children: Vec::new(),
            }],
        );
    }

    let mut sections: Vec<(Option<String>, Vec<String>)> = Vec::new();
    let mut current_title = None;
    let mut current_lines = Vec::new();
    for line in &lines {
        if let Some(title) = detect_log_heading(line) {
            if current_title.is_some() || !current_lines.is_empty() {
                sections.push((current_title.take(), std::mem::take(&mut current_lines)));
            }
            current_title = Some(title);
        } else {
            current_lines.push(line.clone());
        }
    }
    if current_title.is_some() || !current_lines.is_empty() {
        sections.push((current_title, current_lines));
    }

    let has_headings = sections.iter().any(|(title, _)| title.is_some());
    if has_headings {
        let mut chapters = Vec::new();
        let mut toc = Vec::new();
        for (title, paragraphs) in sections {
            if paragraphs.is_empty() && title.is_none() {
                continue;
            }
            let idx = chapters.len();
            let chapter_title = title.unwrap_or_else(|| default_title.to_string());
            chapters.push(BookChapter {
                id: Some(format!("log_head_{idx}")),
                title: chapter_title.clone(),
                paragraphs,
            });
            toc.push(TocItem {
                id: Some(format!("log_head_{idx}")),
                title: chapter_title,
                chapter_index: idx,
                children: Vec::new(),
            });
        }
        if !chapters.is_empty() {
            return (chapters, toc);
        }
    }

    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    for (idx, chunk) in lines.chunks(LINES_PER_CHUNK).enumerate() {
        let start_line = idx * LINES_PER_CHUNK + 1;
        let end_line = start_line + chunk.len() - 1;
        let title = if lines.len() <= LINES_PER_CHUNK {
            default_title.to_string()
        } else {
            format!("{default_title} (строки {start_line}–{end_line})")
        };
        chapters.push(BookChapter {
            id: Some(format!("log_{idx}")),
            title: title.clone(),
            paragraphs: chunk.to_vec(),
        });
        toc.push(TocItem {
            id: Some(format!("log_{idx}")),
            title,
            chapter_index: idx,
            children: Vec::new(),
        });
    }
    (chapters, toc)
}

fn detect_log_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.chars().count() < 4 {
        return None;
    }
    let has_marker =
        trimmed.starts_with("===") || trimmed.starts_with("---") || trimmed.starts_with("###");
    if has_marker {
        let cleaned = trimmed
            .trim_matches(|c: char| matches!(c, '=' | '-' | '#' | ' '))
            .trim();
        if !cleaned.is_empty() && cleaned.chars().count() <= 100 {
            return Some(cleaned.to_string());
        }
    }
    detect_text_heading(trimmed)
}

fn parse_plain_text_chapters(text: &str, default_title: &str) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut sections: Vec<(Option<String>, Vec<String>)> = Vec::new();
    let mut current_title = None;
    let mut current_paragraphs = Vec::new();
    let mut current_p = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(title) = detect_text_heading(trimmed) {
            if !current_p.is_empty() {
                current_paragraphs.push(std::mem::take(&mut current_p));
            }
            if current_title.is_some() || !current_paragraphs.is_empty() {
                sections.push((
                    current_title.take(),
                    std::mem::take(&mut current_paragraphs),
                ));
            }
            current_title = Some(title);
        } else if trimmed.is_empty() {
            if !current_p.is_empty() {
                current_paragraphs.push(std::mem::take(&mut current_p));
            }
        } else {
            if !current_p.is_empty() {
                current_p.push(' ');
            }
            current_p.push_str(trimmed);
        }
    }
    if !current_p.is_empty() {
        current_paragraphs.push(current_p);
    }
    if current_title.is_some() || !current_paragraphs.is_empty() {
        sections.push((current_title, current_paragraphs));
    }

    let has_headings = sections.iter().any(|(title, _)| title.is_some());
    if has_headings {
        let mut chapters = Vec::new();
        let mut toc = Vec::new();
        for (title, paragraphs) in sections {
            if paragraphs.is_empty() && title.is_none() {
                continue;
            }
            let idx = chapters.len();
            let chapter_title = title.unwrap_or_else(|| default_title.to_string());
            chapters.push(BookChapter {
                id: Some(format!("txt_head_{idx}")),
                title: chapter_title.clone(),
                paragraphs,
            });
            toc.push(TocItem {
                id: Some(format!("txt_head_{idx}")),
                title: chapter_title,
                chapter_index: idx,
                children: Vec::new(),
            });
        }
        if !chapters.is_empty() {
            return (chapters, toc);
        }
    }

    let mut paragraphs = Vec::new();
    let mut current_p = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_p.is_empty() {
                paragraphs.push(std::mem::take(&mut current_p));
            }
        } else {
            if !current_p.is_empty() {
                current_p.push(' ');
            }
            current_p.push_str(trimmed);
        }
    }
    if !current_p.is_empty() {
        paragraphs.push(current_p);
    }

    const PARAGRAPHS_PER_CHAPTER: usize = 250;
    if paragraphs.is_empty() {
        paragraphs.push(String::new());
    }

    let mut chapters = Vec::new();
    let mut toc = Vec::new();
    for (idx, chunk) in paragraphs.chunks(PARAGRAPHS_PER_CHAPTER).enumerate() {
        let title = if paragraphs.len() <= PARAGRAPHS_PER_CHAPTER {
            default_title.to_string()
        } else {
            format!("Раздел {}", idx + 1)
        };
        chapters.push(BookChapter {
            id: Some(format!("txt_{idx}")),
            title: title.clone(),
            paragraphs: chunk.to_vec(),
        });
        toc.push(TocItem {
            id: Some(format!("txt_{idx}")),
            title,
            chapter_index: idx,
            children: Vec::new(),
        });
    }
    (chapters, toc)
}

fn detect_text_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 120 {
        return None;
    }
    let lower = trimmed.to_lowercase();
    for prefix in [
        "глава",
        "chapter",
        "часть",
        "part",
        "книга",
        "book",
        "том",
        "volume",
    ] {
        let Some(rest) = lower.strip_prefix(prefix) else {
            continue;
        };
        if !matches!(rest.chars().next(), Some(' ' | '.' | ':' | '—' | '-')) {
            continue;
        }
        let rest = rest.trim_start_matches(|c: char| matches!(c, ' ' | '.' | ':' | '—' | '-'));
        let rest = rest.trim_start();
        if rest.is_empty() {
            continue;
        }
        let first = rest.chars().next().unwrap_or(' ');
        if first.is_ascii_digit() || is_roman_numeral_prefix(rest) || is_ordinal_prefix(rest) {
            return Some(trimmed.to_string());
        }
    }
    let exact_or_ordinal = [
        "пролог",
        "эпилог",
        "вступление",
        "предисловие",
        "послесловие",
        "prologue",
        "epilogue",
        "introduction",
        "preface",
        "afterword",
    ];
    if exact_or_ordinal
        .iter()
        .any(|word| lower == *word || lower.starts_with(&format!("{word} ")))
    {
        return Some(trimmed.to_string());
    }
    None
}

fn is_roman_numeral_prefix(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, 'i' | 'v' | 'x' | 'l' | 'c') {
        return false;
    }
    let mut next = chars.next();
    while let Some(c) = next {
        if matches!(c, 'i' | 'v' | 'x' | 'l' | 'c') {
            next = chars.next();
        } else {
            break;
        }
    }
    next.is_some_and(|c| matches!(c, ' ' | '.' | ':' | '—' | '-'))
}

fn is_ordinal_prefix(text: &str) -> bool {
    [
        "перв",
        "втор",
        "трет",
        "четв",
        "пят",
        "шест",
        "седьм",
        "восьм",
        "девят",
        "десят",
        "одиннадцат",
        "двенадцат",
        "тринадцат",
        "четырнадцат",
        "пятнадцат",
        "шестнадцат",
        "семнадцат",
        "восемнадцат",
        "девятнадцат",
        "двадцат",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix))
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
