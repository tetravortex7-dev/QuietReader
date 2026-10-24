mod config;
mod persistence;
mod secrets;

use super::*;
use crate::test::util::unique_test_dir;

#[test]
fn test_reading_ruler_settings_defaults() {
    let s = AppSettings::default();
    assert!(!s.reading_ruler);
    assert_eq!(s.reading_ruler_lines, 3);
    assert!((s.reading_ruler_opacity - 0.45).abs() < f32::EPSILON);
}

#[test]
fn test_reading_ruler_settings_serde() {
    let json = r#"{
        "reading_ruler": true,
        "reading_ruler_lines": 4,
        "reading_ruler_opacity": 0.65
    }"#;
    let s: AppSettings = serde_json::from_str(json).unwrap();
    assert!(s.reading_ruler);
    assert_eq!(s.reading_ruler_lines, 4);
    assert!((s.reading_ruler_opacity - 0.65).abs() < 1e-4);
}

#[test]
fn test_opds_catalogs_defaults() {
    let s = AppSettings::default();
    assert!(!s.opds_catalogs.is_empty());
    assert!(s.opds_catalogs.iter().any(|c| c.name.contains("Gutenberg")));
}

#[test]
fn test_settings_fallback_path_is_absolute() {
    let base = unique_test_dir("settings_fallback");
    let path = fallback_settings_path(base.clone());
    assert!(path.is_absolute());
    assert!(path.starts_with(&base));
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some("settings.json")
    );
    let _ = std::fs::remove_dir_all(base);
}
