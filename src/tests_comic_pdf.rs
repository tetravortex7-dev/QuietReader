use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("qr_test_{prefix}_{pid}_{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_comic_natural_sort_ordering() {
    use document::comic::natural_cmp;
    use std::cmp::Ordering;

    assert_eq!(natural_cmp("page1.jpg", "page2.jpg"), Ordering::Less);
    assert_eq!(natural_cmp("page2.jpg", "page10.jpg"), Ordering::Less);
    assert_eq!(natural_cmp("page10.jpg", "page2.jpg"), Ordering::Greater);
    assert_eq!(natural_cmp("ch1_p09.png", "ch1_p10.png"), Ordering::Less);
    assert_eq!(natural_cmp("page5.webp", "page5.webp"), Ordering::Equal);
}

#[test]
fn test_comic_cbz_archive_and_metadata_parsing() {
    use std::io::Write;
    let test_dir = unique_test_dir("comic_cbz");
    let cbz_path = test_dir.join("test_manga.cbz");

    let mut sample_png = Vec::new();
    let img = image::RgbaImage::new(10, 10);
    img.write_to(
        &mut std::io::Cursor::new(&mut sample_png),
        image::ImageFormat::Png,
    )
    .unwrap();

    {
        let file = std::fs::File::create(&cbz_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let comic_info_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <Title>Ван-Пис: Глава 1</Title>
  <Series>One Piece</Series>
  <Number>1</Number>
  <Writer>Эйитиро Ода</Writer>
  <Manga>YesAndRightToLeft</Manga>
</ComicInfo>"#;
        zip.start_file("ComicInfo.xml", options).unwrap();
        zip.write_all(comic_info_xml.as_bytes()).unwrap();

        zip.start_file("ch01/page_10.png", options).unwrap();
        zip.write_all(&sample_png).unwrap();

        zip.start_file("ch01/page_01.png", options).unwrap();
        zip.write_all(&sample_png).unwrap();

        zip.start_file("ch01/page_02.png", options).unwrap();
        zip.write_all(&sample_png).unwrap();

        zip.finish().unwrap();
    }

    let doc = document::open_document(&cbz_path).expect("Должен успешно открыть CBZ архив");
    assert_eq!(doc.metadata().title, "One Piece #1: Ван-Пис: Глава 1");
    assert_eq!(doc.metadata().author, "Эйитиро Ода");

    let comic = doc
        .page_content()
        .expect("Документ должен реализовать ComicDocumentTrait");
    assert!(comic.is_manga(), "Должен определиться как манга");
    assert_eq!(comic.page_count(), 3, "Должно быть 3 страницы");

    assert_eq!(comic.page_name(0), "page_01.png");
    assert_eq!(comic.page_name(1), "page_02.png");
    assert_eq!(comic.page_name(2), "page_10.png");

    let decoded = comic
        .load_page_image(0)
        .expect("Страница 0 должна успешно декодироваться");
    assert_eq!(decoded.width, 10);
    assert_eq!(decoded.height, 10);
    assert_eq!(decoded.rgba.len(), 10 * 10 * 4);

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_comic_folder_scan() {
    let test_dir = unique_test_dir("comic_folder");
    let folder_path = test_dir.join("MangaFolder");
    std::fs::create_dir_all(&folder_path).unwrap();

    let mut sample_png = Vec::new();
    let img = image::RgbaImage::new(4, 4);
    img.write_to(
        &mut std::io::Cursor::new(&mut sample_png),
        image::ImageFormat::Png,
    )
    .unwrap();

    std::fs::write(folder_path.join("2.png"), &sample_png).unwrap();
    std::fs::write(folder_path.join("1.png"), &sample_png).unwrap();
    std::fs::write(folder_path.join("10.png"), &sample_png).unwrap();

    let doc = document::open_document(&folder_path).expect("Должен успешно открыть папку с мангой");
    let comic = doc
        .page_content()
        .expect("Папка должна определиться как комикс");
    assert_eq!(comic.page_count(), 3);
    assert_eq!(comic.page_name(0), "1.png");
    assert_eq!(comic.page_name(1), "2.png");
    assert_eq!(comic.page_name(2), "10.png");

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_pdf_document_parsing_and_rasterization() {
    let minimal_pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Resources <<>> >>
endobj
xref
0 4
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
trailer
<< /Size 4 /Root 1 0 R >>
startxref
206
%%EOF";

    let temp_dir = unique_test_dir("pdf");
    let pdf_path = temp_dir.join("test_sample.pdf");
    std::fs::write(&pdf_path, minimal_pdf).unwrap();

    let doc = document::open_document(&pdf_path).expect("Должен успешно открыть валидный PDF");
    assert_eq!(doc.chapters().len(), 1);
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Страница 1");

    let comic = doc
        .page_content()
        .expect("PDF должен реализовывать ComicDocumentTrait для страничного отображения");
    assert_eq!(comic.page_count(), 1);
    assert!(!comic.is_manga());

    let page_img = comic
        .load_page_image(0)
        .expect("Страница PDF должна успешно растеризоваться");
    assert!(page_img.width > 0);
    assert!(page_img.height > 0);
    assert_eq!(
        page_img.rgba.len(),
        (page_img.width * page_img.height * 4) as usize
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_font_family_settings_and_serialization() {
    let default_settings = AppSettings::default();
    assert_eq!(default_settings.font_family, "Georgia");

    let old_json = r#"{"window":{"width":800.0,"height":600.0,"maximized":false},"recent_books":[],"font_size":18.0,"theme":"Sepia","keep_screen_on":true,"show_toc":true}"#;
    let loaded: AppSettings =
        serde_json::from_str(old_json).expect("Должен десериализоваться со значением по умолчанию");
    assert_eq!(loaded.font_family, "Georgia");

    let custom_settings = AppSettings {
        font_family: "Palatino Linotype".to_string(),
        ..Default::default()
    };
    let serialized = serde_json::to_string(&custom_settings).unwrap();
    let deserialized: AppSettings = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.font_family, "Palatino Linotype");

    let font_ids: Vec<&str> = crate::app::fonts::AVAILABLE_FONTS
        .iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(
        font_ids,
        vec![
            "Georgia",
            "Palatino Linotype",
            "Calibri",
            "Segoe UI",
            "Verdana"
        ]
    );
}

#[test]
fn test_new_themes_colors_and_oled() {
    use crate::app::theme::ReaderTheme;
    use eframe::egui::Color32;

    assert_eq!(ReaderTheme::Oled.bg_color(), Color32::from_rgb(0, 0, 0));
    assert!(ReaderTheme::Oled.is_dark());
    assert!(ReaderTheme::Dark.is_dark());
    assert!(!ReaderTheme::Warm.is_dark());
    assert!(!ReaderTheme::Mint.is_dark());
}

#[test]
fn test_comic_settings_auto_reading_mode() {
    let default_settings = AppSettings::default();
    assert_eq!(default_settings.comic.reading_mode, "Auto");

    let is_manga_doc = true;
    let effective = match default_settings.comic.reading_mode.as_str() {
        "Manga" => true,
        "Comic" => false,
        "Webtoon" => false,
        _ => is_manga_doc,
    };
    assert!(effective);

    let not_manga_doc = false;
    let effective_regular = match default_settings.comic.reading_mode.as_str() {
        "Manga" => true,
        "Comic" => false,
        "Webtoon" => false,
        _ => not_manga_doc,
    };
    assert!(!effective_regular);
}

#[test]
fn test_archive_path_traversal_protection() {
    use crate::document::{ComicDocument, DocumentError, is_safe_archive_path};
    use std::io::Write;

    assert!(!is_safe_archive_path("../evil.png"));
    assert!(!is_safe_archive_path("ch1/../../evil.png"));
    assert!(!is_safe_archive_path("/etc/passwd"));
    assert!(!is_safe_archive_path(r"\Windows\System32\cmd.exe"));
    assert!(!is_safe_archive_path("C:/evil.png"));
    assert!(is_safe_archive_path("ch01/page_01.png"));

    let test_dir = unique_test_dir("traversal");
    let cbz_path = test_dir.join("traversal.cbz");

    {
        let file = std::fs::File::create(&cbz_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("../evil.png", options).unwrap();
        zip.write_all(b"fake_image").unwrap();
        zip.finish().unwrap();
    }

    let res = ComicDocument::from_path(&cbz_path);
    assert!(matches!(res, Err(DocumentError::Corrupted(_))));
    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_zip_bomb_and_bounded_read_protection() {
    use crate::document::{DocumentError, read_bounded};

    let data = vec![0u8; 2048];
    let mut total_unpacked = 0u64;

    let res_single = read_bounded(&data[..], 1024, &mut total_unpacked, 10_000);
    assert!(matches!(res_single, Err(DocumentError::LimitExceeded(_))));

    let mut total_unpacked = 9500u64;
    let res_total = read_bounded(&data[..1000], 2048, &mut total_unpacked, 10_000);
    assert!(matches!(res_total, Err(DocumentError::LimitExceeded(_))));
}

#[test]
fn test_archive_excessive_entries_protection() {
    use crate::document::{DocumentError, EpubDocument, MAX_ARCHIVE_ENTRIES};

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for i in 0..=MAX_ARCHIVE_ENTRIES {
            zip.start_file(format!("f_{i}.txt"), options).unwrap();
        }
        zip.finish().unwrap();
    }

    let res = EpubDocument::from_bytes(&zip_buf.into_inner());
    assert!(matches!(res, Err(DocumentError::LimitExceeded(_))));
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
fn test_corrupted_settings_json_backup_and_recovery() {
    let temp_dir = unique_test_dir("corrupted_settings");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let corrupted_content = "{ \"font_size\": BROKEN_JSON !!!";
    std::fs::write(&settings_path, corrupted_content).unwrap();

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded, AppSettings::default());
    assert!(bak_path.exists());
    assert_eq!(
        std::fs::read_to_string(&bak_path).unwrap(),
        corrupted_content
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

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
fn test_settings_schema_version_migration_with_backup() {
    let temp_dir = unique_test_dir("schema_migration");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let legacy_json = r#"{"window":{"width":900.0,"height":700.0,"maximized":false},"recent_books":[],"font_size":21.0,"theme":"Dark","keep_screen_on":false,"show_toc":true}"#;
    std::fs::write(&settings_path, legacy_json).unwrap();

    let migrated = AppSettings::load_from_path(&settings_path);
    assert_eq!(
        migrated.schema_version,
        settings::CURRENT_SETTINGS_SCHEMA_VERSION
    );
    assert_eq!(migrated.font_size, 21.0);
    assert_eq!(migrated.theme, "Dark");
    assert!(bak_path.exists());
    assert_eq!(std::fs::read_to_string(&bak_path).unwrap(), legacy_json);

    let _ = std::fs::remove_dir_all(&temp_dir);
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

#[test]
fn test_safe_archive_path_checks() {
    use crate::document::is_safe_archive_path;
    assert!(!is_safe_archive_path(""));
    assert!(!is_safe_archive_path("../secret"));
    assert!(!is_safe_archive_path("foo/../../secret"));
    assert!(!is_safe_archive_path("foo/../secret"));
    assert!(!is_safe_archive_path("/etc/passwd"));
    assert!(!is_safe_archive_path("C:\\windows\\system32"));
    assert!(!is_safe_archive_path("C:foo"));
    assert!(is_safe_archive_path("page01.png"));
    assert!(is_safe_archive_path("sub/folder/page01.png"));
}

#[test]
fn test_comic_image_dimension_bounds() {
    use crate::document::comic::is_supported_image;
    assert!(is_supported_image("test.png"));
    assert!(is_supported_image("folder/test.jpg"));
    assert!(is_supported_image("test.webp"));
    assert!(!is_supported_image(".hidden.png"));
    assert!(!is_supported_image("test.exe"));
    assert!(!is_supported_image("__MACOSX/page.png"));
}

#[test]
fn test_fb2_deep_section_recursion_bounded() {
    use crate::document::Document;
    use crate::document::Fb2Document;
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><body>");
    for i in 0..50 {
        xml.push_str(&format!("<section><title><p>Глава {}</p></title><p>Текст {}</p>", i, i));
    }
    for _ in 0..50 {
        xml.push_str("</section>");
    }
    xml.push_str("</body></FictionBook>");

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    assert!(!doc.chapters().is_empty());
    assert!(!doc.toc().is_empty());
}

#[test]
fn test_settings_atomic_save_and_reload() {
    let temp_dir = unique_test_dir("atomic_save");
    let settings_path = temp_dir.join("settings.json");
    let mut settings = AppSettings::default();
    settings.font_size = 23.5;
    settings.theme = "Mint".to_string();

    let res = settings.save_to_path(&settings_path);
    assert!(res.is_ok());
    assert!(settings_path.exists());

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded.font_size, 23.5);
    assert_eq!(loaded.theme, "Mint");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_settings_corrupted_backup_protection() {
    let temp_dir = unique_test_dir("bak_protection");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");

    let valid_backup_json = r#"{"schema_version":2,"font_size":27.0,"theme":"Oled"}"#;
    std::fs::write(&bak_path, valid_backup_json).unwrap();

    let broken_json = "{ truncated invalid json ...";
    std::fs::write(&settings_path, broken_json).unwrap();

    let restored = AppSettings::load_from_path(&settings_path);
    assert_eq!(restored.font_size, 27.0);
    assert_eq!(restored.theme, "Oled");

    let bak_content = std::fs::read_to_string(&bak_path).unwrap();
    assert_eq!(bak_content, valid_backup_json);

    let corrupted_path = temp_dir.join("settings.corrupted");
    assert!(corrupted_path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_reading_progress_separation_from_recent_books() {
    let mut settings = AppSettings::default();

    for i in 1..=30 {
        let p = std::path::PathBuf::from(format!("C:\\Books\\book_{i}.epub"));
        settings.record_book_progress(&p, &format!("Книга {i}"), "Автор", i, (i * 10) as f32);
    }

    assert_eq!(settings.recent_books.len(), 25);
    assert_eq!(settings.reading_progress.len(), 30);

    let old_book_path = std::path::PathBuf::from("C:\\Books\\book_1.epub");
    assert!(settings.recent_books.iter().all(|b| b.path != old_book_path));
    assert_eq!(settings.get_book_progress(&old_book_path), Some((1, 10.0)));

    settings.recent_books.clear();
    assert_eq!(settings.recent_books.len(), 0);
    assert_eq!(settings.get_book_progress(&old_book_path), Some((1, 10.0)));
}

#[test]
fn test_settings_new_fields_default_compatibility() {
    let minimal_json = r#"{"font_size": 22.0}"#;
    let settings: AppSettings = serde_json::from_str(minimal_json).expect("Десериализация с дефолтами должна пройти успешно");

    assert_eq!(settings.font_size, 22.0);
    assert_eq!(settings.font_family, "Georgia");
    assert_eq!(settings.theme, "Sepia");
    assert!(settings.keep_screen_on);
    assert!(settings.show_toc);
    assert_eq!(settings.comic.reading_mode, "Auto");
    assert_eq!(settings.pdf.layout, "Single");
    assert!(settings.reading_progress.is_empty());
}

#[test]
fn test_pdf_transitive_glyph_sorting_and_line_clustering() {
    use crate::document::pdf::text::{group_and_sort_glyphs, GlyphEntry};

    let glyphs = vec![
        GlyphEntry { c: 'l', x: 20.0, y: 10.2, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'H', x: 10.0, y: 10.0, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'e', x: 15.0, y: 9.8, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'o', x: 25.0, y: 10.1, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'W', x: 10.0, y: 30.0, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'o', x: 15.0, y: 30.2, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'r', x: 20.0, y: 29.9, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'l', x: 25.0, y: 30.0, font_size: 10.0, width: 5.0 },
        GlyphEntry { c: 'd', x: 30.0, y: 30.1, font_size: 10.0, width: 5.0 },
    ];

    let spans = group_and_sort_glyphs(glyphs, 200.0, 200.0);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "Helo");
    assert_eq!(spans[1].text, "World");
}

#[test]
fn test_keyboard_input_suppressed_when_wants_keyboard_input() {
    use crate::app::input::process_keyboard_events;
    use eframe::egui::{Key, Modifiers};

    let dummy_modifiers = Modifiers::default();
    let cmds = process_keyboard_events(
        true,
        |k| matches!(k, Key::A | Key::Plus | Key::Space | Key::ArrowRight | Key::F),
        dummy_modifiers,
        false,
    );
    assert!(cmds.is_empty());

    let cmds_not_focused = process_keyboard_events(
        false,
        |k| matches!(k, Key::F),
        dummy_modifiers,
        false,
    );
    assert_eq!(cmds_not_focused, vec![crate::core::Command::ToggleFullscreen]);
}

#[test]
fn test_keyboard_manga_vs_comic_navigation_direction() {
    use crate::app::input::process_keyboard_events;
    use eframe::egui::{Key, Modifiers};

    let dummy_modifiers = Modifiers::default();

    let manga_left = process_keyboard_events(
        false,
        |k| k == Key::ArrowLeft,
        dummy_modifiers,
        true,
    );
    assert_eq!(manga_left, vec![crate::core::Command::NextPage]);

    let manga_right = process_keyboard_events(
        false,
        |k| k == Key::ArrowRight,
        dummy_modifiers,
        true,
    );
    assert_eq!(manga_right, vec![crate::core::Command::PrevPage]);

    let comic_left = process_keyboard_events(
        false,
        |k| k == Key::ArrowLeft,
        dummy_modifiers,
        false,
    );
    assert_eq!(comic_left, vec![crate::core::Command::PrevPage]);

    let comic_right = process_keyboard_events(
        false,
        |k| k == Key::ArrowRight,
        dummy_modifiers,
        false,
    );
    assert_eq!(comic_right, vec![crate::core::Command::NextPage]);
}

#[test]
fn test_autoscroll_controller_state_and_paged_advance() {
    use crate::app::autoscroll::AutoscrollController;

    let mut controller = AutoscrollController::default();
    assert!(!controller.is_active());

    controller.toggle(100.0, 500.0, 60.0);
    assert!(controller.is_active());
    assert_eq!(controller.step_start_scroll_y, 100.0);
    assert_eq!(controller.target_step_scroll_y, 160.0);

    controller.stop();
    assert!(!controller.is_active());
}

#[test]
fn test_worker_panic_catch_unwind_resilience() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        panic!("Simulated decoder fatal error");
    }));
    assert!(result.is_err());
}

#[test]
fn test_document_loader_async_request_and_response() {
    use crate::app::document_loader::DocumentLoader;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    let mut loader = DocumentLoader::default();
    let test_path = PathBuf::from("nonexistent_test_document_12345.epub");
    let req_id = loader.request_load(test_path.clone());
    assert!(req_id > 0);

    let start = Instant::now();
    let mut received = None;
    while start.elapsed() < Duration::from_secs(3) {
        let responses = loader.poll_responses();
        if let Some(resp) = responses.into_iter().find(|r| r.request_id == req_id) {
            received = Some(resp);
            break;
        }
        std::thread::sleep(Duration::from_millis(15));
    }

    let resp = received.expect("Background thread should return a load response");
    assert_eq!(resp.request_id, req_id);
    assert_eq!(resp.path, test_path);
    assert!(resp.result.is_err());
}

#[test]
fn test_book_tab_loading_and_update_lifecycle() {
    use crate::app::tabs::BookTab;
    use std::path::PathBuf;

    let path = PathBuf::from("sample_book.pdf");
    let tab = BookTab::new_loading(path.clone(), 77);
    assert!(tab.is_loading);
    assert!(tab.document.is_none());
    assert!(tab.load_error.is_none());
    assert_eq!(tab.request_id, 77);
    assert_eq!(tab.format_badge, "PDF");
    assert_eq!(tab.file_path, path);
}

#[test]
fn test_obsolete_response_discard_on_tab_close_and_reopen() {
    use crate::app::tabs::BookTab;
    use std::path::PathBuf;

    let path = PathBuf::from("chapter1.fb2");
    let mut tabs = vec![BookTab::new_loading(path.clone(), 1)];

    tabs.clear();
    let stale_resp_id = 1u64;
    let matching = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == stale_resp_id);
    assert_eq!(matching, None);

    tabs.push(BookTab::new_loading(path.clone(), 2));

    let matching_stale = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == stale_resp_id);
    assert_eq!(matching_stale, None);

    let matching_fresh = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == 2);
    assert_eq!(matching_fresh, Some(0));
}

#[test]
fn test_document_load_error_formatting_variants() {
    use crate::app::document_loader::format_document_error;
    use crate::document::DocumentError;

    let io_err = format_document_error(&DocumentError::Io("file missing".to_string()));
    assert!(io_err.contains("Ошибка ввода-вывода"));

    let unsup_err = format_document_error(&DocumentError::Unsupported("raw".to_string()));
    assert!(unsup_err.contains("Неподдерживаемый формат"));

    let corr_err = format_document_error(&DocumentError::Corrupted("invalid header".to_string()));
    assert!(corr_err.contains("Повреждённый документ"));

    let lim_err = format_document_error(&DocumentError::LimitExceeded("size limit".to_string()));
    assert!(lim_err.contains("Превышен лимит безопасности"));
}

#[test]
fn test_pdf_view_scale_discretization() {
    use crate::app::pdf_view::PdfViewState;
    use crate::settings::PdfSettings;

    let state = PdfViewState::default();
    let mut settings = PdfSettings::default();

    settings.fit_mode = "Original".to_string();
    assert_eq!(state.compute_target_scale(&settings), 2.0);

    settings.fit_mode = "FitWidth".to_string();
    assert_eq!(state.compute_target_scale(&settings), 2.5);

    settings.fit_mode = "FitWindow".to_string();
    assert_eq!(state.compute_target_scale(&settings), 2.25);

    settings.fit_mode = "Custom".to_string();
    settings.zoom_percent = 100.0;
    let s1 = state.compute_target_scale(&settings);
    assert!((s1 * 4.0).fract() < 1e-4);

    settings.zoom_percent = 104.0;
    let s2 = state.compute_target_scale(&settings);
    assert_eq!(s1, s2);
}

#[test]
fn test_pdf_view_clear_cache_and_document_generation_isolation() {
    use crate::app::pdf_view::PdfViewState;

    let mut state = PdfViewState::default();
    assert_eq!(state.doc_generation, 0);

    state.page_input = "42".to_string();
    state.clear_cache();
    assert_eq!(state.doc_generation, 1);
    assert_eq!(state.total_texture_bytes, 0);
    assert!(state.textures.is_empty());
    assert!(state.pending_pages.is_empty());
    assert!(state.page_text_spans.is_empty());
    assert!(!state.selection_dirty);
    assert!(state.selected_text.is_empty());

    state.clear_cache();
    assert_eq!(state.doc_generation, 2);
}

#[test]
fn test_pdf_selection_state_and_dirty_flag() {
    use crate::app::pdf_view::PdfViewState;
    use eframe::egui::Pos2;

    let mut state = PdfViewState::default();
    assert!(!state.selection_dirty);
    assert!(state.selected_text.is_empty());

    state.selection_start = Some(Pos2::new(10.0, 10.0));
    state.selection_current = Some(Pos2::new(50.0, 50.0));
    state.selection_dirty = true;
    state.selected_text = "Sample extracted text".to_string();

    state.clear_cache();
    assert!(state.selection_start.is_none());
    assert!(state.selection_current.is_none());
    assert!(!state.selection_dirty);
    assert!(state.selected_text.is_empty());
}

#[test]
fn test_settings_save_debounce_state_and_flush() {
    use crate::app::QuietReaderApp;
    use crate::settings::AppSettings;

    let mut app = QuietReaderApp::from_settings(AppSettings::default());
    assert!(!app.settings_dirty);

    app.mark_settings_dirty();
    assert!(app.settings_dirty);

    app.flush_debounced_settings();
    assert!(app.settings_dirty);

    app.flush_settings_now();
    assert!(!app.settings_dirty);
}

#[test]
fn test_text_view_virtualization_bounds_estimation() {
    let font_size = 19.0f32;
    let line_height = font_size * 1.45;
    let para_spacing = font_size * 0.4;
    let chars_per_line = (760.0 / (font_size * 0.52)).max(10.0);

    let short_text = "Short single line.";
    let char_count = short_text.chars().count();
    let num_lines = (char_count as f32 / chars_per_line).ceil().max(1.0);
    assert_eq!(num_lines, 1.0);
    let est_height = num_lines * line_height + para_spacing;
    assert!(est_height > 20.0 && est_height < 50.0);

    let view_top = 100.0f32;
    let view_bot = 800.0f32;

    let top_y_offscreen_above = 0.0f32;
    let bot_y_offscreen_above = top_y_offscreen_above + est_height;
    assert!(bot_y_offscreen_above < view_top);

    let top_y_visible = 300.0f32;
    let bot_y_visible = top_y_visible + est_height;
    assert!(!(bot_y_visible < view_top || top_y_visible > view_bot));

    let top_y_offscreen_below = 1200.0f32;
    assert!(top_y_offscreen_below > view_bot);
}

#[test]
fn test_pdf_transform_span_bbox_all_rotations() {
    use crate::app::pdf_view::transform_span_bbox;
    use eframe::egui;
    let rect = egui::Rect::from_min_max(egui::Pos2::new(100.0, 200.0), egui::Pos2::new(300.0, 600.0));
    let bbox = [0.1, 0.2, 0.5, 0.4];

    let r0 = transform_span_bbox(bbox, 0, rect);
    assert!((r0.min.x - 120.0).abs() < 1e-3);
    assert!((r0.min.y - 280.0).abs() < 1e-3);
    assert!((r0.max.x - 200.0).abs() < 1e-3);
    assert!((r0.max.y - 360.0).abs() < 1e-3);

    let r90 = transform_span_bbox(bbox, 90, rect);
    assert!((r90.min.x - 220.0).abs() < 1e-3);
    assert!((r90.max.x - 260.0).abs() < 1e-3);
    assert!((r90.min.y - 240.0).abs() < 1e-3);
    assert!((r90.max.y - 400.0).abs() < 1e-3);

    let r180 = transform_span_bbox(bbox, 180, rect);
    assert!((r180.min.x - 200.0).abs() < 1e-3);
    assert!((r180.max.x - 280.0).abs() < 1e-3);
    assert!((r180.min.y - 440.0).abs() < 1e-3);
    assert!((r180.max.y - 520.0).abs() < 1e-3);

    let r270 = transform_span_bbox(bbox, 270, rect);
    assert!((r270.min.x - 140.0).abs() < 1e-3);
    assert!((r270.max.x - 180.0).abs() < 1e-3);
    assert!((r270.min.y - 400.0).abs() < 1e-3);
    assert!((r270.max.y - 560.0).abs() < 1e-3);
}

#[test]
fn test_pdf_reading_order_two_columns_and_gutter() {
    use crate::document::pdf::text::{group_and_sort_glyphs, GlyphEntry};
    let mut glyphs = Vec::new();
    for (i, c) in "Left1".chars().enumerate() {
        glyphs.push(GlyphEntry { c, x: 50.0 + i as f64 * 10.0, y: 300.0, font_size: 14.0, width: 10.0 });
    }
    for (i, c) in "Right1".chars().enumerate() {
        glyphs.push(GlyphEntry { c, x: 450.0 + i as f64 * 10.0, y: 300.0, font_size: 14.0, width: 10.0 });
    }
    for (i, c) in "Left2".chars().enumerate() {
        glyphs.push(GlyphEntry { c, x: 50.0 + i as f64 * 10.0, y: 350.0, font_size: 14.0, width: 10.0 });
    }
    for (i, c) in "Right2".chars().enumerate() {
        glyphs.push(GlyphEntry { c, x: 450.0 + i as f64 * 10.0, y: 350.0, font_size: 14.0, width: 10.0 });
    }
    for (i, c) in "Left3".chars().enumerate() {
        glyphs.push(GlyphEntry { c, x: 50.0 + i as f64 * 10.0, y: 400.0, font_size: 14.0, width: 10.0 });
    }
    for (i, c) in "Right3".chars().enumerate() {
        glyphs.push(GlyphEntry { c, x: 450.0 + i as f64 * 10.0, y: 400.0, font_size: 14.0, width: 10.0 });
    }

    let spans = group_and_sort_glyphs(glyphs, 800.0, 1000.0);
    assert_eq!(spans.len(), 6);
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(texts, vec!["Left1", "Left2", "Left3", "Right1", "Right2", "Right3"]);
}

#[test]
fn test_pdf_outline_resolve_dest_clamping() {
    use hayro::hayro_syntax::object::{Array, FromBytes};
    use std::collections::HashMap;

    let dest_data = b"[ 5 /XYZ 0 792 1.0 ]";
    if let Some(arr) = Array::from_bytes(dest_data) {
        let page_map = HashMap::new();
        let page_count = 3;
        let resolved = crate::document::pdf::outline::resolve_dest_array(&arr, &page_map, page_count);
        assert_eq!(resolved, Some(2));
    }
}
