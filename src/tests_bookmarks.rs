use crate::app::bookmarks_view::generate_markdown_conspectus;
use crate::app::input::process_keyboard_events;
use crate::core::{Bookmark, BookmarkColor, BookmarkKind, Command};
use crate::i18n::UiLanguage;
use crate::settings::AppSettings;
use eframe::egui::{Key, Modifiers};
use std::path::PathBuf;

#[test]
fn test_bookmark_colors_and_highlights() {
    assert_eq!(BookmarkColor::ALL.len(), 4);

    let yellow = BookmarkColor::Yellow;
    assert_eq!(
        yellow.to_color32(),
        eframe::egui::Color32::from_rgb(235, 195, 60)
    );
    let y_light = yellow.to_highlight_color(false);
    let y_dark = yellow.to_highlight_color(true);
    assert_ne!(y_light.a(), 0);
    assert_ne!(y_dark.a(), 0);

    let green = BookmarkColor::Green;
    assert_eq!(
        green.to_color32(),
        eframe::egui::Color32::from_rgb(72, 187, 120)
    );

    let blue = BookmarkColor::Blue;
    assert_eq!(
        blue.to_color32(),
        eframe::egui::Color32::from_rgb(66, 153, 225)
    );

    let red = BookmarkColor::Red;
    assert_eq!(
        red.to_color32(),
        eframe::egui::Color32::from_rgb(245, 101, 101)
    );
}

#[test]
fn test_bookmark_serialization_and_defaults() {
    let bm = Bookmark {
        id: "bm_1".to_string(),
        book_path: PathBuf::from("test_book.epub"),
        chapter: 2,
        scroll_y: 350.5,
        label: "Chapter 3".to_string(),
        text: "Some quote text here".to_string(),
        note: "Important thought".to_string(),
        color: BookmarkColor::Green,
        kind: BookmarkKind::Quote,
        timestamp: 1700000000,
        progress_percent: 45.2,
        page_number: None,
    };

    let json = serde_json::to_string(&bm).expect("serialize bookmark");
    let deserialized: Bookmark = serde_json::from_str(&json).expect("deserialize bookmark");

    assert_eq!(deserialized.id, "bm_1");
    assert_eq!(deserialized.book_path, PathBuf::from("test_book.epub"));
    assert_eq!(deserialized.chapter, 2);
    assert!((deserialized.scroll_y - 350.5).abs() < f32::EPSILON);
    assert_eq!(deserialized.label, "Chapter 3");
    assert_eq!(deserialized.text, "Some quote text here");
    assert_eq!(deserialized.note, "Important thought");
    assert_eq!(deserialized.color, BookmarkColor::Green);
    assert_eq!(deserialized.kind, BookmarkKind::Quote);
    assert_eq!(deserialized.timestamp, 1700000000);
    assert!((deserialized.progress_percent - 45.2).abs() < f32::EPSILON);
    assert_eq!(deserialized.page_number, None);
}

#[test]
fn test_app_settings_bookmarks_default_and_roundtrip() {
    let settings = AppSettings::default();
    assert!(settings.bookmarks.is_empty());

    let mut custom = AppSettings::default();
    custom.bookmarks.push(Bookmark {
        id: "b1".to_string(),
        book_path: PathBuf::from("book.pdf"),
        chapter: 0,
        scroll_y: 0.0,
        label: "Page 1".to_string(),
        text: "Preview text".to_string(),
        note: String::new(),
        color: BookmarkColor::Yellow,
        kind: BookmarkKind::Bookmark,
        timestamp: 100,
        progress_percent: 10.0,
        page_number: Some(0),
    });

    let json = serde_json::to_string(&custom).expect("serialize settings");
    let loaded: AppSettings = serde_json::from_str(&json).expect("deserialize settings");
    assert_eq!(loaded.bookmarks.len(), 1);
    assert_eq!(loaded.bookmarks[0].id, "b1");
    assert_eq!(loaded.bookmarks[0].page_number, Some(0));
}

#[test]
fn test_hotkey_ctrl_d_toggle_bookmark() {
    let mut ctrl_mod = Modifiers::default();
    ctrl_mod.ctrl = true;
    let cmds_ctrl_d = process_keyboard_events(false, |k| k == Key::D, ctrl_mod, false);
    assert_eq!(cmds_ctrl_d, vec![Command::ToggleBookmark]);

    let cmds_plain_d = process_keyboard_events(false, |k| k == Key::D, Modifiers::default(), false);
    assert!(cmds_plain_d.is_empty());

    let cmds_typing = process_keyboard_events(true, |k| k == Key::D, ctrl_mod, false);
    assert!(cmds_typing.is_empty());
}

#[test]
fn test_markdown_conspectus_generation() {
    let t = UiLanguage::Ru.t();
    let b1 = Bookmark {
        id: "1".to_string(),
        book_path: PathBuf::from("war_and_peace.epub"),
        chapter: 0,
        scroll_y: 100.0,
        label: "Глава 1".to_string(),
        text: "Елена Васильевна улыбнулась...".to_string(),
        note: "Характеристика персонажа".to_string(),
        color: BookmarkColor::Yellow,
        kind: BookmarkKind::Quote,
        timestamp: 10,
        progress_percent: 5.0,
        page_number: None,
    };
    let b2 = Bookmark {
        id: "2".to_string(),
        book_path: PathBuf::from("war_and_peace.epub"),
        chapter: 0,
        scroll_y: 50.0,
        label: "Глава 1".to_string(),
        text: "В салоне Анны Павловны...".to_string(),
        note: String::new(),
        color: BookmarkColor::Blue,
        kind: BookmarkKind::Bookmark,
        timestamp: 5,
        progress_percent: 2.0,
        page_number: None,
    };
    let b3 = Bookmark {
        id: "3".to_string(),
        book_path: PathBuf::from("war_and_peace.epub"),
        chapter: 1,
        scroll_y: 200.0,
        label: "Глава 2".to_string(),
        text: "Пьер был неуклюж.".to_string(),
        note: "Важная сцена".to_string(),
        color: BookmarkColor::Red,
        kind: BookmarkKind::Quote,
        timestamp: 20,
        progress_percent: 15.0,
        page_number: None,
    };

    let bookmarks = vec![b1, b3, b2];
    let md = generate_markdown_conspectus("Война и мир", "Лев Толстой", &bookmarks, t);

    assert!(md.contains("# Война и мир"));
    assert!(md.contains("**Author**: Лев Толстой"));
    assert!(md.contains("## Глава 1"));
    assert!(md.contains("## Глава 2"));
    assert!(md.contains("> Елена Васильевна улыбнулась..."));
    assert!(md.contains("**Заметка**: Характеристика персонажа"));
    assert!(md.contains("> Пьер был неуклюж."));
    assert!(md.contains("**Заметка**: Важная сцена"));

    let pos_ch1 = md.find("## Глава 1").unwrap();
    let pos_ch2 = md.find("## Глава 2").unwrap();
    assert!(pos_ch1 < pos_ch2);

    let pos_b2 = md.find("В салоне Анны Павловны...").unwrap();
    let pos_b1 = md.find("Елена Васильевна улыбнулась...").unwrap();
    assert!(pos_b2 < pos_b1);
}

#[test]
fn test_all_languages_bookmark_translations() {
    for lang in UiLanguage::CONCRETE {
        let t = lang.t();
        assert!(!t.bookmarks.is_empty());
        assert!(!t.bookmarks_tooltip.is_empty());
        assert!(!t.add_bookmark.is_empty());
        assert!(!t.bookmark_added.is_empty());
        assert!(!t.bookmark_removed.is_empty());
        assert!(!t.save_quote_bookmark.is_empty());
        assert!(!t.quote_saved.is_empty());
        assert!(!t.bookmark_search_placeholder.is_empty());
        assert!(!t.filter_all.is_empty());
        assert!(!t.color_yellow.is_empty());
        assert!(!t.color_green.is_empty());
        assert!(!t.color_blue.is_empty());
        assert!(!t.color_red.is_empty());
        assert!(!t.add_note.is_empty());
        assert!(!t.edit_note.is_empty());
        assert!(!t.save_note.is_empty());
        assert!(!t.cancel_note.is_empty());
        assert!(!t.note_placeholder.is_empty());
        assert!(!t.delete_bookmark.is_empty());
        assert!(!t.return_to_reading.is_empty());
        assert!(!t.copy_all_bookmarks.is_empty());
        assert!(!t.export_markdown.is_empty());
        assert!(!t.bookmarks_copied.is_empty());
        assert!(!t.bookmarks_exported.is_empty());
        assert!(!t.no_bookmarks_hint.is_empty());
        assert!(!t.no_matching_bookmarks.is_empty());
        assert!(!t.bookmark_kind_quote.is_empty());
        assert!(!t.bookmark_kind_bookmark.is_empty());
    }
}
