use crate::settings::AppSettings;
use crate::test::util::unique_test_dir;
use crate::*;
use std::path::PathBuf;

#[test]
fn test_reading_progress_persistence_and_normalization() {
    let mut settings = AppSettings::default();
    let p1 = PathBuf::from("C:\\Books\\Library\\Novel.epub");
    let p1_lower = PathBuf::from("c:\\books\\library\\novel.epub");
    let p2 = PathBuf::from("D:\\Comics\\Manga_Vol1.cbz");

    settings.record_book_progress(&p1, "Novel Title", "Author Name", 4, 320.0);
    assert_eq!(settings.get_book_progress(&p1), Some((4, 320.0)));
    assert_eq!(settings.get_book_progress(&p1_lower), Some((4, 320.0)));

    settings.record_book_progress(&p2, "Manga Title", "Mangaka", 12, 0.0);
    assert_eq!(settings.get_book_progress(&p2), Some((12, 0.0)));

    assert_eq!(settings.reading_progress.len(), 2);
    assert_eq!(settings.recent_books.len(), 2);

    settings.recent_books.clear();
    assert_eq!(settings.recent_books.len(), 0);
    assert_eq!(settings.get_book_progress(&p1), Some((4, 320.0)));
    assert_eq!(settings.get_book_progress(&p2), Some((12, 0.0)));
}

#[test]
fn test_atomic_settings_save_and_backup_safety() {
    let temp_dir = unique_test_dir("save_safety");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let corrupted_path = temp_dir.join("settings.corrupted");

    let mut settings = AppSettings::default();
    settings.font_size = 28.0;
    settings.theme = "Sepia".to_string();

    let save_res = settings.save_to_path(&settings_path);
    assert!(save_res.is_ok());
    assert!(settings_path.exists());

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded.font_size, 28.0);
    assert_eq!(loaded.theme, "Sepia");

    std::fs::write(
        &bak_path,
        r#"{"schema_version":2,"font_size":25.0,"theme":"Dark"}"#,
    )
    .unwrap();
    std::fs::write(&settings_path, "{ broken invalid json...").unwrap();

    let recovered = AppSettings::load_from_path(&settings_path);
    assert_eq!(recovered.font_size, 25.0);
    assert_eq!(recovered.theme, "Dark");
    assert!(corrupted_path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_font_family_settings_and_serialization() {
    let default_settings = AppSettings::default();
    assert_eq!(default_settings.font_family, "Georgia");

    let old_json = r#"{"window":{"width":800.0,"height":600.0,"maximized":false},"recent_books":[],"font_size":18.0,"theme":"Sepia","keep_screen_on":true,"show_toc":true}"#;
    let loaded: AppSettings =
        serde_json::from_str(old_json).expect("Должен десериализоваться со значением по умолчанию");
    assert_eq!(loaded.font_family, "Georgia");

    let custom_settings = AppSettings {
        font_family: "Palatino Linotype".to_string(),
        ..Default::default()
    };
    let serialized = serde_json::to_string(&custom_settings).unwrap();
    let deserialized: AppSettings = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.font_family, "Palatino Linotype");

    let font_ids: Vec<&str> = crate::app::fonts::AVAILABLE_FONTS
        .iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(
        font_ids,
        vec![
            "Georgia",
            "Palatino Linotype",
            "Calibri",
            "Segoe UI",
            "Verdana",
            "Comic Sans MS",
            "Sitka",
            "OpenDyslexic",
            "Lexend"
        ]
    );
}

#[test]
fn test_corrupted_settings_json_backup_and_recovery() {
    let temp_dir = unique_test_dir("corrupted_settings");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let corrupted_content = "{ \"font_size\": BROKEN_JSON !!!";
    std::fs::write(&settings_path, corrupted_content).unwrap();

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded, AppSettings::default());
    assert!(bak_path.exists());
    assert_eq!(
        std::fs::read_to_string(&bak_path).unwrap(),
        corrupted_content
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_settings_schema_version_migration_with_backup() {
    let temp_dir = unique_test_dir("schema_migration");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let legacy_json = r#"{"window":{"width":900.0,"height":700.0,"maximized":false},"recent_books":[],"font_size":21.0,"theme":"Dark","keep_screen_on":false,"show_toc":true}"#;
    std::fs::write(&settings_path, legacy_json).unwrap();

    let migrated = AppSettings::load_from_path(&settings_path);
    assert_eq!(
        migrated.schema_version,
        settings::CURRENT_SETTINGS_SCHEMA_VERSION
    );
    assert_eq!(migrated.font_size, 21.0);
    assert_eq!(migrated.theme, "Dark");
    assert!(bak_path.exists());
    assert_eq!(std::fs::read_to_string(&bak_path).unwrap(), legacy_json);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_settings_atomic_save_and_reload() {
    let temp_dir = unique_test_dir("atomic_save");
    let settings_path = temp_dir.join("settings.json");
    let mut settings = AppSettings::default();
    settings.font_size = 23.5;
    settings.theme = "Mint".to_string();

    let res = settings.save_to_path(&settings_path);
    assert!(res.is_ok());
    assert!(settings_path.exists());

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded.font_size, 23.5);
    assert_eq!(loaded.theme, "Mint");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_settings_corrupted_backup_protection() {
    let temp_dir = unique_test_dir("bak_protection");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");

    let valid_backup_json = r#"{"schema_version":2,"font_size":27.0,"theme":"Oled"}"#;
    std::fs::write(&bak_path, valid_backup_json).unwrap();

    let broken_json = "{ truncated invalid json ...";
    std::fs::write(&settings_path, broken_json).unwrap();

    let restored = AppSettings::load_from_path(&settings_path);
    assert_eq!(restored.font_size, 27.0);
    assert_eq!(restored.theme, "Oled");

    let bak_content = std::fs::read_to_string(&bak_path).unwrap();
    assert_eq!(bak_content, valid_backup_json);

    let corrupted_path = temp_dir.join("settings.corrupted");
    assert!(corrupted_path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_reading_progress_separation_from_recent_books() {
    let mut settings = AppSettings::default();

    for i in 1..=30 {
        let p = std::path::PathBuf::from(format!("C:\\Books\\book_{i}.epub"));
        settings.record_book_progress(&p, &format!("Книга {i}"), "Автор", i, (i * 10) as f32);
    }

    assert_eq!(settings.recent_books.len(), 25);
    assert_eq!(settings.reading_progress.len(), 30);

    let old_book_path = std::path::PathBuf::from("C:\\Books\\book_1.epub");
    assert!(
        settings
            .recent_books
            .iter()
            .all(|b| b.path != old_book_path)
    );
    assert_eq!(settings.get_book_progress(&old_book_path), Some((1, 10.0)));

    settings.recent_books.clear();
    assert_eq!(settings.recent_books.len(), 0);
    assert_eq!(settings.get_book_progress(&old_book_path), Some((1, 10.0)));
}

#[test]
fn test_settings_new_fields_default_compatibility() {
    let minimal_json = r#"{"font_size": 22.0}"#;
    let settings: AppSettings = serde_json::from_str(minimal_json)
        .expect("Десериализация с дефолтами должна пройти успешно");

    assert_eq!(settings.font_size, 22.0);
    assert_eq!(settings.font_family, "Georgia");
    assert_eq!(settings.theme, "Sepia");
    assert!(settings.keep_screen_on);
    assert!(settings.show_toc);
    assert_eq!(settings.comic.reading_mode, "Auto");
    assert_eq!(settings.pdf.layout, "Single");
    assert!(settings.reading_progress.is_empty());
}

#[test]
fn test_settings_save_debounce_state_and_flush() {
    use crate::app::QuietReaderApp;
    use crate::settings::AppSettings;

    let mut app = QuietReaderApp::from_settings(AppSettings::default());
    assert!(!app.settings_dirty);

    app.mark_settings_dirty();
    assert!(app.settings_dirty);

    app.flush_debounced_settings();
    assert!(app.settings_dirty);

    app.flush_settings_now();
    assert!(!app.settings_dirty);
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
fn test_reading_progress_case_insensitive_lookup() {
    use std::path::PathBuf;
    let mut settings = settings::AppSettings::default();
    let path_upper = PathBuf::from("C:\\Books\\MyBook.epub");
    let path_lower = PathBuf::from("c:\\books\\mybook.epub");

    settings.record_book_progress(&path_upper, "My Book", "Author", 3, 145.5);
    let prog = settings.get_book_progress(&path_lower);
    assert_eq!(prog, Some((3, 145.5)));
}
