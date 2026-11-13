use crate::document::BookChapter;

pub fn extract_fb2_lenient(xml: &str) -> (String, String, Vec<BookChapter>) {
    let mut title = String::new();
    let mut author = String::new();
    let mut chapters = Vec::new();
    let mut current_paragraphs = Vec::new();
    let mut current_title = String::new();
    let mut title_parts: Vec<String> = Vec::new();
    let mut title_direct_buf = String::new();
    let mut book_title_buf = String::new();
    let mut author_buf = String::new();
    let mut in_description = false;
    let mut in_binary = false;
    let mut in_book_title = false;
    let mut in_author = false;
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
                let is_self_closing = !is_closing && tag_raw.ends_with('/');
                let tag_name = tag_raw
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_start_matches('/')
                    .trim_end_matches('/')
                    .to_ascii_lowercase();

                match tag_name.as_str() {
                    "description" => {
                        in_description = !is_closing && !is_self_closing;
                        in_book_title = false;
                        in_author = false;
                        book_title_buf.clear();
                        author_buf.clear();
                    }
                    "binary" => {
                        in_binary = !is_closing && !is_self_closing;
                    }
                    "body" => {
                        in_description = false;
                        in_binary = false;
                        in_book_title = false;
                        in_author = false;
                        in_title = false;
                        in_p = false;
                        book_title_buf.clear();
                        author_buf.clear();
                        title_parts.clear();
                        title_direct_buf.clear();
                        current_p.clear();
                    }
                    "book-title" => {
                        in_author = false;
                        author_buf.clear();
                        if is_closing {
                            if in_book_title && title.is_empty() {
                                let cleaned = normalize_whitespace(&book_title_buf);
                                if !cleaned.is_empty() {
                                    title = cleaned;
                                }
                            }
                            in_book_title = false;
                            book_title_buf.clear();
                        } else if !is_self_closing {
                            in_book_title = true;
                            book_title_buf.clear();
                        }
                    }
                    "author" => {
                        in_book_title = false;
                        book_title_buf.clear();
                        if is_closing {
                            if in_author && author.is_empty() {
                                let cleaned = normalize_whitespace(&author_buf);
                                if !cleaned.is_empty() {
                                    author = cleaned;
                                }
                            }
                            in_author = false;
                            author_buf.clear();
                        } else if !is_self_closing {
                            in_author = true;
                            author_buf.clear();
                        }
                    }
                    "first-name" | "middle-name" | "last-name" | "nickname"
                        if is_closing
                            && in_author
                            && !author_buf.is_empty()
                            && !author_buf.ends_with(char::is_whitespace) =>
                    {
                        author_buf.push(' ');
                    }
                    "title" if !in_description && !in_binary => {
                        if is_closing {
                            if in_p {
                                let p_clean = normalize_whitespace(&current_p);
                                if !p_clean.is_empty() {
                                    title_parts.push(p_clean);
                                }
                            }
                            let resolved = if !title_parts.is_empty() {
                                title_parts.join(" — ")
                            } else {
                                normalize_whitespace(&title_direct_buf)
                            };
                            if !resolved.is_empty() {
                                current_title = resolved;
                            }
                            in_title = false;
                            in_p = false;
                            current_p.clear();
                            title_parts.clear();
                            title_direct_buf.clear();
                        } else if !is_self_closing {
                            in_title = true;
                            in_p = false;
                            current_p.clear();
                            title_parts.clear();
                            title_direct_buf.clear();
                        }
                    }
                    "section" if !in_description && !in_binary => {
                        in_book_title = false;
                        in_author = false;
                        book_title_buf.clear();
                        author_buf.clear();
                        in_p = false;
                        current_p.clear();
                        in_title = false;
                        title_parts.clear();
                        title_direct_buf.clear();
                        if is_closing {
                            if !current_paragraphs.is_empty() || !current_title.is_empty() {
                                chapters.push(BookChapter {
                                    id: None,
                                    title: std::mem::take(&mut current_title),
                                    paragraphs: std::mem::take(&mut current_paragraphs),
                                });
                            }
                        } else if !is_self_closing && !current_paragraphs.is_empty() {
                            chapters.push(BookChapter {
                                id: None,
                                title: std::mem::take(&mut current_title),
                                paragraphs: std::mem::take(&mut current_paragraphs),
                            });
                        }
                    }
                    "p" | "v" | "subtitle" if !in_description && !in_binary => {
                        if is_closing {
                            in_p = false;
                            let p_clean = normalize_whitespace(&current_p);
                            if !p_clean.is_empty() {
                                if in_title {
                                    title_parts.push(p_clean);
                                } else {
                                    current_paragraphs.push(p_clean);
                                }
                            }
                            current_p.clear();
                        } else if !is_self_closing {
                            in_p = true;
                            current_p.clear();
                        }
                    }
                    "strong" | "b" if !in_description && !in_binary => {
                        if in_p && !in_title {
                            if is_closing {
                                current_p.push_str("</b>");
                            } else if !is_self_closing {
                                current_p.push_str("<b>");
                            }
                        }
                    }
                    "emphasis" | "i" | "em" if !in_description && !in_binary => {
                        if in_p && !in_title {
                            if is_closing {
                                current_p.push_str("</i>");
                            } else if !is_self_closing {
                                current_p.push_str("<i>");
                            }
                        }
                    }
                    _ => {}
                }
                i += 1 + end_bracket + 1;
                continue;
            }
            break;
        }

        let captures_text =
            in_book_title || in_author || (!in_description && !in_binary && (in_p || in_title));
        if captures_text {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' {
                i += 1;
            }
            if let Ok(text_slice) = std::str::from_utf8(&bytes[start..i]) {
                let unescaped = quick_xml::escape::unescape(text_slice)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| text_slice.to_string());
                if in_book_title {
                    book_title_buf.push_str(&unescaped);
                } else if in_author {
                    author_buf.push_str(&unescaped);
                } else if in_p {
                    current_p.push_str(&unescaped);
                } else if in_title {
                    title_direct_buf.push_str(&unescaped);
                }
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
