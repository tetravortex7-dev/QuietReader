mod persistence;

use super::*;

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
