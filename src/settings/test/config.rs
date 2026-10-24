use crate::settings::{
    AutoScrollSettings, BookProgress, ComicSettings, PdfSettings, RecentBook, TranslationProvider,
    TranslationSettings, TtsSettings, WindowState,
};

#[test]
fn test_moved_settings_structs_keep_defaults() {
    let window = WindowState::default();
    assert_eq!(window.width, 1050.0);
    assert_eq!(window.height, 750.0);
    assert!(window.x.is_none());

    let autoscroll = AutoScrollSettings::default();
    assert!(autoscroll.pause_mode);
    assert_eq!(autoscroll.step_distance, 90.0);

    let comic = ComicSettings::default();
    assert_eq!(comic.reading_mode, "Auto");
    assert_eq!(comic.auto_crop_tolerance, 24);

    let pdf = PdfSettings::default();
    assert_eq!(pdf.layout, "Single");
    assert_eq!(pdf.auto_crop_tolerance, 24);

    let tts = TtsSettings::default();
    assert_eq!(tts.engine, "system");
    assert!(tts.voice_id.is_none());
    assert_eq!(tts.rate, 1.0);
}

#[test]
fn test_moved_settings_structs_serde_roundtrip() {
    let progress = BookProgress {
        chapter: 4,
        scroll_y: 128.5,
        paragraph: Some(7),
        timestamp: 42,
    };
    let json = serde_json::to_string(&progress).unwrap();
    let loaded: BookProgress = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, progress);

    let recent = RecentBook {
        path: std::path::PathBuf::from("book.epub"),
        title: "Book".to_string(),
        author: "Author".to_string(),
        chapter: 1,
        scroll_y: 2.0,
        timestamp: 3,
    };
    let json = serde_json::to_string(&recent).unwrap();
    let loaded: RecentBook = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, recent);

    let tts = TtsSettings {
        engine: "edge".to_string(),
        voice_id: Some("ru-RU-DmitryNeural".to_string()),
        rate: 1.25,
    };
    let json = serde_json::to_string(&tts).unwrap();
    let loaded: TtsSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded, tts);
}

#[test]
fn test_translation_key_is_not_serialized_or_debugged() {
    let mut settings = TranslationSettings::default();
    settings.provider = TranslationProvider::DeepL;
    settings.deepl_api_key = "super-secret".to_string();

    let json = serde_json::to_string(&settings).unwrap();
    assert!(json.contains("DeepL"));
    assert!(!json.contains("super-secret"));

    let debug = format!("{settings:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("super-secret"));
}

#[test]
fn test_translation_key_is_ignored_on_load() {
    let json = r#"{"provider":"DeepL","deepl_api_key":"leaked"}"#;
    let settings: TranslationSettings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.provider, TranslationProvider::DeepL);
    assert!(settings.deepl_api_key.is_empty());
}
