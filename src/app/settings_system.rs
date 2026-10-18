use super::icons::{Icon, IconButton, IconLabel};
use super::theme::ReaderTheme;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText};

pub fn render_system_tab(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    on_register_association: &mut bool,
    on_save_settings: &mut bool,
    theme: &ReaderTheme,
    updater: &crate::updater::UpdateChecker,
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

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(card_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t.update_title)
                    .strong()
                    .color(text_color),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{}: v{}", t.update_current_version, crate::updater::CURRENT_VERSION))
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
                ui.label(
                    RichText::new(t.assoc_title)
                        .strong()
                        .color(text_color),
                );
                ui.add_space(4.0);
                ui.label(t.assoc_hint);
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.small_button(t.select_all).clicked() {
                        settings.associated_extensions =
                            crate::platform::SUPPORTED_EXTENSIONS_INFO
                                .iter()
                                .map(|(ext, _)| ext.to_string())
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
                        for &(ext, label) in crate::platform::SUPPORTED_EXTENSIONS_INFO {
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
                        egui::Button::new(
                            RichText::new(t.register_btn)
                                .strong(),
                        ),
                    )
                    .clicked()
                {
                    *on_register_association = true;
                }

                ui.add_space(8.0);
                if ui
                    .checkbox(
                        &mut settings.dismiss_association_prompt,
                        t.dont_show_assoc,
                    )
                    .changed()
                {
                    *on_save_settings = true;
                }
            });
        ui.add_space(8.0);
    }

    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(card_stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t.settings_storage)
                    .strong()
                    .color(text_color),
            );
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
