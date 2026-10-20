use crate::document::{Document, Fb2Document};
use crate::settings::AppSettings;

#[test]
fn test_platform_multi_extension_selection() {
    use crate::platform::SUPPORTED_EXTENSIONS_INFO;

    let exts: Vec<&str> = SUPPORTED_EXTENSIONS_INFO.iter().map(|(e, _)| *e).collect();
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
