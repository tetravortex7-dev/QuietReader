use crate::document::BookChapter;

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
                    "strong" | "b" => {
                        if in_p && !in_title {
                            if is_closing {
                                current_p.push_str("</b>");
                            } else {
                                current_p.push_str("<b>");
                            }
                        }
                    }
                    "emphasis" | "i" | "em" => {
                        if in_p && !in_title {
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

pub(super) fn normalize_whitespace(s: &str) -> String {
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
