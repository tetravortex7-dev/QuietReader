use crate::core::bookmarks_export::{
    NotesFormat, bookmarks_to_csv, bookmarks_to_html, bookmarks_to_markdown, bookmarks_to_text,
    color_label,
};
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

#[test]
fn test_bookmarks_to_csv_escapes_and_orders() {
    let bookmarks = vec![
        bookmark(
            "quote",
            0,
            100.0,
            "Chapter, One",
            "Line with, comma\nand \"quotes\"",
            "Note, with comma",
            BookmarkColor::Yellow,
            BookmarkKind::Quote,
            12.5,
            20,
        ),
        bookmark(
            "bookmark",
            0,
            50.0,
            "Chapter, One",
            "Preview",
            "",
            BookmarkColor::Blue,
            BookmarkKind::Bookmark,
            5.0,
            10,
        ),
    ];

    let csv = bookmarks_to_csv("Test Book", "Author", &bookmarks);
    assert!(csv.starts_with("Book,Author,Chapter,Color,Kind,Progress %,Text,Note\n"));
    assert!(csv.contains(
        "Test Book,Author,\"Chapter, One\",Yellow,Quote,12.5,\"Line with, comma\nand \"\"quotes\"\"\",\"Note, with comma\"\n"
    ));
    assert!(csv.contains("Test Book,Author,\"Chapter, One\",Blue,Bookmark,5.0,Preview,\n"));
}

#[test]
fn test_bookmarks_to_html_escapes_markup() {
    let bookmarks = vec![bookmark(
        "quote",
        0,
        0.0,
        "Chapter <b>One</b>",
        "<script>alert('x')</script> & more",
        "Note \"quoted\"",
        BookmarkColor::Red,
        BookmarkKind::Quote,
        50.0,
        1,
    )];

    let html = bookmarks_to_html("Book & Title", "O'Author", &bookmarks);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(html.contains("<h1>Book &amp; Title</h1>"));
    assert!(html.contains("O&#39;Author"));
    assert!(html.contains("<h2>Chapter &lt;b&gt;One&lt;/b&gt;</h2>"));
    assert!(html.contains("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; more"));
    assert!(html.contains("Note &quot;quoted&quot;"));
    assert!(html.contains("Red · Quote · 50.0%"));
    assert!(html.trim_end().ends_with("</html>"));
}

#[test]
fn test_bookmarks_to_text_plain_layout() {
    let bookmarks = vec![bookmark(
        "bookmark",
        1,
        0.0,
        "",
        "Quoted line",
        "My note",
        BookmarkColor::Green,
        BookmarkKind::Bookmark,
        30.0,
        5,
    )];

    let text = bookmarks_to_text("Book", "", &bookmarks);
    assert!(text.starts_with("Book\n"));
    assert!(text.contains("========\n"));
    assert!(text.contains("Chapter 2\n------"));
    assert!(text.contains("Green · Bookmark · 30.0%"));
    assert!(text.contains("> Quoted line"));
    assert!(text.contains("Note: My note"));
}

#[test]
fn test_notes_format_render_and_extensions() {
    let bookmarks = vec![bookmark(
        "bm",
        0,
        0.0,
        "Chapter",
        "Text",
        "Note",
        BookmarkColor::Yellow,
        BookmarkKind::Quote,
        1.0,
        1,
    )];

    for (format, extension, marker) in [
        (NotesFormat::Markdown, "md", "# "),
        (NotesFormat::Csv, "csv", "Book,Author"),
        (NotesFormat::Html, "html", "<!DOCTYPE html>"),
        (NotesFormat::Text, "txt", "Book\n"),
    ] {
        assert_eq!(format.extension(), extension);
        let rendered = format.render("Book", "Author", &bookmarks);
        assert!(
            rendered.contains(marker),
            "{extension} output missing marker {marker}"
        );
    }
    assert_eq!(NotesFormat::default(), NotesFormat::Markdown);
}
