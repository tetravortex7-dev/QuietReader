use super::*;

#[test]
#[ignore]
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
#[ignore]
fn test_comic_cbz_archive_and_metadata_parsing() {
    use std::io::Write;
    let test_dir = std::env::temp_dir().join("quietreader_test_comic_cbz");
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir).unwrap();
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
#[ignore]
fn test_comic_folder_scan() {
    let test_dir = std::env::temp_dir().join("quietreader_test_comic_folder");
    let _ = std::fs::remove_dir_all(&test_dir);
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
#[ignore]
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

    let temp_dir = std::env::temp_dir().join("quietreader_test_pdf");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
fn test_archive_path_traversal_protection() {
    use crate::document::{ComicDocument, DocumentError, is_safe_archive_path};
    use std::io::Write;

    assert!(!is_safe_archive_path("../evil.png"));
    assert!(!is_safe_archive_path("ch1/../../evil.png"));
    assert!(!is_safe_archive_path("/etc/passwd"));
    assert!(!is_safe_archive_path(r"\Windows\System32\cmd.exe"));
    assert!(!is_safe_archive_path("C:/evil.png"));
    assert!(is_safe_archive_path("ch01/page_01.png"));

    let test_dir = std::env::temp_dir().join("quietreader_test_traversal");
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir).unwrap();
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
#[ignore]
fn test_zip_bomb_and_bounded_read_protection() {
    use crate::document::{DocumentError, read_bounded};

    let data = vec![0u8; 2048];
    let mut total_unpacked = 0u64;

    // Превышение лимита одной записи
    let res_single = read_bounded(&data[..], 1024, &mut total_unpacked, 10_000);
    assert!(matches!(res_single, Err(DocumentError::LimitExceeded(_))));

    // Превышение общего лимита распаковки
    let mut total_unpacked = 9500u64;
    let res_total = read_bounded(&data[..1000], 2048, &mut total_unpacked, 10_000);
    assert!(matches!(res_total, Err(DocumentError::LimitExceeded(_))));
}

#[test]
#[ignore]
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
#[ignore]
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
#[ignore]
fn test_corrupted_settings_json_backup_and_recovery() {
    let temp_dir = std::env::temp_dir().join("quietreader_test_corrupted_settings");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let corrupted_content = "{ \"font_size\": BROKEN_JSON !!!";
    std::fs::write(&settings_path, corrupted_content).unwrap();

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded, AppSettings::default());
    assert!(
        bak_path.exists(),
        "Резервная копия .bak должна быть создана при повреждении settings.json"
    );
    assert_eq!(
        std::fs::read_to_string(&bak_path).unwrap(),
        corrupted_content
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore]
fn test_txt_md_log_format_via_registry() {
    let temp_dir = std::env::temp_dir().join("quietreader_test_txt_md_log");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

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
#[ignore]
fn test_settings_schema_version_migration_with_backup() {
    let temp_dir = std::env::temp_dir().join("quietreader_test_schema_migration");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    // Старый файл настроек без schema_version (v0)
    let legacy_json = r#"{"window":{"width":900.0,"height":700.0,"maximized":false},"recent_books":[],"font_size":21.0,"theme":"Dark","keep_screen_on":false,"show_toc":true}"#;
    std::fs::write(&settings_path, legacy_json).unwrap();

    let migrated = AppSettings::load_from_path(&settings_path);
    assert_eq!(
        migrated.schema_version,
        settings::CURRENT_SETTINGS_SCHEMA_VERSION
    );
    assert_eq!(migrated.font_size, 21.0);
    assert_eq!(migrated.theme, "Dark");
    assert!(
        bak_path.exists(),
        "При миграции со старой версии схемы должен создаваться .bak файл"
    );
    assert_eq!(std::fs::read_to_string(&bak_path).unwrap(), legacy_json);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
#[ignore]
fn test_utf16_txt_decoding_and_pageup_prev_chapter() {
    use crate::document::Document;
    // 1. Проверка декодирования UTF-16LE с BOM
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

    // 2. Проверка перехода PageUp в начало главы 2 -> попадание в конец главы 1
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
