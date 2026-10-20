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
