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
