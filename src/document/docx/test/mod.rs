#[test]
fn test_docx_images_tables_and_footnotes() {
    use crate::document::Document;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    let mut png = Vec::new();
    image::RgbaImage::from_pixel(6, 8, image::Rgba([9, 8, 7, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options = SimpleFileOptions::default();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p><w:r><w:t>Text before image.</w:t></w:r></w:p>
    <w:p><w:r><w:drawing><a:blip r:embed="rId5"/></w:drawing></w:r></w:p>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Cell A</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Cell B</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
    <w:p><w:r><w:t>Footnote</w:t><w:footnoteReference w:id="1"/></w:r></w:p>
  </w:body>
</w:document>"#,
        )
        .unwrap();

        zip.start_file("word/_rels/document.xml.rels", options)
            .unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/pic.png"/>
</Relationships>"#,
        )
        .unwrap();

        zip.start_file("word/footnotes.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="1"><w:p><w:r><w:t>Footnote text.</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#,
        )
        .unwrap();

        zip.start_file("word/header1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>Header text</w:t></w:r></w:p></w:hdr>"#,
        )
        .unwrap();

        zip.start_file("word/footer1.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:p><w:r><w:t>Footer text</w:t></w:r></w:p></w:ftr>"#,
        )
        .unwrap();

        zip.start_file("word/media/pic.png", options).unwrap();
        zip.write_all(&png).unwrap();
        zip.finish().unwrap();
    }

    let doc = crate::document::DocxDocument::from_bytes(&buf.into_inner(), "Fallback").unwrap();
    let paragraphs = &doc.chapters()[0].paragraphs;
    let token = paragraphs
        .iter()
        .find_map(|paragraph| crate::document::image_token_id(paragraph))
        .expect("image token");
    assert_eq!(doc.image_bytes(token).as_deref(), Some(png.as_slice()));
    assert!(
        paragraphs
            .iter()
            .any(|paragraph| paragraph == "Cell A | Cell B")
    );
    assert!(
        paragraphs
            .iter()
            .any(|paragraph| paragraph.contains("{{note:docx_fn_1}}"))
    );
    assert_eq!(doc.footnote("docx_fn_1"), Some("Footnote text."));
    assert_eq!(doc.header_text(), Some("Header text"));
    assert_eq!(doc.footer_text(), Some("Footer text"));
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
