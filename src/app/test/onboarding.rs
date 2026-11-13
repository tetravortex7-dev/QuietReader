use crate::app::onboarding::{
    WELCOME_BOOK_AUTHOR, WELCOME_BOOK_FILENAME, WELCOME_BOOK_TITLE, ensure_cozy_showcase_library,
    ensure_welcome_book, resolve_books_dir_from,
};
use crate::core::library::{LibraryMode, LibraryStore};
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

#[test]
fn test_resolve_books_dir_returns_none_when_missing() {
    let dir = unique_test_dir("books_dir_missing");
    let exe_path = dir
        .join("bin")
        .join("target")
        .join("release")
        .join("app.exe");
    let cwd = dir.join("workspace");
    fs::create_dir_all(exe_path.parent().expect("exe parent")).expect("create exe parent");
    fs::create_dir_all(&cwd).expect("create cwd");

    let resolved = resolve_books_dir_from(Some(&exe_path), Some(&cwd));
    assert_eq!(resolved, None);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_resolve_books_dir_finds_portable_dir_and_skips_files() {
    let dir = unique_test_dir("books_dir_portable");
    let exe_dir = dir.join("app");
    fs::create_dir_all(&exe_dir).expect("create exe dir");
    let exe_path = exe_dir.join("QuietReader.exe");
    fs::write(exe_dir.join("books"), b"not a directory").expect("write file named books");

    let cwd = dir.join("cwd");
    let cwd_books = cwd.join("books");
    fs::create_dir_all(&cwd_books).expect("create cwd books dir");

    let resolved = resolve_books_dir_from(Some(&exe_path), Some(&cwd));
    assert_eq!(resolved.as_deref(), Some(cwd_books.as_path()));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_ensure_cozy_showcase_library_preserves_existing_data_and_reader_mode() {
    let mut with_entries = LibraryStore::default();
    with_entries.ensure_book(Path::new("user_book.epub"), "My Book", "Author", 1);
    assert!(!ensure_cozy_showcase_library(&mut with_entries));
    assert_eq!(with_entries.entries.len(), 1);
    assert_eq!(with_entries.entries[0].title, "My Book");

    let mut with_reading = LibraryStore::default();
    with_reading.daily_reading_seconds.insert(20000, 1800);
    assert!(!ensure_cozy_showcase_library(&mut with_reading));
    assert!(with_reading.entries.is_empty());
    assert_eq!(with_reading.daily_reading_seconds.get(&20000), Some(&1800));

    let mut reader_only = LibraryStore::default();
    reader_only.mode = LibraryMode::ReaderOnly;
    assert!(!ensure_cozy_showcase_library(&mut reader_only));
    assert_eq!(reader_only.mode, LibraryMode::ReaderOnly);
    assert!(reader_only.entries.is_empty());
}
