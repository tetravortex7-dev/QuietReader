use crate::core::library::LibraryStore;
use std::fs;
use std::path::Path;

pub const WELCOME_BOOK_FILENAME: &str = "welcome_guide.fb2";
pub const WELCOME_BOOK_TITLE: &str = "Добро пожаловать в QuietReader";
pub const WELCOME_BOOK_AUTHOR: &str = "QuietReader";
pub const WELCOME_BOOK_CONTENT: &str = include_str!("../../assets/welcome_guide.fb2");

pub fn ensure_welcome_book(library_path: &Path, library: &mut LibraryStore) -> bool {
    if !library.entries.is_empty() {
        return false;
    }
    let Some(dir) = library_path.parent() else {
        return false;
    };
    let path = dir.join(WELCOME_BOOK_FILENAME);
    if !path.is_file() {
        if fs::create_dir_all(dir).is_err() {
            return false;
        }
        if fs::write(&path, WELCOME_BOOK_CONTENT).is_err() {
            return false;
        }
    }
    library.ensure_book(
        &path,
        WELCOME_BOOK_TITLE,
        WELCOME_BOOK_AUTHOR,
        crate::core::license::unix_now(),
    )
}
