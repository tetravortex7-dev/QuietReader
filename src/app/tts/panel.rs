use eframe::egui::{self, Color32, Frame, Id, Order, RichText, Stroke, Vec2};

use super::controller::TtsController;
use super::types::{TtsEngineKind, TtsVoiceInfo};
use super::voices::{format_voice_name, voice_matches_lang};
use crate::app::icons::{Icon, IconButton, IconLabel};
use crate::app::theme::ReaderTheme;

impl TtsController {
    pub fn render_panel(
        &mut self,
        ctx: &egui::Context,
        theme: &ReaderTheme,
        current_book_has_text: bool,
        book_language: Option<&str>,
        on_play_requested: &mut bool,
        lang: crate::i18n::UiLanguage,
    ) {
        if !self.show_panel {
            return;
        }

        let t = lang.t();
        let st = self.status();

        egui::Area::new(Id::unique("tts_floating_panel"))
            .order(Order::Foreground)
            .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -18.0))
            .show(ctx, |ui| {
                let frame = Frame::popup(ui.style())
                    .fill(theme.panel_bg_color())
                    .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.4)))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(14, 10));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            IconLabel::new(Icon::Speech, t.tts_title)
                                .size(13.5)
                                .strong()
                                .color(theme.text_color()),
                        );

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);

                        let is_sys = st.engine == TtsEngineKind::System;
                        let is_edge = st.engine == TtsEngineKind::Edge;

                        if ui
                            .selectable_label(is_sys, t.tts_system)
                            .on_hover_text(t.tts_system_tip)
                            .clicked()
                            && !is_sys
                        {
                            self.set_engine(TtsEngineKind::System);
                        }
                        if ui
                            .selectable_label(is_edge, t.tts_edge)
                            .on_hover_text(t.tts_internet_tip)
                            .clicked()
                            && !is_edge
                        {
                            self.set_engine(TtsEngineKind::Edge);
                        }

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);

                        if !st.is_available {
                            ui.label(
                                RichText::new(t.tts_unavailable)
                                    .color(Color32::RED)
                                    .size(12.0),
                            );
                        } else if !current_book_has_text {
                            ui.label(
                                RichText::new(t.tts_open_text_book)
                                    .color(theme.secondary_text_color())
                                    .size(12.0),
                            );
                        } else {
                            let prev_btn = ui
                                .add(
                                    IconButton::new(Icon::ChevronUp, "")
                                        .min_size(Vec2::new(26.0, 24.0)),
                                )
                                .on_hover_text(t.tts_prev_paragraph);
                            if prev_btn.clicked() {
                                self.prev_paragraph();
                            }

                            if st.is_playing && !st.is_paused {
                                let pause_btn = ui.add(
                                    IconButton::new(Icon::Pause, t.tts_pause)
                                        .min_size(Vec2::new(76.0, 24.0)),
                                );
                                if pause_btn.clicked() {
                                    self.pause();
                                }
                            } else {
                                let play_icon = Icon::Play;
                                let play_label = if st.is_paused { t.tts_play } else { t.speech };
                                let play_btn = ui.add(
                                    IconButton::new(play_icon, play_label)
                                        .min_size(Vec2::new(88.0, 24.0)),
                                );
                                if play_btn.clicked() {
                                    if st.is_paused {
                                        self.resume();
                                    } else {
                                        *on_play_requested = true;
                                    }
                                }
                            }

                            let next_btn = ui
                                .add(
                                    IconButton::new(Icon::ChevronDown, "")
                                        .min_size(Vec2::new(26.0, 24.0)),
                                )
                                .on_hover_text(t.tts_next_paragraph);
                            if next_btn.clicked() {
                                self.next_paragraph();
                            }

                            if st.is_playing || st.is_paused {
                                let stop_btn = ui.add(
                                    IconButton::new(Icon::Close, t.tts_stop)
                                        .min_size(Vec2::new(64.0, 24.0)),
                                );
                                if stop_btn.clicked() {
                                    self.stop();
                                }
                            }

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);

                            let rates = [0.8, 1.0, 1.25, 1.5, 2.0];
                            for r in rates {
                                let is_curr = (st.rate - r).abs() < 0.05;
                                let label = format!("{r}x");
                                let btn = ui.selectable_label(is_curr, label);
                                if btn.clicked() {
                                    self.set_rate(r);
                                }
                            }

                            let matched_voices: Vec<&TtsVoiceInfo> =
                                if let Some(lang) = book_language {
                                    let m: Vec<&TtsVoiceInfo> = st
                                        .voices
                                        .iter()
                                        .filter(|v| voice_matches_lang(&v.language, lang))
                                        .collect();
                                    if m.is_empty() {
                                        st.voices.iter().collect()
                                    } else {
                                        m
                                    }
                                } else {
                                    st.voices.iter().collect()
                                };

                            if !matched_voices.is_empty() {
                                ui.add_space(6.0);
                                ui.separator();
                                ui.add_space(6.0);

                                if !matched_voices
                                    .iter()
                                    .any(|v| Some(&v.id) == st.selected_voice_id.as_ref())
                                {
                                    if let Some(first) = matched_voices.first() {
                                        self.set_voice(first.id.clone());
                                    }
                                }

                                let current_voice_name = st
                                    .selected_voice_id
                                    .as_ref()
                                    .and_then(|id| matched_voices.iter().find(|v| v.id == *id))
                                    .map(|v| format_voice_name(&v.name))
                                    .unwrap_or_else(|| t.tts_voice.to_string());

                                egui::ComboBox::from_id_salt("tts_voice_combo")
                                    .selected_text(current_voice_name)
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        for v in &matched_voices {
                                            let is_sel =
                                                st.selected_voice_id.as_deref() == Some(&v.id);
                                            let clean_name = format_voice_name(&v.name);
                                            let label = format!("{} ({})", clean_name, v.language);
                                            if ui.selectable_label(is_sel, label).clicked() {
                                                self.set_voice(v.id.clone());
                                            }
                                        }
                                    });
                            }

                            let error_text = st.error.as_deref().or(if st.playback_failed {
                                Some(t.extra.tts_playback_error)
                            } else {
                                None
                            });
                            if let Some(err) = error_text {
                                ui.add_space(6.0);
                                ui.label(RichText::new(err).size(11.5).color(Color32::RED));
                            } else if st.is_playing || st.is_paused {
                                ui.add_space(6.0);
                                let status_str = crate::i18n::fill(
                                    t.extra.tts_chapter_paragraph,
                                    &[
                                        ("{chapter}", &(st.current_chapter + 1).to_string()),
                                        ("{paragraph}", &(st.current_paragraph + 1).to_string()),
                                        ("{total}", &st.total_paragraphs.to_string()),
                                    ],
                                );
                                ui.label(
                                    RichText::new(status_str)
                                        .size(12.0)
                                        .color(theme.accent_color())
                                        .strong(),
                                );
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    IconButton::new(Icon::Close, "")
                                        .min_size(Vec2::new(20.0, 20.0)),
                                )
                                .on_hover_text(t.tts_close_panel)
                                .clicked()
                            {
                                self.show_panel = false;
                            }
                        });
                    });
                });
            });
    }
}
