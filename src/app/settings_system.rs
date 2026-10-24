use super::icons::{Icon, IconButton, IconLabel};
use super::license_view::{ProUpgradeState, render_license_section};
use super::theme::ReaderTheme;
use crate::core::license::ProFeature;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText};

pub fn render_system_tab(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    on_register_association: &mut bool,
    on_save_settings: &mut bool,
    on_reload_text_encoding: &mut bool,
    theme: &ReaderTheme,
    updater: &crate::updater::UpdateChecker,
    pro: &mut ProUpgradeState,
) {
    let t = settings.ui_language.t();
    let text_color = theme.text_color();
    let card_stroke = egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25));

    ui.label(
        RichText::new(t.system_title)
            .size(17.0)
            .strong()
            .color(text_color),
    );
    ui.add_space(6.0);

    render_license_section(ui, settings, theme, pro, on_save_settings);
    ui.add_space(8.0);

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(card_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t.text_encoding_label)
                    .strong()
                    .color(text_color),
            );
            ui.add_space(4.0);
            let encodings = [
                ("auto", t.text_encoding_auto),
                ("utf-8", "UTF-8"),
                ("windows-1251", "Windows-1251"),
                ("windows-1252", "Windows-1252"),
                ("koi8-r", "KOI8-R"),
                ("ibm866", "CP866"),
                ("utf-16le", "UTF-16LE"),
                ("utf-16be", "UTF-16BE"),
            ];
            let selected = encodings
                .iter()
                .find(|(value, _)| *value == settings.text_encoding)
                .map(|(_, label)| (*label).to_string())
                .unwrap_or_else(|| t.text_encoding_auto.to_string());
            egui::ComboBox::from_id_salt("text_encoding")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for (value, label) in encodings {
                        if ui
                            .selectable_label(settings.text_encoding == value, label)
                            .clicked()
                        {
                            settings.text_encoding = value.to_string();
                            *on_save_settings = true;
                            *on_reload_text_encoding = true;
                        }
                    }
                });
        });
    ui.add_space(8.0);

    render_library_pin_section(ui, settings, theme, on_save_settings, t);
    ui.add_space(8.0);

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(card_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(RichText::new(t.update_title).strong().color(text_color));
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!(
                    "{}: v{}",
                    t.update_current_version,
                    crate::updater::CURRENT_VERSION
                ))
                .size(12.0)
                .color(text_color.gamma_multiply(0.8)),
            );
            ui.add_space(6.0);
            if ui
                .checkbox(&mut settings.check_updates_on_startup, t.update_auto_check)
                .changed()
            {
                *on_save_settings = true;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(t.update_check_btn).clicked() {
                    updater.check_now();
                }
                match updater.status() {
                    crate::updater::UpdateStatus::Idle => {}
                    crate::updater::UpdateStatus::Checking => {
                        ui.label(RichText::new(t.update_checking).color(theme.accent_color()));
                    }
                    crate::updater::UpdateStatus::UpToDate { version } => {
                        ui.label(
                            RichText::new(format!("{} (v{})", t.update_up_to_date, version))
                                .color(Color32::from_rgb(46, 139, 87))
                                .strong(),
                        );
                    }
                    crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
                        let msg = format!("{} v{}!", t.update_available, info.latest_version);
                        ui.label(
                            RichText::new(msg)
                                .color(Color32::from_rgb(220, 140, 20))
                                .strong(),
                        );
                        if ui.button(t.update_download).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(&info.release_url));
                        }
                    }
                    crate::updater::UpdateStatus::Error(ref err) => {
                        ui.label(
                            RichText::new(format!("{}: {}", t.update_error, err))
                                .color(Color32::RED)
                                .size(11.0),
                        );
                    }
                }
            });
        });
    ui.add_space(8.0);

    if crate::platform::supports_file_association() {
        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(card_stroke)
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(RichText::new(t.assoc_title).strong().color(text_color));
                ui.add_space(4.0);
                ui.label(t.assoc_hint);
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.small_button(t.select_all).clicked() {
                        settings.associated_extensions = crate::platform::SUPPORTED_EXTENSIONS
                            .iter()
                            .map(|ext| ext.to_string())
                            .collect();
                        *on_save_settings = true;
                    }
                    if ui.small_button(t.deselect_all).clicked() {
                        settings.associated_extensions.clear();
                        *on_save_settings = true;
                    }
                    if ui.small_button(t.only_books).clicked() {
                        settings.associated_extensions =
                            vec!["fb2".to_string(), "epub".to_string()];
                        *on_save_settings = true;
                    }
                });
                ui.add_space(6.0);

                egui::Grid::new("settings_associations_grid")
                    .num_columns(2)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for &ext in crate::platform::SUPPORTED_EXTENSIONS {
                            let label = crate::i18n::format::format_extension_description(
                                ext,
                                settings.ui_language,
                            );
                            let mut is_checked = settings
                                .associated_extensions
                                .iter()
                                .any(|e| e.eq_ignore_ascii_case(ext));
                            if ui.checkbox(&mut is_checked, label).changed() {
                                if is_checked {
                                    if !settings
                                        .associated_extensions
                                        .iter()
                                        .any(|e| e.eq_ignore_ascii_case(ext))
                                    {
                                        settings.associated_extensions.push(ext.to_string());
                                    }
                                } else {
                                    settings
                                        .associated_extensions
                                        .retain(|e| !e.eq_ignore_ascii_case(ext));
                                }
                                *on_save_settings = true;
                            }

                            match crate::platform::check_extension_association_status(ext) {
                                crate::platform::AssociationStatus::Default => {
                                    ui.add(
                                        IconLabel::new(Icon::Check, t.status_default)
                                            .size(11.0)
                                            .color(Color32::from_rgb(46, 139, 87))
                                            .strong(),
                                    );
                                }
                                crate::platform::AssociationStatus::Registered => {
                                    ui.label(
                                        RichText::new(t.status_registered)
                                            .size(11.0)
                                            .color(Color32::GRAY),
                                    );
                                }
                                _ => {
                                    ui.label("");
                                }
                            }
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                let has_selected = !settings.associated_extensions.is_empty();
                if ui
                    .add_enabled(
                        has_selected,
                        egui::Button::new(RichText::new(t.register_btn).strong()),
                    )
                    .clicked()
                {
                    *on_register_association = true;
                }

                ui.add_space(8.0);
                if ui
                    .checkbox(&mut settings.dismiss_association_prompt, t.dont_show_assoc)
                    .changed()
                {
                    *on_save_settings = true;
                }
            });
        ui.add_space(8.0);
    }

    let translation_allowed = settings.can_use(ProFeature::Translation);
    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(card_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t.translation_provider)
                    .strong()
                    .color(text_color),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let is_google =
                    settings.translation.provider == crate::settings::TranslationProvider::Google;
                let is_deepl =
                    settings.translation.provider == crate::settings::TranslationProvider::DeepL;
                if ui.selectable_label(is_google, "Google Translate").clicked() && !is_google {
                    if translation_allowed {
                        settings.translation.provider =
                            crate::settings::TranslationProvider::Google;
                        *on_save_settings = true;
                    } else {
                        pro.request(ProFeature::Translation);
                    }
                }
                if ui.selectable_label(is_deepl, "DeepL (API)").clicked() && !is_deepl {
                    if translation_allowed {
                        settings.translation.provider = crate::settings::TranslationProvider::DeepL;
                        *on_save_settings = true;
                    } else {
                        pro.request(ProFeature::Translation);
                    }
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t.deepl_api_key).color(text_color).size(13.0));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut settings.translation.deepl_api_key)
                        .password(true)
                        .hint_text(t.deepl_key_hint)
                        .desired_width(260.0),
                );
                if resp.changed() || resp.lost_focus() {
                    *on_save_settings = true;
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(t.deepl_desc)
                    .size(11.5)
                    .color(text_color.gamma_multiply(0.7)),
            );
        });
    ui.add_space(8.0);

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(card_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(RichText::new(t.settings_storage).strong().color(text_color));
            ui.add_space(4.0);
            let path_str = AppSettings::get_settings_path().display().to_string();
            ui.label(
                RichText::new(path_str)
                    .size(12.0)
                    .color(text_color.gamma_multiply(0.7)),
            );
            ui.add_space(6.0);
            if ui
                .add(IconButton::new(Icon::Check, t.save_settings_now))
                .clicked()
            {
                *on_save_settings = true;
            }
        });
}

fn render_library_pin_section(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    theme: &ReaderTheme,
    on_save_settings: &mut bool,
    t: &crate::i18n::Translations,
) {
    let draft_id = egui::Id::unique("library_pin_draft");
    let confirm_id = egui::Id::unique("library_pin_confirm");
    let error_id = egui::Id::unique("library_pin_error");
    let mut draft: String = ui.data(|data| data.get_temp(draft_id).unwrap_or_default());
    let mut confirm: String = ui.data(|data| data.get_temp(confirm_id).unwrap_or_default());
    let mut error: Option<String> = ui.data(|data| data.get_temp(error_id));
    let text_color = theme.text_color();

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(egui::Stroke::new(
            1.0,
            theme.accent_color().gamma_multiply(0.25),
        ))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t.library_lock_title)
                    .strong()
                    .color(text_color),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(t.library_lock_desc)
                    .size(12.0)
                    .color(text_color.gamma_multiply(0.75)),
            );
            ui.add_space(6.0);

            if settings.has_library_pin() {
                if ui.button(t.library_pin_remove).clicked() {
                    settings.clear_library_pin();
                    *on_save_settings = true;
                    draft.clear();
                    confirm.clear();
                    error = None;
                }
            } else {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft)
                            .password(true)
                            .hint_text(t.library_pin_enter)
                            .desired_width(130.0),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut confirm)
                            .password(true)
                            .hint_text(t.library_pin_repeat)
                            .desired_width(130.0),
                    );
                });
                ui.add_space(6.0);
                if ui.button(t.library_pin_set).clicked() {
                    let digits = !draft.is_empty()
                        && draft.chars().count() >= 4
                        && draft.chars().count() <= 12
                        && draft.chars().all(|ch| ch.is_ascii_digit());
                    if !digits {
                        error = Some(t.library_pin_too_short.to_string());
                    } else if draft != confirm {
                        error = Some(t.library_pin_mismatch.to_string());
                    } else {
                        settings.set_library_pin(&draft);
                        *on_save_settings = true;
                        draft.clear();
                        confirm.clear();
                        error = None;
                    }
                }
            }

            if let Some(message) = error.as_deref() {
                ui.add_space(6.0);
                ui.label(RichText::new(message).size(12.0).color(egui::Color32::RED));
            }
        });

    ui.data_mut(|data| {
        data.insert_temp(draft_id, draft);
        data.insert_temp(confirm_id, confirm);
        data.insert_temp(error_id, error);
    });
}
