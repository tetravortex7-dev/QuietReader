use crate::document::{Document, DocumentError, Fb2Document};
use crate::test::util::unique_test_dir;
use crate::*;

#[test]
fn test_corrupted_and_malformed_fb2_handling() {
    let res_empty = Fb2Document::from_bytes(&[]);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_garbage = Fb2Document::from_bytes(b"PLAIN TEXT WITHOUT ANY XML TAGS");
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let res_invalid_xml = Fb2Document::from_bytes(b"<FictionBook><body><section><title><p>Broken");
    assert!(matches!(res_invalid_xml, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_fb2_formatting_bold_and_italic() {
    let xml = r#"<FictionBook><body>
        <section>
            <title><p>Заголовок раздела</p></title>
            <p>Обычный текст с <strong>жирным словом</strong> и <emphasis>курсивом</emphasis>.</p>
            <p><strong><em>Жирный курсив</em></strong> вместе.</p>
            <p><strong>В</strong> начале было Слово.</p>
            <v>Строка стиха с <strong>жирным</strong> знаком.</v>
        </section>
    </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let chs = doc.chapters();
    assert_eq!(chs.len(), 1);
    assert_eq!(chs[0].paragraphs.len(), 4);

    assert_eq!(
        chs[0].paragraphs[0],
        "Обычный текст с <b>жирным словом</b> и <i>курсивом</i>."
    );
    assert_eq!(chs[0].paragraphs[1], "<b><i>Жирный курсив</i></b> вместе.");
    assert_eq!(chs[0].paragraphs[2], "<b>В</b> начале было Слово.");
    assert_eq!(chs[0].paragraphs[3], "Строка стиха с <b>жирным</b> знаком.");

    let spans = crate::document::parse_formatted_spans(&chs[0].paragraphs[2]);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "В");
    assert!(spans[0].bold);
    assert!(!spans[0].italic);
    assert_eq!(spans[1].text, " начале было Слово.");
    assert!(!spans[1].bold);
    assert!(!spans[1].italic);

    let clean = crate::document::strip_formatting_tags(&chs[0].paragraphs[0]);
    assert_eq!(clean, "Обычный текст с жирным словом и курсивом.");
}

#[test]
fn test_fb2_deep_section_recursion_bounded() {
    use crate::document::Document;
    use crate::document::Fb2Document;
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><FictionBook xmlns=\"http://www.gribuser.ru/xml/fictionbook/2.0\"><body>",
    );
    for i in 0..50 {
        xml.push_str(&format!(
            "<section><title><p>Глава {}</p></title><p>Текст {}</p>",
            i, i
        ));
    }
    for _ in 0..50 {
        xml.push_str("</section>");
    }
    xml.push_str("</body></FictionBook>");

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    assert!(!doc.chapters().is_empty());
    assert!(!doc.toc().is_empty());
}

#[test]
fn test_parse_simple_fb2() {
    let sample_fb2 = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <description>
    <title-info>
      <author>
        <first-name>Лев</first-name>
        <last-name>Толстой</last-name>
      </author>
      <book-title>Война и мир</book-title>
    </title-info>
  </description>
  <body>
    <title>
      <p>Том первый</p>
    </title>
    <section>
      <title>
        <p>Часть первая. Глава I</p>
      </title>
      <p>— Eh bien, mon prince. Gênes et Lucques ne sont plus que des apanages...</p>
      <p>Так говорила в июле 1805 года известная Анна Павловна Шерер...</p>
    </section>
  </body>
</FictionBook>"#;

    let doc = Fb2Document::from_bytes(sample_fb2.as_bytes()).expect("Should parse");
    assert_eq!(doc.metadata().title, "Война и мир");
    assert_eq!(doc.metadata().author, "Лев Толстой");
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Том первый");
    assert_eq!(doc.toc()[0].children.len(), 1);
    assert_eq!(doc.toc()[0].children[0].title, "Часть первая. Глава I");
}

#[test]
fn test_parse_windows_1251_fb2() {
    let xml_str = r#"<?xml version="1.0" encoding="windows-1251"?>
<FictionBook>
  <description>
    <title-info>
      <author><first-name>Александр</first-name><last-name>Пушкин</last-name></author>
      <book-title>Евгений Онегин</book-title>
    </title-info>
  </description>
  <body>
    <section>
      <title><p>Глава первая</p></title>
      <p>«Мой дядя самых честных правил...</p>
    </section>
  </body>
</FictionBook>"#;

    let (encoded_bytes, _, _) = encoding_rs::WINDOWS_1251.encode(xml_str);
    let doc = Fb2Document::from_bytes(&encoded_bytes).expect("Should parse windows-1251");
    assert_eq!(doc.metadata().title, "Евгений Онегин");
    assert_eq!(doc.metadata().author, "Александр Пушкин");
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Глава первая");
    assert_eq!(
        doc.chapters()[doc.toc()[0].chapter_index].paragraphs[0],
        "«Мой дядя самых честных правил..."
    );
}

#[test]
fn test_example_book_chapter7_content_and_toc() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0">
  <body>
    <section>
      <title><p>Глава 7: Живая плоть</p></title>
      <p>Диктовка. Глава 7: «Живая плоть»</p>
      <p>Продолжение повествования главы.</p>
    </section>
  </body>
</FictionBook>"#;
    let doc = Fb2Document::from_bytes(xml.as_bytes()).expect("Should parse");
    let toc = doc.toc();
    let ch7_toc = toc
        .iter()
        .find(|item| item.title.contains("Живая плоть"))
        .expect("Глава 7 должна присутствовать в TOC");
    let ch7 = &doc.chapters()[ch7_toc.chapter_index];
    assert!(ch7.title.contains("Живая плоть"));
    assert!(!ch7.paragraphs.is_empty());
    assert!(
        ch7.paragraphs
            .iter()
            .any(|p| p.contains("Диктовка. Глава 7: «Живая плоть»"))
    );
}

#[test]
fn test_section_with_untitled_preamble_subsection() {
    let xml = r#"<FictionBook><body>
            <section>
                <title>
                    <p>Глава 7</p>
                    <p>Живая плоть</p>
                </title>
                <section>
                    <p>Вводный текст главы 7, диктовка.</p>
                </section>
                <section>
                    <title><p>Сеанс 631</p></title>
                    <p>Текст сеанса 631.</p>
                </section>
            </section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Глава 7 — Живая плоть");
    assert_eq!(toc[0].children.len(), 1);
    assert_eq!(toc[0].children[0].title, "Сеанс 631");

    let ch0 = &doc.chapters()[toc[0].chapter_index];
    assert_eq!(ch0.title, "Глава 7 — Живая плоть");
    assert_eq!(ch0.paragraphs.len(), 1);
    assert_eq!(ch0.paragraphs[0], "Вводный текст главы 7, диктовка.");

    let ch1 = &doc.chapters()[toc[0].children[0].chapter_index];
    assert_eq!(ch1.title, "Сеанс 631");
    assert_eq!(ch1.paragraphs[0], "Текст сеанса 631.");
}

#[test]
fn test_multiple_top_level_chapters() {
    let xml = r#"<FictionBook><body>
            <section><title><p>Глава 1</p></title><p>Текст 1</p></section>
            <section><title><p>Глава 2</p></title><p>Текст 2</p></section>
            <section><title><p>Глава 3</p></title><p>Текст 3</p></section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 3);
    assert_eq!(toc[0].title, "Глава 1");
    assert_eq!(toc[1].title, "Глава 2");
    assert_eq!(toc[2].title, "Глава 3");
    assert!(toc[0].children.is_empty());
    assert!(toc[1].children.is_empty());
    assert!(toc[2].children.is_empty());
}

#[test]
fn test_nested_subsections_multiple_levels() {
    let xml = r#"<FictionBook><body>
            <section>
                <title><p>Часть 1</p></title>
                <p>Предисловие части 1</p>
                <section>
                    <title><p>Глава 1.1</p></title>
                    <p>Текст главы 1.1</p>
                    <section>
                        <title><p>Подраздел 1.1.1</p></title>
                        <p>Глубокий текст</p>
                    </section>
                </section>
            </section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Часть 1");
    assert_eq!(toc[0].children.len(), 1);
    assert_eq!(toc[0].children[0].title, "Глава 1.1");
    assert_eq!(toc[0].children[0].children.len(), 1);
    assert_eq!(toc[0].children[0].children[0].title, "Подраздел 1.1.1");
}

#[test]
fn test_sections_without_titles_with_nested_titled_sections() {
    let xml = r#"<FictionBook><body>
            <section>
                <p>Вводный текст секции без заголовка</p>
                <section>
                    <title><p>Вложенный заголовок А</p></title>
                    <p>Текст А</p>
                </section>
                <section>
                    <title><p>Вложенный заголовок Б</p></title>
                    <p>Текст Б</p>
                </section>
            </section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].title, "Вложенный заголовок А");
    assert_eq!(toc[1].title, "Вложенный заголовок Б");
}

#[test]
fn test_titles_with_nested_xml_elements() {
    let xml = r#"<FictionBook><body>
            <section>
                <title>
                    <p><strong>Часть I.</strong> <em>Начало</em></p>
                    <p>В <style name="highlight">городе</style> N</p>
                </title>
                <p>Текст главы...</p>
            </section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 1);
    assert_eq!(toc[0].title, "Часть I. Начало — В городе N");
}

#[test]
fn test_multiple_bodies() {
    let xml = r#"<FictionBook>
            <body>
                <title><p>Книга первая</p></title>
                <section><title><p>Глава 1</p></title><p>Текст 1</p></section>
            </body>
            <body>
                <title><p>Примечания</p></title>
                <section><title><p>Примечание 1</p></title><p>Пояснение</p></section>
            </body>
        </FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();
    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].title, "Книга первая");
    assert_eq!(toc[0].children.len(), 1);
    assert_eq!(toc[0].children[0].title, "Глава 1");

    assert_eq!(toc[1].title, "Примечания");
    assert_eq!(toc[1].children.len(), 1);
    assert_eq!(toc[1].children[0].title, "Примечание 1");
}

#[test]
fn test_book_without_titles() {
    let xml = r#"<FictionBook><body>
            <section><p>Просто текст без заголовков.</p></section>
            <section><p>Второй абзац.</p></section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    assert!(doc.toc().is_empty());
    assert_eq!(doc.chapters().len(), 2);
}

#[test]
fn test_toc_navigation_to_correct_section() {
    let xml = r#"<FictionBook><body>
            <section id="sec_intro">
                <title><p>Введение</p></title>
                <p>Текст введения</p>
            </section>
            <section id="sec_part1">
                <title><p>Часть 1</p></title>
                <p>Текст части 1</p>
                <section id="sec_sub1">
                    <title><p>Подраздел 1.1</p></title>
                    <p>Текст подраздела 1.1</p>
                </section>
            </section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();

    let intro_item = &toc[0];
    assert_eq!(intro_item.title, "Введение");
    assert_eq!(
        doc.chapters()[intro_item.chapter_index].paragraphs[0],
        "Текст введения"
    );

    let part1_item = &toc[1];
    assert_eq!(part1_item.title, "Часть 1");
    assert_eq!(
        doc.chapters()[part1_item.chapter_index].paragraphs[0],
        "Текст части 1"
    );

    let sub1_item = &part1_item.children[0];
    assert_eq!(sub1_item.title, "Подраздел 1.1");
    assert_eq!(
        doc.chapters()[sub1_item.chapter_index].paragraphs[0],
        "Текст подраздела 1.1"
    );
}

#[test]
fn test_repeated_chapter_titles() {
    let xml = r#"<FictionBook><body>
            <section>
                <title><p>Том 1</p></title>
                <section><title><p>Глава 1</p></title><p>Текст тома 1 главы 1</p></section>
            </section>
            <section>
                <title><p>Том 2</p></title>
                <section><title><p>Глава 1</p></title><p>Текст тома 2 главы 1</p></section>
            </section>
        </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();

    let ch1_tom1 = &toc[0].children[0];
    let ch1_tom2 = &toc[1].children[0];

    assert_eq!(ch1_tom1.title, "Глава 1");
    assert_eq!(ch1_tom2.title, "Глава 1");
    assert_ne!(ch1_tom1.chapter_index, ch1_tom2.chapter_index);

    assert_eq!(
        doc.chapters()[ch1_tom1.chapter_index].paragraphs[0],
        "Текст тома 1 главы 1"
    );
    assert_eq!(
        doc.chapters()[ch1_tom2.chapter_index].paragraphs[0],
        "Текст тома 2 главы 1"
    );
}

#[test]
fn test_explicit_body_name_toc() {
    let xml = r##"<FictionBook>
            <body name="toc">
                <title><p>Оглавление</p></title>
                <section>
                    <p><a href="#ch1">Глава первая</a></p>
                    <p><a href="#ch2">Глава вторая</a></p>
                </section>
            </body>
            <body>
                <section id="ch1">
                    <p>Содержимое первой главы</p>
                </section>
                <section id="ch2">
                    <p>Содержимое второй главы</p>
                </section>
            </body>
        </FictionBook>"##;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let toc = doc.toc();

    assert_eq!(toc.len(), 2);
    assert_eq!(toc[0].title, "Глава первая");
    assert_eq!(toc[1].title, "Глава вторая");

    assert_eq!(
        doc.chapters()[toc[0].chapter_index].paragraphs[0],
        "Содержимое первой главы"
    );
    assert_eq!(
        doc.chapters()[toc[1].chapter_index].paragraphs[0],
        "Содержимое второй главы"
    );

    assert_eq!(doc.chapters().len(), 2);
}

#[test]
fn test_fb2_explicit_toc_links_with_percent_decoding() {
    use document::Fb2Document;
    let sample = r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description>
    <title-info>
      <author><first-name>Author</first-name></author>
      <book-title>Test Book</book-title>
    </title-info>
  </description>
  <body name="toc">
    <title><p>Оглавление</p></title>
    <p><a href="#%D0%B3%D0%BB%D0%B0%D0%B2%D0%B0_1">Глава первая</a></p>
  </body>
  <body>
    <section id="глава_1">
      <title><p>Глава 1</p></title>
      <p>Текст главы 1</p>
    </section>
  </body>
</FictionBook>"##;

    let doc = Fb2Document::from_bytes(sample.as_bytes()).expect("Should parse");
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Глава первая");
    assert_eq!(doc.toc()[0].chapter_index, 0);
}

#[test]
fn test_fb2_zip_opens_through_registry() {
    use std::io::Write;

    let dir = unique_test_dir("fb2_zip");
    let xml = r#"<FictionBook><description><title-info><book-title>ZIP Book</book-title></title-info></description><body><section><title><p>Глава</p></title><p>Текст из FB2.ZIP.</p></section></body></FictionBook>"#;

    for file_name in ["book.fb2.zip", "book.zip"] {
        let path = dir.join(file_name);
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("BOOK.FB2", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
        zip.finish().unwrap();

        let doc = document::open_document(&path).expect("Должен открыть FB2.ZIP");
        assert_eq!(doc.format_name(), "FB2");
        assert_eq!(doc.metadata().title, "ZIP Book");
        assert_eq!(doc.chapters()[0].paragraphs[0], "Текст из FB2.ZIP.");
    }

    let empty_zip_path = dir.join("empty.zip");
    let file = std::fs::File::create(&empty_zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("notes.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"not fb2").unwrap();
    zip.finish().unwrap();
    assert!(matches!(
        Fb2Document::from_zip_file(&empty_zip_path),
        Err(DocumentError::Unsupported(_))
    ));

    let _ = std::fs::remove_dir_all(&dir);
}
