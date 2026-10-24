use super::icons::{Icon, IconButton};
use super::pdf_view::{PdfViewState, truncate_str};
use super::theme::ReaderTheme;
use crate::core::license::ProFeature;
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
        smart_scan_allowed: bool,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        let t = lang.t();
        let effective_smart_dark = settings.smart_dark && smart_scan_allowed;
        let effective_auto_crop = settings.auto_crop && smart_scan_allowed;
        let copied_text =
            |preview: &str| crate::i18n::fill(t.extra.pdf_copied_prefix, &[("{preview}", preview)]);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if ui
                .add_enabled(
                    self.current_page > 0,
                    IconButton::new(Icon::ArrowLeft, "").min_size(Vec2::new(28.0, 24.0)),
                )
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
                .add_enabled(
                    self.current_page + 1 < page_count,
                    IconButton::new(Icon::ArrowRight, "").min_size(Vec2::new(28.0, 24.0)),
                )
                .on_hover_text(t.next_page)
                .clicked()
            {
                let step = if settings.layout == "Double" { 2 } else { 1 };
                self.go_to_page((self.current_page + step).min(page_count.saturating_sub(1)));
            }

            ui.separator();

            if ui
                .add(IconButton::new(Icon::Minus, "").min_size(Vec2::new(26.0, 24.0)))
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
                .add(IconButton::new(Icon::Plus, "").min_size(Vec2::new(26.0, 24.0)))
                .on_hover_text(t.zoom_in)
                .clicked()
            {
                super::paged_common::zoom_in(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            if ui
                .selectable_label(settings.fit_mode == "FitWidth", t.fit_width_btn)
                .on_hover_text(t.fit_width)
                .clicked()
            {
                settings.fit_mode = "FitWidth".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui
                .selectable_label(settings.fit_mode == "FitWindow", t.fit_window_btn)
                .on_hover_text(t.fit_window)
                .clicked()
            {
                settings.fit_mode = "FitWindow".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui
                .button(t.rotate_clockwise)
                .on_hover_text(t.extra.pdf_rotate_tip)
                .clicked()
            {
                settings.rotation = (settings.rotation + 90) % 360;
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            let dark_resp = ui.add(
                crate::app::icons::IconButton::new(
                    crate::app::icons::Icon::Moon,
                    t.smart_dark_mode,
                )
                .selected(effective_smart_dark),
            );
            if dark_resp.on_hover_text(t.smart_dark_mode_tooltip).clicked() {
                if smart_scan_allowed {
                    settings.smart_dark = !settings.smart_dark;
                    self.clear_cache();
                } else {
                    *on_pro_required = Some(ProFeature::SmartScanProcessing);
                }
            }

            let crop_resp = ui
                .add(
                    crate::app::icons::IconButton::new(
                        crate::app::icons::Icon::Crop,
                        t.auto_crop_margins,
                    )
                    .selected(effective_auto_crop),
                )
                .on_hover_text(t.auto_crop_margins_tooltip);
            if crop_resp.clicked() {
                if smart_scan_allowed {
                    settings.auto_crop = !settings.auto_crop;
                    self.pan_offset = Vec2::ZERO;
                } else {
                    *on_pro_required = Some(ProFeature::SmartScanProcessing);
                }
            }
            if effective_auto_crop {
                crop_resp.context_menu(|ui| {
                    ui.label(RichText::new(t.crop_sensitivity).strong());
                    let slider = egui::Slider::new(&mut settings.auto_crop_tolerance, 5..=80)
                        .text(t.crop_tolerance);
                    if ui.add(slider).changed() {
                        self.clear_cache();
                    }
                });
            }

            ui.separator();

            if ui
                .selectable_label(settings.layout == "Single", t.single_page_btn)
                .on_hover_text(t.extra.pdf_single_tip)
                .clicked()
            {
                settings.layout = "Single".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui
                .selectable_label(settings.layout == "Double", t.double_page_btn)
                .on_hover_text(t.extra.pdf_double_tip)
                .clicked()
            {
                settings.layout = "Double".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui
                .selectable_label(settings.layout == "Continuous", t.scroll_btn)
                .on_hover_text(t.extra.pdf_scroll_tip)
                .clicked()
            {
                settings.layout = "Continuous".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if !self.selected_text.is_empty() {
                if ui
                    .button(RichText::new(t.copy).strong().color(theme.accent_color()))
                    .on_hover_text(t.extra.pdf_copy_selected_tip)
                    .clicked()
                {
                    ui.ctx().copy_text(self.selected_text.clone());
                    let preview = truncate_str(&self.selected_text, 35);
                    let copied = copied_text(&preview);
                    *status_message = Some(copied.clone());
                    self.copy_notification = Some((copied, Instant::now()));
                }
                if ui
                    .add(
                        IconButton::new(Icon::Close, "")
                            .font_size(11.0)
                            .min_size(Vec2::splat(20.0)),
                    )
                    .on_hover_text(t.extra.pdf_clear_selection_tip)
                    .clicked()
                {
                    self.selection_start = None;
                    self.selection_current = None;
                    self.selection_dirty = false;
                    self.selected_text.clear();
                }
            } else {
                ui.label(
                    RichText::new(t.extra.pdf_selection_hint)
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

    pub fn render_selection_popups(
        &mut self,
        ctx: &egui::Context,
        theme: &ReaderTheme,
        status_message: &mut Option<String>,
        lang: crate::i18n::UiLanguage,
        on_translate: &mut Option<String>,
        on_dictionary: &mut Option<String>,
    ) {
        let t = lang.t();
        let copied_text =
            |preview: &str| crate::i18n::fill(t.extra.pdf_copied_prefix, &[("{preview}", preview)]);
        if !self.selected_text.is_empty() {
            let ctrl_c = ctx.input(|i| {
                (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::C)
            });
            if ctrl_c {
                ctx.copy_text(self.selected_text.clone());
                let preview = truncate_str(&self.selected_text, 35);
                let copied = copied_text(&preview);
                *status_message = Some(copied.clone());
                self.copy_notification = Some((copied, Instant::now()));
            }
        }

        if !self.selected_text.is_empty()
            && let Some(curr) = self.selection_current
        {
            let pill_pos = curr + Vec2::new(12.0, 12.0);
            egui::Area::new(egui::Id::unique("pdf_selection_pill"))
                .fixed_pos(pill_pos)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new(t.extra.pdf_copy_btn).size(12.0))
                                .clicked()
                            {
                                ctx.copy_text(self.selected_text.clone());
                                let preview = truncate_str(&self.selected_text, 35);
                                let copied = copied_text(&preview);
                                *status_message = Some(copied.clone());
                                self.copy_notification = Some((copied, Instant::now()));
                            }
                            if ui
                                .button(RichText::new(lang.t().translate_selection).size(12.0))
                                .clicked()
                            {
                                *on_translate = Some(self.selected_text.clone());
                            }
                            if ui
                                .button(RichText::new(lang.t().dictionary_lookup).size(12.0))
                                .clicked()
                            {
                                *on_dictionary = Some(self.selected_text.clone());
                            }
                            if ui
                                .add(
                                    IconButton::new(Icon::Close, "")
                                        .font_size(11.0)
                                        .min_size(Vec2::splat(20.0)),
                                )
                                .clicked()
                            {
                                self.selection_start = None;
                                self.selection_current = None;
                                self.selection_dirty = false;
                                self.selected_text.clear();
                            }
                        });
                    });
                });
        }

        if let Some((msg, time)) = &self.copy_notification {
            if time.elapsed().as_secs_f32() < 2.0 {
                egui::Area::new(egui::Id::unique("pdf_copy_toast"))
                    .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -45.0))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(RichText::new(msg).strong().color(theme.accent_color()));
                        });
                    });
            } else {
                self.copy_notification = None;
            }
        }
    }
}
