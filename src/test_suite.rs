use super::*;
use document::{Document, Fb2Document};

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
fn test_platform_keep_screen_on() {
    crate::platform::set_keep_screen_on(true);
    crate::platform::set_keep_screen_on(false);
}

#[test]
fn test_platform_file_association_query_and_dismiss() {
    let status = crate::platform::check_fb2_association_status();
    assert!(matches!(
        status,
        crate::platform::AssociationStatus::Default
            | crate::platform::AssociationStatus::Registered
            | crate::platform::AssociationStatus::NotRegistered
    ));

    crate::platform::set_association_prompt_dismissed(true);
    assert!(crate::platform::is_association_prompt_dismissed());
    crate::platform::set_association_prompt_dismissed(false);
    assert!(!crate::platform::is_association_prompt_dismissed());
}

#[test]
#[ignore = "Изменяет реальные системные ассоциации файлов в реестре ОС"]
fn test_platform_register_association() {
    let result = crate::platform::register_fb2_association();
    assert!(result.is_ok());

    let status = crate::platform::check_fb2_association_status();
    assert!(
        status == crate::platform::AssociationStatus::Registered
            || status == crate::platform::AssociationStatus::Default
    );
}

#[test]
fn test_command_line_path_handling() {
    let temp_file = std::env::temp_dir().join(format!(
        "test_spaces_{}_{}.fb2",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let xml = r#"<FictionBook><body><section><title><p>Глава 1</p></title><p>Текст</p></section></body></FictionBook>"#;
    std::fs::write(&temp_file, xml).unwrap();

    let doc = Fb2Document::from_file(&temp_file).expect("Должен открыть файл по пути с пробелами");
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Глава 1");

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_settings_window_state_and_recent_books() {
    let mut settings = AppSettings::default();

    settings.update_window_state(Some(120.0), Some(80.0), 1200.0, 800.0, false);
    assert_eq!(settings.window.x, Some(120.0));
    assert_eq!(settings.window.y, Some(80.0));
    assert_eq!(settings.window.width, 1200.0);
    assert_eq!(settings.window.height, 800.0);
    assert!(!settings.window.maximized);

    let path1 = std::path::PathBuf::from(r"C:\Books\WarAndPeace.fb2");
    let path2 = std::path::PathBuf::from(r"C:\Books\EugeneOnegin.fb2");

    settings.record_book_progress(&path1, "Война и мир", "Лев Толстой", 2, 450.0);
    assert_eq!(settings.recent_books.len(), 1);
    assert_eq!(settings.last_opened_book, Some(path1.clone()));
    assert_eq!(settings.get_book_progress(&path1), Some((2, 450.0)));

    settings.record_book_progress(&path2, "Евгений Онегин", "Александр Пушкин", 0, 100.0);
    assert_eq!(settings.recent_books.len(), 2);
    assert_eq!(settings.last_opened_book, Some(path2.clone()));
    assert_eq!(settings.recent_books[0].path, path2);

    settings.record_book_progress(&path1, "Война и мир", "Лев Толстой", 3, 600.0);
    assert_eq!(settings.recent_books.len(), 2);
    assert_eq!(settings.recent_books[0].path, path1);
    assert_eq!(settings.get_book_progress(&path1), Some((3, 600.0)));

    let json = serde_json::to_string(&settings).expect("Сериализация должна работать");
    let loaded: AppSettings = serde_json::from_str(&json).expect("Десериализация должна работать");
    assert_eq!(loaded, settings);
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
fn test_window_state_persistence_and_multimonitor_recovery() {
    let mut settings = AppSettings::default();

    settings.update_window_state(Some(320.0), Some(180.0), 650.0, 450.0, false);
    assert_eq!(settings.window.x, Some(320.0));
    assert_eq!(settings.window.y, Some(180.0));
    assert_eq!(settings.window.width, 650.0);
    assert_eq!(settings.window.height, 450.0);
    assert!(!settings.window.maximized);

    settings.update_window_state(Some(-8.0), Some(-8.0), 1920.0, 1080.0, true);
    assert_eq!(settings.window.x, Some(320.0));
    assert_eq!(settings.window.y, Some(180.0));
    assert_eq!(settings.window.width, 650.0);
    assert_eq!(settings.window.height, 450.0);
    assert!(settings.window.maximized);

    settings.update_window_state(Some(-32000.0), Some(-32000.0), 650.0, 450.0, false);
    assert_eq!(settings.window.x, Some(320.0));
    assert_eq!(settings.window.y, Some(180.0));

    assert!(!crate::platform::is_window_accessible_in_work_area(
        2200, 150, 700, 500, 0, 0, 1920, 1040
    ));
    let (rx, ry, rw, rh) =
        crate::platform::reposition_window_into_work_area(2200, 150, 700, 500, 0, 0, 1920, 1040);
    assert_eq!((rw, rh), (700, 500));
    assert!(crate::platform::is_window_accessible_in_work_area(
        rx, ry, rw, rh, 0, 0, 1920, 1040
    ));

    assert!(!crate::platform::is_window_accessible_in_work_area(
        -1500, 100, 680, 480, 0, 0, 1920, 1040
    ));
    let (lx, ly, lw, lh) =
        crate::platform::reposition_window_into_work_area(-1500, 100, 680, 480, 0, 0, 1920, 1040);
    assert_eq!((lw, lh), (680, 480));
    assert!(crate::platform::is_window_accessible_in_work_area(
        lx, ly, lw, lh, 0, 0, 1920, 1040
    ));

    let (sx, sy, sw, sh) =
        crate::platform::sanitize_window_geometry(Some(9500.0), Some(9500.0), 650.0, 450.0);
    assert!(sw <= 650.0 && sw >= 400.0);
    assert!(sh <= 450.0 && sh >= 300.0);
    if let (Some(px), Some(py)) = (sx, sy) {
        assert!(px < 9000.0 && py < 9000.0);
    }
}

#[test]
fn test_epub_percent_decode_and_path_resolution() {
    use document::epub::{percent_decode_str, resolve_path};
    assert_eq!(percent_decode_str("Chapter%201.xhtml"), "Chapter 1.xhtml");
    assert_eq!(percent_decode_str("%D1%82%D0%B5%D1%81%D1%82"), "тест");
    assert_eq!(percent_decode_str("no_percent.html"), "no_percent.html");

    assert_eq!(resolve_path("OEBPS", "ch1.xhtml"), "OEBPS/ch1.xhtml");
    assert_eq!(resolve_path("OEBPS/text", "../ch1.xhtml"), "OEBPS/ch1.xhtml");
    assert_eq!(resolve_path("OEBPS", "../../outside.xhtml"), "");
    assert_eq!(resolve_path("OEBPS", "http://evil.com/x.html"), "http://evil.com/x.html");
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
fn test_is_safe_archive_path_percent_encoding_traversal() {
    use document::is_safe_archive_path;
    assert!(is_safe_archive_path("OEBPS/ch1.xhtml"));
    assert!(is_safe_archive_path("images/cover.jpg"));
    assert!(!is_safe_archive_path("../escape.xhtml"));
    assert!(!is_safe_archive_path("%2e%2e/escape.xhtml"));
    assert!(!is_safe_archive_path("..%2fescape.xhtml"));
    assert!(!is_safe_archive_path("/absolute/path.xhtml"));
    assert!(!is_safe_archive_path("C:/Windows/system.ini"));
    assert!(!is_safe_archive_path("%2fetc/passwd"));
}

#[test]
fn test_reading_progress_case_insensitive_lookup() {
    use std::path::PathBuf;
    let mut settings = settings::AppSettings::default();
    let path_upper = PathBuf::from("C:\\Books\\MyBook.epub");
    let path_lower = PathBuf::from("c:\\books\\mybook.epub");

    settings.record_book_progress(&path_upper, "My Book", "Author", 3, 145.5);
    let prog = settings.get_book_progress(&path_lower);
    assert_eq!(prog, Some((3, 145.5)));
}

#[test]
fn test_djvu_corrupted_header_and_short_bytes() {
    use document::DjvuDocument;
    use std::path::Path;

    let res_empty = DjvuDocument::from_bytes(vec![], Path::new("dummy.djvu"));
    assert!(res_empty.is_err());

    let res_short = DjvuDocument::from_bytes(vec![1, 2, 3], Path::new("dummy.djvu"));
    assert!(res_short.is_err());

    let res_garbage = DjvuDocument::from_bytes(b"NOT_A_DJVU_FILE_AT_ALL".to_vec(), Path::new("dummy.djvu"));
    assert!(res_garbage.is_err());

    let res_trunc = DjvuDocument::from_bytes(b"AT&TFORM\x00\x00\x00\x04DJVU".to_vec(), Path::new("dummy.djvu"));
    assert!(res_trunc.is_err());
}

#[test]
fn test_djvu_bookmark_page_resolution() {
    use document::djvu::resolve_djvu_bookmark_page;
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert("intro.djvu".to_string(), 0);
    map.insert("ch1.djvu".to_string(), 1);
    map.insert("ch2.djvu".to_string(), 2);
    map.insert("глава.djvu".to_string(), 3);

    assert_eq!(resolve_djvu_bookmark_page("#intro.djvu", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("#ch1.djvu", &map, 10), 1);
    assert_eq!(resolve_djvu_bookmark_page("#ch2", &map, 10), 2);
    assert_eq!(resolve_djvu_bookmark_page("#%D0%B3%D0%BB%D0%B0%D0%B2%D0%B0.djvu", &map, 10), 3);
    assert_eq!(resolve_djvu_bookmark_page("#1", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("#+5", &map, 10), 4);
    assert_eq!(resolve_djvu_bookmark_page("#page0006", &map, 10), 5);
    assert_eq!(resolve_djvu_bookmark_page("#pg_8", &map, 10), 7);
    assert_eq!(resolve_djvu_bookmark_page("#p3", &map, 10), 2);
    assert_eq!(resolve_djvu_bookmark_page("#9999", &map, 10), 9);
    assert_eq!(resolve_djvu_bookmark_page("#0", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("#unknown_target", &map, 10), 0);
}

#[test]
fn test_djvu_percent_decode() {
    use document::djvu::percent_decode_djvu;
    assert_eq!(percent_decode_djvu("hello%20world"), "hello world");
    assert_eq!(percent_decode_djvu("%2B5"), "+5");
    assert_eq!(percent_decode_djvu("%D1%82%D0%B5%D1%81%D1%82"), "тест");
    assert_eq!(percent_decode_djvu("clean"), "clean");
}

#[test]
fn test_djvu_thread_safety_send_sync() {
    use document::DjvuDocument;
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DjvuDocument>();
}

#[test]
fn test_document_format_name_and_custom_toc() {
    use document::{Document, TxtDocument, Fb2Document};
    let txt = TxtDocument::from_text("Sample text", "title");
    assert_eq!(txt.format_name(), "TXT");
    assert!(!txt.has_custom_toc());

    let fb2_sample = r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook>
  <description><title-info><book-title>Test</book-title></title-info></description>
  <body name="toc"><title><p>TOC</p></title><p><a href="#ch1">Ch 1</a></p></body>
  <body><section id="ch1"><p>Text</p></section></body>
</FictionBook>"##;
    let fb2 = Fb2Document::from_bytes(fb2_sample.as_bytes()).unwrap();
    assert_eq!(fb2.format_name(), "FB2");
    assert!(fb2.has_custom_toc());
}

#[test]
fn test_reader_core_bookmarks_and_search() {
    use crate::core::{Bookmark, ReaderCore};
    use crate::document::TxtDocument;
    use std::path::PathBuf;

    let mut core = ReaderCore::default();
    let path = PathBuf::from("test_book.txt");
    let bm1 = Bookmark {
        book_path: path.clone(),
        chapter: 0,
        scroll_y: 100.0,
        label: "Marker 1".to_string(),
        timestamp: 123456,
    };
    let bm2 = Bookmark {
        book_path: path.clone(),
        chapter: 1,
        scroll_y: 200.0,
        label: "Marker 2".to_string(),
        timestamp: 123457,
    };

    core.add_bookmark(bm1.clone());
    core.add_bookmark(bm2.clone());
    core.add_bookmark(bm1.clone());

    assert_eq!(core.bookmarks_for_path(&path).count(), 2);

    core.remove_bookmark(&path, 0);
    assert_eq!(core.bookmarks_for_path(&path).count(), 1);

    core.clear_bookmarks_for_path(&path);
    assert_eq!(core.bookmarks_for_path(&path).count(), 0);

    let doc = TxtDocument::from_text("First chapter with keyword needle inside it\nSecond chapter without it", "Doc");
    let hits = ReaderCore::search_in_document(&doc, "needle");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].chapter_index, 0);
    assert!(hits[0].snippet.contains("needle"));
}

#[test]
fn test_paged_common_helpers() {
    use crate::app::paged_common::{calculate_paged_dimensions, parse_page_input, zoom_in, zoom_out, zoom_reset};
    use eframe::egui::Vec2;

    assert_eq!(parse_page_input("1", 10), Some(0));
    assert_eq!(parse_page_input("5", 10), Some(4));
    assert_eq!(parse_page_input("10", 10), Some(9));
    assert_eq!(parse_page_input("99", 10), Some(9));
    assert_eq!(parse_page_input("0", 10), Some(0));
    assert_eq!(parse_page_input("abc", 10), None);

    let mut zoom = 100.0;
    let mut fit = "FitWidth".to_string();
    let mut pan = Vec2::new(10.0, 20.0);

    zoom_in(&mut zoom, &mut fit);
    assert_eq!(zoom, 115.0);
    assert_eq!(fit, "Custom");

    zoom_out(&mut zoom, &mut fit);
    assert_eq!(zoom, 100.0);

    zoom_reset(&mut zoom, &mut fit, &mut pan);
    assert_eq!(zoom, 100.0);
    assert_eq!(fit, "Original");
    assert_eq!(pan, Vec2::ZERO);

    let (w_fit_w, _) = calculate_paged_dimensions(800.0, 1200.0, 1000.0, 800.0, "FitWidth", 100.0);
    assert_eq!(w_fit_w, 984.0);

    let (_, h_fit_h) = calculate_paged_dimensions(800.0, 1200.0, 1000.0, 800.0, "FitHeight", 100.0);
    assert_eq!(h_fit_h, 784.0);
}

