use serde::{Deserialize, Serialize};

pub mod ar;
pub mod en;
pub mod es;
pub mod format;
pub mod fr;
pub mod id;
pub mod it;
pub mod ja;
pub mod pt;
pub mod ru;
pub mod tr;
pub mod uk;
pub mod zh;

pub use format::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    #[default]
    Auto,
    Ru,
    En,
    Fr,
    Es,
    Zh,
    Ja,
    Pt,
    It,
    Tr,
    Uk,
    Ar,
    Id,
}

impl UiLanguage {
    pub const ALL: [UiLanguage; 13] = [
        UiLanguage::Auto,
        UiLanguage::Ru,
        UiLanguage::En,
        UiLanguage::Fr,
        UiLanguage::Es,
        UiLanguage::Zh,
        UiLanguage::Ja,
        UiLanguage::Pt,
        UiLanguage::It,
        UiLanguage::Tr,
        UiLanguage::Uk,
        UiLanguage::Ar,
        UiLanguage::Id,
    ];

    pub const CONCRETE: [UiLanguage; 12] = [
        UiLanguage::Ru,
        UiLanguage::En,
        UiLanguage::Fr,
        UiLanguage::Es,
        UiLanguage::Zh,
        UiLanguage::Ja,
        UiLanguage::Pt,
        UiLanguage::It,
        UiLanguage::Tr,
        UiLanguage::Uk,
        UiLanguage::Ar,
        UiLanguage::Id,
    ];

    pub fn is_rtl(&self) -> bool {
        self.resolve() == Self::Ar
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ru => "ru",
            Self::En => "en",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::Zh => "zh",
            Self::Ja => "ja",
            Self::Pt => "pt",
            Self::It => "it",
            Self::Tr => "tr",
            Self::Uk => "uk",
            Self::Ar => "ar",
            Self::Id => "id",
        }
    }

    pub fn native_name(&self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Ru => "Русский",
            Self::En => "English",
            Self::Fr => "Français",
            Self::Es => "Español",
            Self::Zh => "简体中文",
            Self::Ja => "日本語",
            Self::Pt => "Português",
            Self::It => "Italiano",
            Self::Tr => "Türkçe",
            Self::Uk => "Українська",
            Self::Ar => "العربية",
            Self::Id => "Bahasa Indonesia",
        }
    }

    pub fn label(&self, t: &Translations) -> String {
        match self {
            Self::Auto => {
                let detected = Self::detect_system();
                format!("{} ({})", t.language_auto, detected.native_name())
            }
            concrete => concrete.native_name().to_string(),
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_lowercase().as_str() {
            "auto" => Self::Auto,
            "ru" => Self::Ru,
            "en" => Self::En,
            "fr" => Self::Fr,
            "es" => Self::Es,
            "zh" | "zh-cn" | "zh-hans" => Self::Zh,
            "ja" => Self::Ja,
            "pt" | "pt-br" | "pt-pt" => Self::Pt,
            "it" => Self::It,
            "tr" => Self::Tr,
            "uk" => Self::Uk,
            "ar" => Self::Ar,
            "id" | "in" => Self::Id,
            _ => Self::Auto,
        }
    }

    pub fn from_locale_str(locale: &str) -> Self {
        let clean = locale.trim().to_lowercase().replace('_', "-");
        let prefix = clean.split(&['-', '.'][..]).next().unwrap_or("");
        match prefix {
            "ru" | "be" | "kk" => Self::Ru,
            "uk" => Self::Uk,
            "en" => Self::En,
            "fr" => Self::Fr,
            "es" => Self::Es,
            "zh" => Self::Zh,
            "ja" => Self::Ja,
            "pt" => Self::Pt,
            "it" => Self::It,
            "tr" => Self::Tr,
            "ar" => Self::Ar,
            "id" | "in" => Self::Id,
            _ => Self::En,
        }
    }

    pub fn detect_system() -> Self {
        if let Some(locale) = crate::platform::detect_system_ui_language() {
            Self::from_locale_str(&locale)
        } else {
            Self::En
        }
    }

    pub fn resolve(&self) -> Self {
        match self {
            Self::Auto => Self::detect_system(),
            concrete => *concrete,
        }
    }

    pub fn t(&self) -> &'static Translations {
        match self.resolve() {
            Self::En => &en::TRANSLATIONS,
            Self::Ru => &ru::TRANSLATIONS,
            Self::Fr => &fr::TRANSLATIONS,
            Self::Es => &es::TRANSLATIONS,
            Self::Zh => &zh::TRANSLATIONS,
            Self::Ja => &ja::TRANSLATIONS,
            Self::Pt => &pt::TRANSLATIONS,
            Self::It => &it::TRANSLATIONS,
            Self::Tr => &tr::TRANSLATIONS,
            Self::Uk => &uk::TRANSLATIONS,
            Self::Ar => &ar::TRANSLATIONS,
            Self::Id => &id::TRANSLATIONS,
            Self::Auto => &en::TRANSLATIONS,
        }
    }
}

pub fn t(lang: UiLanguage) -> &'static Translations {
    lang.t()
}

#[derive(Debug, Clone, Copy)]
pub struct Translations {
    pub open_file: &'static str,
    pub open_file_tooltip: &'static str,
    pub open_folder: &'static str,
    pub open_folder_tooltip: &'static str,
    pub recent_books: &'static str,
    pub recent_books_tooltip: &'static str,
    pub toc: &'static str,
    pub hide_toc: &'static str,
    pub search: &'static str,
    pub search_tooltip: &'static str,
    pub speech: &'static str,
    pub speech_tooltip: &'static str,
    pub fullscreen: &'static str,
    pub fullscreen_tooltip: &'static str,
    pub windowed: &'static str,
    pub settings: &'static str,
    pub settings_tooltip: &'static str,

    pub ready_to_read: &'static str,
    pub page_prefix: &'static str,
    pub page_of: &'static str,
    pub section_x_of_y: &'static str,
    pub close_tab: &'static str,
    pub tab_format: &'static str,
    pub tab_path: &'static str,
    pub tab_close_hint: &'static str,

    pub app_subtitle: &'static str,
    pub choose_book_btn: &'static str,
    pub open_manga_folder_btn: &'static str,
    pub images_folder_hint: &'static str,
    pub drag_drop_hint: &'static str,
    pub recent_books_header: &'static str,
    pub loading_doc: &'static str,
    pub please_wait: &'static str,
    pub failed_open_doc: &'static str,

    pub settings_title: &'static str,
    pub back_to_reading: &'static str,
    pub save_settings_error: &'static str,
    pub tab_fonts: &'static str,
    pub tab_appearance: &'static str,
    pub tab_autoscroll: &'static str,
    pub tab_comics: &'static str,
    pub tab_hotkeys: &'static str,
    pub tab_system: &'static str,

    pub font_choice: &'static str,
    pub font_size: &'static str,
    pub font_decrease_tip: &'static str,
    pub font_increase_tip: &'static str,
    pub live_preview: &'static str,
    pub sample_text: &'static str,

    pub theme_title: &'static str,
    pub interface_language: &'static str,
    pub interface_language_desc: &'static str,
    pub language_auto: &'static str,
    pub keep_screen_on: &'static str,
    pub keep_screen_on_desc: &'static str,

    pub autoscroll_title: &'static str,
    pub autoscroll_desc: &'static str,
    pub autoscroll_stop: &'static str,
    pub autoscroll_start: &'static str,
    pub motion_type: &'static str,
    pub mode_pause: &'static str,
    pub mode_continuous: &'static str,
    pub pause_duration: &'static str,
    pub step_distance: &'static str,
    pub scroll_speed: &'static str,

    pub comics_title: &'static str,
    pub mode_auto: &'static str,
    pub mode_auto_desc: &'static str,
    pub mode_manga: &'static str,
    pub mode_manga_desc: &'static str,
    pub mode_comic: &'static str,
    pub mode_comic_desc: &'static str,
    pub mode_webtoon: &'static str,
    pub mode_webtoon_desc: &'static str,
    pub layout_title: &'static str,
    pub layout_single: &'static str,
    pub layout_double: &'static str,
    pub fit_title: &'static str,
    pub fit_window: &'static str,
    pub fit_width: &'static str,
    pub fit_height: &'static str,

    pub system_title: &'static str,
    pub assoc_title: &'static str,
    pub assoc_hint: &'static str,
    pub select_all: &'static str,
    pub deselect_all: &'static str,
    pub only_books: &'static str,
    pub status_default: &'static str,
    pub status_registered: &'static str,
    pub register_btn: &'static str,
    pub dont_show_assoc: &'static str,
    pub settings_storage: &'static str,
    pub save_settings_now: &'static str,

    pub tts_title: &'static str,
    pub tts_system: &'static str,
    pub tts_system_tip: &'static str,
    pub tts_edge: &'static str,
    pub tts_internet_tip: &'static str,
    pub tts_unavailable: &'static str,
    pub tts_open_text_book: &'static str,
    pub tts_prev_paragraph: &'static str,
    pub tts_next_paragraph: &'static str,
    pub tts_pause: &'static str,
    pub tts_play: &'static str,
    pub tts_stop: &'static str,
    pub tts_speed: &'static str,
    pub tts_voice: &'static str,
    pub tts_close_panel: &'static str,

    pub prev_page: &'static str,
    pub next_page: &'static str,
    pub zoom_in: &'static str,
    pub zoom_out: &'static str,
    pub zoom_reset: &'static str,
    pub fit_width_btn: &'static str,
    pub fit_height_btn: &'static str,
    pub fit_window_btn: &'static str,
    pub original_size: &'static str,
    pub single_page_btn: &'static str,
    pub double_page_btn: &'static str,
    pub scroll_btn: &'static str,
    pub manga_mode_btn: &'static str,
    pub comic_mode_btn: &'static str,
    pub rotate_clockwise: &'static str,
    pub rotate_counter: &'static str,
    pub no_pages_in_doc: &'static str,
    pub page_load_error: &'static str,
    pub page_loading: &'static str,

    pub search_in_book: &'static str,
    pub search_placeholder: &'static str,
    pub search_all_chapters: &'static str,
    pub search_current_chapter: &'static str,
    pub search_found_prefix: &'static str,
    pub search_nothing_found: &'static str,
    pub search_searching: &'static str,
    pub search_prev: &'static str,
    pub search_next: &'static str,

    pub toc_title: &'static str,
    pub toc_search_placeholder: &'static str,
    pub toc_no_chapters: &'static str,
    pub toc_close: &'static str,

    pub reading_history: &'static str,
    pub clear_history: &'static str,
    pub clear_history_tip: &'static str,
    pub history_empty: &'static str,
    pub read_now_btn: &'static str,
    pub book_open_now: &'static str,
    pub close_btn: &'static str,

    pub copy: &'static str,
    pub copy_section: &'static str,
    pub prev_section: &'static str,
    pub next_section: &'static str,
    pub read_aloud_from_here: &'static str,
    pub section_num: &'static str,
    pub status_settings_saved: &'static str,

    pub assoc_dialog_title: &'static str,
    pub assoc_dialog_heading: &'static str,
    pub assoc_dialog_text: &'static str,
    pub remind_later: &'static str,
    pub apply_assoc: &'static str,

    pub format_all_supported: &'static str,
    pub format_word: &'static str,
    pub format_rtf: &'static str,
    pub format_pdf: &'static str,
    pub format_djvu: &'static str,
    pub format_comics: &'static str,
    pub format_epub: &'static str,
    pub format_fb2: &'static str,
    pub format_txt: &'static str,
    pub format_desc_fb2: &'static str,
    pub format_desc_epub: &'static str,
    pub format_desc_txt: &'static str,
    pub format_desc_doc: &'static str,
    pub format_desc_docx: &'static str,
    pub format_desc_rtf: &'static str,
    pub format_desc_pdf: &'static str,
    pub format_desc_djvu: &'static str,
    pub format_desc_cbz: &'static str,
    pub format_desc_cbr: &'static str,
    pub format_desc_cb7: &'static str,
    pub format_desc_md: &'static str,
    pub choose_manga_folder_title: &'static str,

    pub err_io: &'static str,
    pub err_unsupported: &'static str,
    pub err_corrupted: &'static str,
    pub err_limit_exceeded: &'static str,
    pub err_unknown: &'static str,

    pub theme_sepia: &'static str,
    pub theme_twilight: &'static str,
    pub theme_warm: &'static str,
    pub theme_mint: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub theme_oled: &'static str,

    pub theme_sepia_desc: &'static str,
    pub theme_twilight_desc: &'static str,
    pub theme_warm_desc: &'static str,
    pub theme_mint_desc: &'static str,
    pub theme_light_desc: &'static str,
    pub theme_dark_desc: &'static str,
    pub theme_oled_desc: &'static str,

    pub word_pages_one: &'static str,
    pub word_pages_few: &'static str,
    pub word_pages_many: &'static str,
    pub word_chapters_one: &'static str,
    pub word_chapters_few: &'static str,
    pub word_chapters_many: &'static str,
    pub word_matches_one: &'static str,
    pub word_matches_few: &'static str,
    pub word_matches_many: &'static str,
    pub word_books_one: &'static str,
    pub word_books_few: &'static str,
    pub word_books_many: &'static str,

    pub date_today: &'static str,
    pub date_yesterday: &'static str,
    pub date_just_now: &'static str,
    pub date_minutes_ago: &'static str,
    pub date_hours_ago: &'static str,
    pub date_days_ago: &'static str,

    pub update_title: &'static str,
    pub update_current_version: &'static str,
    pub update_check_btn: &'static str,
    pub update_checking: &'static str,
    pub update_up_to_date: &'static str,
    pub update_available: &'static str,
    pub update_available_tip: &'static str,
    pub update_download: &'static str,
    pub update_error: &'static str,
    pub update_auto_check: &'static str,
}

#[cfg(test)]
mod tests {
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
}
