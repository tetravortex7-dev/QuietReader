use super::pdf_view::{truncate_str, PdfViewState};
use super::theme::ReaderTheme;
use crate::settings::PdfSettings;
use eframe::egui::{self, RichText, Vec2};
use std::time::Instant;

impl PdfViewState {
    pub fn render_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        page_count: usize,
        settings: &mut PdfSettings,
        theme: &ReaderTheme,
        status_message: &mut Option<String>,
        format_label: &str,
        lang: crate::i18n::UiLanguage,
    ) {
        let t = lang.t();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if ui
                .add_enabled(self.current_page > 0, egui::Button::new("◀"))
                .on_hover_text(t.prev_page)
                .clicked()
            {
                let step = if settings.layout == "Double" { 2 } else { 1 };
                self.go_to_page(self.current_page.saturating_sub(step));
            }

            if self.page_input.is_empty() {
                self.page_input = (self.current_page + 1).to_string();
            }
            let cur_display = self.current_page + 1;
            ui.label(RichText::new(t.page_prefix).size(13.0));
            let text_edit = ui.add(
                egui::TextEdit::singleline(&mut self.page_input)
                    .desired_width(38.0)
                    .font(egui::TextStyle::Body),
            );
            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(target) =
                    super::paged_common::parse_page_input(&self.page_input, page_count)
                {
                    self.go_to_page(target);
                }
                self.page_input = (self.current_page + 1).to_string();
            } else if !text_edit.has_focus() {
                self.page_input = cur_display.to_string();
            }

            ui.label(RichText::new(format!("{} {page_count}", t.page_of)).size(13.0));

            if ui
                .add_enabled(self.current_page + 1 < page_count, egui::Button::new("▶"))
                .on_hover_text(t.next_page)
                .clicked()
            {
                let step = if settings.layout == "Double" { 2 } else { 1 };
                self.go_to_page((self.current_page + step).min(page_count.saturating_sub(1)));
            }

            ui.separator();

            if ui
                .button("−")
                .on_hover_text(t.zoom_out)
                .clicked()
            {
                super::paged_common::zoom_out(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            let zoom_lbl = format!("{:.0}%", settings.zoom_percent);
            if ui
                .button(RichText::new(zoom_lbl).strong())
                .on_hover_text(t.zoom_reset)
                .clicked()
            {
                super::paged_common::zoom_reset(
                    &mut settings.zoom_percent,
                    &mut settings.fit_mode,
                    &mut self.pan_offset,
                );
            }

            if ui
                .button("+")
                .on_hover_text(t.zoom_in)
                .clicked()
            {
                super::paged_common::zoom_in(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            if ui
                .selectable_label(settings.fit_mode == "FitWidth", t.fit_width_btn)
                .on_hover_text("Масштабировать по ширине окна")
                .clicked()
            {
                settings.fit_mode = "FitWidth".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui
                .selectable_label(settings.fit_mode == "FitWindow", t.fit_window_btn)
                .on_hover_text("Уместить страницу целиком в окно")
                .clicked()
            {
                settings.fit_mode = "FitWindow".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui
                .button(t.rotate_clockwise)
                .on_hover_text("Повернуть страницы на 90° по часовой стрелке")
                .clicked()
            {
                settings.rotation = (settings.rotation + 90) % 360;
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui
                .selectable_label(settings.layout == "Single", t.single_page_btn)
                .on_hover_text("Одиночная страница")
                .clicked()
            {
                settings.layout = "Single".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui
                .selectable_label(settings.layout == "Double", t.double_page_btn)
                .on_hover_text("Две страницы (разворот)")
                .clicked()
            {
                settings.layout = "Double".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui
                .selectable_label(settings.layout == "Continuous", t.scroll_btn)
                .on_hover_text("Непрерывный вертикальный свиток страниц")
                .clicked()
            {
                settings.layout = "Continuous".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if !self.selected_text.is_empty() {
                if ui
                    .button(
                        RichText::new(t.copy)
                            .strong()
                            .color(theme.accent_color()),
                    )
                    .on_hover_text("Скопировать выделенный текст (Ctrl+C)")
                    .clicked()
                {
                    ui.ctx().copy_text(self.selected_text.clone());
                    let preview = truncate_str(&self.selected_text, 35);
                    *status_message = Some(format!("Скопировано: \"{preview}\""));
                    self.copy_notification =
                        Some((format!("Скопировано: \"{preview}\""), Instant::now()));
                }
                if ui
                    .small_button("X")
                    .on_hover_text("Снять выделение")
                    .clicked()
                {
                    self.selection_start = None;
                    self.selection_current = None;
                    self.selection_dirty = false;
                    self.selected_text.clear();
                }
            } else {
                ui.label(
                    RichText::new("Выделение: зажмите ЛКМ на странице")
                        .size(11.5)
                        .italics()
                        .color(theme.text_color().gamma_multiply(0.65)),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format_label)
                        .size(11.0)
                        .strong()
                        .color(theme.accent_color()),
                );
            });
        });
    }
}
