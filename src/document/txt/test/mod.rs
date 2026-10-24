use crate::test::util::unique_test_dir;
use crate::*;

#[test]
fn test_txt_md_log_format_via_registry() {
    let temp_dir = unique_test_dir("txt_md_log");
    let md_path = temp_dir.join("notes.md");
    std::fs::write(
        &md_path,
        "# Глава 1. Введение\n\nПервый абзац.\n\n## Глава 2. Детали\n\nВторой абзац.",
    )
    .unwrap();

    let doc = document::open_document(&md_path).expect("Должен открыть MD через реестр");
    assert_eq!(doc.chapters().len(), 2);
    assert_eq!(doc.toc().len(), 2);
    assert_eq!(doc.toc()[0].title, "Глава 1. Введение");
    assert_eq!(doc.toc()[1].title, "Глава 2. Детали");

    let txt_path = temp_dir.join("sample.txt");
    std::fs::write(&txt_path, "Абзац первый.\nВторая строка.\n\nАбзац второй.").unwrap();
    let txt_doc = document::open_document(&txt_path).expect("Должен открыть TXT через реестр");
    assert_eq!(txt_doc.chapters().len(), 1);
    assert_eq!(txt_doc.chapters()[0].paragraphs.len(), 2);

    let log_path = temp_dir.join("system.log");
    std::fs::write(&log_path, "[INFO] Started\n[WARN] Low memory\n").unwrap();
    let log_doc = document::open_document(&log_path).expect("Должен открыть LOG через реестр");
    assert_eq!(log_doc.chapters().len(), 1);
    assert_eq!(log_doc.chapters()[0].paragraphs.len(), 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_txt_forced_encoding_override() {
    use crate::document::Document;
    let text = "Привет, мир! Это принудительная кодировка.";
    let (bytes, _, _) = encoding_rs::WINDOWS_1251.encode(text);
    let doc = document::TxtDocument::from_bytes_with_encoding(
        &bytes,
        "Encoded",
        "txt",
        Some("windows-1251"),
    )
    .unwrap();
    assert_eq!(doc.chapters()[0].paragraphs[0], text);

    let utf16: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let utf16_doc =
        document::TxtDocument::from_bytes_with_encoding(&utf16, "Encoded", "txt", Some("utf-16le"))
            .unwrap();
    assert_eq!(utf16_doc.chapters()[0].paragraphs[0], text);
}

#[test]
fn test_txt_single_byte_encoding_detection() {
    use crate::document::Document;

    let cases = [
        (
            encoding_rs::WINDOWS_1251,
            "Привет, мир! Это простой текст для проверки кодировки.",
        ),
        (
            encoding_rs::WINDOWS_1252,
            "Café déjà vu — naïve façade with some accents.",
        ),
        (
            encoding_rs::KOI8_R,
            "Привет, мир! Это простой текст для проверки кодировки.",
        ),
        (
            encoding_rs::IBM866,
            "Привет, мир! Это простой текст для проверки кодировки.",
        ),
    ];

    for (encoding, text) in cases {
        let (bytes, _, had_errors) = encoding.encode(text);
        assert!(!had_errors);
        let doc = document::TxtDocument::from_bytes(&bytes, "Encoded", "txt")
            .expect("Должен декодировать однобайтовую кодировку");
        assert_eq!(doc.chapters()[0].paragraphs[0], text);
    }
}

#[test]
fn test_txt_chapter_templates_split_sections() {
    use crate::document::Document;
    let text = "Пролог\n\nТекст пролога.\n\nГлава 1. Начало\n\nПервый абзац.\n\nГлава 2. Конец\n\nВторой абзац.";
    let doc = document::TxtDocument::from_bytes(text.as_bytes(), "Book", "txt").unwrap();
    assert_eq!(doc.chapters().len(), 3);
    assert_eq!(doc.toc()[0].title, "Пролог");
    assert_eq!(doc.toc()[1].title, "Глава 1. Начало");
    assert_eq!(doc.toc()[2].title, "Глава 2. Конец");
    assert!(doc.has_custom_toc());
}

#[test]
fn test_log_section_headings_split_sections() {
    use crate::document::Document;
    let text = "=== Start ===\nline one\n=== End ===\nline two";
    let doc = document::TxtDocument::from_bytes(text.as_bytes(), "Log", "log").unwrap();
    assert_eq!(doc.chapters().len(), 2);
    assert_eq!(doc.toc()[0].title, "Start");
    assert_eq!(doc.toc()[1].title, "End");
}

#[test]
fn test_markdown_inline_formatting_and_lists() {
    use crate::document::Document;
    let text = "# Title\n\n**bold** and *italic* and [link](https://example.com)\n\n- one\n- two\n\n> quote";
    let doc = document::TxtDocument::from_bytes(text.as_bytes(), "Md", "md").unwrap();
    let paragraphs = &doc.chapters()[0].paragraphs;
    assert!(paragraphs.iter().any(|p| p.contains("<b>bold</b>")));
    assert!(paragraphs.iter().any(|p| p.contains("<i>italic</i>")));
    assert!(paragraphs.iter().any(|p| p.contains("link")));
    assert!(!paragraphs.iter().any(|p| p.contains("https://")));
    assert!(paragraphs.iter().any(|p| p == "• one"));
    assert!(paragraphs.iter().any(|p| p == "• two"));
    assert!(paragraphs.iter().any(|p| p == "<i>quote</i>"));
}

#[test]
fn test_utf16_txt_decoding_and_pageup_prev_chapter() {
    use crate::document::Document;
    let text = "Привет, мир!\n\nВторой абзац.";
    let mut utf16le_bytes = vec![0xFF, 0xFE];
    for code_unit in text.encode_utf16() {
        utf16le_bytes.extend_from_slice(&code_unit.to_le_bytes());
    }
    let doc = document::TxtDocument::from_bytes(&utf16le_bytes, "UTF16Test", "txt")
        .expect("Должен успешно декодировать UTF-16LE");
    assert_eq!(doc.chapters().len(), 1);
    assert_eq!(doc.chapters()[0].paragraphs[0], "Привет, мир!");
    assert_eq!(doc.chapters()[0].paragraphs[1], "Второй абзац.");

    let md_bytes = b"# Ch1\n\nText 1\n\n# Ch2\n\nText 2";
    let md_doc = document::TxtDocument::from_bytes(md_bytes, "MdBook", "md").unwrap();
    let mut ch = 1usize;
    let mut last_rendered = 1usize;
    let mut pending_scroll = None;
    let changed = crate::core::ReaderCore::apply_text_command(
        &crate::core::Command::PageUp,
        &md_doc,
        &mut ch,
        &mut last_rendered,
        0.0,
        500.0,
        600.0,
        &mut pending_scroll,
    );
    assert!(changed);
    assert_eq!(ch, 0);
    assert_eq!(pending_scroll, Some(999_999.0));
}
