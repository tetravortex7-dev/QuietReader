use crate::document::{Document, DocumentError};
use crate::settings::AppSettings;
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

pub fn format_document_error(err: &DocumentError) -> String {
    crate::i18n::format::format_document_error(err, crate::i18n::UiLanguage::Auto)
}

pub fn format_document_error_with_lang(
    err: &DocumentError,
    lang: crate::i18n::UiLanguage,
) -> String {
    crate::i18n::format::format_document_error(err, lang)
}

pub const ERR_PAGED_UNSUPPORTED: &str = "__paged_unsupported__";
pub const ERR_COMIC_DECODE_CRASH: &str = "__comic_decode_crash__";
pub const ERR_PAGE_DECODE_CRASH: &str = "__page_decode_crash__";
pub const ERR_DECODE_STREAM_CRASH: &str = "__decode_stream_crash__";

pub fn format_page_error(err: &DocumentError, lang: crate::i18n::UiLanguage) -> String {
    let t = lang.t();
    match err {
        DocumentError::Unsupported(message) if message == ERR_PAGED_UNSUPPORTED => {
            t.extra.document_unsupported_paged.to_string()
        }
        DocumentError::Corrupted(message) if message == ERR_COMIC_DECODE_CRASH => {
            t.extra.comic_decode_crash.to_string()
        }
        DocumentError::Corrupted(message) if message == ERR_PAGE_DECODE_CRASH => {
            t.extra.page_decode_crash.to_string()
        }
        DocumentError::Corrupted(message) if message == ERR_DECODE_STREAM_CRASH => {
            t.extra.decode_stream_crashed.to_string()
        }
        _ => format_document_error_with_lang(err, lang),
    }
}

pub struct DocumentLoadRequest {
    pub request_id: u64,
    pub path: PathBuf,
    pub lang: crate::i18n::UiLanguage,
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
                let lang = req.lang;
                let load_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::document::open_document(&path)
                        .map(|doc| {
                            super::localized_document::LocalizedDocument::wrap(Arc::from(doc), lang)
                        })
                        .map_err(|e| crate::i18n::format::format_document_error(&e, lang))
                }));

                let result = match load_res {
                    Ok(res) => res,
                    Err(_) => Err(lang.t().err_unknown.to_string()),
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

    pub fn request_load(&mut self, path: PathBuf, lang: crate::i18n::UiLanguage) -> u64 {
        self.request_counter = self.request_counter.wrapping_add(1);
        let request_id = self.request_counter;
        let _ = self.req_tx.send(DocumentLoadRequest {
            request_id,
            path,
            lang,
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
        let t = settings.ui_language.t();
        let status = if is_comic {
            if comic_page > 0 {
                crate::i18n::fill(
                    t.extra.status_opened_comic,
                    &[
                        ("{fmt}", fmt),
                        ("{title}", &title),
                        ("{page}", &(comic_page + 1).to_string()),
                        ("{total}", &comic_page_count.to_string()),
                    ],
                )
            } else {
                crate::i18n::fill(
                    t.extra.status_opened_comic_total,
                    &[
                        ("{fmt}", fmt),
                        ("{title}", &title),
                        ("{total}", &comic_page_count.to_string()),
                    ],
                )
            }
        } else if effective_ch > 0 || scroll_y > 10.0 {
            crate::i18n::fill(
                t.extra.status_opened_book_resume,
                &[
                    ("{title}", &title),
                    ("{section}", &(effective_ch + 1).to_string()),
                ],
            )
        } else if !author.is_empty() {
            crate::i18n::fill(
                t.extra.status_opened_book_author,
                &[("{author}", &author), ("{title}", &title)],
            )
        } else {
            crate::i18n::fill(t.extra.status_opened_book, &[("{title}", &title)])
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
        let lang = settings.ui_language;
        let doc = super::localized_document::LocalizedDocument::wrap(
            Arc::from(
                crate::document::open_document(&path)
                    .map_err(|e| crate::i18n::format::format_document_error(&e, lang))?,
            ),
            lang,
        );
        Ok(Self::finalize_loaded_document(doc, path, settings))
    }
}
