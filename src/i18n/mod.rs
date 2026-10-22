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

pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in values {
        result = result.replace(key, value);
    }
    result
}

#[derive(Debug, Clone, Copy)]
pub struct FontText {
    pub badge: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct HotkeyText {
    pub keys: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtraTranslations {
    pub default_document_title: &'static str,
    pub default_unknown_author: &'static str,
    pub default_untitled: &'static str,
    pub default_comic_title: &'static str,
    pub default_section: &'static str,
    pub default_page: &'static str,
    pub default_chapter: &'static str,
    pub default_part: &'static str,
    pub default_no_text: &'static str,
    pub default_word_document: &'static str,
    pub default_text_document: &'static str,
    pub default_section_plain: &'static str,
    pub default_section_page: &'static str,
    pub default_pdf_document: &'static str,
    pub default_djvu_document: &'static str,
    pub default_mobi_document: &'static str,
    pub default_chapter_part: &'static str,
    pub default_lines_range: &'static str,
    pub comic_total_pages: &'static str,
    pub comic_pages_range: &'static str,
    pub document_unsupported_paged: &'static str,
    pub comic_decode_crash: &'static str,
    pub page_decode_crash: &'static str,
    pub decode_stream_crashed: &'static str,
    pub page_load_error_detail: &'static str,
    pub page_error_detail: &'static str,
    pub page_loading_number: &'static str,
    pub copy_page_done: &'static str,
    pub copy_section_done: &'static str,
    pub status_assoc_error: &'static str,
    pub status_assoc_success: &'static str,
    pub status_all_tabs_closed: &'static str,
    pub status_tab_loading: &'static str,
    pub status_tab_error: &'static str,
    pub status_tab_active: &'static str,
    pub status_opened_comic: &'static str,
    pub status_opened_comic_total: &'static str,
    pub status_opened_book_resume: &'static str,
    pub status_opened_book_author: &'static str,
    pub status_opened_book: &'static str,
    pub toc_all_pages: &'static str,
    pub toc_pages: &'static str,
    pub clear_search: &'static str,
    pub queued_download: &'static str,
    pub pdf_rotate_tip: &'static str,
    pub pdf_single_tip: &'static str,
    pub pdf_double_tip: &'static str,
    pub pdf_scroll_tip: &'static str,
    pub pdf_copy_selected_tip: &'static str,
    pub pdf_clear_selection_tip: &'static str,
    pub pdf_selection_hint: &'static str,
    pub pdf_copied_prefix: &'static str,
    pub pdf_copy_btn: &'static str,
    pub seconds_suffix: &'static str,
    pub pixels_per_second_suffix: &'static str,
    pub tts_chapter_paragraph: &'static str,
    pub tts_playback_error: &'static str,
    pub translate_missing_api_key: &'static str,
    pub translate_invalid_key: &'static str,
    pub translate_quota_exceeded: &'static str,
    pub translate_too_many_requests: &'static str,
    pub translate_no_translation: &'static str,
    pub translate_empty_translation: &'static str,
    pub translate_provider_error: &'static str,
    pub opds_network_error: &'static str,
    pub opds_read_response_error: &'static str,
    pub opds_http_status: &'static str,
    pub opds_opensearch_error: &'static str,
    pub opds_invalid_opensearch: &'static str,
    pub opds_no_search_interface: &'static str,
    pub opds_download_connect_error: &'static str,
    pub opds_download_server_error: &'static str,
    pub opds_create_file_error: &'static str,
    pub opds_transfer_error: &'static str,
    pub opds_write_file_error: &'static str,
    pub opds_flush_error: &'static str,
    pub opds_move_file_error: &'static str,
    pub format_desc_mobi: &'static str,
    pub format_desc_azw3: &'static str,
    pub format_desc_log: &'static str,
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
    pub toc_tooltip: &'static str,
    pub search: &'static str,
    pub search_tooltip: &'static str,
    pub speech: &'static str,
    pub speech_tooltip: &'static str,
    pub reading_ruler: &'static str,
    pub hide_reading_ruler: &'static str,
    pub reading_ruler_tooltip: &'static str,
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
    pub secrets_storage_error: &'static str,
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
    pub bionic_reading_title: &'static str,
    pub bionic_reading_desc: &'static str,
    pub reading_ruler_title: &'static str,
    pub reading_ruler_desc: &'static str,
    pub reading_ruler_height_label: &'static str,
    pub reading_ruler_dim_label: &'static str,
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
    pub translate: &'static str,
    pub translate_paragraph: &'static str,
    pub translate_selection: &'static str,
    pub translate_selection_tip: &'static str,
    pub translate_title: &'static str,
    pub translating: &'static str,
    pub copy_translation: &'static str,
    pub translate_error: &'static str,
    pub translation_provider: &'static str,
    pub deepl_api_key: &'static str,
    pub deepl_key_hint: &'static str,
    pub deepl_desc: &'static str,
    pub save_key: &'static str,
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

    pub opds_catalogs: &'static str,
    pub opds_catalogs_tooltip: &'static str,
    pub opds_add_catalog: &'static str,
    pub opds_delete_catalog: &'static str,
    pub opds_catalog_name: &'static str,
    pub opds_catalog_url: &'static str,
    pub opds_username: &'static str,
    pub opds_password: &'static str,
    pub opds_auth_required: &'static str,
    pub opds_search_placeholder: &'static str,
    pub opds_search_btn: &'static str,
    pub opds_empty_feed: &'static str,
    pub opds_back: &'static str,
    pub opds_root: &'static str,
    pub opds_refresh: &'static str,
    pub opds_prev_page: &'static str,
    pub opds_next_page: &'static str,
    pub opds_save_btn: &'static str,
    pub opds_cancel_btn: &'static str,
    pub opds_my_catalogs: &'static str,
    pub opds_categories: &'static str,
    pub opds_books: &'static str,
    pub opds_download_format: &'static str,
    pub opds_downloading: &'static str,
    pub opds_download_done: &'static str,
    pub opds_book_downloaded_opened: &'static str,
    pub opds_expand: &'static str,
    pub opds_collapse: &'static str,
    pub opds_retry: &'static str,
    pub opds_no_results: &'static str,
    pub opds_select_catalog: &'static str,
    pub opds_url_required: &'static str,
    pub opds_invalid_url: &'static str,
    pub opds_unsupported_scheme: &'static str,
    pub opds_https_required: &'static str,
    pub opds_response_too_large: &'static str,
    pub opds_download_too_large: &'static str,

    pub bookmarks: &'static str,
    pub bookmarks_tooltip: &'static str,
    pub add_bookmark: &'static str,
    pub bookmark_added: &'static str,
    pub bookmark_removed: &'static str,
    pub save_quote_bookmark: &'static str,
    pub quote_saved: &'static str,
    pub bookmark_search_placeholder: &'static str,
    pub filter_all: &'static str,
    pub color_yellow: &'static str,
    pub color_green: &'static str,
    pub color_blue: &'static str,
    pub color_red: &'static str,
    pub add_note: &'static str,
    pub edit_note: &'static str,
    pub save_note: &'static str,
    pub cancel_note: &'static str,
    pub note_placeholder: &'static str,
    pub delete_bookmark: &'static str,
    pub return_to_reading: &'static str,
    pub copy_all_bookmarks: &'static str,
    pub export_markdown: &'static str,
    pub bookmarks_copied: &'static str,
    pub bookmarks_exported: &'static str,
    pub no_bookmarks_hint: &'static str,
    pub no_matching_bookmarks: &'static str,
    pub bookmark_kind_quote: &'static str,
    pub bookmark_kind_bookmark: &'static str,

    pub ambient: &'static str,
    pub ambient_tooltip: &'static str,
    pub ambient_title: &'static str,
    pub ambient_play: &'static str,
    pub ambient_pause: &'static str,
    pub ambient_master_volume: &'static str,
    pub ambient_tone_softness: &'static str,
    pub ambient_timer_off: &'static str,
    pub ambient_timer_15m: &'static str,
    pub ambient_timer_30m: &'static str,
    pub ambient_timer_60m: &'static str,
    pub preset_laser_focus: &'static str,
    pub preset_rainy_evening: &'static str,
    pub preset_night_lofi: &'static str,
    pub preset_deep_space: &'static str,
    pub layer_brown_noise: &'static str,
    pub layer_brown_noise_desc: &'static str,
    pub layer_pink_noise: &'static str,
    pub layer_pink_noise_desc: &'static str,
    pub layer_rain: &'static str,
    pub layer_rain_desc: &'static str,
    pub layer_fireplace: &'static str,
    pub layer_fireplace_desc: &'static str,
    pub layer_lofi: &'static str,
    pub layer_lofi_desc: &'static str,
    pub layer_binaural: &'static str,
    pub layer_binaural_desc: &'static str,
    pub ambient_duck_tts: &'static str,

    pub smart_dark_mode: &'static str,
    pub smart_dark_mode_tooltip: &'static str,
    pub auto_crop_margins: &'static str,
    pub auto_crop_margins_tooltip: &'static str,
    pub crop_sensitivity: &'static str,
    pub crop_tolerance: &'static str,

    pub extra: ExtraTranslations,
    pub fonts: [FontText; 9],
    pub hotkeys: [HotkeyText; 18],
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
