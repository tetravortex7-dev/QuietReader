use super::super::ambient::AmbientController;
use super::super::theme::ReaderTheme;
use super::super::tts::TtsController;
use crate::core::library::{LibraryMode, LibraryStore, UiDesignMode};
use crate::i18n::Translations;
use crate::settings::{AppSettings, TtsSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSectionId {
    ThemeAndDisplay,
    Language,
    Typography,
    FocusTools,
    NavigationDisplay,
    AutoScroll,
    ComicsAndPdf,
    Hotkeys,
    Ambience,
    TextToSpeech,
    Translation,
    DictionariesVocab,
    Encoding,
    LibraryPin,
    Updates,
    FileAssociations,
    Storage,
    ProUpgrade,
    AboutVersion,
}

pub fn apply_default_app_preferences(
    settings: &mut AppSettings,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    theme: &mut ReaderTheme,
    font_size: &mut f32,
    keep_screen_on: &mut bool,
) {
    let defaults = AppSettings::default();
    settings.text_encoding = defaults.text_encoding;
    settings.font_size = defaults.font_size;
    settings.font_family = defaults.font_family;
    settings.line_height_scale = defaults.line_height_scale;
    settings.max_column_width = defaults.max_column_width;
    settings.reading_brightness = defaults.reading_brightness;
    settings.trackpad_swipe_pages = defaults.trackpad_swipe_pages;
    settings.text_paged_scroll = defaults.text_paged_scroll;
    settings.text_two_columns = defaults.text_two_columns;
    settings.text_paragraph_mode = defaults.text_paragraph_mode;
    settings.rsvp_wpm = defaults.rsvp_wpm;
    settings.rsvp_auto_accelerate = defaults.rsvp_auto_accelerate;
    settings.theme = defaults.theme;
    settings.keep_screen_on = defaults.keep_screen_on;
    settings.auto_scroll = defaults.auto_scroll;
    settings.comic = defaults.comic;
    settings.pdf = defaults.pdf;
    settings.ui_language = defaults.ui_language;
    settings.check_updates_on_startup = defaults.check_updates_on_startup;
    settings.bionic_reading = defaults.bionic_reading;
    settings.reading_ruler = defaults.reading_ruler;
    settings.reading_ruler_lines = defaults.reading_ruler_lines;
    settings.reading_ruler_opacity = defaults.reading_ruler_opacity;
    settings.translation.provider = defaults.translation.provider;
    settings.ambient.master_volume = defaults.ambient.master_volume;
    settings.ambient.tone_softness = defaults.ambient.tone_softness;
    settings.ambient.duck_during_tts = defaults.ambient.duck_during_tts;
    settings.ambient.duck_amount = defaults.ambient.duck_amount;
    settings.ambient.resume_on_startup = defaults.ambient.resume_on_startup;

    let default_tts = TtsSettings::default();
    settings.tts.rate = default_tts.rate;
    settings.tts.engine = default_tts.engine;

    *theme = ReaderTheme::Sepia;
    *font_size = defaults.font_size;
    *keep_screen_on = defaults.keep_screen_on;

    if library.design != UiDesignMode::Cozy || library.mode != LibraryMode::Shelf {
        library.design = UiDesignMode::Cozy;
        library.mode = LibraryMode::Shelf;
        *library_dirty = true;
    }
}

pub fn apply_default_preferences(
    settings: &mut AppSettings,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    theme: &mut ReaderTheme,
    font_size: &mut f32,
    keep_screen_on: &mut bool,
    ambient: &mut AmbientController,
    tts: &mut TtsController,
) {
    apply_default_app_preferences(
        settings,
        library,
        library_dirty,
        theme,
        font_size,
        keep_screen_on,
    );
    ambient.settings.master_volume = settings.ambient.master_volume;
    ambient.settings.tone_softness = settings.ambient.tone_softness;
    ambient.settings.duck_during_tts = settings.ambient.duck_during_tts;
    ambient.settings.duck_amount = settings.ambient.duck_amount;
    ambient.settings.resume_on_startup = settings.ambient.resume_on_startup;
    ambient.sync_shared_params();

    tts.set_rate(settings.tts.rate);
    tts.set_engine(crate::app::tts::TtsEngineKind::System);
    crate::platform::set_keep_screen_on(*keep_screen_on);
}

pub fn matching_sections(query: &str, t: &Translations) -> Vec<SettingsSectionId> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }

    let contains =
        |haystack: &[&str]| -> bool { haystack.iter().any(|s| s.to_lowercase().contains(&q)) };

    let mut out = Vec::new();
    if contains(&[
        t.extra.settings_nav_appearance,
        t.extra.settings_themes_title,
        t.extra.settings_themes_subtitle,
        t.theme_sepia,
        t.theme_twilight,
        t.theme_warm,
        t.theme_mint,
        t.theme_light,
        t.theme_dark,
        t.theme_oled,
        t.brightness_title,
        t.extra.settings_rounding_label,
        t.library_design_cozy,
        t.library_design_studio,
        t.library_design_tooltip,
        t.extra.settings_startup_mode_label,
        "theme",
        "oled",
        "brightness",
        "cozy",
        "studio",
    ]) {
        out.push(SettingsSectionId::ThemeAndDisplay);
    }
    if contains(&[
        t.interface_language,
        t.interface_language_desc,
        "language",
        "locale",
        "english",
        "russian",
    ]) {
        out.push(SettingsSectionId::Language);
    }
    if contains(&[
        t.extra.settings_nav_typography,
        t.extra.settings_typography_title,
        t.font_choice,
        t.font_size,
        t.line_height_label,
        t.column_width_label,
        t.paged_scroll_title,
        t.two_columns_title,
        t.extra.text_layout_title,
        t.extra.text_mode_scroll,
        t.extra.text_mode_spread,
        t.extra.text_scroll_tip,
        t.extra.text_spread_tip,
        t.paragraph_mode,
        t.live_preview,
        "font",
        "georgia",
        "palatino",
        "opendyslexic",
        "lexend",
        "typography",
    ]) {
        out.push(SettingsSectionId::Typography);
    }
    if contains(&[
        t.extra.settings_nav_reading,
        t.extra.settings_focus_title,
        t.reading_ruler_title,
        t.reading_ruler_desc,
        t.bionic_reading_title,
        t.rsvp_title,
        t.rsvp_speed,
        t.rsvp_auto_accelerate,
        "ruler",
        "bionic",
        "rsvp",
        "focus",
        "wpm",
    ]) {
        out.push(SettingsSectionId::FocusTools);
    }
    if contains(&[
        t.extra.settings_navigation_title,
        t.trackpad_swipes_title,
        t.trackpad_swipes_desc,
        t.keep_screen_on,
        t.keep_screen_on_desc,
        "trackpad",
        "swipe",
        "sleep",
    ]) {
        out.push(SettingsSectionId::NavigationDisplay);
    }
    if contains(&[
        t.autoscroll_title,
        t.autoscroll_desc,
        t.pause_duration,
        t.step_distance,
        t.scroll_speed,
        "autoscroll",
        "scroll",
    ]) {
        out.push(SettingsSectionId::AutoScroll);
    }
    if contains(&[
        t.comics_title,
        t.mode_manga,
        t.mode_comic,
        t.mode_webtoon,
        t.layout_title,
        t.fit_title,
        t.extra.settings_pdf_smart_section,
        t.smart_dark_mode,
        t.auto_crop_margins,
        "pdf",
        "manga",
        "cbz",
        "cbr",
        "webtoon",
        "crop",
    ]) {
        out.push(SettingsSectionId::ComicsAndPdf);
    }
    let hk_hit = contains(&[t.tab_hotkeys, "hotkey", "shortcut", "ctrl"])
        || t.hotkeys.iter().any(|hk| {
            hk.keys.to_lowercase().contains(&q) || hk.description.to_lowercase().contains(&q)
        });
    if hk_hit {
        out.push(SettingsSectionId::Hotkeys);
    }
    if contains(&[
        t.extra.settings_nav_ambience_tts,
        t.extra.settings_ambient_section_title,
        t.ambient_master_volume,
        t.ambient_tone_softness,
        t.ambient_duck_tts,
        t.ambient_resume_on_startup,
        "ambient",
        "noise",
        "rain",
        "lofi",
    ]) {
        out.push(SettingsSectionId::Ambience);
    }
    if contains(&[
        t.extra.settings_tts_section_title,
        t.tts_title,
        t.tts_system,
        t.tts_edge,
        t.tts_speed,
        t.tts_voice,
        "tts",
        "speech",
        "voice",
    ]) {
        out.push(SettingsSectionId::TextToSpeech);
    }
    if contains(&[
        t.extra.settings_nav_dict_translate,
        t.translation_provider,
        t.deepl_api_key,
        t.deepl_desc,
        "deepl",
        "google",
        "translate",
        "api",
    ]) {
        out.push(SettingsSectionId::Translation);
    }
    if contains(&[
        t.extra.settings_dict_section_title,
        t.dictionary_title,
        t.vocabulary_title,
        "stardict",
        "dictionary",
        "vocabulary",
    ]) {
        out.push(SettingsSectionId::DictionariesVocab);
    }
    if contains(&[
        t.extra.settings_nav_system,
        t.text_encoding_label,
        "utf-8",
        "windows-1251",
        "encoding",
        "koi8",
    ]) {
        out.push(SettingsSectionId::Encoding);
    }
    if contains(&[
        t.library_lock_title,
        t.library_lock_desc,
        "pin",
        "sha-256",
        "lock",
        "security",
    ]) {
        out.push(SettingsSectionId::LibraryPin);
    }
    if contains(&[
        t.update_title,
        t.update_check_btn,
        t.update_auto_check,
        "update",
    ]) {
        out.push(SettingsSectionId::Updates);
    }
    if crate::platform::supports_file_association()
        && contains(&[
            t.assoc_title,
            t.assoc_hint,
            "epub",
            "fb2",
            "file",
            "association",
        ])
    {
        out.push(SettingsSectionId::FileAssociations);
    }
    if contains(&[
        t.settings_storage,
        t.settings_portable,
        "json",
        "storage",
        "portable",
    ]) {
        out.push(SettingsSectionId::Storage);
    }
    if contains(&[
        t.extra.settings_nav_pro_about,
        t.license_title,
        t.license_pro_badge,
        t.license_key_label,
        t.license_try_trial,
        "pro",
        "license",
        "trial",
    ]) {
        out.push(SettingsSectionId::ProUpgrade);
    }
    if contains(&[
        t.tab_about,
        t.about_version,
        t.about_third_party,
        "unrar",
        "sil",
        "version",
        "about",
    ]) {
        out.push(SettingsSectionId::AboutVersion);
    }
    out
}
