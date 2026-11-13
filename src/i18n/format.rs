use super::UiLanguage;
use crate::document::DocumentError;
use crate::opds::{OpdsAuthError, OpdsError};

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

fn plural_word<'a>(
    n: usize,
    lang: UiLanguage,
    one: &'a str,
    two: &'a str,
    few: &'a str,
    many: &'a str,
) -> String {
    let num_str = format_number(n, lang);
    let resolved = lang.resolve();
    let word = match plural_category(n, resolved) {
        PluralCategory::Zero => {
            if matches!(resolved, UiLanguage::Ar) {
                few
            } else {
                many
            }
        }
        PluralCategory::One => one,
        PluralCategory::Two => {
            if matches!(resolved, UiLanguage::Ar) {
                two
            } else {
                few
            }
        }
        PluralCategory::Few => few,
        PluralCategory::Many => many,
        PluralCategory::Other => {
            if matches!(resolved, UiLanguage::Ar) {
                one
            } else {
                many
            }
        }
    };
    format!("{num_str} {word}")
}

pub fn plural_pages(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    plural_word(
        n,
        lang,
        t.word_pages_one,
        t.extra.word_pages_two,
        t.word_pages_few,
        t.word_pages_many,
    )
}

pub fn plural_chapters(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    plural_word(
        n,
        lang,
        t.word_chapters_one,
        t.extra.word_chapters_two,
        t.word_chapters_few,
        t.word_chapters_many,
    )
}

pub fn plural_matches(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    plural_word(
        n,
        lang,
        t.word_matches_one,
        t.extra.word_matches_two,
        t.word_matches_few,
        t.word_matches_many,
    )
}

pub fn plural_books(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    plural_word(
        n,
        lang,
        t.word_books_one,
        t.extra.word_books_two,
        t.word_books_few,
        t.word_books_many,
    )
}

pub fn plural_days(n: usize, lang: UiLanguage) -> String {
    let t = lang.t();
    plural_word(
        n,
        lang,
        t.extra.word_days_one,
        t.extra.word_days_two,
        t.extra.word_days_few,
        t.extra.word_days_many,
    )
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
    if timestamp_secs == 0 {
        return t.extra.date_recently.to_string();
    }
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

fn format_error_detail(prefix: &str, message: &str, lang: UiLanguage) -> String {
    let has_cyrillic = message
        .chars()
        .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    if has_cyrillic && lang.resolve() != UiLanguage::Ru {
        prefix.to_string()
    } else {
        format!("{prefix}: {message}")
    }
}

pub fn format_document_error(err: &DocumentError, lang: UiLanguage) -> String {
    let t = lang.t();
    match err {
        DocumentError::Io(msg) => format_doc_error_detail(t.err_io, msg, lang),
        DocumentError::Unsupported(msg) => format_doc_error_detail(t.err_unsupported, msg, lang),
        DocumentError::Corrupted(msg) => format_doc_error_detail(t.err_corrupted, msg, lang),
        DocumentError::LimitExceeded(msg) => {
            format_doc_error_detail(t.err_limit_exceeded, msg, lang)
        }
        DocumentError::PasswordRequired => t.err_password_required.to_string(),
    }
}

fn format_doc_error_detail(prefix: &str, msg: &str, lang: UiLanguage) -> String {
    if let Some(decoded) = super::error_format::decode_document_error_detail(msg, lang) {
        return format!("{prefix}: {decoded}");
    }
    format_error_detail(prefix, msg, lang)
}

pub fn format_opds_auth_error(err: OpdsAuthError, lang: UiLanguage) -> String {
    let t = lang.t();
    match err {
        OpdsAuthError::InvalidUrl => t.opds_invalid_url.to_string(),
        OpdsAuthError::UnsupportedScheme => t.opds_unsupported_scheme.to_string(),
        OpdsAuthError::HttpsRequired => t.opds_https_required.to_string(),
    }
}

pub fn format_opds_error(err: &OpdsError, lang: UiLanguage) -> String {
    let t = lang.t();
    match err {
        OpdsError::Message(message) => format_error_detail(t.extra.opds_feed_error, message, lang),
        OpdsError::ResponseTooLarge { limit } => t
            .opds_response_too_large
            .replace("{}", &(limit / (1024 * 1024)).to_string()),
        OpdsError::DownloadTooLarge { limit } => t
            .opds_download_too_large
            .replace("{}", &(limit / (1024 * 1024)).to_string()),
        OpdsError::Network(message) => {
            super::fill(t.extra.opds_network_error, &[("{error}", message)])
        }
        OpdsError::ReadResponse(message) => {
            super::fill(t.extra.opds_read_response_error, &[("{error}", message)])
        }
        OpdsError::HttpStatus(status) => super::fill(
            t.extra.opds_http_status,
            &[("{status}", &status.to_string())],
        ),
        OpdsError::OpenSearch(message) => {
            super::fill(t.extra.opds_opensearch_error, &[("{error}", message)])
        }
        OpdsError::InvalidOpenSearch => t.extra.opds_invalid_opensearch.to_string(),
        OpdsError::NoSearchInterface => t.extra.opds_no_search_interface.to_string(),
        OpdsError::DownloadConnect(message) => {
            super::fill(t.extra.opds_download_connect_error, &[("{error}", message)])
        }
        OpdsError::DownloadServer(status) => super::fill(
            t.extra.opds_download_server_error,
            &[("{status}", &status.to_string())],
        ),
        OpdsError::CreateFile(message) => {
            super::fill(t.extra.opds_create_file_error, &[("{error}", message)])
        }
        OpdsError::Transfer(message) => {
            super::fill(t.extra.opds_transfer_error, &[("{error}", message)])
        }
        OpdsError::WriteFile(message) => {
            super::fill(t.extra.opds_write_file_error, &[("{error}", message)])
        }
        OpdsError::Flush(message) => super::fill(t.extra.opds_flush_error, &[("{error}", message)]),
        OpdsError::MoveFile(message) => {
            super::fill(t.extra.opds_move_file_error, &[("{error}", message)])
        }
    }
}

pub fn format_extension_description(ext: &str, lang: UiLanguage) -> &'static str {
    use crate::document::registry::FormatDescKey;
    let Some(key) = crate::document::registry::description_key_for_extension(ext) else {
        return "";
    };
    let t = lang.t();
    match key {
        FormatDescKey::Fb2 => t.format_desc_fb2,
        FormatDescKey::Fb2Zip => t.extra.format_desc_fb2_zip,
        FormatDescKey::Epub => t.format_desc_epub,
        FormatDescKey::Mobi => t.extra.format_desc_mobi,
        FormatDescKey::Azw3 => t.extra.format_desc_azw3,
        FormatDescKey::Log => t.extra.format_desc_log,
        FormatDescKey::Txt => t.format_desc_txt,
        FormatDescKey::Doc => t.format_desc_doc,
        FormatDescKey::Dot => t.extra.format_desc_dot,
        FormatDescKey::Docx => t.format_desc_docx,
        FormatDescKey::Rtf => t.format_desc_rtf,
        FormatDescKey::Pdf => t.format_desc_pdf,
        FormatDescKey::Djvu => t.format_desc_djvu,
        FormatDescKey::Cbz => t.format_desc_cbz,
        FormatDescKey::Cbr => t.format_desc_cbr,
        FormatDescKey::Cb7 => t.format_desc_cb7,
        FormatDescKey::Cbt => t.format_desc_cbt,
        FormatDescKey::Zip => t.extra.format_desc_zip,
        FormatDescKey::Rar => t.extra.format_desc_rar,
        FormatDescKey::SevenZ => t.extra.format_desc_7z,
        FormatDescKey::Tar => t.extra.format_desc_tar,
        FormatDescKey::Md => t.format_desc_md,
        FormatDescKey::Html => t.extra.format_desc_html,
        FormatDescKey::Xml => t.extra.format_desc_xml,
    }
}
