use crate::document::{Document, DocumentError};
use crate::settings::AppSettings;
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

pub fn format_document_error(err: &DocumentError) -> String {
    match err {
        DocumentError::Io(msg) => format!("Ошибка ввода-вывода: {msg}"),
        DocumentError::Unsupported(msg) => format!("Неподдерживаемый формат: {msg}"),
        DocumentError::Corrupted(msg) => format!("Повреждённый документ: {msg}"),
        DocumentError::LimitExceeded(msg) => format!("Превышен лимит безопасности: {msg}"),
    }
}

pub struct DocumentLoadRequest {
    pub request_id: u64,
    pub path: PathBuf,
    pub ctx: Option<egui::Context>,
}

pub struct DocumentLoadResponse {
    pub request_id: u64,
    pub path: PathBuf,
    pub result: Result<Arc<dyn Document>, String>,
}

pub struct LoadedBookState {
    pub document: Arc<dyn Document>,
    pub file_path: PathBuf,
    pub chapter: usize,
    pub scroll_y: f32,
    pub comic_page: usize,
    pub status_message: String,
}

pub struct DocumentLoader {
    req_tx: Sender<DocumentLoadRequest>,
    resp_rx: Receiver<DocumentLoadResponse>,
    request_counter: u64,
    ctx: Option<egui::Context>,
}

fn spawn_loader_worker(
    req_rx: Receiver<DocumentLoadRequest>,
    resp_tx: Sender<DocumentLoadResponse>,
) {
    std::thread::Builder::new()
        .name("document-loader".to_string())
        .spawn(move || {
            while let Ok(req) = req_rx.recv() {
                let path = req.path.clone();
                let load_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::document::open_document(&path)
                        .map(Arc::from)
                        .map_err(|e| format_document_error(&e))
                }));

                let result = match load_res {
                    Ok(res) => res,
                    Err(_) => Err("Аварийный сбой при открытии документа".to_string()),
                };

                let _ = resp_tx.send(DocumentLoadResponse {
                    request_id: req.request_id,
                    path: req.path,
                    result,
                });
                if let Some(ctx) = req.ctx {
                    ctx.request_repaint();
                }
            }
        })
        .ok();
}

impl Default for DocumentLoader {
    fn default() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<DocumentLoadRequest>();
        let (resp_tx, resp_rx) = mpsc::channel::<DocumentLoadResponse>();
        spawn_loader_worker(req_rx, resp_tx);

        Self {
            req_tx,
            resp_rx,
            request_counter: 0,
            ctx: None,
        }
    }
}

impl DocumentLoader {
    pub fn update_ctx(&mut self, ctx: &egui::Context) {
        if self.ctx.is_none() {
            self.ctx = Some(ctx.clone());
        }
    }

    pub fn request_load(&mut self, path: PathBuf) -> u64 {
        self.request_counter = self.request_counter.wrapping_add(1);
        let request_id = self.request_counter;
        let _ = self.req_tx.send(DocumentLoadRequest {
            request_id,
            path,
            ctx: self.ctx.clone(),
        });
        request_id
    }

    pub fn poll_responses(&mut self) -> Vec<DocumentLoadResponse> {
        let mut responses = Vec::new();
        loop {
            match self.resp_rx.try_recv() {
                Ok(resp) => responses.push(resp),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let (req_tx, req_rx) = mpsc::channel::<DocumentLoadRequest>();
                    let (resp_tx, resp_rx) = mpsc::channel::<DocumentLoadResponse>();
                    spawn_loader_worker(req_rx, resp_tx);
                    self.req_tx = req_tx;
                    self.resp_rx = resp_rx;
                    break;
                }
            }
        }
        responses
    }

    pub fn finalize_loaded_document(
        doc: Arc<dyn Document>,
        path: PathBuf,
        settings: &mut AppSettings,
    ) -> LoadedBookState {
        let title = doc.metadata().title.clone();
        let author = doc.metadata().author.clone();
        let total_chapters = doc.chapters().len();
        let is_comic = doc.page_content().is_some();
        let comic_page_count = doc.page_content().map(|c| c.page_count()).unwrap_or(0);

        let (ch, scroll_y) =
            if let Some((saved_ch, saved_scroll)) = settings.get_book_progress(&path) {
                (saved_ch, saved_scroll)
            } else {
                (0, 0.0)
            };

        let comic_page = if is_comic && comic_page_count > 0 {
            ch.min(comic_page_count.saturating_sub(1))
        } else {
            0
        };

        let effective_ch = if is_comic {
            0
        } else {
            ch.min(total_chapters.saturating_sub(1))
        };

        settings.record_book_progress(
            &path,
            &title,
            &author,
            if is_comic { comic_page } else { effective_ch },
            scroll_y,
        );
        let fmt = doc.format_name();
        let status = if is_comic {
            if comic_page > 0 {
                format!(
                    "Открыт {fmt}: {title} (стр. {}/{})",
                    comic_page + 1,
                    comic_page_count
                )
            } else {
                format!("Открыт {fmt}: {title} (всего {} стр.)", comic_page_count)
            }
        } else if effective_ch > 0 || scroll_y > 10.0 {
            format!(
                "Открыта книга: {title} (продолжение с раздела {})",
                effective_ch + 1
            )
        } else if !author.is_empty() {
            format!("Открыта книга: {author} — {title}")
        } else {
            format!("Открыта книга: {title}")
        };

        LoadedBookState {
            document: doc,
            file_path: path,
            chapter: effective_ch,
            scroll_y,
            comic_page,
            status_message: status,
        }
    }

    pub fn load_file_sync(
        path: PathBuf,
        settings: &mut AppSettings,
    ) -> Result<LoadedBookState, String> {
        let doc: Arc<dyn Document> = Arc::from(
            crate::document::open_document(&path).map_err(|e| format_document_error(&e))?,
        );
        Ok(Self::finalize_loaded_document(doc, path, settings))
    }
}
