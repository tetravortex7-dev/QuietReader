pub(super) struct ParsedComicInfo {
    pub(super) title: Option<String>,
    pub(super) writer: Option<String>,
    pub(super) is_manga: Option<bool>,
}

pub(super) fn parse_comic_info(bytes: &[u8]) -> Result<ParsedComicInfo, String> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let (mut title, mut series, mut number, mut writer, mut manga_tag) =
        (None, None, None, None, None);
    let mut current_tag = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                current_tag = e.name().as_ref().to_ascii_lowercase();
            }
            Ok(quick_xml::events::Event::Text(ref e)) => {
                let raw = e.as_ref();
                let t = quick_xml::escape::unescape(raw)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                match current_tag.as_str() {
                    "title" => title = Some(t),
                    "series" => series = Some(t),
                    "number" => number = Some(t),
                    "writer" => writer = Some(t),
                    "manga" => manga_tag = Some(t),
                    _ => {}
                }
            }
            Ok(quick_xml::events::Event::End(_)) => current_tag.clear(),
            Ok(quick_xml::events::Event::Eof) => break,
            Err(e) => return Err(crate::document::errors::comic_xml_parse(&e)),
            _ => {}
        }
        buf.clear();
    }

    let full_title = match (series, number, title) {
        (Some(s), Some(n), Some(t)) if !t.is_empty() => Some(format!("{s} #{n}: {t}")),
        (Some(s), Some(n), _) => Some(format!("{s} #{n}")),
        (Some(s), None, Some(t)) if !t.is_empty() => Some(format!("{s}: {t}")),
        (Some(s), None, None) => Some(s),
        (None, _, Some(t)) => Some(t),
        _ => None,
    };
    let is_manga = manga_tag.map(|m| {
        let ml = m.to_ascii_lowercase();
        ml.contains("yes") || ml.contains("righttoleft")
    });
    Ok(ParsedComicInfo {
        title: full_title,
        writer,
        is_manga,
    })
}

fn consume_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u64 {
    let mut num: u64 = 0;
    while let Some(&d) = chars.peek() {
        if let Some(digit) = d.to_digit(10) {
            num = num.saturating_mul(10).saturating_add(digit as u64);
            chars.next();
        } else {
            break;
        }
    }
    num
}

pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let cmp = consume_number(&mut a_chars).cmp(&consume_number(&mut b_chars));
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                } else if ca.is_ascii_digit() {
                    return std::cmp::Ordering::Less;
                } else if cb.is_ascii_digit() {
                    return std::cmp::Ordering::Greater;
                } else {
                    let al = ca.to_lowercase().next().unwrap_or(ca);
                    let bl = cb.to_lowercase().next().unwrap_or(cb);
                    let cmp = al.cmp(&bl);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
        }
    }
}
