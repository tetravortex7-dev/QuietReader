use crate::document::registry::{DocumentFormat, HtmlFormat};
use crate::document::{Document, HtmlDocument};

#[test]
fn test_html_splits_chapters_by_headings() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
  <title>Saved Article</title>
  <meta name="author" content="Jane Doe">
  <style>body { color: red; }</style>
  <script>alert('x');</script>
</head>
<body>
  <p>Intro &amp; preface.</p>
  <h1>First Chapter</h1>
  <p>Hello <b>world</b>.</p>
  <p>Second paragraph.</p>
  <h2>Second Chapter</h2>
  <p>More text &mdash; with dash.</p>
</body>
</html>"#;

    let doc = HtmlDocument::from_bytes(html.as_bytes(), "fallback", None).unwrap();
    assert_eq!(doc.metadata().title, "Saved Article");
    assert_eq!(doc.metadata().author, "Jane Doe");
    assert_eq!(doc.chapters().len(), 3);
    assert_eq!(doc.toc().len(), 3);
    assert_eq!(doc.toc()[0].title, "Saved Article");
    assert_eq!(doc.toc()[1].title, "First Chapter");
    assert_eq!(doc.toc()[2].title, "Second Chapter");
    assert!(
        doc.chapters()[0]
            .paragraphs
            .join(" ")
            .contains("Intro & preface")
    );
    assert!(
        doc.chapters()[1]
            .paragraphs
            .join(" ")
            .contains("Hello <b>world</b>")
    );
    assert!(
        doc.chapters()[2]
            .paragraphs
            .join(" ")
            .contains("More text — with dash")
    );
    let all = doc
        .chapters()
        .iter()
        .flat_map(|chapter| chapter.paragraphs.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(!all.contains("alert"));
    assert!(!all.contains("color: red"));
}

#[test]
fn test_html_without_headings_is_single_chapter() {
    let html = "<html><head><title>One Page</title></head><body><p>Only text.</p></body></html>";
    let doc = HtmlDocument::from_bytes(html.as_bytes(), "fallback", None).unwrap();
    assert_eq!(doc.chapters().len(), 1);
    assert!(!doc.has_custom_toc());
    assert_eq!(doc.chapters()[0].title, "One Page");
    assert_eq!(doc.chapters()[0].paragraphs[0], "Only text.");
}

#[test]
fn test_html_rejects_empty_documents() {
    let error = HtmlDocument::from_bytes(b"   ", "empty", None)
        .err()
        .unwrap();
    assert!(matches!(
        error,
        crate::document::DocumentError::Corrupted(_)
    ));
}

#[test]
fn test_html_registry_probe_and_open() {
    let dir = crate::test::util::unique_test_dir("html_probe");
    let path = dir.join("article.html");
    std::fs::write(
        &path,
        "<html><body><h1>Title</h1><p>Body text.</p></body></html>",
    )
    .unwrap();
    assert!(HtmlFormat.probe(&path));
    assert!(crate::document::registry::is_supported_document_path(&path));

    let doc = crate::document::open_document(&path).unwrap();
    assert_eq!(doc.format_name(), "HTML");
    assert_eq!(doc.chapters().len(), 1);

    let sniff = dir.join("no_extension");
    std::fs::write(
        &sniff,
        "<!doctype html><html><body><p>Sniffed.</p></body></html>",
    )
    .unwrap();
    assert!(HtmlFormat.probe(&sniff));

    let _ = std::fs::remove_dir_all(&dir);
}
