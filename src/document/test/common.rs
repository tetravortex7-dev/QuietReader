use crate::test::util::unique_test_dir;
use crate::*;

#[test]
fn test_xml_fb2_requires_fictionbook_root() {
    use crate::document::registry::{DocumentFormat, Fb2Format};

    let dir = unique_test_dir("xml_fb2");
    let generic_xml = dir.join("generic.xml");
    std::fs::write(
        &generic_xml,
        r#"<?xml version="1.0" encoding="utf-8"?><root><item>value</item></root>"#,
    )
    .unwrap();
    assert!(!Fb2Format.probe(&generic_xml));
    assert!(document::open_document(&generic_xml).is_err());

    let fb2_xml = dir.join("book.xml");
    std::fs::write(
        &fb2_xml,
        r#"<?xml version="1.0" encoding="utf-8"?><FictionBook><body><section><title><p>Title</p></title><p>Text</p></section></body></FictionBook>"#,
    )
    .unwrap();
    assert!(Fb2Format.probe(&fb2_xml));
    let doc = document::open_document(&fb2_xml).expect("FictionBook XML must open as FB2");
    assert_eq!(doc.format_name(), "FB2");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_registry_docx_rtf_doc_support() {
    use crate::document::FORMAT_REGISTRY;

    let has_docx = FORMAT_REGISTRY
        .iter()
        .any(|f| f.extensions().contains(&"docx"));
    let has_rtf = FORMAT_REGISTRY
        .iter()
        .any(|f| f.extensions().contains(&"rtf"));
    let has_doc = FORMAT_REGISTRY
        .iter()
        .any(|f| f.extensions().contains(&"doc"));

    assert!(has_docx);
    assert!(has_rtf);
    assert!(has_doc);

    let test_dir = unique_test_dir("reg_probe");
    let docx_path = test_dir.join("book.docx");
    let rtf_path = test_dir.join("note.rtf");
    let doc_path = test_dir.join("old.doc");

    std::fs::write(&docx_path, b"PK\x03\x04dummy").unwrap();
    std::fs::write(&rtf_path, b"{\\rtf1dummy}").unwrap();
    std::fs::write(
        &doc_path,
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0x00],
    )
    .unwrap();

    assert!(FORMAT_REGISTRY.iter().any(|f| f.probe(&docx_path)));
    assert!(FORMAT_REGISTRY.iter().any(|f| f.probe(&rtf_path)));
    assert!(FORMAT_REGISTRY.iter().any(|f| f.probe(&doc_path)));

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_corrupted_pdf_epub_fb2_handling() {
    use crate::document::{DocumentError, EpubDocument, Fb2Document, PdfDocument};
    use std::path::Path;

    let bad_pdf = PdfDocument::from_bytes(b"NOT_A_VALID_PDF_FILE".to_vec(), Path::new("bad.pdf"));
    assert!(matches!(bad_pdf, Err(DocumentError::Corrupted(_))));

    let bad_pdf_header =
        PdfDocument::from_bytes(b"%PDF-1.4\nBROKEN_GARBAGE".to_vec(), Path::new("bad2.pdf"));
    assert!(matches!(bad_pdf_header, Err(DocumentError::Corrupted(_))));

    let bad_epub = EpubDocument::from_bytes(b"NOT_A_ZIP_ARCHIVE");
    assert!(matches!(bad_epub, Err(DocumentError::Corrupted(_))));

    let bad_fb2 = Fb2Document::from_bytes(b"THIS IS NOT XML OR FICTIONBOOK AT ALL");
    assert!(matches!(bad_fb2, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_document_format_name_and_custom_toc() {
    use document::{Document, Fb2Document, TxtDocument};
    let txt = TxtDocument::from_text("Sample text", "title");
    assert_eq!(txt.format_name(), "TXT");
    assert!(!txt.has_custom_toc());

    let fb2_sample = r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description><title-info><book-title>Test</book-title></title-info></description>
  <body name="toc"><title><p>TOC</p></title><p><a href="#ch1">Ch 1</a></p></body>
  <body><section id="ch1"><p>Text</p></section></body>
</FictionBook>"##;
    let fb2 = Fb2Document::from_bytes(fb2_sample.as_bytes()).unwrap();
    assert_eq!(fb2.format_name(), "FB2");
    assert!(fb2.has_custom_toc());
}
