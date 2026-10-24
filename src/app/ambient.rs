use super::ambient_synth::{AmbientSharedParams, AmbientSynth, AmbientSynthParams};
use super::icons::{Icon, IconButton, paint_icon};
use super::theme::ReaderTheme;
pub use crate::settings::AmbientSettings;
use eframe::egui::{self, Color32, Frame, RichText, Sense, Stroke, Vec2};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct AmbientController {
    pub is_open: bool,
    pub settings: AmbientSettings,
    shared_params: Arc<AmbientSharedParams>,
    audio_device: Option<rodio::MixerDeviceSink>,
    audio_player: Option<rodio::Player>,
    audio_initialized: bool,
    audio_error: Option<String>,
    timer_preset_minutes: Option<u64>,
    pause_after: Option<Instant>,
}

fn params_from_settings(settings: &AmbientSettings) -> AmbientSynthParams {
    AmbientSynthParams {
        is_playing: settings.is_playing,
        master_volume: settings.master_volume,
        tone_softness: settings.tone_softness,
        duck_during_tts: settings.duck_during_tts,
        duck_amount: settings.duck_amount,
        tts_is_active: false,
        brown_noise_vol: settings.brown_noise_vol,
        pink_noise_vol: settings.pink_noise_vol,
        rain_vol: settings.rain_vol,
        fireplace_vol: settings.fireplace_vol,
        lofi_vol: settings.lofi_vol,
        binaural_vol: settings.binaural_vol,
    }
}

impl AmbientController {
    pub fn new(settings: AmbientSettings) -> Self {
        let shared_params = Arc::new(AmbientSharedParams::new(params_from_settings(&settings)));

        let mut ctrl = Self {
            is_open: false,
            settings,
            shared_params,
            audio_device: None,
            audio_player: None,
            audio_initialized: false,
            audio_error: None,
            timer_preset_minutes: None,
            pause_after: None,
        };
        if ctrl.settings.is_playing {
            if ctrl.settings.resume_on_startup {
                ctrl.init_audio_device();
            }
            if ctrl.audio_player.is_none() {
                ctrl.settings.is_playing = false;
                ctrl.settings.timer_end_unix = None;
                ctrl.sync_shared_params();
            }
        }
        ctrl
    }

    fn init_audio_device(&mut self) {
        if self.audio_initialized && self.audio_player.is_some() {
            return;
        }

        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => sink,
            Err(error) => {
                self.audio_initialized = false;
                self.audio_device = None;
                self.audio_player = None;
                self.audio_error = Some(error.to_string());
                return;
            }
        };

        let sample_rate = sink.config().sample_rate().get();
        let player = rodio::Player::connect_new(&sink.mixer());
        let synth = AmbientSynth::new(Arc::clone(&self.shared_params), sample_rate);
        player.append(synth);
        player.play();
        self.audio_device = Some(sink);
        self.audio_player = Some(player);
        self.audio_initialized = true;
        self.audio_error = None;
    }

    pub fn toggle_panel(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn suspend(&mut self) {
        if !self.is_open && !self.settings.is_playing {
            return;
        }
        self.is_open = false;
        self.settings.is_playing = false;
        self.pause_after = None;
        if let Some(player) = &self.audio_player {
            player.pause();
        }
        self.sync_shared_params();
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        self.settings.is_playing
    }

    pub fn audio_error(&self) -> Option<&str> {
        self.audio_error.as_deref()
    }

    pub fn toggle_play(&mut self) {
        self.settings.is_playing = !self.settings.is_playing;
        self.pause_after = None;
        if self.settings.is_playing {
            self.init_audio_device();
            if let Some(ref p) = self.audio_player {
                p.play();
            } else {
                self.settings.is_playing = false;
            }
        } else if let Some(player) = &self.audio_player {
            player.pause();
        }
        self.sync_shared_params();
    }

    pub fn set_tts_active(&mut self, active: bool) {
        self.shared_params.set_tts_active(active);
    }

    pub fn apply_preset(&mut self, preset_id: &str) {
        self.settings.selected_preset = Some(preset_id.to_string());
        match preset_id {
            "laser_focus" => {
                self.settings.brown_noise_vol = 0.80;
                self.settings.binaural_vol = 0.40;
                self.settings.pink_noise_vol = 0.0;
                self.settings.rain_vol = 0.0;
                self.settings.fireplace_vol = 0.0;
                self.settings.lofi_vol = 0.0;
            }
            "rainy_evening" => {
                self.settings.rain_vol = 0.75;
                self.settings.fireplace_vol = 0.60;
                self.settings.brown_noise_vol = 0.0;
                self.settings.pink_noise_vol = 0.0;
                self.settings.lofi_vol = 0.0;
                self.settings.binaural_vol = 0.0;
            }
            "night_lofi" => {
                self.settings.lofi_vol = 0.80;
                self.settings.rain_vol = 0.35;
                self.settings.brown_noise_vol = 0.0;
                self.settings.pink_noise_vol = 0.0;
                self.settings.fireplace_vol = 0.0;
                self.settings.binaural_vol = 0.0;
            }
            "deep_space" => {
                self.settings.binaural_vol = 0.75;
                self.settings.brown_noise_vol = 0.40;
                self.settings.pink_noise_vol = 0.0;
                self.settings.rain_vol = 0.0;
                self.settings.fireplace_vol = 0.0;
                self.settings.lofi_vol = 0.0;
            }
            _ => {}
        }
        self.sync_shared_params();
    }

    pub fn set_timer(&mut self, mins: Option<u64>) {
        self.timer_preset_minutes = mins;
        self.pause_after = None;
        self.settings.timer_end_unix = mins.map(|minutes| {
            crate::core::license::unix_now().saturating_add(minutes.saturating_mul(60))
        });
    }

    pub fn remaining_timer_seconds(&self) -> Option<u64> {
        self.settings
            .timer_end_unix
            .map(|end| end.saturating_sub(crate::core::license::unix_now()))
    }

    pub fn tick(&mut self, ctx: &egui::Context) -> bool {
        let mut state_changed = false;
        if self.settings.is_playing {
            if let Some(remaining) = self.remaining_timer_seconds() {
                if remaining == 0 {
                    self.settings.is_playing = false;
                    self.settings.timer_end_unix = None;
                    self.timer_preset_minutes = None;
                    self.pause_after = Some(Instant::now() + Duration::from_secs(2));
                    self.sync_shared_params();
                    state_changed = true;
                } else {
                    let delay = if remaining <= 1 { 200 } else { 1000 };
                    ctx.request_repaint_after(Duration::from_millis(delay));
                }
            }
        }

        if let Some(pause_at) = self.pause_after {
            if Instant::now() >= pause_at {
                if let Some(player) = &self.audio_player {
                    player.pause();
                }
                self.pause_after = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
        }

        state_changed
    }

    pub fn sync_shared_params(&mut self) {
        let mut params = params_from_settings(&self.settings);
        params.tts_is_active = self.shared_params.tts_is_active();
        self.shared_params.store(params);
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
    ) -> bool {
        if !self.is_open {
            return false;
        }

        let mut changed = false;
        let mut close_panel = false;
        let t = lang.t();

        let window = egui::Window::new(t.ambient_title)
            .id(egui::Id::unique("ambient_sound_panel"))
            .resizable(false)
            .collapsible(false)
            .default_width(340.0)
            .max_height((ctx.content_rect().height() - 80.0).max(240.0))
            .vscroll(true)
            .anchor(egui::Align2::RIGHT_TOP, [-16.0, 50.0])
            .frame(
                Frame::window(&ctx.global_style())
                    .fill(theme.panel_bg_color())
                    .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.35)))
                    .corner_radius(8.0)
                    .inner_margin(12.0),
            );

        window.show(ctx, |ui| {
            ui.set_max_width(330.0);
            ui.style_mut().spacing.item_spacing = Vec2::new(8.0, 6.0);

            ui.horizontal(|ui| {
                let (ic_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                let head_color = if self.settings.is_playing {
                    theme.accent_color()
                } else {
                    theme.secondary_text_color()
                };
                paint_icon(ui.painter(), Icon::Headphones, ic_rect, head_color);
                ui.label(RichText::new(t.ambient_title).strong().size(15.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_btn = IconButton::new(Icon::Cross, "").min_size(Vec2::splat(18.0));
                    if ui.add(close_btn).clicked() {
                        close_panel = true;
                    }
                });
            });

            ui.add_space(4.0);

            if let Some(error) = self.audio_error() {
                ui.label(
                    RichText::new(t.ambient_audio_error.replace("{error}", error))
                        .size(10.5)
                        .color(Color32::from_rgb(200, 90, 70)),
                );
                ui.add_space(2.0);
            }

            ui.horizontal(|ui| {
                let play_icon = if self.settings.is_playing {
                    Icon::Pause
                } else {
                    Icon::Play
                };
                let play_label = if self.settings.is_playing {
                    t.ambient_pause
                } else {
                    t.ambient_play
                };

                let btn = IconButton::new(play_icon, play_label).min_size(Vec2::new(96.0, 28.0));
                if ui.add(btn).clicked() {
                    self.toggle_play();
                    changed = true;
                }

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t.ambient_master_volume).size(12.0));
                        let pct = (self.settings.master_volume * 100.0).round() as u32;
                        ui.label(RichText::new(format!("{pct}%")).size(12.0).strong());
                    });
                    if ui
                        .add(
                            egui::Slider::new(&mut self.settings.master_volume, 0.0..=1.0)
                                .show_value(false),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
            });

            ui.add_space(2.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new(t.ambient_tone_softness).size(12.0));
                let soft_pct = (self.settings.tone_softness * 100.0).round() as u32;
                ui.label(RichText::new(format!("{soft_pct}%")).size(12.0).strong());
            });
            if ui
                .add(
                    egui::Slider::new(&mut self.settings.tone_softness, 0.0..=1.0)
                        .show_value(false),
                )
                .changed()
            {
                changed = true;
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                let is_off = self.timer_preset_minutes.is_none();
                if ui.selectable_label(is_off, t.ambient_timer_off).clicked() {
                    self.set_timer(None);
                }
                let is_15 = self.timer_preset_minutes == Some(15);
                if ui.selectable_label(is_15, t.ambient_timer_15m).clicked() {
                    self.set_timer(Some(15));
                }
                let is_30 = self.timer_preset_minutes == Some(30);
                if ui.selectable_label(is_30, t.ambient_timer_30m).clicked() {
                    self.set_timer(Some(30));
                }
                let is_60 = self.timer_preset_minutes == Some(60);
                if ui.selectable_label(is_60, t.ambient_timer_60m).clicked() {
                    self.set_timer(Some(60));
                }

                if let Some(rem) = self.remaining_timer_seconds() {
                    let mins = rem / 60;
                    let secs = rem % 60;
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{mins:02}:{secs:02}"))
                                .color(theme.accent_color())
                                .strong(),
                        );
                    });
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);

            ui.columns(2, |cols| {
                let p1_sel = self.settings.selected_preset.as_deref() == Some("laser_focus");
                if cols[0]
                    .selectable_label(p1_sel, t.preset_laser_focus)
                    .clicked()
                {
                    self.apply_preset("laser_focus");
                    changed = true;
                }
                let p2_sel = self.settings.selected_preset.as_deref() == Some("rainy_evening");
                if cols[1]
                    .selectable_label(p2_sel, t.preset_rainy_evening)
                    .clicked()
                {
                    self.apply_preset("rainy_evening");
                    changed = true;
                }
            });
            ui.columns(2, |cols| {
                let p3_sel = self.settings.selected_preset.as_deref() == Some("night_lofi");
                if cols[0]
                    .selectable_label(p3_sel, t.preset_night_lofi)
                    .clicked()
                {
                    self.apply_preset("night_lofi");
                    changed = true;
                }
                let p4_sel = self.settings.selected_preset.as_deref() == Some("deep_space");
                if cols[1]
                    .selectable_label(p4_sel, t.preset_deep_space)
                    .clicked()
                {
                    self.apply_preset("deep_space");
                    changed = true;
                }
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);

            let mut render_layer_row =
                |ui: &mut egui::Ui, title: &str, desc: &str, vol: &mut f32| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).strong().size(12.5));
                            ui.label(
                                RichText::new(desc)
                                    .color(theme.secondary_text_color())
                                    .size(10.5),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let pct = (*vol * 100.0).round() as u32;
                            ui.label(RichText::new(format!("{pct}%")).size(11.5));
                            let slider = egui::Slider::new(vol, 0.0..=1.0).show_value(false);
                            if ui.add_sized([100.0, 16.0], slider).changed() {
                                changed = true;
                                self.settings.selected_preset = None;
                            }
                        });
                    });
                    ui.add_space(2.0);
                };

            render_layer_row(
                ui,
                t.layer_brown_noise,
                t.layer_brown_noise_desc,
                &mut self.settings.brown_noise_vol,
            );
            render_layer_row(
                ui,
                t.layer_pink_noise,
                t.layer_pink_noise_desc,
                &mut self.settings.pink_noise_vol,
            );
            render_layer_row(
                ui,
                t.layer_rain,
                t.layer_rain_desc,
                &mut self.settings.rain_vol,
            );
            render_layer_row(
                ui,
                t.layer_fireplace,
                t.layer_fireplace_desc,
                &mut self.settings.fireplace_vol,
            );
            render_layer_row(
                ui,
                t.layer_lofi,
                t.layer_lofi_desc,
                &mut self.settings.lofi_vol,
            );
            render_layer_row(
                ui,
                t.layer_binaural,
                t.layer_binaural_desc,
                &mut self.settings.binaural_vol,
            );
            if self.settings.binaural_vol > 0.01 {
                ui.label(
                    RichText::new(t.ambient_headphones_hint)
                        .size(10.0)
                        .italics()
                        .color(theme.secondary_text_color()),
                );
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);

            if ui
                .checkbox(
                    &mut self.settings.duck_during_tts,
                    RichText::new(t.ambient_duck_tts).size(12.0),
                )
                .changed()
            {
                changed = true;
            }
            if self.settings.duck_during_tts {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.ambient_duck_amount).size(11.0));
                    let mut percent = (self.settings.duck_amount * 100.0).round() as u32;
                    let response = ui.add(egui::Slider::new(&mut percent, 20..=90).suffix("%"));
                    if response.changed() {
                        self.settings.duck_amount = percent as f32 / 100.0;
                        changed = true;
                    }
                });
            }
            if ui
                .checkbox(
                    &mut self.settings.resume_on_startup,
                    RichText::new(t.ambient_resume_on_startup).size(12.0),
                )
                .changed()
            {
                changed = true;
            }
        });

        if close_panel {
            self.is_open = false;
        }

        if changed {
            self.sync_shared_params();
        }

        changed
    }
}
