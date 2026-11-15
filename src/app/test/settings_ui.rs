use crate::app::settings_view::{
    SettingsSectionId, apply_default_app_preferences, compute_font_grid_cols,
    compute_theme_swatch_cols, matching_sections,
};
use crate::app::theme::ReaderTheme;
use crate::core::Bookmark;
use crate::core::library::{LibraryEntry, LibraryMode, LibraryStore, UiDesignMode};
use crate::i18n::UiLanguage;
use crate::settings::{AppSettings, BookProgress, SavedWord, TranslationProvider};
use std::path::PathBuf;

#[test]
fn test_responsive_swatch_and_font_columns() {
    assert_eq!(compute_theme_swatch_cols(800.0), 7);
    assert_eq!(compute_theme_swatch_cols(680.0), 7);
    assert_eq!(compute_theme_swatch_cols(500.0), 4);
    assert_eq!(compute_theme_swatch_cols(340.0), 2);

    assert_eq!(compute_font_grid_cols(760.0), 3);
    assert_eq!(compute_font_grid_cols(620.0), 3);
    assert_eq!(compute_font_grid_cols(450.0), 2);
    assert_eq!(compute_font_grid_cols(320.0), 1);
}

#[test]
fn test_settings_search_matches_sections_across_locales() {
    let ru = UiLanguage::Ru.t();
    let en = UiLanguage::En.t();

    assert!(matching_sections("   ", &ru).is_empty());
    assert!(matching_sections("oled", &ru).contains(&SettingsSectionId::ThemeAndDisplay));
    assert!(matching_sections("уютный", &ru).contains(&SettingsSectionId::ThemeAndDisplay));
    assert!(matching_sections("студия", &ru).contains(&SettingsSectionId::ThemeAndDisplay));
    assert!(matching_sections("свиток", &ru).contains(&SettingsSectionId::Typography));
    assert!(matching_sections("разворот", &ru).contains(&SettingsSectionId::Typography));
    assert!(matching_sections("bionic", &en).contains(&SettingsSectionId::FocusTools));
    assert!(matching_sections("DeepL", &ru).contains(&SettingsSectionId::Translation));
    assert!(matching_sections("UTF-8", &en).contains(&SettingsSectionId::Encoding));
    assert!(matching_sections("PIN", &ru).contains(&SettingsSectionId::LibraryPin));
    assert!(matching_sections("unrar", &en).contains(&SettingsSectionId::AboutVersion));
    assert!(matching_sections("zzz_not_found_123", &en).is_empty());
}

#[test]
fn test_apply_default_app_preferences_preserves_user_data() {
    let mut settings = AppSettings::default();
    let book_path = PathBuf::from("books/sample.epub");
    settings.recent_books.clear();
    settings.reading_progress.insert(
        book_path.clone(),
        BookProgress {
            chapter: 4,
            scroll_y: 240.0,
            paragraph: Some(12),
            timestamp: 1_700_000_000,
        },
    );
    settings.bookmarks.push(Bookmark {
        id: "bm-1".to_string(),
        book_path: book_path.clone(),
        chapter: 4,
        scroll_y: 240.0,
        label: "Chapter 4".to_string(),
        text: "Excerpt".to_string(),
        note: String::new(),
        color: crate::core::BookmarkColor::Yellow,
        ..Bookmark::default()
    });
    settings.saved_words.push(SavedWord {
        word: "ephemeral".to_string(),
        definition: "lasting for a very short time".to_string(),
        pack: "en".to_string(),
        language: "en".to_string(),
        book: "Sample".to_string(),
        created_at: 1_700_000_000,
    });
    settings.set_library_pin("1234");
    settings.translation.deepl_api_key = "secret-key".to_string();
    settings.translation.provider = TranslationProvider::DeepL;
    settings.license_key = "test-license-key".to_string();

    settings.font_size = 28.0;
    settings.line_height_scale = 1.9;
    settings.max_column_width = 1100.0;
    settings.reading_brightness = 0.55;
    settings.text_two_columns = false;
    settings.bionic_reading = true;
    settings.ui_language = UiLanguage::Ja;
    settings.theme = "Oled".to_string();

    let mut library = LibraryStore::default();
    library.design = UiDesignMode::Studio;
    library.mode = LibraryMode::ReaderOnly;
    library.entries.push(LibraryEntry {
        path: book_path.clone(),
        title: "Sample".to_string(),
        author: "Author".to_string(),
        ..LibraryEntry::default()
    });
    let mut library_dirty = false;
    let mut theme = ReaderTheme::Oled;
    let mut font_size = 28.0;
    let mut keep_screen_on = false;

    apply_default_app_preferences(
        &mut settings,
        &mut library,
        &mut library_dirty,
        &mut theme,
        &mut font_size,
        &mut keep_screen_on,
    );

    assert_eq!(theme, ReaderTheme::Sepia);
    assert_eq!(font_size, AppSettings::default().font_size);
    assert!(keep_screen_on);
    assert_eq!(settings.font_size, AppSettings::default().font_size);
    assert_eq!(
        settings.line_height_scale,
        AppSettings::default().line_height_scale
    );
    assert_eq!(
        settings.max_column_width,
        AppSettings::default().max_column_width
    );
    assert_eq!(
        settings.text_two_columns,
        AppSettings::default().text_two_columns
    );
    assert!(settings.text_two_columns);
    assert_eq!(settings.ui_language, UiLanguage::Auto);
    assert_eq!(settings.translation.provider, TranslationProvider::Google);
    assert_eq!(library.design, UiDesignMode::Cozy);
    assert_eq!(library.mode, LibraryMode::Shelf);
    assert!(library_dirty);

    assert_eq!(library.entries.len(), 1);
    assert!(settings.reading_progress.contains_key(&book_path));
    assert_eq!(settings.bookmarks.len(), 1);
    assert_eq!(settings.saved_words.len(), 1);
    assert!(settings.has_library_pin());
    assert_eq!(settings.translation.deepl_api_key, "secret-key");
    assert_eq!(settings.license_key, "test-license-key");
}
