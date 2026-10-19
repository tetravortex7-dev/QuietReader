pub use crate::app::opds_view::OpdsBrowserState;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsCatalog {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsAcquisitionLink {
    pub href: String,
    pub mime_type: String,
    pub title: String,
    pub format_label: String,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsEntry {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub updated: Option<String>,
    pub categories: Vec<String>,
    pub thumbnail_url: Option<String>,
    pub acquisition_links: Vec<OpdsAcquisitionLink>,
    pub navigation_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsLink {
    pub rel: String,
    pub href: String,
    pub link_type: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsFeed {
    pub title: String,
    pub id: String,
    pub icon: Option<String>,
    pub links: Vec<OpdsLink>,
    pub entries: Vec<OpdsEntry>,
    pub next_url: Option<String>,
    pub prev_url: Option<String>,
    pub search_template: Option<String>,
    pub search_descriptor_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    Queued,
    Downloading {
        downloaded: usize,
        total: Option<usize>,
    },
    Completed {
        file_path: PathBuf,
    },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct DownloadItem {
    pub book_id: String,
    pub title: String,
    pub author: String,
    pub format_label: String,
    pub status: DownloadStatus,
}

pub fn default_opds_catalogs() -> Vec<OpdsCatalog> {
    vec![
        OpdsCatalog {
            id: "standard_ebooks".into(),
            name: "Standard Ebooks".into(),
            url: "https://standardebooks.org/opds/all-records".into(),
            username: None,
            password: None,
            is_default: true,
        },
        OpdsCatalog {
            id: "gutenberg".into(),
            name: "Project Gutenberg".into(),
            url: "https://m.gutenberg.org/ebooks.opds/".into(),
            username: None,
            password: None,
            is_default: true,
        },
        OpdsCatalog {
            id: "feedbooks".into(),
            name: "Feedbooks Free".into(),
            url: "https://openbooks.feedbooks.com/catalog.atom".into(),
            username: None,
            password: None,
            is_default: true,
        },
    ]
}

pub fn resolve_relative_url(base: &str, relative: &str) -> String {
    let rel = relative.trim();
    if rel.is_empty() {
        return base.to_string();
    }
    if rel.starts_with("http://") || rel.starts_with("https://") {
        return rel.to_string();
    }
    if rel.starts_with("//") {
        let scheme = if base.starts_with("https:") {
            "https:"
        } else {
            "http:"
        };
        return format!("{scheme}{rel}");
    }

    let (scheme, rest) = if let Some((s, r)) = base.split_once("://") {
        (s, r)
    } else {
        ("http", base)
    };

    let (host_port, path_query) = if let Some((hp, pq)) = rest.split_once('/') {
        (hp, format!("/{pq}"))
    } else {
        (rest, "/".to_string())
    };

    if rel.starts_with('/') {
        return format!("{scheme}://{host_port}{rel}");
    }

    let path_only = path_query.split('?').next().unwrap_or("/");
    let dir = if let Some(idx) = path_only.rfind('/') {
        &path_only[..=idx]
    } else {
        "/"
    };

    let combined = format!("{dir}{rel}");
    let mut segments = Vec::new();
    for seg in combined.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        } else if seg == ".." {
            segments.pop();
        } else {
            segments.push(seg);
        }
    }

    let normalized_path = segments.join("/");
    format!("{scheme}://{host_port}/{normalized_path}")
}

pub fn percent_encode_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

pub fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in chunks.by_ref() {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(CHARSET[((n >> 18) & 63) as usize] as char);
        out.push(CHARSET[((n >> 12) & 63) as usize] as char);
        out.push(CHARSET[((n >> 6) & 63) as usize] as char);
        out.push(CHARSET[(n & 63) as usize] as char);
    }
    let rem = chunks.remainder();
    if rem.len() == 1 {
        let n = (rem[0] as u32) << 16;
        out.push(CHARSET[((n >> 18) & 63) as usize] as char);
        out.push(CHARSET[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem.len() == 2 {
        let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
        out.push(CHARSET[((n >> 18) & 63) as usize] as char);
        out.push(CHARSET[((n >> 12) & 63) as usize] as char);
        out.push(CHARSET[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

pub fn sanitize_filename(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = clean.trim().trim_matches('.');
    if trimmed.is_empty() {
        "book".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn detect_format_and_ext(mime: &str, href: &str) -> Option<(&'static str, &'static str)> {
    let m = mime.to_ascii_lowercase();
    let h = href.to_ascii_lowercase();
    const FORMATS: &[(&[&str], &[&str], &str, &str)] = &[
        (&["epub"], &[".epub"], "EPUB", "epub"),
        (&["fictionbook", "fb2"], &[".fb2", ".fb2.zip"], "FB2", "fb2"),
        (&["x-mobi8", "azw3"], &[".azw3"], "AZW3", "azw3"),
        (&["mobipocket", "mobi"], &[".mobi", ".prc"], "MOBI", "mobi"),
        (&["pdf"], &[".pdf"], "PDF", "pdf"),
        (&["djvu"], &[".djvu", ".djv"], "DJVU", "djvu"),
        (&["cbz"], &[".cbz"], "CBZ", "cbz"),
        (&["cbr"], &[".cbr"], "CBR", "cbr"),
        (&["text/plain"], &[".txt"], "TXT", "txt"),
    ];
    for (mimes, exts, badge, ext) in FORMATS {
        if mimes.iter().any(|k| m.contains(k)) || exts.iter().any(|e| h.ends_with(e)) {
            return Some((badge, ext));
        }
    }
    None
}

pub fn make_unique_filepath(downloads_dir: &Path, author: &str, title: &str, ext: &str) -> PathBuf {
    let a = sanitize_filename(author);
    let t = sanitize_filename(title);
    let prefix = if a.is_empty() {
        t
    } else {
        format!("{a} - {t}")
    };
    let base_name: String = prefix.chars().take(180).collect();
    let candidate = downloads_dir.join(format!("{base_name}.{ext}"));
    if !candidate.exists() {
        return candidate;
    }
    for i in 1..1000 {
        let candidate = downloads_dir.join(format!("{base_name} ({i}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    downloads_dir.join(format!(
        "{base_name}_{}.{ext}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    ))
}

pub fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

pub fn parse_opensearch_template(xml_str: &str, base_url: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                if name.as_ref().eq_ignore_ascii_case("url") {
                    let mut is_atom = false;
                    let mut template = None;
                    for attr in e.attributes().flatten() {
                        let key = attr.key.local_name();
                        if key.as_ref().eq_ignore_ascii_case("type") {
                            let val = attr.value.as_ref();
                            if val.contains("atom+xml") || val.contains("opds") {
                                is_atom = true;
                            }
                        } else if key.as_ref().eq_ignore_ascii_case("template") {
                            template = Some(attr.value.to_string());
                        }
                    }
                    if is_atom && let Some(tmpl) = template {
                        return Some(resolve_relative_url(base_url, &tmpl));
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}

pub fn parse_opds_feed(xml_str: &str, base_url: &str) -> Result<OpdsFeed, String> {
    let mut reader = Reader::from_str(xml_str);
    reader.config_mut().trim_text(false);

    let mut feed_title = String::new();
    let mut feed_id = String::new();
    let mut feed_icon_raw = String::new();
    let mut feed_links = Vec::new();
    let mut entries = Vec::new();

    let mut next_url = None;
    let mut prev_url = None;
    let mut search_template = None;
    let mut search_descriptor_url = None;

    let mut in_feed = false;
    let mut in_entry = false;
    let mut in_author = false;
    let mut current_tag = String::new();

    let mut cur_entry_id = String::new();
    let mut cur_entry_title = String::new();
    let mut cur_author_name = String::new();
    let mut cur_entry_authors: Vec<String> = Vec::new();
    let mut cur_entry_summary_raw = String::new();
    let mut cur_entry_summary = None;
    let mut cur_entry_updated = None;
    let mut cur_entry_categories = Vec::new();
    let mut cur_entry_thumb = None;
    let mut cur_entry_acquisitions: Vec<OpdsAcquisitionLink> = Vec::new();
    let mut cur_entry_nav = None;

    let mut buf = Vec::new();

    macro_rules! append_active {
        ($txt:expr) => {
            if in_entry {
                if current_tag.eq_ignore_ascii_case("title") {
                    cur_entry_title.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("id") {
                    cur_entry_id.push_str($txt.trim());
                } else if in_author && current_tag.eq_ignore_ascii_case("name") {
                    cur_author_name.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("summary")
                    || current_tag.eq_ignore_ascii_case("content")
                {
                    cur_entry_summary_raw.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("updated")
                    || current_tag.eq_ignore_ascii_case("published")
                {
                    if cur_entry_updated.is_none() {
                        cur_entry_updated = Some($txt.trim().to_string());
                    }
                }
            } else if in_feed {
                if current_tag.eq_ignore_ascii_case("title") {
                    feed_title.push_str($txt);
                } else if current_tag.eq_ignore_ascii_case("id") {
                    feed_id.push_str($txt.trim());
                } else if current_tag.eq_ignore_ascii_case("icon")
                    || current_tag.eq_ignore_ascii_case("logo")
                {
                    feed_icon_raw.push_str($txt.trim());
                }
            }
        };
    }

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.local_name().as_ref().to_string();
                if name.eq_ignore_ascii_case("feed") {
                    in_feed = true;
                } else if name.eq_ignore_ascii_case("entry") {
                    in_entry = true;
                    cur_entry_id.clear();
                    cur_entry_title.clear();
                    cur_author_name.clear();
                    cur_entry_authors.clear();
                    cur_entry_summary_raw.clear();
                    cur_entry_summary = None;
                    cur_entry_updated = None;
                    cur_entry_categories.clear();
                    cur_entry_thumb = None;
                    cur_entry_acquisitions.clear();
                    cur_entry_nav = None;
                } else if name.eq_ignore_ascii_case("author") {
                    in_author = true;
                    cur_author_name.clear();
                }
                current_tag = name;
            }
            Ok(Event::Empty(ref e)) => {
                let name = e.local_name();
                if name.as_ref().eq_ignore_ascii_case("link") {
                    let mut href = String::new();
                    let mut rel = String::new();
                    let mut mime_type = String::new();
                    let mut title = String::new();
                    let mut template = None;

                    for attr in e.attributes().flatten() {
                        let k = attr.key.local_name();
                        let v = attr.value.to_string();
                        if k.as_ref().eq_ignore_ascii_case("href") {
                            href = v;
                        } else if k.as_ref().eq_ignore_ascii_case("rel") {
                            rel = v;
                        } else if k.as_ref().eq_ignore_ascii_case("type") {
                            mime_type = v;
                        } else if k.as_ref().eq_ignore_ascii_case("title") {
                            title = v;
                        } else if k.as_ref().eq_ignore_ascii_case("template") {
                            template = Some(v);
                        }
                    }

                    let resolved_href = resolve_relative_url(base_url, &href);

                    if in_entry {
                        let rel_lower = rel.to_ascii_lowercase();
                        let type_lower = mime_type.to_ascii_lowercase();

                        if rel_lower.contains("image")
                            || rel_lower.contains("thumbnail")
                            || rel_lower.contains("cover")
                        {
                            if cur_entry_thumb.is_none() || rel_lower.contains("thumbnail") {
                                cur_entry_thumb = Some(resolved_href.clone());
                            }
                        } else if rel_lower.contains("acquisition")
                            || (rel_lower.is_empty()
                                && detect_format_and_ext(&mime_type, &href).is_some())
                        {
                            if let Some((badge, ext)) = detect_format_and_ext(&mime_type, &href) {
                                cur_entry_acquisitions.push(OpdsAcquisitionLink {
                                    href: resolved_href.clone(),
                                    mime_type: mime_type.clone(),
                                    title: if title.is_empty() {
                                        badge.to_string()
                                    } else {
                                        title.clone()
                                    },
                                    format_label: badge.to_string(),
                                    extension: ext.to_string(),
                                });
                            }
                        } else if rel_lower.contains("subsection")
                            || type_lower.contains("atom+xml")
                            || rel_lower == "alternate" && type_lower.contains("opds")
                        {
                            if cur_entry_nav.is_none() {
                                cur_entry_nav = Some(resolved_href.clone());
                            }
                        }
                    } else if in_feed {
                        let rel_lower = rel.to_ascii_lowercase();
                        let type_lower = mime_type.to_ascii_lowercase();

                        if rel_lower == "next" {
                            next_url = Some(resolved_href.clone());
                        } else if rel_lower == "prev" || rel_lower == "previous" {
                            prev_url = Some(resolved_href.clone());
                        } else if rel_lower == "search" {
                            if let Some(tmpl) = template {
                                search_template = Some(resolve_relative_url(base_url, &tmpl));
                            } else if href.contains("{searchTerms}") {
                                search_template = Some(resolved_href.clone());
                            } else if type_lower.contains("opensearchdescription") {
                                search_descriptor_url = Some(resolved_href.clone());
                            }
                        }

                        feed_links.push(OpdsLink {
                            rel,
                            href: resolved_href,
                            link_type: if mime_type.is_empty() {
                                None
                            } else {
                                Some(mime_type)
                            },
                            title: if title.is_empty() { None } else { Some(title) },
                        });
                    }
                } else if name.as_ref().eq_ignore_ascii_case("category") && in_entry {
                    for attr in e.attributes().flatten() {
                        let k = attr.key.local_name();
                        if k.as_ref().eq_ignore_ascii_case("label")
                            || k.as_ref().eq_ignore_ascii_case("term")
                        {
                            let val = attr.value.trim().to_string();
                            if !val.is_empty() && !cur_entry_categories.contains(&val) {
                                cur_entry_categories.push(val);
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                append_active!(&text);
            }
            Ok(Event::CData(ref e)) => {
                let text = e.as_ref();
                append_active!(text);
            }
            Ok(Event::GeneralRef(ref e)) => {
                let code = e.as_ref();
                let repl_char = match code {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    s if s.starts_with("#x") || s.starts_with("#X") => {
                        u32::from_str_radix(&s[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                    }
                    s if s.starts_with('#') => s[1..].parse::<u32>().ok().and_then(char::from_u32),
                    _ => None,
                };
                if let Some(ch) = repl_char {
                    let mut tmp = [0u8; 4];
                    let text = ch.encode_utf8(&mut tmp);
                    append_active!(text);
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.local_name();
                if name.as_ref().eq_ignore_ascii_case("entry") {
                    let final_title = cur_entry_title.trim().to_string();
                    let final_id = if cur_entry_id.is_empty() {
                        final_title.clone()
                    } else {
                        cur_entry_id.clone()
                    };
                    let final_summary = if !cur_entry_summary_raw.is_empty() {
                        let stripped = strip_html_tags(cur_entry_summary_raw.trim());
                        if !stripped.is_empty() {
                            Some(stripped)
                        } else {
                            None
                        }
                    } else {
                        cur_entry_summary.take()
                    };

                    entries.push(OpdsEntry {
                        id: final_id,
                        title: final_title,
                        authors: cur_entry_authors.clone(),
                        summary: final_summary,
                        updated: cur_entry_updated.take(),
                        categories: cur_entry_categories.clone(),
                        thumbnail_url: cur_entry_thumb.take(),
                        acquisition_links: cur_entry_acquisitions.clone(),
                        navigation_link: cur_entry_nav.take(),
                    });
                    in_entry = false;
                } else if name.as_ref().eq_ignore_ascii_case("author") {
                    let clean_auth = cur_author_name.trim().to_string();
                    if !clean_auth.is_empty() && !cur_entry_authors.contains(&clean_auth) {
                        cur_entry_authors.push(clean_auth);
                    }
                    cur_author_name.clear();
                    in_author = false;
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML error: {e}")),
            _ => {}
        }
        buf.clear();
    }

    let final_feed_title = feed_title.trim().to_string();
    let feed_title = if final_feed_title.is_empty() {
        "OPDS Catalog".to_string()
    } else {
        final_feed_title
    };

    let feed_icon = if !feed_icon_raw.is_empty() {
        Some(resolve_relative_url(base_url, &feed_icon_raw))
    } else {
        None
    };

    Ok(OpdsFeed {
        title: feed_title,
        id: feed_id,
        icon: feed_icon,
        links: feed_links,
        entries,
        next_url,
        prev_url,
        search_template,
        search_descriptor_url,
    })
}

#[derive(Debug, Clone)]
pub enum OpdsCommand {
    FetchFeed {
        request_id: u64,
        url: String,
        auth: Option<(String, String)>,
    },
    ResolveSearchAndFetch {
        request_id: u64,
        search_template: Option<String>,
        descriptor_url: Option<String>,
        query: String,
        auth: Option<(String, String)>,
    },
    DownloadBook {
        download_id: String,
        url: String,
        title: String,
        author: String,
        ext: String,
        auth: Option<(String, String)>,
        downloads_dir: PathBuf,
    },
}

#[derive(Debug)]
pub enum OpdsEvent {
    FeedLoaded {
        request_id: u64,
        url: String,
        result: Result<OpdsFeed, String>,
    },
    DownloadProgress {
        download_id: String,
        downloaded: usize,
        total: Option<usize>,
    },
    DownloadFinished {
        download_id: String,
        result: Result<PathBuf, String>,
    },
}

pub struct OpdsClient {
    tx_cmd: Sender<OpdsCommand>,
    rx_event: Receiver<OpdsEvent>,
    next_id: Arc<AtomicU64>,
}

impl Default for OpdsClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OpdsClient {
    pub fn new() -> Self {
        let (tx_cmd, rx_cmd) = channel::<OpdsCommand>();
        let (tx_event, rx_event) = channel::<OpdsEvent>();
        let next_id = Arc::new(AtomicU64::new(1));

        let tx_evt_clone = tx_event.clone();
        thread::Builder::new()
            .name("opds-worker".to_string())
            .spawn(move || {
                let agent = ureq::Agent::config_builder()
                    .timeout_global(Some(Duration::from_secs(15)))
                    .build()
                    .new_agent();

                while let Ok(cmd) = rx_cmd.recv() {
                    match cmd {
                        OpdsCommand::FetchFeed {
                            request_id,
                            url,
                            auth,
                        } => {
                            let res = fetch_feed_sync(&agent, &url, auth.as_ref());
                            let _ = tx_evt_clone.send(OpdsEvent::FeedLoaded {
                                request_id,
                                url,
                                result: res,
                            });
                        }
                        OpdsCommand::ResolveSearchAndFetch {
                            request_id,
                            search_template,
                            descriptor_url,
                            query,
                            auth,
                        } => {
                            let res = resolve_and_fetch_search(
                                &agent,
                                search_template.as_deref(),
                                descriptor_url.as_deref(),
                                &query,
                                auth.as_ref(),
                            );
                            let final_url =
                                res.as_ref().map(|(u, _)| u.clone()).unwrap_or_default();
                            let feed_res = res.map(|(_, f)| f);
                            let _ = tx_evt_clone.send(OpdsEvent::FeedLoaded {
                                request_id,
                                url: final_url,
                                result: feed_res,
                            });
                        }
                        OpdsCommand::DownloadBook {
                            download_id,
                            url,
                            title,
                            author,
                            ext,
                            auth,
                            downloads_dir,
                        } => {
                            let tx_dl = tx_evt_clone.clone();
                            let dl_id = download_id.clone();
                            let agent_dl = agent.clone();
                            thread::Builder::new()
                                .name(format!("opds-dl-{download_id}"))
                                .spawn(move || {
                                    let res = download_book_sync(
                                        &agent_dl,
                                        &url,
                                        &title,
                                        &author,
                                        &ext,
                                        auth.as_ref(),
                                        &downloads_dir,
                                        |downloaded, total| {
                                            let _ = tx_dl.send(OpdsEvent::DownloadProgress {
                                                download_id: dl_id.clone(),
                                                downloaded,
                                                total,
                                            });
                                        },
                                    );
                                    let _ = tx_dl.send(OpdsEvent::DownloadFinished {
                                        download_id: dl_id,
                                        result: res,
                                    });
                                })
                                .ok();
                        }
                    }
                }
            })
            .expect("Failed to spawn opds worker thread");

        Self {
            tx_cmd,
            rx_event,
            next_id,
        }
    }

    pub fn fetch_feed(&self, url: &str, auth: Option<(String, String)>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx_cmd.send(OpdsCommand::FetchFeed {
            request_id: id,
            url: url.to_string(),
            auth,
        });
        id
    }

    pub fn search(
        &self,
        search_template: Option<String>,
        descriptor_url: Option<String>,
        query: &str,
        auth: Option<(String, String)>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let _ = self.tx_cmd.send(OpdsCommand::ResolveSearchAndFetch {
            request_id: id,
            search_template,
            descriptor_url,
            query: query.to_string(),
            auth,
        });
        id
    }

    pub fn download(
        &self,
        download_id: &str,
        url: &str,
        title: &str,
        author: &str,
        ext: &str,
        auth: Option<(String, String)>,
        downloads_dir: PathBuf,
    ) {
        let _ = self.tx_cmd.send(OpdsCommand::DownloadBook {
            download_id: download_id.to_string(),
            url: url.to_string(),
            title: title.to_string(),
            author: author.to_string(),
            ext: ext.to_string(),
            auth,
            downloads_dir,
        });
    }

    pub fn poll_events(&self) -> Vec<OpdsEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.rx_event.try_recv() {
            events.push(evt);
        }
        events
    }
}

fn fetch_feed_sync(
    agent: &ureq::Agent,
    url: &str,
    auth: Option<&(String, String)>,
) -> Result<OpdsFeed, String> {
    let mut req = agent
        .get(url)
        .header("User-Agent", "QuietReader/0.7.1 (OPDS Client)")
        .header(
            "Accept",
            "application/atom+xml,application/xml,text/xml,*/*",
        );
    if let Some((user, pass)) = auth {
        req = req.header(
            "Authorization",
            format!(
                "Basic {}",
                base64_encode(format!("{user}:{pass}").as_bytes())
            ),
        );
    }
    let mut resp = req
        .call()
        .map_err(|e| format!("Ошибка сетевого запроса: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP статус: {}", resp.status()));
    }
    let mut body_str = String::new();
    resp.body_mut()
        .as_reader()
        .read_to_string(&mut body_str)
        .map_err(|e| format!("Ошибка чтения ответа: {e}"))?;
    parse_opds_feed(&body_str, url)
}

fn resolve_and_fetch_search(
    agent: &ureq::Agent,
    search_template: Option<&str>,
    descriptor_url: Option<&str>,
    query: &str,
    auth: Option<&(String, String)>,
) -> Result<(String, OpdsFeed), String> {
    let template = if let Some(tmpl) = search_template {
        tmpl.to_string()
    } else if let Some(desc_url) = descriptor_url {
        let mut req = agent
            .get(desc_url)
            .header("User-Agent", "QuietReader/0.7.1 (OPDS Client)");
        if let Some((user, pass)) = auth {
            req = req.header(
                "Authorization",
                format!(
                    "Basic {}",
                    base64_encode(format!("{user}:{pass}").as_bytes())
                ),
            );
        }
        let mut resp = req
            .call()
            .map_err(|e| format!("Ошибка запроса OpenSearch: {e}"))?;
        let mut desc_str = String::new();
        resp.body_mut()
            .as_reader()
            .read_to_string(&mut desc_str)
            .map_err(|e| format!("Ошибка чтения OpenSearch: {e}"))?;
        parse_opensearch_template(&desc_str, desc_url)
            .ok_or_else(|| "Не удалось разобрать шаблон поиска OpenSearch".to_string())?
    } else {
        return Err("Каталог не предоставляет поисковый интерфейс".to_string());
    };

    let encoded_query = percent_encode_query(query);
    let search_url = if template.contains("{searchTerms}") {
        template.replace("{searchTerms}", &encoded_query)
    } else if template.contains('?') {
        format!("{template}&q={encoded_query}")
    } else {
        format!("{template}?q={encoded_query}")
    };

    let feed = fetch_feed_sync(agent, &search_url, auth)?;
    Ok((search_url, feed))
}

fn download_book_sync<F>(
    agent: &ureq::Agent,
    url: &str,
    title: &str,
    author: &str,
    ext: &str,
    auth: Option<&(String, String)>,
    downloads_dir: &Path,
    mut on_progress: F,
) -> Result<PathBuf, String>
where
    F: FnMut(usize, Option<usize>),
{
    let _ = std::fs::create_dir_all(downloads_dir);
    let target_path = make_unique_filepath(downloads_dir, author, title, ext);
    let temp_path = target_path.with_extension(format!("tmp.{}", std::process::id()));

    let mut req = agent
        .get(url)
        .header("User-Agent", "QuietReader/0.7.1 (OPDS Downloader)");
    if let Some((user, pass)) = auth {
        req = req.header(
            "Authorization",
            format!(
                "Basic {}",
                base64_encode(format!("{user}:{pass}").as_bytes())
            ),
        );
    }
    let mut resp = req
        .call()
        .map_err(|e| format!("Ошибка соединения при скачивании: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Ошибка сервера при скачивании: HTTP {}",
            resp.status()
        ));
    }

    let content_len = resp
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Не удалось создать файл для сохранения: {e}"))?;
    let mut reader = resp.body_mut().as_reader();
    let mut buffer = [0u8; 32768];
    let mut downloaded = 0;
    let mut last_progress_report = 0;

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("Ошибка передачи данных: {e}"))?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Ошибка записи файла: {e}"))?;
        downloaded += bytes_read;
        if downloaded - last_progress_report >= 65536 {
            last_progress_report = downloaded;
            on_progress(downloaded, content_len);
        }
    }

    file.flush()
        .map_err(|e| format!("Ошибка сброса буфера записи: {e}"))?;
    drop(file);
    on_progress(downloaded, content_len);
    std::fs::rename(&temp_path, &target_path)
        .map_err(|e| format!("Не удалось переместить загруженный файл: {e}"))?;
    Ok(target_path)
}

#[cfg(all(test, feature = "enable_tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative_url() {
        assert_eq!(
            resolve_relative_url("https://example.com/opds/catalog.atom", "/search?q=1"),
            "https://example.com/search?q=1"
        );
        assert_eq!(
            resolve_relative_url("https://example.com/opds/catalog.atom", "next.atom"),
            "https://example.com/opds/next.atom"
        );
        assert_eq!(
            resolve_relative_url("https://example.com/catalog", "sub/books.atom"),
            "https://example.com/sub/books.atom"
        );
        assert_eq!(
            resolve_relative_url("http://example.com/a/b/", "c.atom"),
            "http://example.com/a/b/c.atom"
        );
        assert_eq!(
            resolve_relative_url(
                "https://example.com/page",
                "https://cdn.example.com/book.epub"
            ),
            "https://cdn.example.com/book.epub"
        );
    }

    #[test]
    fn test_percent_encode_query() {
        assert_eq!(percent_encode_query("hello world"), "hello+world");
        assert_eq!(
            percent_encode_query("tolstoy&dostoevsky"),
            "tolstoy%26dostoevsky"
        );
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"admin:secret123"), "YWRtaW46c2VjcmV0MTIz");
    }

    #[test]
    fn test_detect_format_and_ext() {
        assert_eq!(
            detect_format_and_ext("application/epub+zip", "book"),
            Some(("EPUB", "epub"))
        );
        assert_eq!(
            detect_format_and_ext("application/octet-stream", "https://site/book.mobi"),
            Some(("MOBI", "mobi"))
        );
        assert_eq!(
            detect_format_and_ext("application/x-fictionbook+xml", ""),
            Some(("FB2", "fb2"))
        );
        assert_eq!(
            detect_format_and_ext("application/pdf", ""),
            Some(("PDF", "pdf"))
        );
    }

    #[test]
    fn test_parse_simple_feed() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>urn:catalog:root</id>
  <title>Test Catalog</title>
  <link rel="self" href="/opds" type="application/atom+xml"/>
  <link rel="next" href="/opds?page=2" type="application/atom+xml"/>
  <link rel="search" type="application/atom+xml" href="/search?q={searchTerms}"/>
  <entry>
    <id>urn:book:1</id>
    <title>War and Peace</title>
    <author><name>Leo Tolstoy</name></author>
    <summary>A legendary novel &amp; historical epic.</summary>
    <link rel="http://opds-spec.org/acquisition" href="/download/war_and_peace.epub" type="application/epub+zip"/>
    <link rel="http://opds-spec.org/image/thumbnail" href="/covers/war.jpg" type="image/jpeg"/>
  </entry>
  <entry>
    <id>urn:nav:fiction</id>
    <title>Fiction Books</title>
    <link rel="subsection" href="/opds/fiction" type="application/atom+xml;profile=opds-catalog"/>
  </entry>
</feed>"#;

        let feed = parse_opds_feed(xml, "https://example.com/opds").expect("Parse failed");
        assert_eq!(feed.title, "Test Catalog");
        assert_eq!(
            feed.next_url,
            Some("https://example.com/opds?page=2".to_string())
        );
        assert_eq!(
            feed.search_template,
            Some("https://example.com/search?q={searchTerms}".to_string())
        );
        assert_eq!(feed.entries.len(), 2);

        let book = &feed.entries[0];
        assert_eq!(book.title, "War and Peace");
        assert_eq!(book.authors, vec!["Leo Tolstoy".to_string()]);
        assert_eq!(
            book.summary,
            Some("A legendary novel & historical epic.".to_string())
        );
        assert_eq!(book.acquisition_links.len(), 1);
        assert_eq!(book.acquisition_links[0].format_label, "EPUB");
        assert_eq!(
            book.thumbnail_url,
            Some("https://example.com/covers/war.jpg".to_string())
        );

        let cat = &feed.entries[1];
        assert_eq!(cat.title, "Fiction Books");
        assert_eq!(
            cat.navigation_link,
            Some("https://example.com/opds/fiction".to_string())
        );
    }

    #[test]
    fn test_parse_opensearch_description() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/">
  <ShortName>Catalog Search</ShortName>
  <Description>Search eBooks</Description>
  <Url type="application/atom+xml" template="/search?query={searchTerms}"/>
</OpenSearchDescription>"#;

        let tmpl = parse_opensearch_template(xml, "https://openbooks.com/catalog/")
            .expect("OpenSearch parse failed");
        assert_eq!(tmpl, "https://openbooks.com/search?query={searchTerms}");
    }
}
