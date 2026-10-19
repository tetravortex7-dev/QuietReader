use super::parser::{
    base64_encode, parse_opds_feed, parse_opensearch_template, percent_encode_query,
    sanitize_filename,
};
use super::types::OpdsFeed;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

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
