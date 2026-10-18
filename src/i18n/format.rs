use super::UiLanguage;
use crate::document::DocumentError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

pub fn plural_category(n: usize, lang: UiLanguage) -> PluralCategory {
    match lang.resolve() {
        UiLanguage::Ru | UiLanguage::Uk => {
            let mod10 = n % 10;
            let mod100 = n % 100;
            if mod10 == 1 && mod100 != 11 {
                PluralCategory::One
            } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
                PluralCategory::Few
            } else {
                PluralCategory::Many
            }
        }
        UiLanguage::Ar => {
            let mod100 = n % 100;
            if n == 0 {
                PluralCategory::Zero
            } else if n == 1 {
                PluralCategory::One
            } else if n == 2 {
                PluralCategory::Two
            } else if (3..=10).contains(&mod100) {
                PluralCategory::Few
            } else if (11..=99).contains(&mod100) {
                PluralCategory::Many
            } else {
                PluralCategory::Other
            }
        }
        UiLanguage::Fr => {
            if n <= 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }
        UiLanguage::En | UiLanguage::Es | UiLanguage::It | UiLanguage::Pt | UiLanguage::Tr => {
            if n == 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }
        UiLanguage::Zh | UiLanguage::Ja | UiLanguage::Id => PluralCategory::Other,
        UiLanguage::Auto => PluralCategory::Other,
    }
}

pub fn plural_pages(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    let num_str = format_number(n, lang);
    match plural_category(n, lang) {
        PluralCategory::One => format!("{num_str} {}", t.word_pages_one),
        PluralCategory::Few => format!("{num_str} {}", t.word_pages_few),
        _ => format!("{num_str} {}", t.word_pages_many),
    }
}

pub fn plural_chapters(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    let num_str = format_number(n, lang);
    match plural_category(n, lang) {
        PluralCategory::One => format!("{num_str} {}", t.word_chapters_one),
        PluralCategory::Few => format!("{num_str} {}", t.word_chapters_few),
        _ => format!("{num_str} {}", t.word_chapters_many),
    }
}

pub fn plural_matches(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    let num_str = format_number(n, lang);
    match plural_category(n, lang) {
        PluralCategory::One => format!("{num_str} {}", t.word_matches_one),
        PluralCategory::Few => format!("{num_str} {}", t.word_matches_few),
        _ => format!("{num_str} {}", t.word_matches_many),
    }
}

pub fn plural_books(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    let num_str = format_number(n, lang);
    match plural_category(n, lang) {
        PluralCategory::One => format!("{num_str} {}", t.word_books_one),
        PluralCategory::Few => format!("{num_str} {}", t.word_books_few),
        _ => format!("{num_str} {}", t.word_books_many),
    }
}

pub fn format_number(n: usize, lang: UiLanguage) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    let sep = match lang.resolve() {
        UiLanguage::Ru | UiLanguage::Uk | UiLanguage::Fr => ' ',
        UiLanguage::En | UiLanguage::Zh | UiLanguage::Ja => ',',
        UiLanguage::Es | UiLanguage::It | UiLanguage::Pt | UiLanguage::Tr | UiLanguage::Id => '.',
        UiLanguage::Ar => ',',
        UiLanguage::Auto => ' ',
    };
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let rem = s.len() % 3;
    if rem > 0 {
        out.push_str(&s[..rem]);
    }
    for chunk in s[rem..].as_bytes().chunks(3) {
        if !out.is_empty() {
            out.push(sep);
        }
        out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
    }
    out
}

pub fn format_float(val: f32, decimals: usize, lang: UiLanguage) -> String {
    let formatted = format!("{val:.decimals$}");
    let sep = match lang.resolve() {
        UiLanguage::Ru
        | UiLanguage::Uk
        | UiLanguage::Fr
        | UiLanguage::Es
        | UiLanguage::It
        | UiLanguage::Pt
        | UiLanguage::Tr
        | UiLanguage::Ar => ',',
        UiLanguage::En | UiLanguage::Zh | UiLanguage::Ja | UiLanguage::Id | UiLanguage::Auto => '.',
    };
    if sep != '.' {
        formatted.replace('.', ",")
    } else {
        formatted
    }
}

pub fn format_percent(val: f32, lang: UiLanguage) -> String {
    let num = format_float(val, 0, lang);
    match lang.resolve() {
        UiLanguage::Tr => format!("%{num}"),
        UiLanguage::Ar => format!("%{num}"),
        _ => format!("{num}%"),
    }
}

pub fn format_date(year: i32, month: u32, day: u32, lang: UiLanguage) -> String {
    match lang.resolve() {
        UiLanguage::Ru | UiLanguage::Uk | UiLanguage::Tr | UiLanguage::It => {
            format!("{day:02}.{month:02}.{year}")
        }
        UiLanguage::Fr | UiLanguage::Es | UiLanguage::Pt | UiLanguage::Id => {
            format!("{day:02}/{month:02}/{year}")
        }
        UiLanguage::Zh | UiLanguage::Ja => {
            format!("{year}年{month}月{day}日")
        }
        UiLanguage::En => {
            format!("{month:02}/{day:02}/{year}")
        }
        UiLanguage::Ar => {
            format!("{year}/{month:02}/{day:02}")
        }
        UiLanguage::Auto => {
            format!("{day:02}.{month:02}.{year}")
        }
    }
}

pub fn format_relative_time(timestamp_secs: u64, current_secs: u64, lang: UiLanguage) -> String {
    let t = lang.t();
    if current_secs <= timestamp_secs || current_secs - timestamp_secs < 60 {
        return t.date_just_now.to_string();
    }
    let diff = current_secs - timestamp_secs;
    let mins = diff / 60;
    if mins < 60 {
        return t.date_minutes_ago.replace("{}", &mins.to_string());
    }
    let hours = mins / 60;
    if hours < 24 {
        return t.date_hours_ago.replace("{}", &hours.to_string());
    }
    let days = hours / 24;
    if days == 1 {
        return t.date_yesterday.to_string();
    }
    if days < 7 {
        return t.date_days_ago.replace("{}", &days.to_string());
    }
    let days_since_epoch = timestamp_secs / 86400;
    let (y, m, d) = days_to_ymd(days_since_epoch);
    format_date(y, m, d, lang)
}

fn days_to_ymd(days: u64) -> (i32, u32, u32) {
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

pub fn format_document_error(err: &DocumentError, lang: UiLanguage) -> String {
    let t = lang.t();
    match err {
        DocumentError::Io(msg) => format!("{}: {msg}", t.err_io),
        DocumentError::Unsupported(msg) => format!("{}: {msg}", t.err_unsupported),
        DocumentError::Corrupted(msg) => format!("{}: {msg}", t.err_corrupted),
        DocumentError::LimitExceeded(msg) => format!("{}: {msg}", t.err_limit_exceeded),
    }
}

pub fn format_extension_description(ext: &str, lang: UiLanguage) -> &'static str {
    let t = lang.t();
    match ext.to_ascii_lowercase().as_str() {
        "fb2" => t.format_desc_fb2,
        "epub" => t.format_desc_epub,
        "txt" => t.format_desc_txt,
        "doc" => t.format_desc_doc,
        "docx" => t.format_desc_docx,
        "rtf" => t.format_desc_rtf,
        "pdf" => t.format_desc_pdf,
        "djvu" | "djv" => t.format_desc_djvu,
        "cbz" => t.format_desc_cbz,
        "cbr" => t.format_desc_cbr,
        "cb7" => t.format_desc_cb7,
        "md" | "markdown" => t.format_desc_md,
        _ => "",
    }
}
