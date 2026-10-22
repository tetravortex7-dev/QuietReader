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
