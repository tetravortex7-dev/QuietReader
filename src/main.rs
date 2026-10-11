#![windows_subsystem = "windows"]

mod app;
mod document;
mod platform;
mod settings;

use app::QuietReaderApp;
use settings::AppSettings;

fn main() -> eframe::Result<()> {
    let settings = AppSettings::load();
    let initial_file = std::env::args_os().nth(1).map(std::path::PathBuf::from);

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_min_inner_size([600.0, 400.0])
        .with_title("QuietReader");

    let w = settings.window.width.clamp(600.0, 3840.0);
    let h = settings.window.height.clamp(400.0, 2160.0);
    viewport = viewport.with_inner_size([w, h]);

    if let (Some(x), Some(y)) = (settings.window.x, settings.window.y) {
        if x >= -2000.0 && x <= 4000.0 && y >= -2000.0 && y <= 3000.0 {
            viewport = viewport.with_position([x, y]);
        }
    }

    if settings.window.maximized {
        viewport = viewport.with_maximized(true);
    }

    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/app_64.png")) {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "QuietReader",
        native_options,
        Box::new(move |cc| Ok(Box::new(QuietReaderApp::new(cc, initial_file, settings)))),
    )
}

#[cfg(test)]
mod tests {
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
        assert_eq!(doc.chapters()[doc.toc()[0].chapter_index].paragraphs[0], "«Мой дядя самых честных правил...");
    }

    // 1. Книга с несколькими главами верхнего уровня
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

    // 2. Книга с вложенными подразделами в несколько уровней
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

    // 3. Секции без заголовков, внутри которых есть секции с заголовками
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
        // Внешняя секция не имеет заголовка, поэтому пустой пункт не создается,
        // а вложенные подразделы поднимаются наверх
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].title, "Вложенный заголовок А");
        assert_eq!(toc[1].title, "Вложенный заголовок Б");
    }

    // 4. Заголовки с вложенными XML-элементами форматирования
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

    // 5. Несколько элементов body
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

    // 6. Книга без заголовков
    #[test]
    fn test_book_without_titles() {
        let xml = r#"<FictionBook><body>
            <section><p>Просто текст без заголовков.</p></section>
            <section><p>Второй абзац.</p></section>
        </body></FictionBook>"#;

        let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
        // Никаких фиктивных глав в оглавлении быть не должно
        assert!(doc.toc().is_empty());
        // Текст книги при этом не потерян
        assert_eq!(doc.chapters().len(), 2);
    }

    // 7. Переход из оглавления к правильной секции
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

        // Проверяем, что chapter_index ведёт точно к соответствующей секции
        let intro_item = &toc[0];
        assert_eq!(intro_item.title, "Введение");
        assert_eq!(doc.chapters()[intro_item.chapter_index].paragraphs[0], "Текст введения");

        let part1_item = &toc[1];
        assert_eq!(part1_item.title, "Часть 1");
        assert_eq!(doc.chapters()[part1_item.chapter_index].paragraphs[0], "Текст части 1");

        let sub1_item = &part1_item.children[0];
        assert_eq!(sub1_item.title, "Подраздел 1.1");
        assert_eq!(doc.chapters()[sub1_item.chapter_index].paragraphs[0], "Текст подраздела 1.1");
    }

    // 8. Повторяющиеся названия глав
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
        // Индексы глав обязательно должны различаться
        assert_ne!(ch1_tom1.chapter_index, ch1_tom2.chapter_index);

        assert_eq!(doc.chapters()[ch1_tom1.chapter_index].paragraphs[0], "Текст тома 1 главы 1");
        assert_eq!(doc.chapters()[ch1_tom2.chapter_index].paragraphs[0], "Текст тома 2 главы 1");
    }

    // 9. Книга с явным body name="toc" и ссылками
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

        // Оглавление сформировалось по ссылкам из body name="toc"
        assert_eq!(toc.len(), 2);
        assert_eq!(toc[0].title, "Глава первая");
        assert_eq!(toc[1].title, "Глава вторая");

        assert_eq!(doc.chapters()[toc[0].chapter_index].paragraphs[0], "Содержимое первой главы");
        assert_eq!(doc.chapters()[toc[1].chapter_index].paragraphs[0], "Содержимое второй главы");

        // При этом body name="toc" не попало в главы книги
        assert_eq!(doc.chapters().len(), 2);
    }

    #[test]
    fn test_platform_keep_screen_on() {
        // Проверяем, что вызов платформенной функции не падает и корректно включает и выключает режим
        crate::platform::set_keep_screen_on(true);
        crate::platform::set_keep_screen_on(false);
    }

    #[test]
    fn test_platform_file_association_query_and_dismiss() {
        // Проверяем, что запрос статуса возвращает корректный enum и не падает
        let status = crate::platform::check_fb2_association_status();
        assert!(matches!(
            status,
            crate::platform::AssociationStatus::Default
                | crate::platform::AssociationStatus::Registered
                | crate::platform::AssociationStatus::NotRegistered
        ));

        // Проверяем установку и чтение флага отклонения запроса
        crate::platform::set_association_prompt_dismissed(true);
        assert!(crate::platform::is_association_prompt_dismissed());
        crate::platform::set_association_prompt_dismissed(false);
        assert!(!crate::platform::is_association_prompt_dismissed());
    }

    #[test]
    fn test_platform_register_association() {
        // Проверяем успешную регистрацию в HKCU
        let result = crate::platform::register_fb2_association();
        assert!(result.is_ok());

        // После регистрации статус должен быть либо Registered, либо Default
        let status = crate::platform::check_fb2_association_status();
        assert!(
            status == crate::platform::AssociationStatus::Registered
                || status == crate::platform::AssociationStatus::Default
        );
    }

    #[test]
    fn test_command_line_path_handling() {
        // Проверяем, что путь с пробелами и кириллицей корректно распознаётся
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("Тестовая книга с пробелами.fb2");

        let xml = r#"<FictionBook><body><section><title><p>Глава 1</p></title><p>Текст</p></section></body></FictionBook>"#;
        std::fs::write(&test_file, xml).unwrap();

        let doc = Fb2Document::from_file(&test_file).expect("Должен открыть файл по пути с пробелами");
        assert_eq!(doc.toc().len(), 1);
        assert_eq!(doc.toc()[0].title, "Глава 1");

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_settings_window_state_and_recent_books() {
        let mut settings = AppSettings::default();

        // 1. Положение и размер окна
        settings.update_window_state(Some(120.0), Some(80.0), 1200.0, 800.0, false);
        assert_eq!(settings.window.x, Some(120.0));
        assert_eq!(settings.window.y, Some(80.0));
        assert_eq!(settings.window.width, 1200.0);
        assert_eq!(settings.window.height, 800.0);
        assert!(!settings.window.maximized);

        // 2. Добавление книг в недавние и сохранение прогресса чтения
        let path1 = std::path::PathBuf::from(r"C:\Books\WarAndPeace.fb2");
        let path2 = std::path::PathBuf::from(r"C:\Books\EugeneOnegin.fb2");

        settings.record_book_progress(&path1, "Война и мир", "Лев Толстой", 2, 450.0);
        assert_eq!(settings.recent_books.len(), 1);
        assert_eq!(settings.last_opened_book, Some(path1.clone()));
        assert_eq!(settings.get_book_progress(&path1), Some((2, 450.0)));

        settings.record_book_progress(&path2, "Евгений Онегин", "Александр Пушкин", 0, 100.0);
        assert_eq!(settings.recent_books.len(), 2);
        assert_eq!(settings.last_opened_book, Some(path2.clone()));
        // Последняя открытая должна быть первой в списке
        assert_eq!(settings.recent_books[0].path, path2);

        // Обновление прогресса первой книги переносит её на первое место
        settings.record_book_progress(&path1, "Война и мир", "Лев Толстой", 3, 600.0);
        assert_eq!(settings.recent_books.len(), 2);
        assert_eq!(settings.recent_books[0].path, path1);
        assert_eq!(settings.get_book_progress(&path1), Some((3, 600.0)));

        // 3. Сериализация и десериализация
        let json = serde_json::to_string(&settings).expect("Сериализация должна работать");
        let loaded: AppSettings = serde_json::from_str(&json).expect("Десериализация должна работать");
        assert_eq!(loaded, settings);
    }
}

