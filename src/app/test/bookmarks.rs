use crate::app::input::process_keyboard_events;
use crate::app::theme::{bookmark_color, bookmark_highlight_color};
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
        bookmark_color(yellow),
        eframe::egui::Color32::from_rgb(235, 195, 60)
    );
    let y_light = bookmark_highlight_color(yellow, false);
    let y_dark = bookmark_highlight_color(yellow, true);
    assert_ne!(y_light.a(), 0);
    assert_ne!(y_dark.a(), 0);

    let green = BookmarkColor::Green;
    assert_eq!(
        bookmark_color(green),
        eframe::egui::Color32::from_rgb(72, 187, 120)
    );

    let blue = BookmarkColor::Blue;
    assert_eq!(
        bookmark_color(blue),
        eframe::egui::Color32::from_rgb(66, 153, 225)
    );

    let red = BookmarkColor::Red;
    assert_eq!(
        bookmark_color(red),
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
