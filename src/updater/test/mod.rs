use super::*;

#[test]
fn test_version_parsing_and_comparison() {
    assert_eq!(parse_version_tuple("0.1.0"), Some((0, 1, 0)));
    assert_eq!(parse_version_tuple("v0.2.1"), Some((0, 2, 1)));
    assert_eq!(parse_version_tuple("1.0"), Some((1, 0, 0)));

    assert!(is_newer_version("0.1.0", "0.2.0"));
    assert!(is_newer_version("0.1.0", "v0.1.1"));
    assert!(is_newer_version("0.1.0", "1.0.0"));
    assert!(!is_newer_version("0.1.0", "0.1.0"));
    assert!(!is_newer_version("0.2.0", "0.1.9"));
    assert!(!is_newer_version("1.0.0", "0.9.9"));
}

#[test]
fn test_extract_version_from_cargo_toml() {
    let sample = r#"
[package]
name = "QuietReader"
version = "0.2.5"
edition = "2024"
"#;
    assert_eq!(
        extract_version_from_cargo_toml(sample),
        Some("0.2.5".to_string())
    );
}
