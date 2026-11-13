use super::{delay_factor, nearest_text_paragraph, pivot_char_index, text_paragraph_position};
use crate::app::QuietReaderApp;
use crate::app::icons::{Icon, IconButton};
use crate::app::text_view::create_focus_layout_job;
use crate::document::strip_formatting_tags;
use eframe::egui::{self, Align, FontId, Layout, RichText, ScrollArea, Sense, TextFormat, Vec2};

fn icon_button_width(ui: &egui::Ui, text: &str) -> f32 {
    let padding = ui.spacing().button_padding;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let width = ui
        .painter()
        .layout_no_wrap(text.to_string(), font, ui.visuals().text_color())
        .size()
        .x;
    padding.x * 2.0 + 21.0 + width
}

impl QuietReaderApp {
    pub(super) fn render_paragraph(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let chapter_count = doc.chapters().len();
        if chapter_count == 0 {
            return;
        }
        if self.current_chapter >= chapter_count {
            self.current_chapter = chapter_count - 1;
        }
        let Some(chapter) = doc.chapters().get(self.current_chapter) else {
            return;
        };
        if self.focus_read.paragraph_file != self.current_file_path {
            self.focus_read.paragraph_file = self.current_file_path.clone();
            self.focus_read.paragraph_initialized = false;
        }
        if !self.focus_read.paragraph_initialized
            || self.focus_read.paragraph_chapter != self.current_chapter
        {
            let estimated = self.focus_paragraph_from_scroll();
            self.focus_read.paragraph_chapter = self.current_chapter;
            self.focus_read.paragraph_index =
                nearest_text_paragraph(&chapter.paragraphs, estimated, true)
                    .or_else(|| nearest_text_paragraph(&chapter.paragraphs, estimated, false))
                    .unwrap_or(0);
            self.focus_read.paragraph_initialized = true;
            self.apply_paragraph_progress();
        }

        let t = self.settings.ui_language.t();
        let text_color = self.theme.text_color();
        let secondary = self.theme.secondary_text_color();
        let accent = self.theme.accent_color();
        let is_rtl = self.settings.ui_language.is_rtl();

        if !ui.ctx().egui_wants_keyboard_input() {
            let (space, shift, left, right, up, down, page_up, page_down, home, end) =
                ui.input(|input| {
                    (
                        input.key_pressed(egui::Key::Space),
                        input.modifiers.shift,
                        input.key_pressed(egui::Key::ArrowLeft),
                        input.key_pressed(egui::Key::ArrowRight),
                        input.key_pressed(egui::Key::ArrowUp),
                        input.key_pressed(egui::Key::ArrowDown),
                        input.key_pressed(egui::Key::PageUp),
                        input.key_pressed(egui::Key::PageDown),
                        input.key_pressed(egui::Key::Home),
                        input.key_pressed(egui::Key::End),
                    )
                });
            let forward = (space && !shift) || page_down || down || right;
            let backward = (space && shift) || page_up || up || left;
            if forward {
                self.move_paragraph(true);
            } else if backward {
                self.move_paragraph(false);
            } else if home {
                let target = nearest_text_paragraph(&chapter.paragraphs, 0, true);
                if let Some(target) = target {
                    self.focus_read.paragraph_index = target;
                    self.apply_paragraph_progress();
                }
            } else if end {
                let target = nearest_text_paragraph(
                    &chapter.paragraphs,
                    chapter.paragraphs.len().saturating_sub(1),
                    false,
                );
                if let Some(target) = target {
                    self.focus_read.paragraph_index = target;
                    self.apply_paragraph_progress();
                }
            }
        }

        let Some(chapter) = doc.chapters().get(self.current_chapter) else {
            return;
        };
        let paragraph_count = chapter.paragraphs.len();
        let index = self
            .focus_read
            .paragraph_index
            .min(paragraph_count.saturating_sub(1));
        let (position, text_total) = text_paragraph_position(&chapter.paragraphs, index);
        let progress = t
            .paragraph_progress
            .replace("{current}", &position.max(1).to_string())
            .replace("{total}", &text_total.max(1).to_string());

        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Label::new(
                    RichText::new(strip_formatting_tags(&chapter.title))
                        .size(13.0)
                        .color(secondary),
                )
                .wrap(),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(progress).size(12.0).color(secondary));
        });
        ui.separator();

        let horiz_margin = if ui.available_width() < 420.0 {
            8.0
        } else {
            16.0
        };
        let avail_col = (ui.available_width() - horiz_margin).max(140.0);
        let max_width = self.settings.effective_max_column_width().min(avail_col);
        let paragraph = chapter
            .paragraphs
            .get(index)
            .map(String::as_str)
            .unwrap_or("");
        let line_height = self.font_size * self.settings.effective_line_height_scale();
        let top_space = (ui.available_height() * 0.12).clamp(12.0, 140.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(max_width);
                    ui.add_space(top_space);
                    let job = create_focus_layout_job(
                        paragraph,
                        self.font_size,
                        text_color,
                        accent,
                        line_height,
                        is_rtl,
                        self.settings.effective_bionic_reading(),
                        max_width,
                    );
                    ui.add(egui::Label::new(job).wrap().selectable(true));
                    ui.add_space(20.0);
                });
            });

        ui.add_space(4.0);
        ui.add(
            egui::Label::new(
                RichText::new(t.paragraph_mode_desc)
                    .size(11.0)
                    .color(secondary),
            )
            .wrap(),
        );
    }

    pub(super) fn render_rsvp(&mut self, ui: &mut egui::Ui) {
        if !self.focus_read.rsvp.active {
            return;
        }
        let t = self.settings.ui_language.t();
        let text_color = self.theme.text_color();
        let secondary = self.theme.secondary_text_color();
        let accent = self.theme.accent_color();

        if !ui.ctx().egui_wants_keyboard_input() {
            let (space, left, right, up, down, page_up, page_down, home) = ui.input(|input| {
                (
                    input.key_pressed(egui::Key::Space),
                    input.key_pressed(egui::Key::ArrowLeft),
                    input.key_pressed(egui::Key::ArrowRight),
                    input.key_pressed(egui::Key::ArrowUp),
                    input.key_pressed(egui::Key::ArrowDown),
                    input.key_pressed(egui::Key::PageUp),
                    input.key_pressed(egui::Key::PageDown),
                    input.key_pressed(egui::Key::Home),
                )
            });
            if space {
                if self.focus_read.rsvp.finished {
                    self.restart_rsvp();
                    self.focus_read.rsvp.playing = true;
                } else {
                    self.focus_read.rsvp.playing = !self.focus_read.rsvp.playing;
                }
                self.focus_read.rsvp.accumulator = 0.0;
            }
            if right || down {
                self.focus_read.rsvp.playing = false;
                self.step_rsvp_word(true);
            } else if left || up {
                self.focus_read.rsvp.playing = false;
                self.step_rsvp_word(false);
            } else if page_down {
                self.focus_read.rsvp.playing = false;
                self.jump_rsvp_paragraph(true);
            } else if page_up {
                self.focus_read.rsvp.playing = false;
                self.jump_rsvp_paragraph(false);
            } else if home {
                self.focus_read.rsvp.playing = false;
                self.restart_rsvp();
            }
        }

        let target_wpm = self.settings.rsvp_wpm.clamp(100, 1000) as f32;
        let effective_wpm = if self.settings.rsvp_auto_accelerate {
            let start_wpm = (target_wpm * 0.65).max(100.0);
            let progress = (self.focus_read.rsvp.elapsed_reading_secs / 30.0).clamp(0.0, 1.0);
            start_wpm + (target_wpm - start_wpm) * progress
        } else {
            target_wpm
        };

        if self.focus_read.rsvp.playing && !self.focus_read.rsvp.finished {
            let dt = ui.input(|input| input.stable_dt).min(0.2);
            self.focus_read.rsvp.elapsed_reading_secs += dt;
            let interval = 60.0 / effective_wpm;
            let mut elapsed = self.focus_read.rsvp.accumulator + dt;
            let mut steps = 0;
            loop {
                let factor = self
                    .focus_read
                    .rsvp
                    .current_word()
                    .map(|word| delay_factor(&word.text))
                    .unwrap_or(1.0);
                let needed = interval * factor;
                if elapsed < needed || steps >= 8 {
                    break;
                }
                elapsed -= needed;
                self.advance_rsvp_word();
                if !self.focus_read.rsvp.playing || self.focus_read.rsvp.finished {
                    elapsed = 0.0;
                    break;
                }
                steps += 1;
            }
            self.focus_read.rsvp.accumulator = elapsed;
            ui.ctx().request_repaint();
        }

        let word = self
            .focus_read
            .rsvp
            .current_word()
            .map(|word| word.text.clone())
            .unwrap_or_default();
        let chapter = self.focus_read.rsvp.chapter;
        let total_paragraphs = self
            .document
            .as_ref()
            .and_then(|doc| doc.chapters().get(chapter))
            .map(|chapter| chapter.paragraphs.len())
            .unwrap_or(0);
        let paragraph_index = self
            .focus_read
            .rsvp
            .current_word()
            .map(|word| word.paragraph)
            .unwrap_or(0);
        let progress = t
            .paragraph_progress
            .replace("{current}", &(paragraph_index + 1).to_string())
            .replace("{total}", &total_paragraphs.max(1).to_string());

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t.rsvp_title)
                    .size(15.0)
                    .strong()
                    .color(accent),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(IconButton::new(Icon::Cross, t.close_btn))
                    .on_hover_text(t.close_btn)
                    .clicked()
                {
                    self.close_speed_reading();
                }
            });
        });
        ui.separator();

        let avail_w = ui.available_width().max(180.0);
        let avail_h = ui.available_height().max(160.0);
        let viewport_cap = (avail_w * 0.11)
            .clamp(22.0, 72.0)
            .min((avail_h * 0.18).clamp(22.0, 72.0));
        let mut font_size = (self.font_size * 2.4).clamp(22.0, viewport_cap);
        let base_word_w = ui
            .painter()
            .layout_no_wrap(word.clone(), FontId::proportional(font_size), text_color)
            .size()
            .x;
        let max_word_w = (avail_w - 24.0).max(80.0);
        if base_word_w > max_word_w && base_word_w > 0.0 {
            font_size = (font_size * (max_word_w / base_word_w)).max(16.0);
        }

        let font = FontId::proportional(font_size);
        let pivot = pivot_char_index(&word);
        let pivot_byte = word
            .char_indices()
            .nth(pivot)
            .map(|(byte, _)| byte)
            .unwrap_or(0);
        let pivot_end = word[pivot_byte..]
            .chars()
            .next()
            .map(|ch| pivot_byte + ch.len_utf8())
            .unwrap_or(pivot_byte);
        let normal = TextFormat {
            font_id: font.clone(),
            color: text_color,
            ..Default::default()
        };
        let highlighted = TextFormat {
            font_id: font.clone(),
            color: accent,
            ..Default::default()
        };
        let mut job = egui::text::LayoutJob::default();
        job.append(&word[..pivot_byte], 0.0, normal.clone());
        job.append(&word[pivot_byte..pivot_end], 0.0, highlighted.clone());
        job.append(&word[pivot_end..], 0.0, normal);
        job.wrap.max_width = f32::INFINITY;
        let galley = ui.painter().layout_job(job);
        let prefix_width = ui
            .painter()
            .layout_no_wrap(word[..pivot_byte].to_string(), font.clone(), text_color)
            .size()
            .x;
        let pivot_width = ui
            .painter()
            .layout_no_wrap(
                word[pivot_byte..pivot_end].to_string(),
                font.clone(),
                accent,
            )
            .size()
            .x;
        let pivot_x = prefix_width + pivot_width * 0.5;
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(avail_w, (font_size * 2.2).clamp(44.0, 160.0)),
            Sense::hover(),
        );
        let center = rect.center();
        let min_x = rect.left() + 8.0;
        let max_x = (rect.right() - galley.size().x - 8.0).max(min_x);
        let pos_x = (center.x - pivot_x).clamp(min_x, max_x);
        let position = egui::pos2(pos_x, center.y - galley.size().y * 0.5);
        ui.painter().galley(position, galley, text_color);

        let tick_len = (font_size * 0.22).clamp(6.0, 12.0);
        let anchor_x = (pos_x + pivot_x).clamp(rect.left() + 8.0, rect.right() - 8.0);
        ui.painter().line_segment(
            [
                egui::pos2(anchor_x, rect.top() + 4.0),
                egui::pos2(anchor_x, (rect.top() + 4.0 + tick_len).min(center.y - 4.0)),
            ],
            egui::Stroke::new(1.0, accent.gamma_multiply(0.5)),
        );
        ui.painter().line_segment(
            [
                egui::pos2(
                    anchor_x,
                    (rect.bottom() - 4.0 - tick_len).max(center.y + 4.0),
                ),
                egui::pos2(anchor_x, rect.bottom() - 4.0),
            ],
            egui::Stroke::new(1.0, accent.gamma_multiply(0.5)),
        );

        let index = if self.focus_read.rsvp.words.is_empty() {
            0
        } else {
            self.focus_read.rsvp.index + 1
        };
        let total = self.focus_read.rsvp.words.len();
        let controls_max_w = avail_w.min(860.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(controls_max_w);
            if self.focus_read.rsvp.finished {
                ui.label(RichText::new(t.rsvp_finished).size(15.0).color(accent));
            }
            ui.label(
                RichText::new(format!("{index} / {total}"))
                    .size(12.0)
                    .color(secondary),
            );
            ui.label(RichText::new(progress).size(12.0).color(secondary));
            let actual_wpm = if self.focus_read.rsvp.elapsed_reading_secs >= 1.0
                && self.focus_read.rsvp.words_read_in_session > 0
            {
                ((self.focus_read.rsvp.words_read_in_session as f32)
                    / (self.focus_read.rsvp.elapsed_reading_secs / 60.0))
                    .round() as usize
            } else {
                effective_wpm.round() as usize
            };
            let elapsed_secs = self.focus_read.rsvp.elapsed_reading_secs.round() as u64;
            let elapsed_mins = elapsed_secs / 60;
            let elapsed_rem = elapsed_secs % 60;
            let words_left = self
                .focus_read
                .rsvp
                .words
                .len()
                .saturating_sub(self.focus_read.rsvp.index);
            let rem_secs = ((words_left as f32) / (effective_wpm / 60.0)).round() as u64;
            let rem_mins = rem_secs / 60;
            let rem_rem = rem_secs % 60;
            let rate_part = format!("{actual_wpm} WPM ({})", t.rsvp_actual_speed);
            let elapsed_part = format!("{elapsed_mins:02}:{elapsed_rem:02} {}", t.rsvp_elapsed);
            let rem_part = format!("{rem_mins:02}:{rem_rem:02} {}", t.rsvp_remaining);
            crate::app::icons::dotted_responsive_row(
                ui,
                &[&rate_part, &elapsed_part, &rem_part],
                11.0,
                secondary,
                secondary.gamma_multiply(0.75),
            );
            ui.add_space(8.0);
            let play_text = if self.focus_read.rsvp.playing {
                t.rsvp_pause
            } else {
                t.rsvp_play
            };
            let speed_label = if self.settings.rsvp_auto_accelerate {
                format!(
                    "{}/{} {}",
                    effective_wpm.round() as usize,
                    self.settings.rsvp_wpm,
                    t.rsvp_speed
                )
            } else {
                format!("{} {}", self.settings.rsvp_wpm, t.rsvp_speed)
            };
            let row_width = icon_button_width(ui, play_text)
                + icon_button_width(ui, t.rsvp_restart)
                + icon_button_width(ui, t.rsvp_auto_accelerate)
                + icon_button_width(ui, "-")
                + icon_button_width(ui, "+")
                + ui.painter()
                    .layout_no_wrap(speed_label.clone(), FontId::proportional(12.0), secondary)
                    .size()
                    .x
                + ui.spacing().item_spacing.x * 5.0;
            let row_avail = ui.available_width();
            let render_controls = |ui: &mut egui::Ui| {
                let play_icon = if self.focus_read.rsvp.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                };
                if ui
                    .add(
                        IconButton::new(play_icon, play_text)
                            .selected(self.focus_read.rsvp.playing),
                    )
                    .clicked()
                {
                    if self.focus_read.rsvp.finished {
                        self.restart_rsvp();
                        self.focus_read.rsvp.playing = true;
                    } else {
                        self.focus_read.rsvp.playing = !self.focus_read.rsvp.playing;
                    }
                    self.focus_read.rsvp.accumulator = 0.0;
                }
                if ui
                    .add(IconButton::new(Icon::Refresh, t.rsvp_restart))
                    .clicked()
                {
                    self.restart_rsvp();
                }
                if ui
                    .add(
                        IconButton::new(Icon::AutoScroll, t.rsvp_auto_accelerate)
                            .selected(self.settings.rsvp_auto_accelerate),
                    )
                    .on_hover_text(t.rsvp_auto_accelerate_desc)
                    .clicked()
                {
                    self.settings.rsvp_auto_accelerate = !self.settings.rsvp_auto_accelerate;
                    self.focus_read.rsvp.words_read_in_session = 0;
                    self.focus_read.rsvp.elapsed_reading_secs = 0.0;
                    self.mark_settings_dirty();
                }
                if ui.add(IconButton::new(Icon::Minus, "-")).clicked() {
                    self.cycle_rsvp_speed(false);
                }
                ui.label(RichText::new(speed_label).size(12.0).color(secondary));
                if ui.add(IconButton::new(Icon::Plus, "+")).clicked() {
                    self.cycle_rsvp_speed(true);
                }
            };
            if row_width <= row_avail {
                let row_offset = ((row_avail - row_width) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(row_offset);
                    render_controls(ui);
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    render_controls(ui);
                });
            }
            ui.add_space(6.0);
            ui.add(egui::Label::new(RichText::new(t.rsvp_hint).size(11.0).color(secondary)).wrap());
        });
    }
}
