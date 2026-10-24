use super::theme::ReaderTheme;
use crate::settings::AppSettings;
use eframe::egui::{self, RichText};

pub struct LibraryLockView;

impl LibraryLockView {
    pub fn render(
        ui: &mut egui::Ui,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
        unlocked: &mut bool,
        settings: &AppSettings,
        draft: &mut String,
        error: &mut Option<String>,
    ) {
        let t = lang.t();
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            ui.label(
                RichText::new(t.library_locked_title)
                    .size(22.0)
                    .strong()
                    .color(theme.text_color()),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(t.library_locked_hint)
                    .size(13.0)
                    .color(theme.secondary_text_color()),
            );
            ui.add_space(16.0);
            let response = ui.add(
                egui::TextEdit::singleline(draft)
                    .password(true)
                    .hint_text(t.library_pin_enter)
                    .desired_width(180.0),
            );
            if response.gained_focus() {
                error.take();
            }
            let submitted =
                response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            ui.add_space(8.0);
            if ui.button(t.library_pin_unlock).clicked() || submitted {
                if settings.verify_library_pin(draft) {
                    *unlocked = true;
                    draft.clear();
                    error.take();
                } else {
                    *error = Some(t.library_pin_wrong.to_string());
                    draft.clear();
                }
            }
            if let Some(message) = error.as_deref() {
                ui.add_space(8.0);
                ui.label(RichText::new(message).color(theme.accent_color()));
            }
        });
    }
}
