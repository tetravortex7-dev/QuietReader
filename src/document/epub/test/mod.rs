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
    assert_eq!(paragraphs.len(), 3);
    assert_eq!(paragraphs[0], "Title");
    assert!(paragraphs[1].contains("AT&T"));
    assert!(paragraphs[1].contains("Barnes & Noble"));
    assert!(paragraphs[1].contains("—"));
    assert!(paragraphs[1].contains("5 < 10"));
    assert!(paragraphs[2].contains("Second paragraph"));
}
