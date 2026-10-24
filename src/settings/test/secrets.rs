use super::*;
use crate::test::util::unique_test_dir;
use serde_json::json;

#[test]
fn test_secret_fields_are_not_serialized() {
    let mut settings = AppSettings::default();
    settings.translation.deepl_api_key = "deepl-secret-value".to_string();
    settings.opds_catalogs[0].password = Some("opds-secret-value".to_string());
    settings.license_key = "license-secret-value".to_string();

    let json = serde_json::to_string(&settings).unwrap();

    assert!(!json.contains("deepl-secret-value"));
    assert!(!json.contains("opds-secret-value"));
    assert!(!json.contains("license-secret-value"));
    assert!(!json.contains("deepl_api_key"));
    assert!(!json.contains("\"password\""));
    assert!(!json.contains("license_key"));
}

#[test]
fn test_keyring_failure_does_not_overwrite_settings() {
    let dir = unique_test_dir("settings_keyring_failure");
    let path = dir.join("settings.json");
    let mut settings = AppSettings::default();
    settings.save_to_path(&path).expect("initial save");
    let original = std::fs::read_to_string(&path).expect("read original");

    settings.reading_ruler = !settings.reading_ruler;
    let result = settings.save_with_secret_result(&path, Err("keyring unavailable".to_string()));

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(&path).expect("read preserved"),
        original
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_legacy_secret_extraction() {
    let content = r#"{
        "translation": { "deepl_api_key": "deepl-legacy" },
        "license_key": "license-legacy",
        "opds_catalogs": [
            { "id": "catalog-one", "password": "opds-legacy" },
            { "id": "catalog-two", "password": "" }
        ]
    }"#;

    let entries = AppSettings::extract_legacy_secret_entries(content);

    assert!(entries.contains(&("deepl_api_key".to_string(), "deepl-legacy".to_string())));
    assert!(entries.contains(&("license_key".to_string(), "license-legacy".to_string())));
    assert!(entries.contains(&(
        "opds_password:catalog-one".to_string(),
        "opds-legacy".to_string()
    )));
    assert!(!entries.iter().any(|(key, _)| key.contains("catalog-two")));
}

#[test]
fn test_secret_fields_are_sanitized() {
    let content = r#"{
        "translation": { "provider": "Google", "deepl_api_key": "deepl-legacy" },
        "opds_catalogs": [
            { "id": "catalog-one", "name": "One", "password": "opds-legacy" }
        ]
    }"#;

    let sanitized = AppSettings::sanitize_secret_fields(content).unwrap();

    assert!(!sanitized.contains("deepl-legacy"));
    assert!(!sanitized.contains("opds-legacy"));
    assert!(!sanitized.contains("deepl_api_key"));
    assert!(!sanitized.contains("\"password\""));
    assert!(sanitized.contains("\"provider\": \"Google\""));
}

#[test]
fn test_corrupted_json_secret_redaction() {
    let content = r#"{"translation":{"deepl_api_key":"deepl-broken"},"license_key":"license-broken","opds_catalogs":[{"id":"c","password":"opds-broken"}"#;

    let sanitized = AppSettings::sanitize_secret_fields(content).unwrap();

    assert!(!sanitized.contains("deepl-broken"));
    assert!(!sanitized.contains("opds-broken"));
    assert!(!sanitized.contains("license-broken"));
}

#[test]
fn test_migration_backup_does_not_contain_secrets() {
    let dir = unique_test_dir("settings_secrets_backup");
    let path = dir.join("settings.json");
    let backup_path = AppSettings::make_backup_path(&path);

    let mut value = serde_json::to_value(AppSettings::default()).unwrap();
    value["schema_version"] = json!(0);
    value["translation"]["deepl_api_key"] = json!("deepl-legacy");
    value["opds_catalogs"][0]["password"] = json!("opds-legacy");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();

    let settings = AppSettings::load_from_path(&path);
    assert_eq!(settings.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);

    let saved = std::fs::read_to_string(&path).unwrap();
    let backup = std::fs::read_to_string(&backup_path).unwrap();
    assert!(!saved.contains("deepl-legacy"));
    assert!(!saved.contains("opds-legacy"));
    assert!(!backup.contains("deepl-legacy"));
    assert!(!backup.contains("opds-legacy"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_existing_backup_is_sanitized() {
    let dir = unique_test_dir("settings_secrets_existing_backup");
    let path = dir.join("settings.json");
    let backup_path = AppSettings::make_backup_path(&path);

    let legacy = r#"{
        "translation": { "deepl_api_key": "backup-deepl" },
        "opds_catalogs": [ { "id": "catalog", "password": "backup-opds" } ]
    }"#;
    std::fs::write(&backup_path, legacy).unwrap();

    AppSettings::default().save_to_path(&path).unwrap();

    let backup = std::fs::read_to_string(&backup_path).unwrap();
    assert!(!backup.contains("backup-deepl"));
    assert!(!backup.contains("backup-opds"));

    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn test_saved_settings_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = unique_test_dir("settings_secrets_permissions");
    let path = dir.join("settings.json");

    AppSettings::default().save_to_path(&path).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    let _ = std::fs::remove_dir_all(dir);
}
