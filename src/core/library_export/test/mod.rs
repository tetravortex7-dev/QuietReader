use crate::core::library::{LibraryEntry, LibraryReadingStatus};
use crate::core::library_export::export_library;
use crate::test::util::unique_test_dir;

#[test]
fn exports_books_deduplicates_names_and_writes_csv() {
    let source = unique_test_dir("library_export_src");
    let destination = unique_test_dir("library_export_dst");
    let first_dir = source.join("first");
    let second_dir = source.join("second");
    std::fs::create_dir_all(&first_dir).unwrap();
    std::fs::create_dir_all(&second_dir).unwrap();

    let first = first_dir.join("book.epub");
    let second = second_dir.join("book.epub");
    let missing = source.join("missing.fb2");
    std::fs::write(&first, b"first").unwrap();
    std::fs::write(&second, b"second").unwrap();

    let entries = vec![
        LibraryEntry {
            path: first,
            title: "First, Book".to_string(),
            author: "Author \"One\"".to_string(),
            status: LibraryReadingStatus::Reading,
            current_unit: 1,
            total_units: 2,
            ..Default::default()
        },
        LibraryEntry {
            path: second,
            title: "Second Book".to_string(),
            ..Default::default()
        },
        LibraryEntry {
            path: missing,
            title: "Missing".to_string(),
            ..Default::default()
        },
    ];

    let report = export_library(&destination, &entries).unwrap();
    assert_eq!(report.exported, 2);
    assert_eq!(report.skipped, 1);
    assert_eq!(
        std::fs::read(destination.join("book.epub")).unwrap(),
        b"first"
    );
    assert_eq!(
        std::fs::read(destination.join("book-2.epub")).unwrap(),
        b"second"
    );

    let csv = std::fs::read_to_string(destination.join("library.csv")).unwrap();
    assert!(csv.starts_with("Title,Author,Status,Progress,File\n"));
    assert!(csv.contains("\"First, Book\""));
    assert!(csv.contains("\"Author \"\"One\"\"\""));
    assert!(csv.contains("Reading,50.0%,book.epub"));
    assert!(csv.contains("Second Book,,Unread,0.0%,book-2.epub"));

    let _ = std::fs::remove_dir_all(&source);
    let _ = std::fs::remove_dir_all(&destination);
}
