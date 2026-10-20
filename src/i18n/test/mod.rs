use super::*;

#[test]
fn test_ui_languages_all() {
    assert_eq!(UiLanguage::ALL.len(), 13);
    assert_eq!(UiLanguage::CONCRETE.len(), 12);
    for &lang in &UiLanguage::ALL {
        assert!(!lang.code().is_empty());
        assert!(!lang.native_name().is_empty());
        assert_eq!(UiLanguage::from_code(lang.code()), lang);

        let t = lang.t();
        assert!(!t.open_file.is_empty());
        assert!(!t.settings.is_empty());
        assert!(!t.settings_title.is_empty());
        assert!(!t.tab_fonts.is_empty());
        assert!(!t.tab_appearance.is_empty());
        assert!(!t.interface_language.is_empty());
        assert!(!t.language_auto.is_empty());
        assert!(!t.speech.is_empty());
        assert!(!t.tts_title.is_empty());
        assert!(!t.copy.is_empty());
        assert!(!t.read_aloud_from_here.is_empty());

        let label = lang.label(t);
        assert!(!label.is_empty());
    }
}

#[test]
fn test_from_locale_str() {
    assert_eq!(UiLanguage::from_locale_str("ru_RU.UTF-8"), UiLanguage::Ru);
    assert_eq!(UiLanguage::from_locale_str("ru-RU"), UiLanguage::Ru);
    assert_eq!(UiLanguage::from_locale_str("be_BY"), UiLanguage::Ru);
    assert_eq!(UiLanguage::from_locale_str("uk_UA"), UiLanguage::Uk);
    assert_eq!(UiLanguage::from_locale_str("it_IT"), UiLanguage::It);
    assert_eq!(UiLanguage::from_locale_str("tr_TR"), UiLanguage::Tr);
    assert_eq!(UiLanguage::from_locale_str("ar_SA"), UiLanguage::Ar);
    assert_eq!(UiLanguage::from_locale_str("id_ID"), UiLanguage::Id);
    assert_eq!(UiLanguage::from_locale_str("en-US"), UiLanguage::En);
    assert_eq!(UiLanguage::from_locale_str("en_GB.UTF-8"), UiLanguage::En);
    assert_eq!(UiLanguage::from_locale_str("fr-FR"), UiLanguage::Fr);
    assert_eq!(UiLanguage::from_locale_str("es-ES"), UiLanguage::Es);
    assert_eq!(UiLanguage::from_locale_str("zh-CN"), UiLanguage::Zh);
    assert_eq!(UiLanguage::from_locale_str("ja_JP"), UiLanguage::Ja);
    assert_eq!(UiLanguage::from_locale_str("pt-BR"), UiLanguage::Pt);
    assert_eq!(UiLanguage::from_locale_str("de-DE"), UiLanguage::En);
}

#[test]
fn test_rtl_detection() {
    assert!(UiLanguage::Ar.is_rtl());
    assert!(!UiLanguage::Ru.is_rtl());
    assert!(!UiLanguage::En.is_rtl());
    assert!(!UiLanguage::Zh.is_rtl());
}

#[test]
fn test_formatting_and_plurals() {
    assert_eq!(plural_pages(1, UiLanguage::Ru), "1 страница");
    assert_eq!(plural_pages(3, UiLanguage::Ru), "3 страницы");
    assert_eq!(plural_pages(5, UiLanguage::Ru), "5 страниц");

    assert_eq!(plural_pages(1, UiLanguage::Uk), "1 сторінка");
    assert_eq!(plural_pages(3, UiLanguage::Uk), "3 сторінки");
    assert_eq!(plural_pages(5, UiLanguage::Uk), "5 сторінок");

    assert_eq!(plural_pages(1, UiLanguage::En), "1 page");
    assert_eq!(plural_pages(5, UiLanguage::En), "5 pages");

    assert_eq!(plural_pages(1, UiLanguage::It), "1 pagina");
    assert_eq!(plural_pages(5, UiLanguage::It), "5 pagine");

    assert_eq!(plural_pages(1, UiLanguage::Tr), "1 sayfa");
    assert_eq!(plural_pages(5, UiLanguage::Tr), "5 sayfa");

    assert_eq!(plural_pages(1, UiLanguage::Id), "1 halaman");
    assert_eq!(plural_pages(5, UiLanguage::Id), "5 halaman");

    assert_eq!(format_number(1234567, UiLanguage::Ru), "1 234 567");
    assert_eq!(format_number(1234567, UiLanguage::En), "1,234,567");

    assert_eq!(format_float(19.5, 1, UiLanguage::Ru), "19,5");
    assert_eq!(format_float(19.5, 1, UiLanguage::En), "19.5");

    assert_eq!(format_date(2026, 10, 19, UiLanguage::Ru), "19.10.2026");
    assert_eq!(format_date(2026, 10, 19, UiLanguage::En), "10/19/2026");

    let err = crate::document::DocumentError::Corrupted("test".to_string());
    assert!(format_document_error(&err, UiLanguage::Ru).contains("Повреждённый документ"));
    assert!(format_document_error(&err, UiLanguage::En).contains("Corrupted Document"));

    assert!(!format_extension_description("fb2", UiLanguage::Ru).is_empty());
    assert!(!format_extension_description("fb2", UiLanguage::En).is_empty());
}

#[test]
fn test_ui_language_serialization() {
    let serialized = serde_json::to_string(&UiLanguage::Zh).unwrap();
    assert_eq!(serialized, "\"zh\"");
    let deserialized: UiLanguage = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized, UiLanguage::Zh);

    let auto_ser = serde_json::to_string(&UiLanguage::Auto).unwrap();
    assert_eq!(auto_ser, "\"auto\"");
    let auto_de: UiLanguage = serde_json::from_str(&auto_ser).unwrap();
    assert_eq!(auto_de, UiLanguage::Auto);
}

#[test]
fn test_app_settings_language_defaults_and_deserialization() {
    let json_without_lang = r#"{
        "font_size": 18.0,
        "theme": "Sepia"
    }"#;
    let s: crate::settings::AppSettings = serde_json::from_str(json_without_lang).unwrap();
    assert_eq!(s.ui_language, UiLanguage::Auto);

    let json_with_ja = r#"{
        "font_size": 18.0,
        "theme": "Sepia",
        "ui_language": "ja"
    }"#;
    let s2: crate::settings::AppSettings = serde_json::from_str(json_with_ja).unwrap();
    assert_eq!(s2.ui_language, UiLanguage::Ja);
}

#[test]
fn test_reading_ruler_translations() {
    for lang in UiLanguage::CONCRETE {
        let t = lang.t();
        assert!(!t.reading_ruler.is_empty());
        assert!(!t.hide_reading_ruler.is_empty());
        assert!(!t.reading_ruler_tooltip.is_empty());
        assert!(!t.reading_ruler_title.is_empty());
        assert!(!t.reading_ruler_desc.is_empty());
        assert!(!t.reading_ruler_height_label.is_empty());
        assert!(!t.reading_ruler_dim_label.is_empty());
    }
}
