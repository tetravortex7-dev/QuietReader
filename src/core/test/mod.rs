#[test]
fn test_reader_core_bookmarks_and_search() {
    use crate::core::{Bookmark, ReaderCore};
    use crate::document::TxtDocument;
    use std::path::PathBuf;

    let mut core = ReaderCore::default();
    let path = PathBuf::from("test_book.txt");
    let bm1 = Bookmark {
        book_path: path.clone(),
        chapter: 0,
        scroll_y: 100.0,
        label: "Marker 1".to_string(),
        timestamp: 123456,
        ..Default::default()
    };
    let bm2 = Bookmark {
        book_path: path.clone(),
        chapter: 1,
        scroll_y: 200.0,
        label: "Marker 2".to_string(),
        timestamp: 123457,
        ..Default::default()
    };

    core.add_bookmark(bm1.clone());
    core.add_bookmark(bm2.clone());
    core.add_bookmark(bm1.clone());

    assert_eq!(core.bookmarks_for_path(&path).count(), 2);

    core.remove_bookmark(&path, 0);
    assert_eq!(core.bookmarks_for_path(&path).count(), 1);

    core.clear_bookmarks_for_path(&path);
    assert_eq!(core.bookmarks_for_path(&path).count(), 0);

    let doc = TxtDocument::from_text(
        "First chapter with keyword needle inside it\nSecond chapter without it",
        "Doc",
    );
    let hits = ReaderCore::search_in_document(&doc, "needle");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chapter_index, 0);
    assert!(hits[0].snippet.contains("needle"));
}
