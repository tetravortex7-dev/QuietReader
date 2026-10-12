use super::comic_view::ComicViewState;
use crate::document::{Document, DocumentError};
use crate::settings::AppSettings;
use std::path::PathBuf;
use std::sync::Arc;

/// Форматирует типизированную ошибку документа в сообщение для пользователя в UI
pub fn format_document_error(err: &DocumentError) -> String {
    match err {
        DocumentError::Io(msg) => format!("Ошибка ввода-вывода: {msg}"),
        DocumentError::Unsupported(msg) => format!("Неподдерживаемый формат: {msg}"),
        DocumentError::Corrupted(msg) => format!("Повреждённый документ: {msg}"),
        DocumentError::LimitExceeded(msg) => format!("Превышен лимит безопасности: {msg}"),
    }
}

/// Сервис загрузки и связывания документов с состоянием приложения
pub struct DocumentLoader;

pub struct LoadedBookState {
    pub document: Arc<dyn Document>,
    pub file_path: PathBuf,
    pub chapter: usize,
    pub scroll_y: f32,
    pub comic_page: usize,
    pub status_message: String,
}

impl DocumentLoader {
    pub fn load_file(
        path: PathBuf,
        settings: &mut AppSettings,
        comic_view: &mut ComicViewState,
    ) -> Result<LoadedBookState, String> {
        let doc: Arc<dyn Document> = Arc::from(
            crate::document::open_document(&path).map_err(|e| format_document_error(&e))?,
        );
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

        comic_view.clear_cache();
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
        let _ = settings.save();

        let is_pdf = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        let status = if is_comic {
            if is_pdf {
                if comic_page > 0 {
                    format!(
                        "Открыт PDF: {title} (стр. {}/{})",
                        comic_page + 1,
                        comic_page_count
                    )
                } else {
                    format!("Открыт PDF: {title} (всего {} стр.)", comic_page_count)
                }
            } else if comic_page > 0 {
                format!(
                    "Открыт комикс/манга: {title} (стр. {}/{})",
                    comic_page + 1,
                    comic_page_count
                )
            } else {
                format!(
                    "Открыт комикс/манга: {title} (всего {} стр.)",
                    comic_page_count
                )
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

        Ok(LoadedBookState {
            document: doc,
            file_path: path,
            chapter: effective_ch,
            scroll_y,
            comic_page,
            status_message: status,
        })
    }
}
