use super::icons::{Icon, IconButton, IconLabel};
use super::license_view::{LicenseNotice, ProUpgradeState, error_text, feature_name, status_text};
use super::settings_view::widgets::{
    SectionBadge, SettingsPalette, pill_badge, row_card, section_card, themed_button,
};
use crate::core::license::{LicenseStatus, ProFeature};
use crate::i18n::fill;
use crate::settings::AppSettings;
use eframe::egui::{self, RichText, Vec2};

pub fn render_about_tab(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    settings: &mut AppSettings,
    pro: &mut ProUpgradeState,
    on_save_settings: &mut bool,
) {
    let t = settings.ui_language.t();

    render_pro_section(ui, p, &t, settings, pro, on_save_settings);
    ui.add_space(14.0);
    render_version_and_licenses_section(ui, p, &t);
}

pub(super) fn render_pro_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &crate::i18n::Translations,
    settings: &mut AppSettings,
    pro: &mut ProUpgradeState,
    on_save_settings: &mut bool,
) {
    let status = settings.license_status();
    let is_active = status.is_pro_active();
    section_card(
        ui,
        p,
        t.license_title,
        Some(SectionBadge::Pro(t.license_pro_badge)),
        Some(t.license_upgrade_subtitle),
        |ui| {
            row_card(ui, p, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(status_text(t, &status))
                            .size(13.5)
                            .strong()
                            .color(p.text),
                    );
                    if is_active {
                        pill_badge(ui, p.tertiary_bg, p.tertiary, t.license_pro_badge, 10.0);
                    }
                });
                if let LicenseStatus::ProActivated { email, .. } = &status {
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(fill(t.license_licensed_to, &[("{email}", email)]))
                            .size(11.5)
                            .color(p.text_variant),
                    );
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t.license_features_title)
                        .size(12.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);

                let cols = if ui.available_width() >= 500.0 { 2 } else { 1 };
                for chunk in ProFeature::ALL.chunks(cols) {
                    if cols == 2 {
                        ui.columns(2, |cols_ui| {
                            for (idx, &feat) in chunk.iter().enumerate() {
                                cols_ui[idx].add(
                                    IconLabel::new(Icon::Check, feature_name(t, feat))
                                        .size(12.0)
                                        .color(p.text),
                                );
                            }
                        });
                    } else {
                        for &feat in chunk {
                            ui.add(
                                IconLabel::new(Icon::Check, feature_name(t, feat))
                                    .size(12.0)
                                    .color(p.text),
                            );
                        }
                    }
                    ui.add_space(2.0);
                }

                ui.add_space(6.0);
                ui.label(
                    RichText::new(t.license_free_forever)
                        .size(11.0)
                        .color(p.text_variant),
                );
            });

            if !is_active {
                ui.add_space(10.0);
                row_card(ui, p, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if settings.trial_started_at_unix.is_none() {
                            if themed_button(ui, t.license_try_trial, true, p).clicked() {
                                pro.start_trial(settings, on_save_settings);
                            }
                        } else {
                            ui.label(
                                RichText::new(t.license_trial_one_time)
                                    .size(11.5)
                                    .color(p.text_variant),
                            );
                        }
                    });
                    if pro.trial_error {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(t.license_trial_error)
                                .size(11.5)
                                .color(p.error),
                        );
                    }

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(t.license_key_label)
                            .size(12.0)
                            .strong()
                            .color(p.text),
                    );
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut pro.key_input)
                                .hint_text(t.license_key_hint)
                                .desired_width((ui.available_width() - 140.0).clamp(180.0, 360.0)),
                        );
                        let submitted = resp.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        let activate_clicked = ui
                            .add(
                                IconButton::new(Icon::Key, t.license_activate)
                                    .min_size(Vec2::new(115.0, 28.0)),
                            )
                            .clicked();
                        if (activate_clicked || submitted) && !pro.key_input.trim().is_empty() {
                            pro.activate(settings, on_save_settings);
                        }
                    });
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(t.license_enter_key_hint)
                            .size(11.0)
                            .color(p.text_variant),
                    );
                    if let Some(error) = pro.error {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(error_text(t, error))
                                .size(11.5)
                                .color(p.error),
                        );
                    }
                    if let Some(notice) = pro.notice {
                        ui.add_space(4.0);
                        let msg = match notice {
                            LicenseNotice::TrialStarted => t.license_trial_started,
                            LicenseNotice::Activated => t.license_activated_thanks,
                        };
                        ui.add(
                            IconLabel::new(Icon::Check, msg)
                                .size(12.0)
                                .color(p.tertiary)
                                .strong(),
                        );
                    }
                });
            }
        },
    );
}

pub(super) fn render_version_and_licenses_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &crate::i18n::Translations,
) {
    section_card(ui, p, t.tab_about, None, None, |ui| {
        row_card(ui, p, |ui| {
            ui.label(
                RichText::new(format!(
                    "{}: v{}",
                    t.about_version,
                    crate::updater::CURRENT_VERSION
                ))
                .size(13.0)
                .strong()
                .color(p.text),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(t.about_third_party)
                    .size(12.5)
                    .strong()
                    .color(p.text),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t.about_unrar_title)
                    .size(12.0)
                    .strong()
                    .color(p.accent),
            );
            ui.label(
                RichText::new(t.about_unrar_text)
                    .size(11.5)
                    .color(p.text_variant),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(t.about_sil_title)
                    .size(12.0)
                    .strong()
                    .color(p.accent),
            );
            ui.label(
                RichText::new(t.about_sil_text)
                    .size(11.5)
                    .color(p.text_variant),
            );
        });
    });
}
