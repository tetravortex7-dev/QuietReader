use super::auth::OpdsAuth;
use super::parser::{
    parse_opds_feed, parse_opensearch_template, percent_encode_query, sanitize_filename,
};
use super::types::OpdsFeed;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

const MAX_FEED_BYTES: u64 = 10 * 1024 * 1024;
const MAX_SEARCH_DESCRIPTOR_BYTES: u64 = 1024 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpdsError {
    Message(String),
    ResponseTooLarge { limit: u64 },
    DownloadTooLarge { limit: u64 },
    Network(String),
    ReadResponse(String),
    HttpStatus(u16),
    OpenSearch(String),
    InvalidOpenSearch,
    NoSearchInterface,
    DownloadConnect(String),
    DownloadServer(u16),
    CreateFile(String),
    Transfer(String),
    WriteFile(String),
    Flush(String),
    MoveFile(String),
}

impl From<String> for OpdsError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for OpdsError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub enum OpdsCommand {
    FetchFeed {
        request_id: u64,
        url: String,
        auth: Option<OpdsAuth>,
    },
    ResolveSearchAndFetch {
        request_id: u64,
        search_template: Option<String>,
        descriptor_url: Option<String>,
        query: String,
        auth: Option<OpdsAuth>,
    },
    DownloadBook {
        download_id: String,
        url: String,
        title: String,
        author: String,
        ext: String,
        auth: Option<OpdsAuth>,
        downloads_dir: PathBuf,
    },
}

#[derive(Debug)]
pub enum OpdsEvent {
    FeedLoaded {
        request_id: u64,
        url: String,
        result: Result<OpdsFeed, OpdsError>,
    },
    DownloadProgress {
        download_id: String,
        downloaded: usize,
        total: Option<usize>,
    },
    DownloadFinished {
        download_id: String,
        result: Result<PathBuf, OpdsError>,
    },
}

pub struct OpdsClient {
    tx_cmd: Sender<OpdsCommand>,
    tx_event: Sender<OpdsEvent>,
    rx_event: Receiver<OpdsEvent>,
    next_id: Arc<AtomicU64>,
    worker_started: bool,
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
        let worker_started = thread::Builder::new()
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
            .is_ok();

        Self {
            tx_cmd,
            tx_event,
            rx_event,
            next_id,
            worker_started,
        }
    }

    pub fn fetch_feed(&self, url: &str, auth: Option<OpdsAuth>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        if !self.worker_started {
            let _ = self.tx_event.send(OpdsEvent::FeedLoaded {
                request_id: id,
                url: url.to_string(),
                result: Err(OpdsError::Network(
                    "OPDS worker thread is unavailable".to_string(),
                )),
            });
            return id;
        }
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
        auth: Option<OpdsAuth>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        if !self.worker_started {
            let _ = self.tx_event.send(OpdsEvent::FeedLoaded {
                request_id: id,
                url: String::new(),
                result: Err(OpdsError::Network(
                    "OPDS worker thread is unavailable".to_string(),
                )),
            });
            return id;
        }
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
        auth: Option<OpdsAuth>,
        downloads_dir: PathBuf,
    ) {
        if !self.worker_started {
            let _ = self.tx_event.send(OpdsEvent::DownloadFinished {
                download_id: download_id.to_string(),
                result: Err(OpdsError::DownloadConnect(
                    "OPDS worker thread is unavailable".to_string(),
                )),
            });
            return;
        }
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

    #[cfg(all(test, feature = "enable_tests"))]
    pub(crate) fn new_without_worker_for_test() -> Self {
        let (tx_cmd, _rx_cmd) = channel::<OpdsCommand>();
        let (tx_event, rx_event) = channel::<OpdsEvent>();
        Self {
            tx_cmd,
            tx_event,
            rx_event,
            next_id: Arc::new(AtomicU64::new(1)),
            worker_started: false,
        }
    }

    pub fn poll_events(&self) -> Vec<OpdsEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.rx_event.try_recv() {
            events.push(evt);
        }
        events
    }
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

fn check_content_length(headers: &ureq::http::HeaderMap, limit: u64) -> Result<(), OpdsError> {
    let length = headers
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if length.is_some_and(|value| value > limit) {
        return Err(OpdsError::ResponseTooLarge { limit });
    }
    Ok(())
}

pub(crate) fn read_limited_text(reader: &mut dyn Read, limit: u64) -> Result<String, OpdsError> {
    let mut body = String::new();
    reader
        .take(limit + 1)
        .read_to_string(&mut body)
        .map_err(|e| OpdsError::ReadResponse(e.to_string()))?;
    if body.len() as u64 > limit {
        return Err(OpdsError::ResponseTooLarge { limit });
    }
    Ok(body)
}

fn fetch_feed_sync(
    agent: &ureq::Agent,
    url: &str,
    auth: Option<&OpdsAuth>,
) -> Result<OpdsFeed, OpdsError> {
    let mut req = agent
        .get(url)
        .header("User-Agent", "QuietReader/0.7.1 (OPDS Client)")
        .header(
            "Accept",
            "application/atom+xml,application/xml,text/xml,*/*",
        );
    if let Some(header) = auth.and_then(|auth| auth.header_for(url)) {
        req = req.header("Authorization", header);
    }
    let mut resp = req.call().map_err(|e| OpdsError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(OpdsError::HttpStatus(resp.status().as_u16()));
    }
    check_content_length(resp.headers(), MAX_FEED_BYTES)?;
    let mut reader = resp.body_mut().as_reader();
    let body = read_limited_text(&mut reader, MAX_FEED_BYTES)?;
    parse_opds_feed(&body, url).map_err(Into::into)
}

fn resolve_and_fetch_search(
    agent: &ureq::Agent,
    search_template: Option<&str>,
    descriptor_url: Option<&str>,
    query: &str,
    auth: Option<&OpdsAuth>,
) -> Result<(String, OpdsFeed), OpdsError> {
    let template = if let Some(tmpl) = search_template {
        tmpl.to_string()
    } else if let Some(desc_url) = descriptor_url {
        let mut req = agent
            .get(desc_url)
            .header("User-Agent", "QuietReader/0.7.1 (OPDS Client)");
        if let Some(header) = auth.and_then(|auth| auth.header_for(desc_url)) {
            req = req.header("Authorization", header);
        }
        let mut resp = req
            .call()
            .map_err(|e| OpdsError::OpenSearch(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(OpdsError::HttpStatus(resp.status().as_u16()));
        }
        check_content_length(resp.headers(), MAX_SEARCH_DESCRIPTOR_BYTES)?;
        let mut reader = resp.body_mut().as_reader();
        let desc_str = read_limited_text(&mut reader, MAX_SEARCH_DESCRIPTOR_BYTES)?;
        parse_opensearch_template(&desc_str, desc_url).ok_or(OpdsError::InvalidOpenSearch)?
    } else {
        return Err(OpdsError::NoSearchInterface);
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

pub(crate) struct TempFileGuard {
    path: PathBuf,
    keep: bool,
}

impl TempFileGuard {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    pub(crate) fn disarm(mut self) {
        self.keep = true;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn download_book_sync<F>(
    agent: &ureq::Agent,
    url: &str,
    title: &str,
    author: &str,
    ext: &str,
    auth: Option<&OpdsAuth>,
    downloads_dir: &Path,
    mut on_progress: F,
) -> Result<PathBuf, OpdsError>
where
    F: FnMut(usize, Option<usize>),
{
    let _ = std::fs::create_dir_all(downloads_dir);
    let target_path = make_unique_filepath(downloads_dir, author, title, ext);
    let temp_path = target_path.with_extension(format!("tmp.{}", std::process::id()));

    let mut req = agent
        .get(url)
        .header("User-Agent", "QuietReader/0.7.1 (OPDS Downloader)");
    if let Some(header) = auth.and_then(|auth| auth.header_for(url)) {
        req = req.header("Authorization", header);
    }
    let mut resp = req
        .call()
        .map_err(|e| OpdsError::DownloadConnect(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(OpdsError::DownloadServer(resp.status().as_u16()));
    }

    let content_len = resp
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    if content_len.is_some_and(|len| len > MAX_DOWNLOAD_BYTES) {
        return Err(OpdsError::DownloadTooLarge {
            limit: MAX_DOWNLOAD_BYTES,
        });
    }
    let content_len = content_len.and_then(|len| usize::try_from(len).ok());

    let guard = TempFileGuard::new(temp_path.clone());
    let mut file =
        std::fs::File::create(&temp_path).map_err(|e| OpdsError::CreateFile(e.to_string()))?;
    let mut reader = resp.body_mut().as_reader();
    let mut buffer = [0u8; 32768];
    let mut downloaded: usize = 0;
    let mut last_progress_report: usize = 0;

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| OpdsError::Transfer(e.to_string()))?;
        if bytes_read == 0 {
            break;
        }
        let next_total = downloaded.saturating_add(bytes_read);
        if next_total as u64 > MAX_DOWNLOAD_BYTES {
            return Err(OpdsError::DownloadTooLarge {
                limit: MAX_DOWNLOAD_BYTES,
            });
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(|e| OpdsError::WriteFile(e.to_string()))?;
        downloaded = next_total;
        if downloaded - last_progress_report >= 65536 {
            last_progress_report = downloaded;
            on_progress(downloaded, content_len);
        }
    }

    file.flush().map_err(|e| OpdsError::Flush(e.to_string()))?;
    drop(file);
    on_progress(downloaded, content_len);
    std::fs::rename(&temp_path, &target_path).map_err(|e| OpdsError::MoveFile(e.to_string()))?;
    guard.disarm();
    Ok(target_path)
}
