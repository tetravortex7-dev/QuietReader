use crate::document::{Document, DocumentError, EpubDocument};
use crate::*;

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
fn test_parse_epub_in_memory() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let container_xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container_xml.as_bytes()).unwrap();

        let content_opf = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="BookId">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Мастер и Маргарита</dc:title>
    <dc:creator>Михаил Булгаков</dc:creator>
  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine toc="ncx">
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(content_opf.as_bytes()).unwrap();

        let toc_ncx = r#"<?xml version="1.0" encoding="utf-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <navMap>
    <navPoint id="np-1" playOrder="1">
      <navLabel><text>Глава 1. Никогда не разговаривайте с неизвестными</text></navLabel>
      <content src="chapter1.xhtml"/>
    </navPoint>
    <navPoint id="np-2" playOrder="2">
      <navLabel><text>Глава 2. Понтий Пилат</text></navLabel>
      <content src="chapter2.xhtml"/>
    </navPoint>
  </navMap>
</ncx>"#;
        zip.start_file("OEBPS/toc.ncx", options).unwrap();
        zip.write_all(toc_ncx.as_bytes()).unwrap();

        let ch1_xhtml = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Глава 1</title></head>
<body>
  <h1>Глава 1. Никогда не разговаривайте с неизвестными</h1>
  <p>Однажды весною, в час небывало жаркого заката, в Москве, на Патриарших прудах, появились два гражданина.</p>
  <p>Первый из них — приблизительно сорокалетний, одетый в серенькую летнюю пару...</p>
</body>
</html>"#;
        zip.start_file("OEBPS/chapter1.xhtml", options).unwrap();
        zip.write_all(ch1_xhtml.as_bytes()).unwrap();

        let ch2_xhtml = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Глава 2</title></head>
<body>
  <h1>Глава 2. Понтий Пилат</h1>
  <p>В белом плаще с кровавым подбоем, шаркающей кавалерийской походкой...</p>
</body>
</html>"#;
        zip.start_file("OEBPS/chapter2.xhtml", options).unwrap();
        zip.write_all(ch2_xhtml.as_bytes()).unwrap();

        zip.finish().unwrap();
    }

    let zip_bytes = zip_buf.into_inner();
    let doc =
        document::EpubDocument::from_bytes(&zip_bytes).expect("Должен успешно распарсить EPUB");

    assert_eq!(doc.metadata().title, "Мастер и Маргарита");
    assert_eq!(doc.metadata().author, "Михаил Булгаков");
    assert_eq!(doc.chapters().len(), 2);
    assert_eq!(doc.chapters()[0].paragraphs.len(), 3);
    assert!(doc.chapters()[0].paragraphs[0].contains("Глава 1"));
    assert!(doc.chapters()[0].paragraphs[1].contains("Патриарших прудах"));
    assert_eq!(doc.chapters()[1].paragraphs.len(), 2);
    assert!(doc.chapters()[1].paragraphs[0].contains("Глава 2"));
    assert!(doc.chapters()[1].paragraphs[1].contains("белом плаще"));

    assert_eq!(doc.toc().len(), 2);
    assert_eq!(
        doc.toc()[0].title,
        "Глава 1. Никогда не разговаривайте с неизвестными"
    );
    assert_eq!(doc.toc()[0].chapter_index, 0);
    assert_eq!(doc.toc()[1].title, "Глава 2. Понтий Пилат");
    assert_eq!(doc.toc()[1].chapter_index, 1);
}

#[test]
fn test_epub_percent_decode_and_path_resolution() {
    use document::epub::{percent_decode_str, resolve_path};
    assert_eq!(percent_decode_str("Chapter%201.xhtml"), "Chapter 1.xhtml");
    assert_eq!(percent_decode_str("%D1%82%D0%B5%D1%81%D1%82"), "тест");
    assert_eq!(percent_decode_str("no_percent.html"), "no_percent.html");

    assert_eq!(resolve_path("OEBPS", "ch1.xhtml"), "OEBPS/ch1.xhtml");
    assert_eq!(
        resolve_path("OEBPS/text", "../ch1.xhtml"),
        "OEBPS/ch1.xhtml"
    );
    assert_eq!(resolve_path("OEBPS", "../../outside.xhtml"), "");
    assert_eq!(
        resolve_path("OEBPS", "http://evil.com/x.html"),
        "http://evil.com/x.html"
    );
}

#[test]
fn test_epub_entity_sanitization_and_lenient_html() {
    use document::epub::{parse_html_chapter, sanitize_xml_entities};
    let raw_html = "<html><body><h1>Title</h1><p>AT&T and Barnes & Noble &nbsp; &mdash; 5 &lt; 10</p><img src=\"broken.jpg\"><p>Second paragraph after unclosed img</p></body></html>";
    let sanitized = sanitize_xml_entities(raw_html);
    assert!(sanitized.contains("&amp;"));
    assert!(sanitized.contains("—"));

    let (title, paragraphs, _) = parse_html_chapter(raw_html);
    assert_eq!(title, "Title");
    assert_eq!(paragraphs.len(), 4);
    assert_eq!(paragraphs[0], "Title");
    assert!(paragraphs[1].contains("AT&T"));
    assert!(paragraphs[1].contains("Barnes & Noble"));
    assert!(paragraphs[1].contains("—"));
    assert!(paragraphs[1].contains("5 < 10"));
    assert_eq!(paragraphs[2], "{{image:broken.jpg}}");
    assert!(paragraphs[3].contains("Second paragraph"));
}

#[test]
fn test_epub_footnote_noteref_and_note_container() {
    let html = r##"<html><body>
        <p>Text<a epub:type="noteref" href="#fn1">1</a>.</p>
        <aside epub:type="footnote" id="fn1"><p>Note text.</p></aside>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(paragraphs.iter().any(|p| p.contains("{{note:fn1}}")));
    assert_eq!(notes.get("fn1").map(String::as_str), Some("Note text."));
}

#[test]
fn test_epub_plain_href_footnote_reference() {
    let html = r##"<html><body>
        <p>Text<a href="#note4">[4]</a>.</p>
        <div id="note4"><p>Note body.</p></div>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(paragraphs.iter().any(|p| p.contains("{{note:note4}}")));
    assert_eq!(notes.get("note4").map(String::as_str), Some("Note body."));
}

#[test]
fn test_epub_plain_href_note_with_short_id() {
    let html = r##"<html><body>
        <p>Text<a href="#n4">4</a>.</p>
        <p id="n4">Note body.</p>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(paragraphs.iter().any(|p| p.contains("{{note:n4}}")));
    assert_eq!(notes.get("n4").map(String::as_str), Some("Note body."));
}

#[test]
fn test_epub_footnote_with_underscore_prefix() {
    let html = r##"<html><body>
        <p>Text<a href="#_ftn4">[4]</a>.</p>
        <p id="_ftn4">Note body.</p>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(paragraphs.iter().any(|p| p.contains("{{note:_ftn4}}")));
    assert_eq!(notes.get("_ftn4").map(String::as_str), Some("Note body."));
}

#[test]
fn test_epub_regular_numbered_link_is_not_a_footnote() {
    let html = r##"<html><body>
        <p>Text<a href="chapter2.xhtml#chapter2">[1]</a>.</p>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(!paragraphs.iter().any(|p| p.contains("{{note:")));
    assert!(notes.is_empty());
}

#[test]
fn test_epub_notes_container_keeps_individual_notes() {
    let html = r##"<html><body>
        <p>Text<a href="#note1">1</a><a href="#note2">2</a>.</p>
        <div id="notes">
            <aside id="note1"><p>First note.</p></aside>
            <aside id="note2"><p>Second note.</p></aside>
        </div>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(paragraphs.iter().any(|p| p.contains("{{note:note1}}")));
    assert!(paragraphs.iter().any(|p| p.contains("{{note:note2}}")));
    assert_eq!(notes.get("note1").map(String::as_str), Some("First note."));
    assert_eq!(notes.get("note2").map(String::as_str), Some("Second note."));
    assert!(!notes.contains_key("notes"));
}

#[test]
fn test_epub_inline_image_token_and_lookup() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut png = Vec::new();
    image::RgbaImage::from_pixel(4, 6, image::Rgba([10, 20, 30, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Images</dc:title></metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="pic" href="images/pic.png" media-type="image/png"/>
  </manifest>
  <spine><itemref idref="ch1"/></spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let chapter = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <p>Before image.</p>
            <img src="images/pic.png" alt="Picture"/>
            <p>After image.</p>
        </body></html>"##;
        zip.start_file("OEBPS/chapter1.xhtml", options).unwrap();
        zip.write_all(chapter.as_bytes()).unwrap();
        zip.start_file("OEBPS/images/pic.png", options).unwrap();
        zip.write_all(&png).unwrap();
        zip.finish().unwrap();
    }

    let doc = EpubDocument::from_bytes(&zip_buf.into_inner()).unwrap();
    let token = doc.chapters()[0]
        .paragraphs
        .iter()
        .find_map(|paragraph| crate::document::image_token_id(paragraph))
        .expect("image token");
    let bytes = doc.image_bytes(token).expect("image bytes");
    assert_eq!(bytes, png);
}

#[test]
fn test_epub_document_footnote_lookup() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Notes</dc:title></metadata>
  <manifest><item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="ch1"/></spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let chapter = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <p>Text<a epub:type="noteref" href="#fn1">1</a>.</p>
            <aside epub:type="footnote" id="fn1"><p>Note text.</p></aside>
        </body></html>"##;
        zip.start_file("OEBPS/chapter1.xhtml", options).unwrap();
        zip.write_all(chapter.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let doc = EpubDocument::from_bytes(&zip_buf.into_inner()).unwrap();
    assert!(
        doc.chapters()
            .iter()
            .flat_map(|chapter| chapter.paragraphs.iter())
            .any(|paragraph| paragraph.contains("{{note:fn1}}"))
    );
    assert_eq!(doc.footnote("fn1"), Some("Note text."));
}

#[test]
fn test_epub_notes_file_outside_spine_is_indexed() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Notes</dc:title></metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="notes" href="notes.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine><itemref idref="ch1"/></spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let chapter = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <p>Text<a href="notes.xhtml#note1">[1]</a>.</p>
        </body></html>"##;
        zip.start_file("OEBPS/chapter1.xhtml", options).unwrap();
        zip.write_all(chapter.as_bytes()).unwrap();

        let notes = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <aside id="note1"><p>Note text.</p></aside>
        </body></html>"##;
        zip.start_file("OEBPS/notes.xhtml", options).unwrap();
        zip.write_all(notes.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let doc = EpubDocument::from_bytes(&zip_buf.into_inner()).unwrap();
    assert!(
        doc.chapters()
            .iter()
            .flat_map(|chapter| chapter.paragraphs.iter())
            .any(|paragraph| paragraph.contains("{{note:note1}}"))
    );
    assert_eq!(doc.footnote("note1"), Some("Note text."));
}

#[test]
fn test_epub_span_note_target_and_number_title() {
    let html = r##"<html><body>
        <p>Text<a href="#id14">[1]</a>.</p>
        <span id="id14"><div class="title"><p>1</p></div><p>Note body.</p></span>
    </body></html>"##;
    let (_, paragraphs, _, notes) = crate::document::epub::parse_html_chapter_with_notes(html);
    assert!(paragraphs.iter().any(|p| p.contains("{{note:id14}}")));
    assert_eq!(notes.get("id14").map(String::as_str), Some("Note body."));
}

#[test]
fn test_epub_cross_file_span_note_and_location() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Notes</dc:title></metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="notes.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine><itemref idref="ch1"/><itemref idref="ch2"/></spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let chapter = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <p>Text<a href="notes.xhtml#id14" class="a">[1]</a>.</p>
        </body></html>"##;
        zip.start_file("OEBPS/chapter1.xhtml", options).unwrap();
        zip.write_all(chapter.as_bytes()).unwrap();

        let notes = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <span id="id14"><div class="title"><p>1</p></div><p>Note body.</p></span>
        </body></html>"##;
        zip.start_file("OEBPS/notes.xhtml", options).unwrap();
        zip.write_all(notes.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let doc = EpubDocument::from_bytes(&zip_buf.into_inner()).unwrap();
    assert!(
        doc.chapters()[0]
            .paragraphs
            .iter()
            .any(|paragraph| paragraph.contains("{{note:id14}}"))
    );
    assert_eq!(doc.footnote("id14"), Some("Note body."));
    let location = doc.note_location("id14").unwrap();
    assert_eq!(location.chapter_index, 1);
    assert_eq!(
        doc.chapters()[1].paragraphs[location.paragraph_index],
        "Note body."
    );
}

#[test]
fn test_epub_note_target_with_generic_name_outside_spine() {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(container.as_bytes()).unwrap();

        let opf = r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Notes</dc:title></metadata>
  <manifest>
    <item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
    <item id="app" href="appendices.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine><itemref idref="ch1"/></spine>
</package>"#;
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();

        let chapter = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <p>Text<a href="appendices.xhtml#id14">[1]</a>.</p>
        </body></html>"##;
        zip.start_file("OEBPS/chapter1.xhtml", options).unwrap();
        zip.write_all(chapter.as_bytes()).unwrap();

        let appendix = r##"<html xmlns="http://www.w3.org/1999/xhtml"><body>
            <span id="id14"><div class="title"><p>1</p></div><p>Note body.</p></span>
        </body></html>"##;
        zip.start_file("OEBPS/appendices.xhtml", options).unwrap();
        zip.write_all(appendix.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let doc = EpubDocument::from_bytes(&zip_buf.into_inner()).unwrap();
    assert_eq!(doc.footnote("id14"), Some("Note body."));
    assert!(doc.note_location("id14").is_none());
}
