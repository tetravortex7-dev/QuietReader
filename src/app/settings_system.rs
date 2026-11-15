use super::icons::{Icon, IconButton, IconLabel};
use super::settings_view::widgets::{
    SettingsPalette, pill_badge, row_card, section_card, themed_button, toggle_switch,
};
use super::theme::ReaderTheme;
use crate::i18n::Translations;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText};

pub fn render_system_tab(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    settings: &mut AppSettings,
    on_register_association: &mut bool,
    on_save_settings: &mut bool,
    on_reload_text_encoding: &mut bool,
    updater: &crate::updater::UpdateChecker,
) {
    let t = settings.ui_language.t();

    render_encoding_section(
        ui,
        p,
        &t,
        settings,
        on_save_settings,
        on_reload_text_encoding,
    );
    ui.add_space(14.0);
    render_library_pin_section(ui, p, &t, settings, on_save_settings);
    ui.add_space(14.0);
    render_updates_section(ui, p, &t, settings, on_save_settings, updater);
    ui.add_space(14.0);

    if crate::platform::supports_file_association() {
        render_associations_section(
            ui,
            p,
            &t,
            settings,
            on_register_association,
            on_save_settings,
        );
        ui.add_space(14.0);
    }

    render_storage_section(ui, p, &t, on_save_settings);
}

pub(super) fn render_encoding_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    on_reload_text_encoding: &mut bool,
) {
    section_card(ui, p, t.text_encoding_label, None, None, |ui| {
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
            .width(220.0)
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
}

pub(super) fn render_library_pin_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let current_id = egui::Id::unique("library_pin_current");
    let draft_id = egui::Id::unique("library_pin_draft");
    let confirm_id = egui::Id::unique("library_pin_confirm");
    let error_id = egui::Id::unique("library_pin_error");
    let mut current_pin: String = ui.data(|data| data.get_temp(current_id).unwrap_or_default());
    let mut draft: String = ui.data(|data| data.get_temp(draft_id).unwrap_or_default());
    let mut confirm: String = ui.data(|data| data.get_temp(confirm_id).unwrap_or_default());
    let mut error: Option<String> = ui.data(|data| data.get_temp(error_id));

    section_card(
        ui,
        p,
        t.library_lock_title,
        None,
        Some(t.library_lock_desc),
        |ui| {
            row_card(ui, p, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if settings.has_library_pin() {
                        pill_badge(
                            ui,
                            p.tertiary_bg,
                            p.tertiary,
                            t.extra.settings_pin_configured,
                            10.0,
                        );
                    } else {
                        pill_badge(
                            ui,
                            p.surface_container,
                            p.text_variant,
                            t.extra.settings_pin_set_prompt,
                            10.0,
                        );
                    }
                });
                ui.add_space(8.0);

                if settings.has_library_pin() {
                    let mut submit_remove = false;
                    ui.horizontal_wrapped(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut current_pin)
                                .password(true)
                                .hint_text(t.library_pin_enter)
                                .desired_width(150.0),
                        );
                        if resp.changed() || resp.gained_focus() {
                            error = None;
                        }
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit_remove = true;
                        }
                        if themed_button(ui, t.library_pin_remove, false, p).clicked() {
                            submit_remove = true;
                        }
                    });
                    if submit_remove {
                        if settings.clear_library_pin(&current_pin) {
                            *on_save_settings = true;
                            current_pin.clear();
                            draft.clear();
                            confirm.clear();
                            error = None;
                        } else {
                            error = Some(t.library_pin_wrong.to_string());
                            current_pin.clear();
                        }
                    }
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut draft)
                                .password(true)
                                .hint_text(t.library_pin_enter)
                                .desired_width(140.0),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut confirm)
                                .password(true)
                                .hint_text(t.library_pin_repeat)
                                .desired_width(140.0),
                        );
                        if themed_button(ui, t.library_pin_set, true, p).clicked() {
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
                                current_pin.clear();
                                draft.clear();
                                confirm.clear();
                                error = None;
                            }
                        }
                    });
                }

                if let Some(message) = error.as_deref() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(message).size(11.5).color(p.error));
                }
            });
        },
    );

    ui.data_mut(|data| {
        data.insert_temp(current_id, current_pin);
        data.insert_temp(draft_id, draft);
        data.insert_temp(confirm_id, confirm);
        data.insert_temp(error_id, error);
    });
}

pub(super) fn render_updates_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    updater: &crate::updater::UpdateChecker,
) {
    section_card(ui, p, t.update_title, None, None, |ui| {
        row_card(ui, p, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}: v{}",
                            t.update_current_version,
                            crate::updater::CURRENT_VERSION
                        ))
                        .size(12.5)
                        .strong()
                        .color(p.text),
                    );
                    ui.label(
                        RichText::new(t.update_auto_check)
                            .size(11.5)
                            .color(p.text_variant),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if toggle_switch(ui, &mut settings.check_updates_on_startup, p).changed() {
                        *on_save_settings = true;
                    }
                });
            });
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                if themed_button(ui, t.update_check_btn, false, p).clicked() {
                    updater.check_now();
                }
                match updater.status() {
                    crate::updater::UpdateStatus::Idle => {}
                    crate::updater::UpdateStatus::Checking => {
                        ui.label(RichText::new(t.update_checking).color(p.accent));
                    }
                    crate::updater::UpdateStatus::UpToDate { version } => {
                        ui.label(
                            RichText::new(format!("{} (v{})", t.update_up_to_date, version))
                                .color(p.tertiary)
                                .strong(),
                        );
                    }
                    crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
                        let msg = format!("{} v{}!", t.update_available, info.latest_version);
                        ui.label(RichText::new(msg).color(p.secondary).strong());
                        if themed_button(ui, t.update_download, true, p).clicked() {
                            crate::platform::open_url(&info.release_url);
                        }
                    }
                    crate::updater::UpdateStatus::Error(ref err) => {
                        ui.label(
                            RichText::new(format!("{}: {}", t.update_error, err))
                                .color(p.error)
                                .size(11.0),
                        );
                    }
                }
            });
        });
    });
}

pub(super) fn render_associations_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_register_association: &mut bool,
    on_save_settings: &mut bool,
) {
    section_card(ui, p, t.assoc_title, None, Some(t.assoc_hint), |ui| {
        super::dialogs::render_association_presets(ui, settings, on_save_settings);
        ui.add_space(8.0);

        row_card(ui, p, |ui| {
            let exts = crate::platform::supported_extensions();
            egui::Grid::new("settings_associations_grid")
                .num_columns(2)
                .spacing([18.0, 4.0])
                .show(ui, |ui| {
                    for &ext in exts {
                        super::dialogs::render_extension_association_checkbox(
                            ui,
                            ext,
                            settings,
                            on_save_settings,
                        );
                        match crate::platform::check_extension_association_status(ext) {
                            crate::platform::AssociationStatus::Default => {
                                ui.add(
                                    IconLabel::new(Icon::Check, t.status_default)
                                        .size(11.0)
                                        .color(p.tertiary)
                                        .strong(),
                                );
                            }
                            crate::platform::AssociationStatus::Registered => {
                                ui.label(
                                    RichText::new(t.status_registered)
                                        .size(11.0)
                                        .color(p.text_variant),
                                );
                            }
                            _ => {
                                ui.label("");
                            }
                        }
                        ui.end_row();
                    }
                });
        });

        ui.add_space(10.0);
        let has_selected = !settings.associated_extensions.is_empty();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(has_selected, |ui| {
                if themed_button(ui, t.register_btn, true, p).clicked() {
                    *on_register_association = true;
                }
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if toggle_switch(ui, &mut settings.dismiss_association_prompt, p).changed() {
                *on_save_settings = true;
            }
            ui.label(
                RichText::new(t.dont_show_assoc)
                    .size(12.0)
                    .color(p.text_variant),
            );
        });
    });
}

pub(super) fn render_storage_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    on_save_settings: &mut bool,
) {
    section_card(ui, p, t.settings_storage, None, None, |ui| {
        row_card(ui, p, |ui| {
            let path_str = if AppSettings::is_portable() {
                format!(
                    "{} ({})",
                    AppSettings::get_settings_path().display(),
                    t.settings_portable
                )
            } else {
                AppSettings::get_settings_path().display().to_string()
            };
            ui.label(
                RichText::new(path_str)
                    .size(11.5)
                    .monospace()
                    .color(p.text_variant),
            );
            ui.add_space(8.0);
            if ui
                .add(IconButton::new(Icon::Check, t.save_settings_now))
                .clicked()
            {
                *on_save_settings = true;
            }
        });
    });
}

#[allow(dead_code)]
fn _unused_theme_ref(_: &ReaderTheme, _: Color32) {}
