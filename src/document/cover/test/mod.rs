use crate::document::cover::{
    cover_cache_path, load_cover_thumbnail, load_or_create_cover_thumbnail,
};
use crate::test::util::unique_test_dir;
use base64::Engine;
use image::{Rgba, RgbaImage};
use std::io::Write;
use std::path::Path;

fn image_bytes(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let image = RgbaImage::from_pixel(width, height, Rgba(color));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn write_cb7(path: &Path, entries: &[(&str, &[u8])]) {
    let mut writer = sevenz_rust::SevenZWriter::create(path).unwrap();
    for (name, bytes) in entries {
        let mut entry = sevenz_rust::SevenZArchiveEntry::new();
        entry.name = (*name).to_string();
        entry.has_stream = true;
        entry.size = bytes.len() as u64;
        writer
            .push_archive_entry(entry, Some(std::io::Cursor::new(*bytes)))
            .unwrap();
    }
    writer.finish().unwrap();
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn test_sibling_cover_thumbnail_and_cache() {
    let directory = unique_test_dir("cover_sibling");
    let book = directory.join("book.epub");
    std::fs::write(&book, b"not an epub").unwrap();
    let cover = image_bytes(400, 600, [220, 80, 70, 255]);
    std::fs::write(directory.join("cover.png"), cover).unwrap();

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 240);
    assert_eq!(thumbnail.height, 360);
    assert_eq!(thumbnail.rgba.len(), 240 * 360 * 4);

    let cache_dir = directory.join("covers");
    let cached = load_or_create_cover_thumbnail(&cache_dir, &book).unwrap();
    assert_eq!(cached.width, 240);
    assert_eq!(cached.height, 360);
    let cache_path = cover_cache_path(&cache_dir, &book);
    assert!(cache_path.is_file());

    std::fs::remove_file(directory.join("cover.png")).unwrap();
    let from_cache = load_or_create_cover_thumbnail(&cache_dir, &book).unwrap();
    assert_eq!(from_cache.width, 240);
    assert_eq!(from_cache.height, 360);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_epub_cover_from_opf_metadata() {
    let directory = unique_test_dir("cover_epub_meta");
    let book = directory.join("book.epub");
    let cover = image_bytes(300, 450, [40, 120, 200, 255]);
    let container = br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test</dc:title>
    <meta name="cover" content="cover-image"/>
  </metadata>
  <manifest>
    <item id="chapter" href="chapter.xhtml" media-type="application/xhtml+xml"/>
    <item id="cover-image" href="images/cover.png" media-type="image/png"/>
  </manifest>
</package>"#;
    write_zip(
        &book,
        &[
            ("META-INF/container.xml", container),
            ("OEBPS/content.opf", opf),
            ("OEBPS/images/cover.png", &cover),
        ],
    );

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 240);
    assert_eq!(thumbnail.height, 360);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_epub_cover_from_manifest_properties() {
    let directory = unique_test_dir("cover_epub_properties");
    let book = directory.join("book.epub");
    let cover = image_bytes(200, 300, [30, 160, 90, 255]);
    let opf = br#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Test</dc:title></metadata>
  <manifest>
    <item id="cover" href="cover.png" media-type="image/png" properties="cover-image"/>
  </manifest>
</package>"#;
    write_zip(&book, &[("content.opf", opf), ("cover.png", &cover)]);

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 200);
    assert_eq!(thumbnail.height, 300);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_fb2_cover_from_coverpage_binary() {
    let directory = unique_test_dir("cover_fb2");
    let book = directory.join("book.fb2");
    let cover = image_bytes(180, 270, [120, 60, 180, 255]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&cover);
    let fb2 = format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns:l="http://www.w3.org/1999/xlink">
  <description><title-info><coverpage><image l:href="#cover.png"/></coverpage></title-info></description>
  <body><section><p>Text</p></section></body>
  <binary id="cover.png" content-type="image/png">{encoded}</binary>
</FictionBook>"##
    );
    std::fs::write(&book, fb2).unwrap();

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 180);
    assert_eq!(thumbnail.height, 270);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_fb2_zip_cover() {
    let directory = unique_test_dir("cover_fb2_zip");
    let book = directory.join("book.fb2.zip");
    let cover = image_bytes(120, 180, [90, 90, 220, 255]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&cover);
    let fb2 = format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns:l="http://www.w3.org/1999/xlink">
  <description><title-info><coverpage><image l:href="#cover.jpg"/></coverpage></title-info></description>
  <binary id="cover.jpg" content-type="image/png">{encoded}</binary>
</FictionBook>"##
    );
    write_zip(&book, &[("book.fb2", fb2.as_bytes())]);

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 120);
    assert_eq!(thumbnail.height, 180);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_cbz_uses_first_naturally_sorted_image() {
    let directory = unique_test_dir("cover_cbz");
    let book = directory.join("book.cbz");
    let page_one = image_bytes(30, 45, [10, 200, 10, 255]);
    let page_two = image_bytes(20, 30, [200, 10, 10, 255]);
    let page_ten = image_bytes(10, 15, [10, 10, 200, 255]);
    write_zip(
        &book,
        &[
            ("ComicInfo.xml", b"<ComicInfo/>"),
            ("page10.png", &page_ten),
            ("page2.png", &page_two),
            ("page1.png", &page_one),
        ],
    );

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 30);
    assert_eq!(thumbnail.height, 45);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_cb7_cover_uses_first_naturally_sorted_image() {
    let directory = unique_test_dir("cover_cb7");
    let book = directory.join("book.cb7");
    let page_one = image_bytes(40, 60, [10, 200, 10, 255]);
    let page_two = image_bytes(20, 30, [200, 10, 10, 255]);
    write_cb7(&book, &[("page2.png", &page_two), ("page1.png", &page_one)]);

    let thumbnail = load_cover_thumbnail(&book).unwrap();
    assert_eq!(thumbnail.width, 40);
    assert_eq!(thumbnail.height, 60);

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn test_mobi_cover_is_read_from_image_records() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("11.mobi");
    if !fixture.exists() {
        return;
    }
    let thumbnail = load_cover_thumbnail(&fixture).unwrap();
    assert!(thumbnail.width > 0);
    assert!(thumbnail.height > 0);
    assert!(thumbnail.width <= 240);
    assert!(thumbnail.height <= 360);
}

#[test]
fn test_comic_folder_uses_first_naturally_sorted_image() {
    let directory = unique_test_dir("cover_comic_folder");
    std::fs::write(
        directory.join("page10.png"),
        image_bytes(10, 15, [10, 10, 200, 255]),
    )
    .unwrap();
    std::fs::write(
        directory.join("page2.png"),
        image_bytes(20, 30, [200, 10, 10, 255]),
    )
    .unwrap();
    std::fs::write(
        directory.join("page1.png"),
        image_bytes(40, 60, [10, 200, 10, 255]),
    )
    .unwrap();

    let thumbnail = load_cover_thumbnail(&directory).unwrap();
    assert_eq!(thumbnail.width, 40);
    assert_eq!(thumbnail.height, 60);

    let _ = std::fs::remove_dir_all(&directory);
}
