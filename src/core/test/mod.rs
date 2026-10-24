mod bookmarks_export;
mod library;
mod license;
mod search;
mod snapshots;

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
    assert_eq!(hits[0].page_index, None);
    assert!(hits[0].snippet.contains("needle"));

    let cancelled = std::sync::atomic::AtomicBool::new(true);
    assert!(ReaderCore::search_in_document_with_cancel(&doc, "needle", &cancelled).is_empty());
}

#[test]
fn test_paragraph_anchor_roundtrip() {
    use crate::core::ReaderCore;
    use crate::document::TxtDocument;

    let text = (0..10)
        .map(|index| {
            format!(
                "Paragraph {index} contains enough words to occupy several lines in the reader column"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let doc = TxtDocument::from_text(&text, "AnchorDoc");
    let scroll = ReaderCore::paragraph_anchor_scroll_y(&doc, 0, 5, 19.0, 1.45, 760.0);
    let visible =
        ReaderCore::estimate_visible_paragraph_with_layout(&doc, 0, scroll, 19.0, 1.45, 760.0);
    assert_eq!(visible, 5);
}
