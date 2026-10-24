use crate::core::library::{
    CURRENT_LIBRARY_SCHEMA_VERSION, LibraryMode, LibraryReadingStatus, LibraryStore, ShelfViewMode,
    UiDesignMode, estimate_minutes_left_in_chapter, read_calibre_metadata, scan_folder,
    unix_day_index,
};
use crate::test::util::unique_test_dir;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_library_mode_switching_and_path() {
    let mut store = LibraryStore::default();
    assert_eq!(store.mode, LibraryMode::Shelf);
    assert_eq!(store.design, UiDesignMode::Cozy);
    assert_eq!(store.shelf_view, ShelfViewMode::Standard);

    store.mode = LibraryMode::Folder;
    store.design = UiDesignMode::Studio;
    store.shelf_view = ShelfViewMode::Maxi;
    assert_eq!(store.mode, LibraryMode::Folder);
    assert_eq!(store.design, UiDesignMode::Studio);
    assert_eq!(store.shelf_view, ShelfViewMode::Maxi);
    assert_eq!(UiDesignMode::Studio.corner_radius(), 6);

    let settings_path = PathBuf::from("config").join("settings.json");
    assert_eq!(
        LibraryStore::path_beside(&settings_path),
        PathBuf::from("config").join("library.json")
    );
}

#[test]
fn test_library_store_save_load_modes_folder_and_design() {
    let dir = unique_test_dir("library_store_modes");
    let path = dir.join("library.json");
    let folder = dir.join("books");
    let mut store = LibraryStore::default();
    store.mode = LibraryMode::Folder;
    store.design = UiDesignMode::Studio;
    store.shelf_view = ShelfViewMode::Mini;
    store.library_folder = Some(folder.clone());
    store.recent_imported = true;
    store.save_to_path(&path).unwrap();

    let loaded = LibraryStore::load_from_path(&path);
    assert_eq!(loaded.mode, LibraryMode::Folder);
    assert_eq!(loaded.design, UiDesignMode::Studio);
    assert_eq!(loaded.shelf_view, ShelfViewMode::Mini);
    assert_eq!(loaded.library_folder, Some(folder));
    assert!(loaded.recent_imported);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_library_store_entry_status_progress_and_remove() {
    let path = Path::new("book.epub");
    let mut store = LibraryStore::default();

    assert!(store.ensure_book(path, "Book", "Author", 10));
    assert!(!store.ensure_book(path, "Book", "Author", 11));
    assert!(store.record_progress(path, "Book", "Author", 2, 10, 20));

    let entry = store.entry(path).unwrap();
    assert_eq!(entry.status, LibraryReadingStatus::Reading);
    assert_eq!(entry.current_unit, 2);
    assert_eq!(entry.total_units, 10);
    assert_eq!(entry.last_opened_unix, 20);

    assert!(!store.record_progress(path, "Book", "Author", 2, 10, 30));
    assert_eq!(store.entry(path).unwrap().last_opened_unix, 20);
    assert!(store.touch(path, 30));
    assert_eq!(store.entry(path).unwrap().last_opened_unix, 30);

    assert!(store.set_status(path, LibraryReadingStatus::Finished));
    assert_eq!(
        store.entry(path).unwrap().status,
        LibraryReadingStatus::Finished
    );
    assert!(!store.set_status(path, LibraryReadingStatus::Finished));

    assert!(store.remove(path));
    assert!(store.entry(path).is_none());
    assert!(!store.remove(path));
}

#[test]
fn test_library_store_backup_recovery() {
    let dir = unique_test_dir("library_store_recovery");
    let path = dir.join("library.json");
    let mut store = LibraryStore::default();
    store.ensure_book(Path::new("first.epub"), "First", "Author", 10);
    store.save_to_path(&path).unwrap();

    store.ensure_book(Path::new("second.epub"), "Second", "Author", 20);
    store.save_to_path(&path).unwrap();
    assert!(dir.join("library.json.bak").exists());

    fs::write(&path, "not valid json").unwrap();
    let loaded = LibraryStore::load_from_path(&path);
    assert!(loaded.entry(Path::new("first.epub")).is_some());
    assert!(loaded.entry(Path::new("second.epub")).is_none());

    let restored = fs::read_to_string(&path).unwrap();
    assert!(serde_json::from_str::<LibraryStore>(&restored).is_ok());
    assert!(dir.join("library.corrupted").exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_scan_folder_filters_extensions_and_depth() {
    let dir = unique_test_dir("library_scan");
    let nested = dir.join("nested");
    let deeper = nested.join("deeper");
    fs::create_dir_all(&deeper).unwrap();
    fs::write(dir.join("a.epub"), b"").unwrap();
    fs::write(dir.join("archive.rar"), b"").unwrap();
    fs::write(dir.join("bad.exe"), b"").unwrap();
    fs::write(nested.join("b.PDF"), b"").unwrap();
    fs::write(deeper.join("c.epub"), b"").unwrap();

    let scan = scan_folder(
        &dir,
        1,
        100,
        crate::document::registry::is_supported_document_path,
    )
    .unwrap();

    let paths: Vec<PathBuf> = scan.books.iter().map(|book| book.path.clone()).collect();
    assert!(paths.iter().any(|path| path.ends_with("a.epub")));
    assert!(paths.iter().any(|path| path.ends_with("archive.rar")));
    assert!(paths.iter().any(|path| path.ends_with("b.PDF")));
    assert!(!paths.iter().any(|path| path.ends_with("bad.exe")));
    assert!(!paths.iter().any(|path| path.ends_with("c.epub")));
    assert!(scan.truncated);

    let scan = scan_folder(
        &dir,
        2,
        100,
        crate::document::registry::is_supported_document_path,
    )
    .unwrap();
    assert!(
        scan.books
            .iter()
            .any(|book| book.path.ends_with("deeper/c.epub"))
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_scan_folder_reads_calibre_metadata_and_imports_shelf() {
    let directory = unique_test_dir("library_calibre");
    let book_directory = directory.join("Calibre Author").join("Calibre Book (1)");
    fs::create_dir_all(&book_directory).unwrap();
    let book = book_directory.join("Calibre Book.epub");
    fs::write(&book, b"").unwrap();
    fs::write(
        book_directory.join("metadata.opf"),
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Calibre &amp; Test</dc:title>
    <dc:creator>First Author</dc:creator>
    <dc:creator>Second Author</dc:creator>
  </metadata>
</package>"#,
    )
    .unwrap();
    let plain = directory.join("Plain Book.epub");
    fs::write(&plain, b"").unwrap();

    let metadata = read_calibre_metadata(&book).unwrap();
    assert_eq!(metadata.title, "Calibre & Test");
    assert_eq!(metadata.author, "First Author, Second Author");

    let scan = scan_folder(
        &directory,
        6,
        100,
        crate::document::registry::is_supported_document_path,
    )
    .unwrap();
    assert_eq!(scan.books.len(), 2);
    let calibre = scan.books.iter().find(|entry| entry.path == book).unwrap();
    assert_eq!(calibre.title, "Calibre & Test");
    assert_eq!(calibre.author, "First Author, Second Author");
    let plain = scan.books.iter().find(|entry| entry.path == plain).unwrap();
    assert_eq!(plain.title, "Plain Book");
    assert!(plain.author.is_empty());

    let mut store = LibraryStore::default();
    assert!(store.import_folder_books(&scan.books, 100));
    assert_eq!(store.entries.len(), 2);
    let entry = store.entry(&book).unwrap();
    assert_eq!(entry.title, "Calibre & Test");
    assert_eq!(entry.author, "First Author, Second Author");
    assert!(!store.import_folder_books(&scan.books, 200));

    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn test_scan_folder_ignores_symlinks() {
    let dir = unique_test_dir("library_scan_symlink");
    let target_dir = unique_test_dir("library_scan_symlink_target");
    let target = target_dir.join("linked.epub");
    fs::write(&target, b"").unwrap();
    fs::write(dir.join("real.epub"), b"").unwrap();

    let link = dir.join("linked.epub");
    if create_file_symlink(&target, &link) {
        let scan = scan_folder(
            &dir,
            2,
            100,
            crate::document::registry::is_supported_document_path,
        )
        .unwrap();
        assert_eq!(scan.books.len(), 1);
        assert!(scan.books[0].path.ends_with("real.epub"));
    }

    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&target_dir);
}

#[test]
fn test_scan_folder_entry_limit() {
    let dir = unique_test_dir("library_scan_limit");
    for index in 0..5 {
        fs::write(dir.join(format!("book{index}.epub")), b"").unwrap();
    }

    let scan = scan_folder(&dir, 2, 3, |_| true).unwrap();
    assert_eq!(scan.books.len(), 3);
    assert!(scan.truncated);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_library_store_backward_compatible_reading_stats() {
    let dir = unique_test_dir("library_stats_compat");
    let path = dir.join("library.json");
    let old_json = r#"{"schema_version":1,"mode":"shelf","design":"cozy","library_folder":null,"recent_imported":true,"entries":[]}"#;
    fs::write(&path, old_json).unwrap();

    let loaded = LibraryStore::load_from_path(&path);
    assert_eq!(loaded.schema_version, CURRENT_LIBRARY_SCHEMA_VERSION);
    assert_eq!(loaded.shelf_view, ShelfViewMode::Standard);
    assert!(loaded.daily_reading_seconds.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_library_store_reading_stats_roundtrip() {
    let dir = unique_test_dir("library_stats_roundtrip");
    let path = dir.join("library.json");
    let mut store = LibraryStore::default();
    assert!(store.record_reading_seconds(20_000, 90));
    assert!(store.record_reading_seconds(20_001, 30));
    store.save_to_path(&path).unwrap();

    let loaded = LibraryStore::load_from_path(&path);
    assert_eq!(loaded.total_reading_seconds(), 120);
    assert_eq!(loaded.total_reading_minutes(), 2);
    assert_eq!(loaded.daily_reading_seconds.get(&20_000), Some(&90));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_library_store_streak_activity_and_completed_books() {
    let mut store = LibraryStore::default();
    assert!(!store.record_reading_seconds(10, 0));
    assert!(store.record_reading_seconds(10, 120));
    assert!(store.record_reading_seconds(10, 60));
    assert!(store.record_reading_seconds(11, 30));
    assert!(store.record_reading_seconds(12, 40));

    assert_eq!(store.total_reading_seconds(), 250);
    assert_eq!(store.total_reading_minutes(), 4);
    assert_eq!(store.current_streak_days(12), 3);
    assert_eq!(store.current_streak_days(13), 3);
    assert_eq!(store.current_streak_days(14), 0);

    assert!(store.record_reading_seconds(14, 1));
    assert_eq!(store.current_streak_days(14), 1);
    assert_eq!(
        store.last_7_days_activity(14),
        vec![
            (8, 0),
            (9, 0),
            (10, 180),
            (11, 30),
            (12, 40),
            (13, 0),
            (14, 1)
        ]
    );

    store.ensure_book(Path::new("first.epub"), "First", "Author", 10);
    store.ensure_book(Path::new("second.epub"), "Second", "Author", 11);
    store.ensure_book(Path::new("third.epub"), "Third", "Author", 12);
    store.set_status(Path::new("first.epub"), LibraryReadingStatus::Finished);
    store.set_status(Path::new("second.epub"), LibraryReadingStatus::Finished);
    assert_eq!(store.completed_books_count(), 2);
}

#[test]
fn test_estimate_minutes_left_in_chapter() {
    assert_eq!(estimate_minutes_left_in_chapter(0, 0.0, 220), 0);
    assert_eq!(estimate_minutes_left_in_chapter(1, 0.0, 220), 1);
    assert_eq!(estimate_minutes_left_in_chapter(1_100, 0.0, 220), 1);
    assert_eq!(estimate_minutes_left_in_chapter(2_200, 0.0, 220), 2);
    assert_eq!(estimate_minutes_left_in_chapter(2_200, 0.5, 220), 1);
    assert_eq!(estimate_minutes_left_in_chapter(2_200, 1.0, 220), 0);
    assert_eq!(estimate_minutes_left_in_chapter(2_200, 2.0, 220), 0);
    assert_eq!(estimate_minutes_left_in_chapter(2_200, -1.0, 220), 2);
    assert_eq!(estimate_minutes_left_in_chapter(1_100, 0.0, 0), 1);
}

#[test]
fn test_unix_day_index() {
    assert_eq!(unix_day_index(0), 0);
    assert_eq!(unix_day_index(86_399), 0);
    assert_eq!(unix_day_index(86_400), 1);
}

fn create_file_symlink(source: &Path, target: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(source, target).is_ok()
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (source, target);
        false
    }
}
