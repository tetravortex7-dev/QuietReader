use crate::app::onboarding::{
    WELCOME_BOOK_AUTHOR, WELCOME_BOOK_FILENAME, WELCOME_BOOK_TITLE, ensure_welcome_book,
};
use crate::core::library::LibraryStore;
use crate::document::{Document, Fb2Document};
use crate::test::util::unique_test_dir;
use std::fs;
use std::path::Path;

#[test]
fn test_ensure_welcome_book_creates_and_registers_book() {
    let dir = unique_test_dir("welcome_book_create");
    let library_path = dir.join("library.json");
    let mut library = LibraryStore::default();

    assert!(ensure_welcome_book(&library_path, &mut library));

    let book_path = dir.join(WELCOME_BOOK_FILENAME);
    let content = fs::read_to_string(&book_path).expect("welcome book content");
    assert!(content.contains("<FictionBook"));
    assert!(content.contains("<coverpage>"));
    assert!(content.contains("type=\"note\""));
    assert!(content.contains("<body name=\"notes\">"));
    assert!(content.contains("чтения вслух"));
    assert!(content.contains("правой кнопкой мыши"));
    assert!(content.contains("линейку фокуса"));
    assert!(content.contains("атмосферу"));
    assert!(content.contains("Ctrl+D"));
    assert_eq!(library.entries.len(), 1);
    assert_eq!(library.entries[0].path, book_path);
    assert_eq!(library.entries[0].title, WELCOME_BOOK_TITLE);
    assert_eq!(library.entries[0].author, WELCOME_BOOK_AUTHOR);
    assert!(!ensure_welcome_book(&library_path, &mut library));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_generated_welcome_book_parses_with_fb2_parser() {
    let dir = unique_test_dir("welcome_book_parse");
    let library_path = dir.join("library.json");
    let mut library = LibraryStore::default();

    assert!(ensure_welcome_book(&library_path, &mut library));

    let book_path = dir.join(WELCOME_BOOK_FILENAME);
    let doc = Fb2Document::from_file(&book_path).expect("generated welcome book should parse");
    assert_eq!(doc.metadata().title, WELCOME_BOOK_TITLE);
    assert_eq!(doc.metadata().author, "QuietReader Team");
    assert!(doc.chapters().len() >= 3);

    let text = doc
        .chapters()
        .iter()
        .flat_map(|chapter| chapter.paragraphs.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("линейку фокуса"));
    assert!(text.contains("Ctrl+D"));
    assert!(text.contains("{{note:note-tts}}"));
    assert!(
        doc.footnote("note-tts")
            .is_some_and(|note| note.contains("Это пример сноски"))
    );
    assert_eq!(library.entries.len(), 1);
    assert_eq!(library.entries[0].path, book_path);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_ensure_welcome_book_does_not_overwrite_existing_file() {
    let dir = unique_test_dir("welcome_book_existing");
    let library_path = dir.join("library.json");
    let book_path = dir.join(WELCOME_BOOK_FILENAME);
    fs::write(&book_path, "custom welcome content").expect("existing welcome book");
    let mut library = LibraryStore::default();

    assert!(ensure_welcome_book(&library_path, &mut library));

    assert_eq!(
        fs::read_to_string(&book_path).expect("welcome book content"),
        "custom welcome content"
    );
    assert_eq!(library.entries.len(), 1);
    assert_eq!(library.entries[0].path, book_path);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_ensure_welcome_book_skips_non_empty_library() {
    let dir = unique_test_dir("welcome_book_non_empty");
    let library_path = dir.join("library.json");
    let book_path = dir.join(WELCOME_BOOK_FILENAME);
    let mut library = LibraryStore::default();
    library.ensure_book(Path::new("existing.epub"), "Existing", "Author", 1);

    assert!(!ensure_welcome_book(&library_path, &mut library));

    assert!(!book_path.exists());
    assert_eq!(library.entries.len(), 1);
    assert_eq!(library.entries[0].path, Path::new("existing.epub"));

    let _ = fs::remove_dir_all(&dir);
}
