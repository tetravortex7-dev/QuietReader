#[test]
fn test_rtf_document_parsing() {
    use crate::document::Document;

    let rtf_data = b"{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0 Arial;}}{\\info{\\title Test Book}{\\author John Doe}}\\viewkind4\\uc1\\pard\\b Chapter 1\\b0\\par First paragraph of the story.\\par Second paragraph.\\par}";
    let doc =
        crate::document::RtfDocument::from_bytes(rtf_data, "Fallback").expect("rtf parse failed");
    assert_eq!(doc.metadata().title, "Test Book");
    assert_eq!(doc.metadata().author, "John Doe");
    assert_eq!(doc.chapters().len(), 1);
    let text = &doc.chapters()[0].paragraphs;
    assert!(text.iter().any(|p| p.contains("Chapter 1")));
    assert!(
        text.iter()
            .any(|p| p.contains("First paragraph of the story."))
    );
    assert!(text.iter().any(|p| p.contains("Second paragraph.")));
    assert_eq!(doc.toc().len(), 0);
}

#[test]
fn test_rtf_footnote_is_extracted() {
    use crate::document::Document;

    let rtf_data = b"{\\rtf1\\ansi Text with note{\\footnote Footnote text.} and end{\\endnote Endnote text.} more.\\par}";
    let doc = crate::document::RtfDocument::from_bytes(rtf_data, "Notes").unwrap();
    let paragraphs = &doc.chapters()[0].paragraphs;
    assert!(paragraphs.iter().any(|p| p.contains("{{note:rtf_fn_0}}")));
    assert!(paragraphs.iter().any(|p| p.contains("{{note:rtf_en_0}}")));
    assert_eq!(doc.footnote("rtf_fn_0"), Some("Footnote text."));
    assert_eq!(doc.footnote("rtf_en_0"), Some("Endnote text."));
    let notes_chapter = doc.chapters().len() - 1;
    assert_eq!(
        doc.note_location("rtf_fn_0")
            .map(|location| location.chapter_index),
        Some(notes_chapter)
    );
    assert_eq!(
        doc.note_location("rtf_en_0")
            .map(|location| location.chapter_index),
        Some(notes_chapter)
    );
}

#[test]
fn test_rtf_header_and_footer_text() {
    use crate::document::Document;

    let rtf_data = b"{\\rtf1{\\header Header text\\par}{\\footer Footer text\\par}Body text\\par}";
    let doc = crate::document::RtfDocument::from_bytes(rtf_data, "Story").unwrap();
    assert_eq!(doc.header_text(), Some("Header text"));
    assert_eq!(doc.footer_text(), Some("Footer text"));
    assert!(
        doc.chapters()[0]
            .paragraphs
            .iter()
            .any(|p| p.contains("Body text"))
    );
}

#[test]
fn test_rtf_corrupted_resilience() {
    let broken1 = b"{\\rtf1\\ansi broken without closing";
    let doc1 = crate::document::RtfDocument::from_bytes(broken1, "Broken");
    assert!(doc1.is_ok());

    let empty = b"";
    let doc2 = crate::document::RtfDocument::from_bytes(empty, "Empty");
    assert!(doc2.is_err());
}
