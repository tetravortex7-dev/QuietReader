use crate::core::bookmarks_export::{bookmarks_to_markdown, color_label};
use crate::core::{Bookmark, BookmarkColor, BookmarkKind};
use std::path::PathBuf;

fn bookmark(
    id: &str,
    chapter: usize,
    scroll_y: f32,
    label: &str,
    text: &str,
    note: &str,
    color: BookmarkColor,
    kind: BookmarkKind,
    progress_percent: f32,
    timestamp: u64,
) -> Bookmark {
    Bookmark {
        id: id.to_string(),
        book_path: PathBuf::from("book.epub"),
        chapter,
        scroll_y,
        label: label.to_string(),
        text: text.to_string(),
        note: note.to_string(),
        color,
        kind,
        timestamp,
        progress_percent,
        page_number: None,
    }
}

#[test]
fn test_bookmarks_to_markdown_groups_sorts_and_formats() {
    let bookmarks = vec![
        bookmark(
            "quote",
            0,
            100.0,
            "Chapter One",
            "First line\nSecond line",
            "Remember this",
            BookmarkColor::Yellow,
            BookmarkKind::Quote,
            12.5,
            20,
        ),
        bookmark(
            "bookmark",
            0,
            50.0,
            "Chapter One",
            "Bookmark preview",
            "",
            BookmarkColor::Blue,
            BookmarkKind::Bookmark,
            5.0,
            10,
        ),
        bookmark(
            "fallback",
            1,
            10.0,
            "",
            "Fallback chapter",
            "Second note",
            BookmarkColor::Green,
            BookmarkKind::Quote,
            33.3,
            30,
        ),
    ];

    let markdown = bookmarks_to_markdown("Test Book", "Author Name", &bookmarks);

    assert!(markdown.starts_with("# Test Book"));
    assert!(markdown.contains("**Author:** Author Name"));
    assert!(markdown.contains("## Chapter One"));
    assert!(markdown.contains("## Chapter 2"));
    assert!(markdown.contains("### Yellow — 12.5%"));
    assert!(markdown.contains("- **Color:** Yellow"));
    assert!(markdown.contains("- **Chapter:** Chapter One"));
    assert!(markdown.contains("- **Progress:** 12.5%"));
    assert!(markdown.contains("- **Kind:** Quote"));
    assert!(markdown.contains("> First line\n> Second line"));
    assert!(markdown.contains("**Note:** Remember this"));
    assert!(markdown.contains("### Blue — 5.0%"));
    assert!(markdown.contains("- **Kind:** Bookmark"));
    assert!(markdown.contains("### Green — 33.3%"));
    assert!(markdown.contains("**Note:** Second note"));

    let preview_position = markdown.find("Bookmark preview").unwrap();
    let quote_position = markdown.find("First line").unwrap();
    assert!(preview_position < quote_position);

    let first_chapter_position = markdown.find("## Chapter One").unwrap();
    let second_chapter_position = markdown.find("## Chapter 2").unwrap();
    assert!(first_chapter_position < second_chapter_position);
}

#[test]
fn test_bookmark_color_labels() {
    assert_eq!(color_label(BookmarkColor::Yellow), "Yellow");
    assert_eq!(color_label(BookmarkColor::Green), "Green");
    assert_eq!(color_label(BookmarkColor::Blue), "Blue");
    assert_eq!(color_label(BookmarkColor::Red), "Red");
}

#[test]
fn test_bookmarks_to_markdown_empty_title_and_author() {
    let markdown = bookmarks_to_markdown("", "", &[]);

    assert!(markdown.starts_with("# QuietReader Notes"));
    assert!(!markdown.contains("**Author:**"));
    assert!(markdown.contains("---"));
}
