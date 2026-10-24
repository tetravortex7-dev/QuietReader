#[test]
fn test_doc_document_resilience_and_parsing() {
    let empty = b"";
    assert!(crate::document::DocDocument::from_bytes(empty, "Empty").is_err());

    let garbage = vec![0x12u8; 1024];
    assert!(crate::document::DocDocument::from_bytes(&garbage, "Garbage").is_err());

    let mut ole_header = vec![0u8; 512];
    ole_header[0..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    let res = crate::document::DocDocument::from_bytes(&ole_header, "TruncatedOLE");
    assert!(res.is_err());
}

#[test]
fn test_doc_story_split_footnotes_and_headers() {
    let main = "Main text\u{0002}";
    let notes = "Note text.\r";
    let headers = "Header text\rFooter text\r";
    let doc_text = format!("{main}{notes}{headers}");
    let mut fib = vec![0u8; 0x58];
    let main_len = main.chars().count() as u32;
    let notes_len = notes.chars().count() as u32;
    let headers_len = headers.chars().count() as u32;
    fib[0x4C..0x50].copy_from_slice(&main_len.to_le_bytes());
    fib[0x50..0x54].copy_from_slice(&notes_len.to_le_bytes());
    fib[0x54..0x58].copy_from_slice(&headers_len.to_le_bytes());

    let (text, footnotes, header, footer) = super::split_doc_stories(doc_text, &fib);
    assert!(text.contains("{{note:doc_fn_0}}"));
    assert_eq!(
        footnotes.get("doc_fn_0").map(String::as_str),
        Some("Note text.")
    );
    assert_eq!(header.as_deref(), Some("Header text"));
    assert_eq!(footer.as_deref(), Some("Footer text"));
}

#[test]
fn test_doc_story_split_endnotes_and_annotations() {
    let main = "Text\u{0002}\u{0002}\r";
    let notes = "Footnote text\r";
    let annotations = "Comment text\r";
    let endnotes = "Endnote text\r";
    let headers = "Head\rFoot\r";
    let doc_text = format!("{main}{notes}{annotations}{endnotes}{headers}");
    let mut fib = vec![0u8; 0x6C];
    fib[0x4C..0x50].copy_from_slice(&(main.chars().count() as u32).to_le_bytes());
    fib[0x50..0x54].copy_from_slice(&(notes.chars().count() as u32).to_le_bytes());
    fib[0x54..0x58].copy_from_slice(&(headers.chars().count() as u32).to_le_bytes());
    fib[0x5C..0x60].copy_from_slice(&(annotations.chars().count() as u32).to_le_bytes());
    fib[0x60..0x64].copy_from_slice(&(endnotes.chars().count() as u32).to_le_bytes());

    let (text, footnotes, header, footer) = super::split_doc_stories(doc_text, &fib);
    assert!(text.contains("{{note:doc_fn_0}}{{note:doc_fn_1}}"));
    assert_eq!(
        footnotes.get("doc_fn_0").map(String::as_str),
        Some("Footnote text")
    );
    assert_eq!(
        footnotes.get("doc_fn_1").map(String::as_str),
        Some("Endnote text")
    );
    assert!(!footnotes.values().any(|note| note.contains("Comment")));
    assert_eq!(header.as_deref(), Some("Head"));
    assert_eq!(footer.as_deref(), Some("Foot"));
}

#[test]
fn test_doc_fat_chain_reads_sector_bodies() {
    let sector_size = 512;
    let mut bytes = vec![0u8; sector_size * 3];
    bytes[512..516].copy_from_slice(b"S01!");
    bytes[1024..1028].copy_from_slice(b"S02!");
    let fat = vec![1u32, 0xFFFF_FFFE];
    let data = super::read_fat_chain(&bytes, sector_size, &fat, 0, sector_size + 4);
    assert_eq!(data.len(), sector_size + 4);
    assert_eq!(&data[0..4], b"S01!");
    assert_eq!(&data[sector_size..], b"S02!");
}

#[test]
fn test_doc_mini_stream_chain() {
    let mut container = vec![0u8; 64 * 3];
    container[0..8].copy_from_slice(b"AAAAAAA\n");
    container[64..72].copy_from_slice(b"BBBBBBB\n");
    container[128..136].copy_from_slice(b"CCCCCCC\n");
    let mini_fat = vec![1u32, 2, 0xFFFF_FFFE];
    let data = super::read_mini_chain(&container, &mini_fat, 0, 64 * 2 + 8, 64);
    assert_eq!(data.len(), 64 * 2 + 8);
    assert_eq!(&data[0..8], b"AAAAAAA\n");
    assert_eq!(&data[64..72], b"BBBBBBB\n");
    assert_eq!(&data[128..136], b"CCCCCCC\n");
}

#[test]
fn test_doc_mini_stream_cycle_and_bounds_are_safe() {
    let container = vec![7u8; 64 * 2];
    let mini_fat = vec![1u32, 0];
    let data = super::read_mini_chain(&container, &mini_fat, 0, 1024, 64);
    assert_eq!(data.len(), 128);

    let short = super::read_mini_chain(&container, &[], 0, 64, 64);
    assert_eq!(short.len(), 64);

    let out_of_range = super::read_mini_chain(&container, &[], 5, 64, 64);
    assert!(out_of_range.is_empty());
}
