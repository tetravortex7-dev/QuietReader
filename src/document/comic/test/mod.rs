use crate::test::util::unique_test_dir;
use crate::*;

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
fn test_comic_cb7_archive_uses_cached_pages() {
    let test_dir = unique_test_dir("comic_cb7");
    let cb7_path = test_dir.join("test.cb7");
    std::fs::write(&cb7_path, include_bytes!("fixtures/two_pages.7z")).unwrap();

    let doc = document::open_document(&cb7_path).expect("Должен открыть CB7 архив");
    let comic = doc.page_content().expect("CB7 должен быть комиксом");
    assert_eq!(comic.page_count(), 2);
    assert_eq!(comic.page_name(0), "2.png");
    assert_eq!(comic.page_name(1), "10.png");
    for _ in 0..2 {
        let decoded = comic.load_page_image(0).expect("Страница 0");
        assert_eq!(decoded.width, 6);
        assert_eq!(decoded.height, 6);
        let decoded = comic.load_page_image(1).expect("Страница 1");
        assert_eq!(decoded.width, 6);
    }

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_comic_cbt_archive_uses_cached_pages() {
    let test_dir = unique_test_dir("comic_cbt");
    let cbt_path = test_dir.join("test.cbt");

    let mut png = Vec::new();
    image::RgbaImage::new(6, 6)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    {
        let file = std::fs::File::create(&cbt_path).unwrap();
        let mut builder = tar::Builder::new(file);
        for name in ["2.png", "10.png"] {
            let mut header = tar::Header::new_gnu();
            header.set_size(png.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, png.as_slice())
                .unwrap();
        }
        builder.finish().unwrap();
    }

    let doc = document::open_document(&cbt_path).expect("Должен открыть CBT архив");
    let comic = doc.page_content().expect("CBT должен быть комиксом");
    assert_eq!(comic.page_count(), 2);
    assert_eq!(comic.page_name(0), "2.png");
    assert_eq!(comic.page_name(1), "10.png");
    for _ in 0..2 {
        let decoded = comic.load_page_image(0).expect("Страница 0");
        assert_eq!(decoded.width, 6);
        assert_eq!(decoded.height, 6);
        let decoded = comic.load_page_image(1).expect("Страница 1");
        assert_eq!(decoded.width, 6);
    }

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_comic_tar_extension_is_comic() {
    let test_dir = unique_test_dir("comic_tar");
    let tar_path = test_dir.join("volume.tar");

    let mut png = Vec::new();
    image::RgbaImage::new(3, 3)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    {
        let file = std::fs::File::create(&tar_path).unwrap();
        let mut builder = tar::Builder::new(file);
        let mut header = tar::Header::new_gnu();
        header.set_size(png.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "page001.png", png.as_slice())
            .unwrap();
        builder.finish().unwrap();
    }

    let doc = document::open_document(&tar_path).expect("Должен открыть TAR архив");
    let comic = doc.page_content().expect("TAR должен быть комиксом");
    assert_eq!(comic.page_count(), 1);
    assert_eq!(comic.page_name(0), "page001.png");
    assert!(comic.load_page_image(0).is_ok());

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
fn test_comic_folder_depth_limit() {
    let test_dir = unique_test_dir("comic_folder_depth");
    let folder_path = test_dir.join("MangaFolder");
    std::fs::create_dir_all(&folder_path).unwrap();

    let mut sample_png = Vec::new();
    let img = image::RgbaImage::new(4, 4);
    img.write_to(
        &mut std::io::Cursor::new(&mut sample_png),
        image::ImageFormat::Png,
    )
    .unwrap();

    std::fs::write(folder_path.join("root.png"), &sample_png).unwrap();

    let mut current = folder_path.clone();
    for _ in 0..(document::MAX_FOLDER_DEPTH + 4) {
        current = current.join("d");
        std::fs::create_dir_all(&current).unwrap();
    }
    std::fs::write(current.join("deep.png"), &sample_png).unwrap();

    let doc = document::open_document(&folder_path).expect("Должен открыть папку");
    let comic = doc.page_content().expect("Папка должна быть комиксом");
    assert_eq!(comic.page_count(), 1);
    assert_eq!(comic.page_name(0), "root.png");

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[cfg(unix)]
#[test]
fn test_comic_folder_skips_symlinks() {
    use std::os::unix::fs::symlink;

    let test_dir = unique_test_dir("comic_folder_symlink");
    let folder_path = test_dir.join("MangaFolder");
    let outside_dir = test_dir.join("Outside");
    std::fs::create_dir_all(&folder_path).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();

    let mut sample_png = Vec::new();
    let img = image::RgbaImage::new(4, 4);
    img.write_to(
        &mut std::io::Cursor::new(&mut sample_png),
        image::ImageFormat::Png,
    )
    .unwrap();

    let outside_file = test_dir.join("outside.png");
    std::fs::write(&outside_file, &sample_png).unwrap();
    std::fs::write(folder_path.join("root.png"), &sample_png).unwrap();
    std::fs::write(outside_dir.join("hidden.png"), &sample_png).unwrap();
    symlink(&outside_file, folder_path.join("link.png")).unwrap();
    symlink(&outside_dir, folder_path.join("linkdir")).unwrap();

    let doc = document::open_document(&folder_path).expect("Должен открыть папку");
    let comic = doc.page_content().expect("Папка должна быть комиксом");
    assert_eq!(comic.page_count(), 1);
    assert_eq!(comic.page_name(0), "root.png");

    let _ = std::fs::remove_dir_all(&test_dir);
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
fn test_comic_image_dimension_bounds() {
    use crate::document::comic::is_supported_image;
    assert!(is_supported_image("test.png"));
    assert!(is_supported_image("folder/test.jpg"));
    assert!(is_supported_image("test.webp"));
    assert!(!is_supported_image(".hidden.png"));
    assert!(!is_supported_image("test.exe"));
    assert!(!is_supported_image("__MACOSX/page.png"));
}
