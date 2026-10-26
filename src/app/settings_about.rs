use crate::app::license_view::status_text;
use crate::app::theme::ReaderTheme;
use crate::settings::AppSettings;
use eframe::egui::{self, RichText};

pub fn render_hotkeys_tab(ui: &mut egui::Ui, theme: &ReaderTheme, lang: crate::i18n::UiLanguage) {
    let t = lang.t();
    let text_color = theme.text_color();
    ui.label(
        RichText::new(t.tab_hotkeys)
            .size(17.0)
            .strong()
            .color(text_color),
    );
    ui.add_space(6.0);

    let stroke = egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.2));
    for hotkey in t.hotkeys {
        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(stroke)
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(hotkey.keys)
                            .strong()
                            .color(theme.accent_color()),
                    );
                    ui.label("—");
                    ui.label(RichText::new(hotkey.description).color(text_color));
                });
            });
        ui.add_space(2.0);
    }
}

pub fn render_about_tab(ui: &mut egui::Ui, settings: &AppSettings, theme: &ReaderTheme) {
    let t = settings.ui_language.t();
    let text_color = theme.text_color();
    let secondary = text_color.gamma_multiply(0.78);
    let status = settings.license_status();
    let tier = if status.is_pro_active() {
        t.license_pro_badge
    } else {
        t.license_free_status
    };

    ui.label(
        RichText::new(t.tab_about)
            .size(17.0)
            .strong()
            .color(text_color),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(format!(
            "{}: v{}",
            t.about_version,
            crate::updater::CURRENT_VERSION
        ))
        .color(text_color),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(format!("{}: {}", t.about_license_status, tier)).color(text_color));
    ui.label(
        RichText::new(status_text(&t, &status))
            .size(12.0)
            .color(secondary),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    ui.label(
        RichText::new(t.about_third_party)
            .size(15.0)
            .strong()
            .color(text_color),
    );
    ui.add_space(6.0);
    let stroke = egui::Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25));
    egui::Frame::new()
        .fill(theme.bg_color())
        .stroke(stroke)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.label(
                RichText::new(t.about_unrar_title)
                    .strong()
                    .color(theme.accent_color()),
            );
            ui.label(
                RichText::new(t.about_unrar_text)
                    .size(12.0)
                    .color(secondary),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(t.about_sil_title)
                    .strong()
                    .color(theme.accent_color()),
            );
            ui.label(RichText::new(t.about_sil_text).size(12.0).color(secondary));
        });
}
