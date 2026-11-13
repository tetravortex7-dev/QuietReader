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
fn test_mobi_kf8_kindle_pos_fid_with_fragment() {
    let html = r#"<html><body><p>Text<a href="kindle:pos:fid:0001:off:0000000020#kf8_fn1">[1]</a>.</p><p id="kf8_fn1">KF8 Note Text</p></body></html>"#;
    let (prepared, notes) = super::links::preprocess_links(html);
    let key = "mobi_kf8_1_20_kf8_fn1";
    assert_eq!(notes.get(key).map(String::as_str), Some("KF8 Note Text"));
    assert!(prepared.contains(&crate::document::footnote_token(key)));
}

#[test]
fn test_mobi_kf8_kindle_pos_fid_with_offset() {
    let template = r#"<html><body><p>Text<a href="kindle:pos:fid:0002:off:0000000000">[*]</a>.</p><p>Target note at offset</p></body></html>"#;
    let position = template.find("Target note at offset").unwrap();
    let html = template.replace("0000000000", &format!("{position:010}"));
    let (prepared, notes) = super::links::preprocess_links(&html);
    let key = format!("mobi_kf8_2_{position}");
    assert_eq!(
        notes.get(&key).map(String::as_str),
        Some("Target note at offset")
    );
    assert!(prepared.contains(&crate::document::footnote_token(&key)));
}

#[test]
fn test_mobi_anchor_name_attribute_extracted() {
    let html = r##"<html><body><p>Text<a href="#named_note">[3]</a>.</p><p><a name="named_note"></a>Content from anchor name.</p></body></html>"##;
    let (prepared, notes) = super::links::preprocess_links(html);
    assert_eq!(
        notes.get("mobi_hash_named_note").map(String::as_str),
        Some("Content from anchor name.")
    );
    assert!(prepared.contains("{{note:mobi_hash_named_note}}"));
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

#[test]
fn test_mobi_extract_note_at_utf8_mid_char_and_4000_byte_boundary() {
    let template = "<html><body><p>Текст <a filepos=\"0000000000\"><sup>1</sup></a></p><p>Сноска: кириллица</p><p id=\"long\">{long}</p><a href=\"#long\">[2]</a></body></html>";
    let mid_char_pos = template.find("Сноска").unwrap() + "Сноска".len() - 1;
    assert!(!template.is_char_boundary(mid_char_pos));
    let long_body = "Ж".repeat(2500);
    let html = template
        .replace("0000000000", &format!("{mid_char_pos:010}"))
        .replace("{long}", &long_body);
    let (prepared, notes) = super::links::preprocess_links(&html);
    let pos_key = format!("mobi_pos_{mid_char_pos}");
    assert_eq!(
        notes.get(&pos_key).map(String::as_str),
        Some("а: кириллица")
    );
    assert!(prepared.contains(&crate::document::footnote_token(&pos_key)));
    let long_note = notes
        .get("mobi_hash_long")
        .expect("4000+ byte Cyrillic note");
    assert!(!long_note.is_empty());
    assert!(long_note.chars().all(|c| c == 'Ж'));
}

#[test]
fn test_mobi_cp1251_filepos_maps_raw_offset_to_decoded_utf8() {
    let filler = "Русский текст до сноски. ".repeat(12);
    let template = format!(
        "<html><body><p>{filler}<a filepos=\"0000000000\"><sup>1</sup></a></p><p>Целевая сноска на кириллице.</p></body></html>"
    );
    let (raw_template, _, _) = encoding_rs::WINDOWS_1251.encode(&template);
    let target_bytes = encoding_rs::WINDOWS_1251
        .encode("<p>Целевая сноска на кириллице.")
        .0;
    let raw_pos = raw_template
        .windows(target_bytes.len())
        .position(|w| w == target_bytes.as_ref())
        .expect("target offset in CP1251 bytes");
    let html = template.replace("0000000000", &format!("{raw_pos:010}"));
    let (raw_bytes, _, _) = encoding_rs::WINDOWS_1251.encode(&html);
    let decoded = super::decode::decode_mobi_content(&raw_bytes, 1251);
    assert!(raw_pos < decoded.len());
    let (prepared, notes) = super::links::preprocess_links_with_raw(&decoded, &raw_bytes, 1251);
    let key = format!("mobi_pos_{raw_pos}");
    assert_eq!(
        notes.get(&key).map(String::as_str),
        Some("Целевая сноска на кириллице.")
    );
    assert!(prepared.contains(&crate::document::footnote_token(&key)));
}

#[test]
fn test_mobi_extract_note_by_attr_whitespace_cyrillic_and_data_id() {
    let html = concat!(
        r##"<html><body><p>Сноска <a href="#n1"><sup>1</sup></a>.</p>"##,
        r##"<p data-id="n1">Ложная сноска</p>"##,
        r##"<p id =  "Привет" data-extra="x">Другой абзац</p>"##,
        r##"<p id = 'n1'>Настоящая сноска.</p></body></html>"##,
    );
    let (prepared, notes) = super::links::preprocess_links(html);
    assert_eq!(
        notes.get("mobi_hash_n1").map(String::as_str),
        Some("Настоящая сноска.")
    );
    assert!(prepared.contains("{{note:mobi_hash_n1}}"));
}

#[test]
fn test_mobi_abbr_aside_ignored_and_nested_marker_tags_supported() {
    let html = concat!(
        r##"<html><body>"##,
        r##"<abbr title="abbr" href="#n1">1</abbr> <aside href="#n1">1</aside> "##,
        r##"<a name="sec"/><a name="unclosed">Раздел <a href="#n1">[<sup>1</sup>]</a>"##,
        r##"<a href="#n2"><sup>2</sup></a> <a href="#n3"><span><i>*</i></span></a> "##,
        r##"<a href="#n4"><sup>&dagger;</sup></a> <a href="#n5"><b>a</b></a>"##,
        r##"<p id="n1">Сноска 1</p><p id="n2">Сноска 2</p><p id="n3">Сноска 3</p>"##,
        r##"<p id="n4">Сноска 4</p><p id="n5">Сноска 5</p></body></html>"##,
    );
    let (prepared, notes) = super::links::preprocess_links(html);
    assert!(prepared.contains(r##"<abbr title="abbr" href="#n1">1</abbr>"##));
    assert!(prepared.contains(r##"<aside href="#n1">1</aside>"##));
    assert_eq!(
        notes.get("mobi_hash_n1").map(String::as_str),
        Some("Сноска 1")
    );
    assert_eq!(
        notes.get("mobi_hash_n2").map(String::as_str),
        Some("Сноска 2")
    );
    assert_eq!(
        notes.get("mobi_hash_n3").map(String::as_str),
        Some("Сноска 3")
    );
    assert_eq!(
        notes.get("mobi_hash_n4").map(String::as_str),
        Some("Сноска 4")
    );
    assert_eq!(
        notes.get("mobi_hash_n5").map(String::as_str),
        Some("Сноска 5")
    );
}

#[test]
fn test_parse_mobi_html_preserves_short_first_paragraph() {
    let html = "<p>Привет!</p><p>Второй абзац главы.</p><mbp:pagebreak/><p>Кто там?</p><p>Тишина в коридоре.</p>";
    let (chapters, _) = parse_mobi_html(html, "Тест");
    assert_eq!(chapters.len(), 2);
    assert_eq!(
        chapters[0].paragraphs,
        vec!["Привет!", "Второй абзац главы."]
    );
    assert_eq!(
        chapters[1].paragraphs,
        vec!["Кто там?", "Тишина в коридоре."]
    );

    let with_h1 = "<h1>Встреча</h1><p>Да!</p><p>Он вошёл.</p>";
    let (h1_chapters, _) = parse_mobi_html(with_h1, "Тест");
    assert_eq!(h1_chapters.len(), 1);
    assert_eq!(h1_chapters[0].title, "Встреча");
    assert_eq!(h1_chapters[0].paragraphs, vec!["Да!", "Он вошёл."]);
}
