pub mod comic;
pub mod epub;
pub mod fb2;

pub use comic::{ComicDocument, ComicDocumentTrait};
pub use epub::EpubDocument;
pub use fb2::Fb2Document;

use std::path::Path;

/// Открывает документ любого поддерживаемого формата по пути к файлу или папке
pub fn open_document<P: AsRef<Path>>(path: P) -> Result<Box<dyn Document>, String> {
    let path_ref = path.as_ref();
    if path_ref.is_dir() {
        let doc = ComicDocument::from_path(path_ref)?;
        return Ok(Box::new(doc));
    }

    let ext = path_ref
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "epub" => {
            let doc = EpubDocument::from_file(path_ref)?;
            Ok(Box::new(doc))
        }
        "fb2" | "xml" => {
            let doc = Fb2Document::from_file(path_ref)?;
            Ok(Box::new(doc))
        }
        "cbz" | "cbr" | "cb7" => {
            let doc = ComicDocument::from_path(path_ref)?;
            Ok(Box::new(doc))
        }
        "zip" | "rar" | "7z" => {
            // Сначала пробуем как комикс, если не выйдет — пробуем как EPUB
            if let Ok(doc) = ComicDocument::from_path(path_ref) {
                return Ok(Box::new(doc));
            }
            if let Ok(doc) = EpubDocument::from_file(path_ref) {
                return Ok(Box::new(doc));
            }
            Err(format!(
                "Не удалось открыть архив как комикс или EPUB: {}",
                path_ref.display()
            ))
        }
        _ => {
            // Если расширение неизвестно, пробуем по очереди: Comic, EPUB, FB2
            if let Ok(doc) = ComicDocument::from_path(path_ref) {
                return Ok(Box::new(doc));
            }
            if let Ok(doc) = EpubDocument::from_file(path_ref) {
                return Ok(Box::new(doc));
            }
            if let Ok(doc) = Fb2Document::from_file(path_ref) {
                return Ok(Box::new(doc));
            }
            Err(format!(
                "Неподдерживаемый формат файла: {}",
                path_ref.display()
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct BookMetadata {
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct BookChapter {
    pub id: Option<String>,
    pub title: String,
    pub paragraphs: Vec<String>,
}

impl BookChapter {
    /// Проверяет, пуст ли раздел (нет заголовка и нет непустых абзацев текста)
    pub fn is_empty(&self) -> bool {
        self.title.trim().is_empty()
            && self.paragraphs.iter().all(|p| p.trim().is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocItem {
    pub id: Option<String>,
    pub title: String,
    pub chapter_index: usize,
    pub children: Vec<TocItem>,
}

pub trait Document: Send + Sync {
    fn metadata(&self) -> &BookMetadata;
    fn chapters(&self) -> &[BookChapter];
    fn toc(&self) -> &[TocItem] {
        &[]
    }
    /// Предоставляет специфичный интерфейс комикса/манги, если формат поддерживается
    fn as_comic(&self) -> Option<&dyn ComicDocumentTrait> {
        None
    }
}
