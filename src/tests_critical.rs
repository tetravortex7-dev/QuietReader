use crate::app::document_loader::DocumentLoader;
use crate::document::{
    DjvuDocument, Document, DocumentError, EpubDocument, Fb2Document, MAX_ARCHIVE_ENTRIES,
    MAX_SINGLE_ENTRY_BYTES, MAX_TOTAL_UNPACK_BYTES, PdfDocument, is_safe_archive_path,
    read_bounded,
};
use crate::settings::AppSettings;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn unique_test_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("quietreader_crit_{prefix}_{pid}_{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_read_bounded_limits_and_overflow_protection() {
    let data = vec![0xAAu8; 4096];
    let mut total_unpacked = 0u64;

    let ok_read = read_bounded(&data[..100], 500, &mut total_unpacked, 1000);
    assert!(ok_read.is_ok());
    assert_eq!(ok_read.unwrap().len(), 100);
    assert_eq!(total_unpacked, 100);

    let err_single = read_bounded(&data[..], 200, &mut total_unpacked, 10000);
    assert!(matches!(err_single, Err(DocumentError::LimitExceeded(_))));

    let mut total_unpacked = 900u64;
    let err_total = read_bounded(&data[..200], 1000, &mut total_unpacked, 1000);
    assert!(matches!(err_total, Err(DocumentError::LimitExceeded(_))));

    let mut total_unpacked = u64::MAX - 5;
    let err_overflow = read_bounded(&data[..10], 100, &mut total_unpacked, u64::MAX);
    assert!(matches!(err_overflow, Err(DocumentError::LimitExceeded(_))));
}

#[test]
fn test_is_safe_archive_path_comprehensive() {
    assert!(!is_safe_archive_path(""));
    assert!(!is_safe_archive_path(" "));
    assert!(!is_safe_archive_path(".."));
    assert!(!is_safe_archive_path("../test.txt"));
    assert!(!is_safe_archive_path("..\\test.txt"));
    assert!(!is_safe_archive_path("foo/../bar"));
    assert!(!is_safe_archive_path("foo/..\\bar"));
    assert!(!is_safe_archive_path("/etc/shadow"));
    assert!(!is_safe_archive_path("\\Windows\\System32"));
    assert!(!is_safe_archive_path("C:\\autoexec.bat"));
    assert!(!is_safe_archive_path("D:file.txt"));
    assert!(!is_safe_archive_path("\\\\server\\share\\file"));
    assert!(!is_safe_archive_path("//server/share/file"));
    assert!(!is_safe_archive_path("%2e%2e/evil.txt"));
    assert!(!is_safe_archive_path("..%2fevil.txt"));
    assert!(!is_safe_archive_path("%2e%2e%5cevil.txt"));
    assert!(!is_safe_archive_path("%2fetc/passwd"));
    assert!(!is_safe_archive_path("%5cWindows"));

    assert!(is_safe_archive_path("chapter1.xhtml"));
    assert!(is_safe_archive_path("images/cover.jpg"));
    assert!(is_safe_archive_path("OEBPS/text/part1.xhtml"));
    assert!(is_safe_archive_path("a/b/c/d/e.png"));
}

#[test]
fn test_corrupted_and_truncated_djvu_handling() {
    let dummy_path = Path::new("test.djvu");

    let res_empty = DjvuDocument::from_bytes(Vec::new(), dummy_path);
    assert!(res_empty.is_err());

    let res_short = DjvuDocument::from_bytes(vec![0x41, 0x54], dummy_path);
    assert!(res_short.is_err());

    let res_garbage = DjvuDocument::from_bytes(b"RANDOM_NON_DJVU_DATA_BYTES".to_vec(), dummy_path);
    assert!(res_garbage.is_err());

    let res_bad_form =
        DjvuDocument::from_bytes(b"AT&TXXXX\x00\x00\x00\x04DJVU".to_vec(), dummy_path);
    assert!(res_bad_form.is_err());

    let res_trunc = DjvuDocument::from_bytes(b"AT&TFORM".to_vec(), dummy_path);
    assert!(res_trunc.is_err());
}

#[test]
fn test_corrupted_and_truncated_pdf_handling() {
    let dummy_path = Path::new("test.pdf");

    let res_empty = PdfDocument::from_bytes(Vec::new(), dummy_path);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_short = PdfDocument::from_bytes(vec![b'%', b'P'], dummy_path);
    assert!(matches!(res_short, Err(DocumentError::Corrupted(_))));

    let res_garbage = PdfDocument::from_bytes(b"THIS_IS_DEFINITELY_NOT_A_PDF".to_vec(), dummy_path);
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let res_header_only = PdfDocument::from_bytes(b"%PDF-1.7\n%%EOF".to_vec(), dummy_path);
    assert!(matches!(res_header_only, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_corrupted_and_malformed_epub_handling() {
    let res_empty = EpubDocument::from_bytes(&[]);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_garbage = EpubDocument::from_bytes(b"PK\x03\x04\x00\x00BROKEN_ZIP_DATA");
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("unrelated.txt", options).unwrap();
        std::io::Write::write_all(&mut zip, b"no container xml").unwrap();
        zip.finish().unwrap();
    }
    let res_no_container = EpubDocument::from_bytes(&zip_buf.into_inner());
    assert!(matches!(res_no_container, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_corrupted_and_malformed_fb2_handling() {
    let res_empty = Fb2Document::from_bytes(&[]);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_garbage = Fb2Document::from_bytes(b"PLAIN TEXT WITHOUT ANY XML TAGS");
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let res_invalid_xml = Fb2Document::from_bytes(b"<FictionBook><body><section><title><p>Broken");
    assert!(matches!(res_invalid_xml, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_reading_progress_persistence_and_normalization() {
    let mut settings = AppSettings::default();
    let p1 = PathBuf::from("C:\\Books\\Library\\Novel.epub");
    let p1_lower = PathBuf::from("c:\\books\\library\\novel.epub");
    let p2 = PathBuf::from("D:\\Comics\\Manga_Vol1.cbz");

    settings.record_book_progress(&p1, "Novel Title", "Author Name", 4, 320.0);
    assert_eq!(settings.get_book_progress(&p1), Some((4, 320.0)));
    assert_eq!(settings.get_book_progress(&p1_lower), Some((4, 320.0)));

    settings.record_book_progress(&p2, "Manga Title", "Mangaka", 12, 0.0);
    assert_eq!(settings.get_book_progress(&p2), Some((12, 0.0)));

    assert_eq!(settings.reading_progress.len(), 2);
    assert_eq!(settings.recent_books.len(), 2);

    settings.recent_books.clear();
    assert_eq!(settings.recent_books.len(), 0);
    assert_eq!(settings.get_book_progress(&p1), Some((4, 320.0)));
    assert_eq!(settings.get_book_progress(&p2), Some((12, 0.0)));
}

#[test]
fn test_atomic_settings_save_and_backup_safety() {
    let temp_dir = unique_test_dir("save_safety");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let corrupted_path = temp_dir.join("settings.corrupted");

    let mut settings = AppSettings::default();
    settings.font_size = 28.0;
    settings.theme = "Sepia".to_string();

    let save_res = settings.save_to_path(&settings_path);
    assert!(save_res.is_ok());
    assert!(settings_path.exists());

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded.font_size, 28.0);
    assert_eq!(loaded.theme, "Sepia");

    std::fs::write(
        &bak_path,
        r#"{"schema_version":2,"font_size":25.0,"theme":"Dark"}"#,
    )
    .unwrap();
    std::fs::write(&settings_path, "{ broken invalid json...").unwrap();

    let recovered = AppSettings::load_from_path(&settings_path);
    assert_eq!(recovered.font_size, 25.0);
    assert_eq!(recovered.theme, "Dark");
    assert!(corrupted_path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_async_loader_cancellation_and_generation_switch() {
    let mut loader = DocumentLoader::default();
    let path_a = PathBuf::from("nonexistent_book_a.epub");
    let path_b = PathBuf::from("nonexistent_book_b.pdf");

    let req_a = loader.request_load(path_a.clone(), crate::i18n::UiLanguage::Ru);
    let req_b = loader.request_load(path_b.clone(), crate::i18n::UiLanguage::Ru);

    assert!(req_a > 0);
    assert!(req_b > req_a);

    let start = std::time::Instant::now();
    let mut finished = Vec::new();
    while start.elapsed() < std::time::Duration::from_secs(3) && finished.len() < 2 {
        for resp in loader.poll_responses() {
            finished.push(resp);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert_eq!(finished.len(), 2);
    let resp_a = finished.iter().find(|r| r.request_id == req_a).unwrap();
    let resp_b = finished.iter().find(|r| r.request_id == req_b).unwrap();
    assert_eq!(resp_a.path, path_a);
    assert_eq!(resp_b.path, path_b);
    assert!(resp_a.result.is_err());
    assert!(resp_b.result.is_err());
}

#[test]
fn test_resource_limits_constants() {
    assert!(MAX_ARCHIVE_ENTRIES >= 1000);
    assert!(MAX_SINGLE_ENTRY_BYTES >= 10 * 1024 * 1024);
    assert!(MAX_TOTAL_UNPACK_BYTES >= 50 * 1024 * 1024);
    assert!(MAX_TOTAL_UNPACK_BYTES > MAX_SINGLE_ENTRY_BYTES);
}

#[test]
fn test_paged_geometry_bounded_and_centered() {
    use crate::app::paged_common::compute_paged_geometry;
    use eframe::egui::Vec2;

    let avail_size = Vec2::new(1200.0, 800.0);
    let page_tex = Vec2::new(600.0, 900.0);

    let geom_fit_window = compute_paged_geometry(avail_size, "FitWindow", 100.0, 0, &[page_tex]);
    assert_eq!(geom_fit_window.max_pan, Vec2::ZERO);
    assert!(geom_fit_window.pad_x > 200.0);
    assert!(geom_fit_window.page_sizes[0].y <= avail_size.y);
    assert!(geom_fit_window.page_sizes[0].x <= avail_size.x);

    let geom_fit_width = compute_paged_geometry(avail_size, "FitWidth", 100.0, 0, &[page_tex]);
    assert_eq!(geom_fit_width.max_pan.x, 0.0);
    assert!(geom_fit_width.max_pan.y > 0.0);
    assert_eq!(
        geom_fit_width.max_pan.y,
        (geom_fit_width.page_sizes[0].y - avail_size.y) / 2.0
    );

    let geom_double =
        compute_paged_geometry(avail_size, "FitWindow", 100.0, 0, &[page_tex, page_tex]);
    assert_eq!(geom_double.max_pan, Vec2::ZERO);
    assert_eq!(geom_double.page_sizes.len(), 2);
    assert!(geom_double.pad_x >= 8.0);
}

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
fn test_rtf_corrupted_resilience() {
    let broken1 = b"{\\rtf1\\ansi broken without closing";
    let doc1 = crate::document::RtfDocument::from_bytes(broken1, "Broken");
    assert!(doc1.is_ok());

    let empty = b"";
    let doc2 = crate::document::RtfDocument::from_bytes(empty, "Empty");
    assert!(doc2.is_err());
}

#[test]
fn test_docx_document_parsing() {
    use crate::document::Document;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options = SimpleFileOptions::default();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Docx Title</dc:title>
    <dc:creator>Docx Author</dc:creator>
</cp:coreProperties>"#).unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
    <w:body>
        <w:p>
            <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
            <w:r><w:t>Heading One</w:t></w:r>
        </w:p>
        <w:p><w:r><w:t>Paragraph under first heading.</w:t></w:r></w:p>
        <w:p>
            <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
            <w:r><w:t>Heading Two</w:t></w:r>
        </w:p>
        <w:p><w:r><w:t>Paragraph under second heading.</w:t></w:r></w:p>
    </w:body>
</w:document>"#,
        )
        .unwrap();

        zip.finish().unwrap();
    }

    let bytes = buf.into_inner();
    let doc =
        crate::document::DocxDocument::from_bytes(&bytes, "Fallback").expect("docx parse failed");
    assert_eq!(doc.metadata().title, "Docx Title");
    assert_eq!(doc.metadata().author, "Docx Author");
    assert_eq!(doc.chapters().len(), 2);
    assert_eq!(doc.chapters()[0].title, "Heading One");
    assert!(
        doc.chapters()[0]
            .paragraphs
            .iter()
            .any(|p| p.contains("Paragraph under first heading."))
    );
    assert_eq!(doc.chapters()[1].title, "Heading Two");
    assert!(
        doc.chapters()[1]
            .paragraphs
            .iter()
            .any(|p| p.contains("Paragraph under second heading."))
    );

    let toc = doc.toc();
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].title, "Heading One");
    assert_eq!(toc[1].title, "Heading Two");
}

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
fn test_platform_multi_extension_selection() {
    use crate::platform::SUPPORTED_EXTENSIONS_INFO;

    let exts: Vec<&str> = SUPPORTED_EXTENSIONS_INFO.iter().map(|(e, _)| *e).collect();
    assert!(exts.contains(&"fb2"));
    assert!(exts.contains(&"txt"));
    assert!(exts.contains(&"doc"));
    assert!(exts.contains(&"docx"));
    assert!(exts.contains(&"epub"));
    assert!(exts.contains(&"mobi"));
    assert!(exts.contains(&"azw3"));
    assert!(exts.contains(&"rtf"));
    assert!(exts.contains(&"pdf"));
    assert!(exts.contains(&"djvu"));
    assert!(exts.contains(&"cbz"));
    assert!(exts.contains(&"cbr"));
    assert!(exts.contains(&"cb7"));
    assert!(exts.contains(&"md"));

    let mut settings = AppSettings::default();
    assert_eq!(settings.associated_extensions, vec!["fb2", "epub"]);

    settings.associated_extensions = vec!["fb2".to_string(), "txt".to_string(), "doc".to_string()];
    let json = serde_json::to_string(&settings).expect("serialize settings");
    let loaded: AppSettings = serde_json::from_str(&json).expect("deserialize settings");
    assert_eq!(loaded.associated_extensions, vec!["fb2", "txt", "doc"]);
}

#[test]
fn test_fb2_formatting_bold_and_italic() {
    let xml = r#"<FictionBook><body>
        <section>
            <title><p>Заголовок раздела</p></title>
            <p>Обычный текст с <strong>жирным словом</strong> и <emphasis>курсивом</emphasis>.</p>
            <p><strong><em>Жирный курсив</em></strong> вместе.</p>
            <p><strong>В</strong> начале было Слово.</p>
            <v>Строка стиха с <strong>жирным</strong> знаком.</v>
        </section>
    </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let chs = doc.chapters();
    assert_eq!(chs.len(), 1);
    assert_eq!(chs[0].paragraphs.len(), 4);

    assert_eq!(
        chs[0].paragraphs[0],
        "Обычный текст с <b>жирным словом</b> и <i>курсивом</i>."
    );
    assert_eq!(chs[0].paragraphs[1], "<b><i>Жирный курсив</i></b> вместе.");
    assert_eq!(chs[0].paragraphs[2], "<b>В</b> начале было Слово.");
    assert_eq!(chs[0].paragraphs[3], "Строка стиха с <b>жирным</b> знаком.");

    let spans = crate::document::parse_formatted_spans(&chs[0].paragraphs[2]);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "В");
    assert!(spans[0].bold);
    assert!(!spans[0].italic);
    assert_eq!(spans[1].text, " начале было Слово.");
    assert!(!spans[1].bold);
    assert!(!spans[1].italic);

    let clean = crate::document::strip_formatting_tags(&chs[0].paragraphs[0]);
    assert_eq!(clean, "Обычный текст с жирным словом и курсивом.");
}

#[test]
fn test_epub_formatting_bold_and_italic() {
    let html = r#"<html><body>
        <h1>Глава</h1>
        <p>Text with <b>bold</b> and <i>italic</i> and <strong>strong</strong> and <em>em</em>.</p>
        <p><b><i>Combined bold italic</i></b> text.</p>
    </body></html>"#;

    let (title, paragraphs, _) = crate::document::epub::parse_html_chapter(html);
    assert_eq!(title, "Глава");
    assert_eq!(paragraphs.len(), 3);
    assert_eq!(paragraphs[0], "Глава");
    assert_eq!(
        paragraphs[1],
        "Text with <b>bold</b> and <i>italic</i> and <b>strong</b> and <i>em</i>."
    );
    assert_eq!(paragraphs[2], "<b><i>Combined bold italic</i></b> text.");

    let (lenient_title, lenient_paras) = crate::document::epub::extract_html_lenient(html);
    assert_eq!(lenient_title, "");
    assert_eq!(lenient_paras.len(), 3);
    assert_eq!(
        lenient_paras[1],
        "Text with <b>bold</b> and <i>italic</i> and <b>strong</b> and <i>em</i>."
    );
}

#[test]
fn test_search_and_copy_with_formatted_text() {
    let xml = r#"<FictionBook><body>
        <section>
            <title><p>Раздел</p></title>
            <p>Здесь есть <strong>секретное</strong> слово.</p>
        </section>
    </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let hits = crate::core::ReaderCore::search_in_document(&doc, "секретное");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].snippet, "Здесь есть секретное слово.");

    let full_text = crate::core::ReaderCore::extract_chapter_text(&doc, 0).unwrap();
    assert_eq!(full_text, "Раздел\n\nЗдесь есть секретное слово.");
}

#[test]
fn test_parse_formatted_spans_edge_cases() {
    let text = "5 < 10 and 10 > 5 and <b>bold</b> and unclosed <i>italic";
    let spans = crate::document::parse_formatted_spans(text);
    assert_eq!(spans[0].text, "5 < 10 and 10 > 5 and ");
    assert!(!spans[0].bold && !spans[0].italic);
    assert_eq!(spans[1].text, "bold");
    assert!(spans[1].bold && !spans[1].italic);
    assert_eq!(spans[2].text, " and unclosed ");
    assert!(!spans[2].bold && !spans[2].italic);
    assert_eq!(spans[3].text, "italic");
    assert!(!spans[3].bold && spans[3].italic);

    let stripped = crate::document::strip_formatting_tags(text);
    assert_eq!(stripped, "5 < 10 and 10 > 5 and bold and unclosed italic");
}

#[test]
fn test_text_pages_calculation_and_progress() {
    let empty_chapters: Vec<crate::document::BookChapter> = Vec::new();
    let (cur, total, pct) = crate::app::bars::calculate_text_pages(&empty_chapters, 0, 0.0, 0.0);
    assert_eq!(cur, 1);
    assert_eq!(total, 1);
    assert_eq!(pct, 0.0);

    let ch1_text = "a".repeat(1800 * 200);
    let ch2_text = "b".repeat(1800 * 299);
    let chapters = vec![
        crate::document::BookChapter {
            id: None,
            title: String::new(),
            paragraphs: vec![ch1_text],
        },
        crate::document::BookChapter {
            id: None,
            title: String::new(),
            paragraphs: vec![ch2_text],
        },
    ];

    let (cur, total, pct) = crate::app::bars::calculate_text_pages(&chapters, 0, 0.0, 1000.0);
    assert_eq!(cur, 1);
    assert_eq!(total, 499);
    assert_eq!(pct, 0.0);

    let (cur, total, _) =
        crate::app::bars::calculate_text_pages(&chapters, 1, (100.0 / 299.0) * 1000.0, 1000.0);
    assert_eq!(total, 499);
    assert_eq!(cur, 301);

    let (cur, total, pct) = crate::app::bars::calculate_text_pages(&chapters, 1, 1000.0, 1000.0);
    assert_eq!(cur, 499);
    assert_eq!(total, 499);
    assert_eq!(pct, 100.0);
}

#[test]
fn test_twilight_evening_theme() {
    use crate::app::theme::ReaderTheme;
    use eframe::egui::Color32;

    assert!(ReaderTheme::Twilight.is_dark());
    assert_eq!(
        ReaderTheme::Twilight.bg_color(),
        Color32::from_rgb(54, 47, 43)
    );
    assert_eq!(
        ReaderTheme::Twilight.text_color(),
        Color32::from_rgb(222, 210, 192)
    );
    assert_eq!(
        ReaderTheme::Twilight.panel_bg_color(),
        Color32::from_rgb(66, 58, 53)
    );
    assert_eq!(
        ReaderTheme::Twilight.accent_color(),
        Color32::from_rgb(224, 150, 68)
    );
}

#[test]
fn test_text_search_multi_chapter_and_title() {
    use crate::core::ReaderCore;
    use crate::document::txt::TxtDocument;

    let text = "Chapter 1\nIntroductory text here\n\nChapter 2\nSecond section with target keyword\n\nSpecial Title\nFinal content";
    let doc = TxtDocument::from_text(text, "SearchDoc");

    let hits = ReaderCore::search_in_document(&doc, "target");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].chapter_index, 0);
    assert!(hits[0].snippet.contains("target"));

    let title_hits = ReaderCore::search_in_document(&doc, "SearchDoc");
    assert!(!title_hits.is_empty());
    assert_eq!(title_hits[0].paragraph_index, None);
}

#[test]
fn test_search_state_cycle_and_scroll_estimate() {
    use crate::app::search_view::SearchState;
    use crate::core::ReaderCore;
    use crate::document::txt::TxtDocument;

    let mut state = SearchState::default();
    assert!(!state.is_open);
    state.open();
    assert!(state.is_open);
    assert!(state.should_focus);

    let doc = TxtDocument::from_text("Alpha\n\nBeta\n\nGamma\n\nDelta", "Doc");
    let y0 = ReaderCore::estimate_paragraph_scroll_y(&doc, 0, None, 18.0);
    assert_eq!(y0, 0.0);

    let y_para = ReaderCore::estimate_paragraph_scroll_y(&doc, 0, Some(2), 18.0);
    assert!(y_para >= 0.0);

    state.hits = ReaderCore::search_in_document(&doc, "Beta");
    state
        .hits
        .extend(ReaderCore::search_in_document(&doc, "Delta"));
    assert_eq!(state.hits.len(), 2);
    let first = state.current_hit().cloned();
    let second = state.next_hit().cloned();
    assert_ne!(first, second);
    let wrapped = state.prev_hit().cloned();
    assert_eq!(first, wrapped);
}

#[test]
fn test_make_safe_snippet_utf8_boundaries() {
    use crate::core::ReaderCore;

    let cyrillic = "Строка с кириллицей и множеством многобайтовых символов UTF-8 для проверки";
    for i in 0..cyrillic.len() {
        let snippet = ReaderCore::make_safe_snippet(cyrillic, i, 4);
        assert!(!snippet.is_empty());
    }
}

#[test]
fn test_light_themes_button_borders() {
    use crate::app::theme::ReaderTheme;

    let light_themes = [
        ReaderTheme::Light,
        ReaderTheme::Sepia,
        ReaderTheme::Warm,
        ReaderTheme::Mint,
    ];

    for theme in light_themes {
        assert!(!theme.is_dark());
        assert!(theme.button_stroke_color().is_none());

        let visuals = theme.create_visuals();
        assert_eq!(visuals.widgets.inactive.bg_stroke.width, 0.0);
        assert_eq!(visuals.widgets.inactive.fg_stroke.color, theme.text_color());
        assert_eq!(visuals.widgets.hovered.fg_stroke.color, theme.text_color());
        assert_eq!(visuals.widgets.active.fg_stroke.color, theme.text_color());
        assert_eq!(visuals.override_text_color, Some(theme.text_color()));
    }

    let dark_themes = [ReaderTheme::Dark, ReaderTheme::Oled, ReaderTheme::Twilight];

    for theme in dark_themes {
        assert!(theme.is_dark());
        assert!(theme.button_stroke_color().is_none());
    }
}

#[test]
fn test_tts_engine_init() {
    let tts_res = tts::Tts::default();
    if let Ok(mut tts) = tts_res {
        let features = tts.supported_features();
        let _ = tts.normal_rate();
        let _ = tts.min_rate();
        let _ = tts.max_rate();
        let _ = tts.voices();
        if features.utterance_callbacks {
            let _ = tts.on_utterance_end(Some(Box::new(|u| {
                let _ = u;
            })));
        }
        let _ = tts.stop();
    }
}

#[test]
fn test_pdf_reading_order_two_columns_and_gutter() {
    use crate::document::pdf::text::{GlyphEntry, group_and_sort_glyphs};
    let mut glyphs = Vec::new();
    for (i, c) in "Left1".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 50.0 + i as f64 * 10.0,
            y: 300.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Right1".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 450.0 + i as f64 * 10.0,
            y: 300.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Left2".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 50.0 + i as f64 * 10.0,
            y: 350.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Right2".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 450.0 + i as f64 * 10.0,
            y: 350.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Left3".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 50.0 + i as f64 * 10.0,
            y: 400.0,
            font_size: 14.0,
            width: 10.0,
        });
    }
    for (i, c) in "Right3".chars().enumerate() {
        glyphs.push(GlyphEntry {
            c,
            x: 450.0 + i as f64 * 10.0,
            y: 400.0,
            font_size: 14.0,
            width: 10.0,
        });
    }

    let spans = group_and_sort_glyphs(glyphs, 800.0, 1000.0);
    assert_eq!(spans.len(), 6);
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left1", "Left2", "Left3", "Right1", "Right2", "Right3"]
    );
}

#[test]
fn test_pdf_outline_resolve_dest_clamping() {
    use hayro::hayro_syntax::object::{Array, FromBytes};
    use std::collections::HashMap;

    let dest_data = b"[ 5 /XYZ 0 792 1.0 ]";
    if let Some(arr) = Array::from_bytes(dest_data) {
        let page_map = HashMap::new();
        let page_count = 3;
        let resolved =
            crate::document::pdf::outline::resolve_dest_array(&arr, &page_map, page_count);
        assert_eq!(resolved, Some(2));
    }
}

#[test]
fn test_edge_tts_synth() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = msedge_tts::tts::SpeechConfig {
        voice_name: "ru-RU-SvetlanaNeural".to_string(),
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: 0,
        rate: 0,
        volume: 0,
    };
    if let Ok(mut client) = msedge_tts::tts::client::connect() {
        if let Ok(audio) = client.synthesize("Привет", &config) {
            assert!(!audio.audio_bytes.is_empty());
            let cursor = std::io::Cursor::new(audio.audio_bytes);
            let decoder = rodio::Decoder::new(cursor);
            assert!(decoder.is_ok());
        }
    }
}

#[test]
fn test_tts_controller_engine_and_rate_switching() {
    use crate::app::tts::{TtsController, TtsEngineKind};

    let mut controller = TtsController::new();
    let initial_status = controller.status();
    assert_eq!(initial_status.rate, 1.0);

    controller.set_rate(1.5);
    assert_eq!(controller.status().rate, 1.5);

    controller.set_engine(TtsEngineKind::Edge);
    assert_eq!(controller.status().engine, TtsEngineKind::Edge);

    controller.set_voice("ru-RU-DmitryNeural".to_string());
    assert_eq!(
        controller.status().selected_voice_id.as_deref(),
        Some("ru-RU-DmitryNeural")
    );

    controller.set_voice("ru-RU-SvetlanaNeural".to_string());
    assert_eq!(
        controller.status().selected_voice_id.as_deref(),
        Some("ru-RU-SvetlanaNeural")
    );

    controller.set_engine(TtsEngineKind::System);
    assert_eq!(controller.status().engine, TtsEngineKind::System);
}

#[test]
fn test_language_detection_and_voice_matching() {
    use crate::app::tts::voice_matches_lang;
    use crate::core::ReaderCore;

    let ru_text = "Здоровье моего тела всегда улучшается и обновляется с каждым новым днем.";
    assert_eq!(ReaderCore::detect_language_from_text(ru_text), "ru");

    let uk_text = "Здоров'я мого тіла завжди покращується та оновлюється з кожним новим днем.";
    assert_eq!(ReaderCore::detect_language_from_text(uk_text), "uk");

    let en_text = "The quick brown fox jumps over the lazy dog and that is with great joy.";
    assert_eq!(ReaderCore::detect_language_from_text(en_text), "en");

    let de_text = "Die Gesundheit meines Körpers verbessert sich und das ist wunderbar für mich.";
    assert_eq!(ReaderCore::detect_language_from_text(de_text), "de");

    let fr_text =
        "La santé de mon corps s'améliore toujours avec le temps et c'est pour une vie saine.";
    assert_eq!(ReaderCore::detect_language_from_text(fr_text), "fr");

    let es_text =
        "La salud de mi cuerpo siempre mejora y se renueva con los días para vivir mejor.";
    assert_eq!(ReaderCore::detect_language_from_text(es_text), "es");

    let zh_text = "阅读能够开阔视野并让我们体验不同的人生与思考方式。";
    assert_eq!(ReaderCore::detect_language_from_text(zh_text), "zh");

    let ja_text = "本を読むことは心を豊かにし、新しい世界を広げてくれます。";
    assert_eq!(ReaderCore::detect_language_from_text(ja_text), "ja");

    let ar_text = "القراءة تفتح آفاقاً جديدة وتغذي العقل بالمعرفة والحكمة.";
    assert_eq!(ReaderCore::detect_language_from_text(ar_text), "ar");

    let it_text = "La lettura della vita è anche questo viaggio magnifico con gli amici.";
    assert_eq!(ReaderCore::detect_language_from_text(it_text), "it");

    let pt_text = "A leitura não é apenas um hábito, mas uma paixão para toda a vida.";
    assert_eq!(ReaderCore::detect_language_from_text(pt_text), "pt");

    let tr_text = "Kitap okumak yeni dünyalar keşfetmek için bir kapı açar ve çok güzeldir.";
    assert_eq!(ReaderCore::detect_language_from_text(tr_text), "tr");

    let id_text = "Membaca buku adalah jendela dunia yang membuka pikiran dan wawasan kita.";
    assert_eq!(ReaderCore::detect_language_from_text(id_text), "id");

    assert!(voice_matches_lang("ru-RU", "ru"));
    assert!(voice_matches_lang("Russian", "ru"));
    assert!(voice_matches_lang("en-US", "en"));
    assert!(voice_matches_lang("English", "en"));
    assert!(voice_matches_lang("uk-UA", "uk"));
    assert!(voice_matches_lang("de-DE", "de"));
    assert!(voice_matches_lang("fr-FR", "fr"));
    assert!(voice_matches_lang("es-ES", "es"));
    assert!(voice_matches_lang("zh-CN", "zh"));
    assert!(voice_matches_lang("ja-JP", "ja"));
    assert!(voice_matches_lang("ar-SA", "ar"));
    assert!(voice_matches_lang("it-IT", "it"));
    assert!(voice_matches_lang("pt-BR", "pt"));
    assert!(voice_matches_lang("tr-TR", "tr"));
    assert!(voice_matches_lang("id-ID", "id"));

    assert!(!voice_matches_lang("ru-RU", "en"));
    assert!(!voice_matches_lang("en-US", "ru"));
    assert!(!voice_matches_lang("de-DE", "ru"));
}

#[test]
fn test_hotkeys_b_v_j() {
    use crate::app::input::process_keyboard_events;
    use crate::core::Command;
    use eframe::egui::{Key, Modifiers};

    let cmds_b = process_keyboard_events(false, |k| k == Key::B, Modifiers::default(), false);
    assert_eq!(cmds_b, vec![Command::ToggleToc]);

    let cmds_v = process_keyboard_events(false, |k| k == Key::V, Modifiers::default(), false);
    assert_eq!(cmds_v, vec![Command::ToggleTTS]);

    let cmds_j = process_keyboard_events(false, |k| k == Key::J, Modifiers::default(), false);
    assert_eq!(cmds_j, vec![Command::ToggleSettings]);

    let mut ctrl_mod = Modifiers::default();
    ctrl_mod.ctrl = true;
    let cmds_ctrl_v = process_keyboard_events(false, |k| k == Key::V, ctrl_mod, false);
    assert!(cmds_ctrl_v.is_empty());

    let cmds_typing = process_keyboard_events(
        true,
        |k| k == Key::B || k == Key::V || k == Key::J,
        Modifiers::default(),
        false,
    );
    assert!(cmds_typing.is_empty());
}

#[test]
fn test_hotkey_l_reading_ruler() {
    use crate::app::input::process_keyboard_events;
    use crate::core::Command;
    use eframe::egui::{Key, Modifiers};

    let cmds_l = process_keyboard_events(false, |k| k == Key::L, Modifiers::default(), false);
    assert_eq!(cmds_l, vec![Command::ToggleReadingRuler]);

    let mut ctrl_mod = Modifiers::default();
    ctrl_mod.ctrl = true;
    let cmds_ctrl_l = process_keyboard_events(false, |k| k == Key::L, ctrl_mod, false);
    assert!(cmds_ctrl_l.is_empty());

    let cmds_typing = process_keyboard_events(true, |k| k == Key::L, Modifiers::default(), false);
    assert!(cmds_typing.is_empty());
}

#[test]
fn test_reading_ruler_settings_and_translations() {
    use crate::i18n::UiLanguage;
    use crate::settings::AppSettings;

    let settings = AppSettings::default();
    assert!(!settings.reading_ruler);
    assert_eq!(settings.reading_ruler_lines, 3);
    assert!((settings.reading_ruler_opacity - 0.45).abs() < f32::EPSILON);

    for lang in UiLanguage::CONCRETE {
        let t = lang.t();
        assert!(!t.reading_ruler.is_empty());
        assert!(!t.hide_reading_ruler.is_empty());
        assert!(!t.reading_ruler_tooltip.is_empty());
        assert!(!t.reading_ruler_title.is_empty());
        assert!(!t.reading_ruler_desc.is_empty());
        assert!(!t.reading_ruler_height_label.is_empty());
        assert!(!t.reading_ruler_dim_label.is_empty());
    }
}
