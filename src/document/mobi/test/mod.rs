mod decode;

use super::*;

#[test]
fn test_clean_chapter_title() {
    assert_eq!(
        clean_chapter_title("<i>Глава четырнадцатая</i>"),
        "Глава четырнадцатая"
    );
    assert_eq!(clean_chapter_title("<b><em>Глава 1</em></b>"), "Глава 1");
    assert_eq!(
        clean_chapter_title("&laquo;Начало&raquo; &amp; конец"),
        "«Начало» & конец"
    );
    assert_eq!(
        clean_chapter_title("<p class=\"p\">   Многомерная    личность  </p>"),
        "Многомерная личность"
    );
}

#[test]
fn test_is_likely_chapter_heading() {
    assert!(is_likely_chapter_heading("<i>Глава четырнадцатая</i>"));
    assert!(is_likely_chapter_heading("<b>Chapter 5</b>"));
    assert!(is_likely_chapter_heading("<em>Сеанс № 509</em>"));
    assert!(is_likely_chapter_heading("Приложение"));
    assert!(!is_likely_chapter_heading(
        "Это обычный абзац текста, который повествует об истории."
    ));
}

#[test]
fn test_split_by_pagebreaks() {
    let html = "<p>Раздел 1</p><mbp:pagebreak/><p>Раздел 2</p>";
    let sections = split_by_pagebreaks(html);
    assert_eq!(sections.len(), 2);
    assert!(sections[0].contains("Раздел 1"));
    assert!(sections[1].contains("Раздел 2"));
}

#[test]
fn test_split_by_headings() {
    let html = "<h1>Глава 1</h1><p>Текст 1</p><h2>Глава 2</h2><p>Текст 2</p>";
    let sections = split_by_headings(html);
    assert_eq!(sections.len(), 2);
    assert!(sections[0].contains("Глава 1"));
    assert!(sections[1].contains("Глава 2"));
}

#[test]
fn test_split_by_html_documents() {
    let html = "<html><body>Doc 1</body></html><html><body>Doc 2</body></html>";
    let sections = split_by_html_documents(html);
    assert_eq!(sections.len(), 2);
}

#[test]
fn test_parse_mobi_html_clean_tags_and_split() {
    let html = "<html><body><h1><i>Глава первая</i></h1><p>Текст главы</p></body></html>";
    let (chapters, toc) = parse_mobi_html(html, "Тест");
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0].title, "Глава первая");
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Глава первая");
}

#[test]
fn test_mobi_format_name_by_extension() {
    let determine_fmt = |ext: &str, is_kf8: bool| match ext {
        "azw3" => "AZW3",
        "azw" => "AZW",
        "prc" => "PRC",
        "mobi" => "MOBI",
        _ => {
            if is_kf8 {
                "AZW3"
            } else {
                "MOBI"
            }
        }
    };

    assert_eq!(determine_fmt("mobi", true), "MOBI");
    assert_eq!(determine_fmt("mobi", false), "MOBI");
    assert_eq!(determine_fmt("azw3", true), "AZW3");
    assert_eq!(determine_fmt("azw", false), "AZW");
    assert_eq!(determine_fmt("prc", false), "PRC");
    assert_eq!(determine_fmt("", true), "AZW3");
    assert_eq!(determine_fmt("", false), "MOBI");
}

#[test]
fn test_mobi_filepos_footnote_is_extracted() {
    let template =
        r#"<html><body><p>Text<a filepos="0000000000">[1]</a>.</p><p>Note text.</p></body></html>"#;
    let position = template.find("Note text.").unwrap();
    let html = template.replace("0000000000", &format!("{position:010}"));
    let (prepared, notes) = super::links::preprocess_links(&html);
    let id = format!("mobi_pos_{position}");
    assert_eq!(notes.get(&id).map(String::as_str), Some("Note text."));
    assert!(prepared.contains(&crate::document::footnote_token(&id)));
}

#[test]
fn test_mobi_hash_footnote_is_extracted() {
    let html =
        r##"<html><body><p>Text<a href="#n1">[2]</a>.</p><p id="n1">Note two.</p></body></html>"##;
    let (prepared, notes) = super::links::preprocess_links(html);
    assert_eq!(
        notes.get("mobi_hash_n1").map(String::as_str),
        Some("Note two.")
    );
    assert!(prepared.contains("{{note:mobi_hash_n1}}"));
}

#[test]
fn test_mobi_image_recindex_token() {
    let html = r#"<html><body><p>Before</p><img recindex="00002"/><p>After</p></body></html>"#;
    let (_, paragraphs) = crate::document::epub::html::extract_html_lenient(html);
    assert!(
        paragraphs
            .iter()
            .any(|paragraph| paragraph.contains("{{image:mobi_rec_2}}"))
    );
}

#[test]
fn test_11_mobi_images_if_present() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("11.mobi");
    if !path.exists() {
        return;
    }
    let doc = MobiDocument::from_file(&path).unwrap();
    let token = Document::chapters(&doc)
        .iter()
        .flat_map(|chapter| chapter.paragraphs.iter())
        .find_map(|paragraph| crate::document::image_token_id(paragraph));
    if let Some(token) = token {
        let bytes = Document::image_bytes(&doc, token).expect("image bytes");
        assert!(image::guess_format(&bytes).is_ok());
    }
}

#[test]
fn test_ms_mobi_clean_toc_and_chapters() {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ms.mobi");
    if !std::path::Path::new(fixture).exists() {
        return;
    }
    let doc = MobiDocument::from_file(fixture).unwrap();
    assert_eq!(doc.metadata().title, "Материалы Сета. Часть 2");
    assert_eq!(doc.metadata().author, "Дж. Робертс");
    let chapters = Document::chapters(&doc);
    assert!(!chapters.is_empty());
    for ch in chapters {
        assert!(!ch.title.contains("<i>"));
        assert!(!ch.title.contains("</i>"));
        assert!(!ch.title.contains("<b>"));
        assert!(!ch.title.contains("</b>"));
        assert!(!ch.title.contains("<em>"));
        assert!(!ch.title.contains("</em>"));
    }
    let toc = Document::toc(&doc);
    assert!(!toc.is_empty());
    for item in toc {
        assert!(!item.title.contains("<i>"));
        assert!(!item.title.contains("</i>"));
        assert!(!item.title.contains("<b>"));
        assert!(!item.title.contains("</b>"));
        assert!(!item.title.contains("<em>"));
        assert!(!item.title.contains("</em>"));
    }
    assert_eq!(doc.format_name(), "MOBI");
}

#[test]
fn test_11_mobi_if_present_decodes_cyrillic() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("11.mobi");
    if !path.exists() {
        return;
    }
    let doc = MobiDocument::from_file(&path).unwrap();
    let text = &Document::chapters(&doc)[0].paragraphs[0];
    assert!(text.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)));
    assert!(!text.contains('\u{FFFD}'));
    assert!(!text.contains("\u{0420}\u{045F}"));
    assert!(!text.contains("\u{0421}\u{0402}"));
}
