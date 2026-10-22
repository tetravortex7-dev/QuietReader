use crate::document::{Document, Fb2Document};
use crate::settings::AppSettings;
use crate::test::util::unique_test_dir;

#[test]
fn test_platform_multi_extension_selection() {
    use crate::platform::SUPPORTED_EXTENSIONS;

    let exts: Vec<&str> = SUPPORTED_EXTENSIONS.to_vec();
    assert!(exts.contains(&"fb2"));
    assert!(exts.contains(&"txt"));
    assert!(exts.contains(&"doc"));
    assert!(exts.contains(&"docx"));
    assert!(exts.contains(&"epub"));
    assert!(exts.contains(&"mobi"));
    assert!(exts.contains(&"azw3"));
    assert!(exts.contains(&"rtf"));
    assert!(exts.contains(&"pdf"));
    assert!(exts.contains(&"djvu"));
    assert!(exts.contains(&"cbz"));
    assert!(exts.contains(&"cbr"));
    assert!(exts.contains(&"cb7"));
    assert!(exts.contains(&"md"));

    let mut settings = AppSettings::default();
    assert_eq!(settings.associated_extensions, vec!["fb2", "epub"]);

    settings.associated_extensions = vec!["fb2".to_string(), "txt".to_string(), "doc".to_string()];
    let json = serde_json::to_string(&settings).expect("serialize settings");
    let loaded: AppSettings = serde_json::from_str(&json).expect("deserialize settings");
    assert_eq!(loaded.associated_extensions, vec!["fb2", "txt", "doc"]);
}

#[test]
fn test_platform_keep_screen_on() {
    crate::platform::set_keep_screen_on(true);
    crate::platform::set_keep_screen_on(false);
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn test_xdg_downloads_dir_parsing() {
    use std::path::{Path, PathBuf};
    let home = Path::new("/home/reader");
    assert_eq!(
        crate::platform::downloads::parse_xdg_download_dir(
            "XDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\n",
            Some(home)
        ),
        Some(PathBuf::from("/home/reader/Downloads"))
    );
    assert_eq!(
        crate::platform::downloads::parse_xdg_download_dir(
            "XDG_DOWNLOAD_DIR=\"/mnt/books/downloads\"\n",
            Some(home)
        ),
        Some(PathBuf::from("/mnt/books/downloads"))
    );
    assert_eq!(
        crate::platform::downloads::parse_xdg_download_dir("XDG_DOWNLOAD_DIR=\"\"\n", Some(home)),
        None
    );
}

#[test]
fn test_system_downloads_dir_is_absolute() {
    assert!(crate::platform::downloads::resolve_downloads_dir().is_absolute());
}

#[test]
fn test_platform_file_association_query_and_dismiss() {
    let status = crate::platform::check_fb2_association_status();
    assert!(matches!(
        status,
        crate::platform::AssociationStatus::Default
            | crate::platform::AssociationStatus::Registered
            | crate::platform::AssociationStatus::NotRegistered
    ));

    crate::platform::set_association_prompt_dismissed(true);
    assert!(crate::platform::is_association_prompt_dismissed());
    crate::platform::set_association_prompt_dismissed(false);
    assert!(!crate::platform::is_association_prompt_dismissed());
}

#[test]
#[ignore = "РР·РјРµРЅСЏРµС‚ СЂРµР°Р»СЊРЅС‹Рµ СЃРёСЃС‚РµРјРЅС‹Рµ Р°СЃСЃРѕС†РёР°С†РёРё С„Р°Р№Р»РѕРІ РІ СЂРµРµСЃС‚СЂРµ РћРЎ"]
fn test_platform_register_association() {
    let result = crate::platform::register_fb2_association();
    assert!(result.is_ok());

    let status = crate::platform::check_fb2_association_status();
    assert!(
        status == crate::platform::AssociationStatus::Registered
            || status == crate::platform::AssociationStatus::Default
    );
}

#[cfg(windows)]
#[test]
fn test_windows_secret_storage_roundtrip() {
    let key = format!(
        "test_secret_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let value = "quietreader-secret-test";

    crate::platform::secrets::store(&key, value).expect("store secret");
    assert_eq!(
        crate::platform::secrets::load(&key)
            .expect("load secret")
            .as_deref(),
        Some(value)
    );
    crate::platform::secrets::delete(&key).expect("delete secret");
    assert_eq!(
        crate::platform::secrets::load(&key).expect("load deleted secret"),
        None
    );
}

#[test]
fn test_command_line_path_handling() {
    let temp_file = std::env::temp_dir().join(format!(
        "test_spaces_{}_{}.fb2",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let xml = r#"<FictionBook><body><section><title><p>Р“Р»Р°РІР° 1</p></title><p>РўРµРєСЃС‚</p></section></body></FictionBook>"#;
    std::fs::write(&temp_file, xml).unwrap();

    let doc = Fb2Document::from_file(&temp_file)
        .expect("Р”РѕР»Р¶РµРЅ РѕС‚РєСЂС‹С‚СЊ С„Р°Р№Р» РїРѕ РїСѓС‚Рё СЃ РїСЂРѕР±РµР»Р°РјРё");
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Р“Р»Р°РІР° 1");

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_ipc_payload_validation() {
    let dir = unique_test_dir("ipc_payload");
    let file = dir.join("book.fb2");
    std::fs::write(&file, b"test").unwrap();
    let canonical = std::fs::canonicalize(&file).unwrap();

    let payload = format!("ACTIVATE\n{}\n{}\n", file.display(), dir.display());
    let parsed = crate::platform::single_instance::parse_ipc_payload(&payload);
    assert_eq!(parsed, vec![canonical]);

    let invalid = format!("OPEN\n{}\n", file.display());
    assert!(crate::platform::single_instance::parse_ipc_payload(&invalid).is_empty());

    let repeated = format!("ACTIVATE\n{}", format!("{}\n", file.display()).repeat(40));
    assert_eq!(
        crate::platform::single_instance::parse_ipc_payload(&repeated).len(),
        32
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_ipc_private_file_limits() {
    let dir = unique_test_dir("ipc_file");
    let path = dir.join("request.req");

    crate::platform::single_instance::write_private_file(&path, b"ACTIVATE\n").unwrap();
    assert_eq!(
        crate::platform::single_instance::read_private_file(&path, false).as_deref(),
        Some("ACTIVATE\n")
    );
    assert!(crate::platform::single_instance::read_private_file(&path, true).is_some());
    assert!(!path.exists());

    let too_large = vec![b'x'; 70 * 1024];
    assert!(crate::platform::single_instance::write_private_file(&path, &too_large).is_err());
    assert!(!path.exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let target = dir.join("target.txt");
        std::fs::write(&target, b"secret").unwrap();
        let link = dir.join("link.req");
        symlink(&target, &link).unwrap();
        assert!(crate::platform::single_instance::read_private_file(&link, true).is_none());
        assert!(!link.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "secret");

        symlink(&target, &link).unwrap();
        crate::platform::single_instance::write_private_file(&link, b"ACTIVATE\n").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "secret");
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "ACTIVATE\n");
    }

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_create_private_dir() {
    let dir = unique_test_dir("private_dir");
    let nested = dir.join("a").join("b");
    crate::platform::permissions::create_private_dir(&nested);
    assert!(nested.is_dir());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&nested).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    let _ = std::fs::remove_dir_all(dir);
}
