use super::ambient_synth::{AmbientSynth, AmbientSynthParams};
use super::icons::{Icon, IconButton, paint_icon};
use super::theme::ReaderTheme;
use eframe::egui::{self, Frame, RichText, Sense, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientSettings {
    #[serde(default)]
    pub is_playing: bool,
    #[serde(default = "default_master_vol")]
    pub master_volume: f32,
    #[serde(default = "default_tone_softness")]
    pub tone_softness: f32,
    #[serde(default = "default_true")]
    pub duck_during_tts: bool,
    #[serde(default = "default_preset")]
    pub selected_preset: Option<String>,
    #[serde(default = "default_brown_vol")]
    pub brown_noise_vol: f32,
    #[serde(default)]
    pub pink_noise_vol: f32,
    #[serde(default)]
    pub rain_vol: f32,
    #[serde(default)]
    pub fireplace_vol: f32,
    #[serde(default)]
    pub lofi_vol: f32,
    #[serde(default = "default_binaural_vol")]
    pub binaural_vol: f32,
}

fn default_master_vol() -> f32 {
    0.65
}

fn default_tone_softness() -> f32 {
    0.50
}

fn default_true() -> bool {
    true
}

fn default_preset() -> Option<String> {
    Some("laser_focus".to_string())
}

fn default_brown_vol() -> f32 {
    0.75
}

fn default_binaural_vol() -> f32 {
    0.35
}

impl Default for AmbientSettings {
    fn default() -> Self {
        Self {
            is_playing: false,
            master_volume: default_master_vol(),
            tone_softness: default_tone_softness(),
            duck_during_tts: default_true(),
            selected_preset: default_preset(),
            brown_noise_vol: default_brown_vol(),
            pink_noise_vol: 0.0,
            rain_vol: 0.0,
            fireplace_vol: 0.0,
            lofi_vol: 0.0,
            binaural_vol: default_binaural_vol(),
        }
    }
}

pub struct AmbientController {
    pub is_open: bool,
    pub settings: AmbientSettings,
    shared_params: Arc<Mutex<AmbientSynthParams>>,
    audio_device: Option<rodio::MixerDeviceSink>,
    audio_player: Option<rodio::Player>,
    audio_initialized: bool,
    timer_total_secs: Option<u64>,
    timer_started_at: Option<Instant>,
}

impl AmbientController {
    pub fn new(settings: AmbientSettings) -> Self {
        let initial_params = AmbientSynthParams {
            is_playing: settings.is_playing,
            master_volume: settings.master_volume,
            tone_softness: settings.tone_softness,
            duck_during_tts: settings.duck_during_tts,
            tts_is_active: false,
            brown_noise_vol: settings.brown_noise_vol,
            pink_noise_vol: settings.pink_noise_vol,
            rain_vol: settings.rain_vol,
            fireplace_vol: settings.fireplace_vol,
            lofi_vol: settings.lofi_vol,
            binaural_vol: settings.binaural_vol,
        };
        let shared_params = Arc::new(Mutex::new(initial_params));

        let mut ctrl = Self {
            is_open: false,
            settings,
            shared_params,
            audio_device: None,
            audio_player: None,
            audio_initialized: false,
            timer_total_secs: None,
            timer_started_at: None,
        };
        if ctrl.settings.is_playing {
            ctrl.init_audio_device();
        }
        ctrl
    }

    fn init_audio_device(&mut self) {
        if self.audio_initialized {
            return;
        }
        self.audio_initialized = true;

        let sink: Option<rodio::MixerDeviceSink> =
            rodio::DeviceSinkBuilder::open_default_sink().ok();
        if let Some(ref dev) = sink {
            let player = rodio::Player::connect_new(&dev.mixer());
            let synth = AmbientSynth::new(Arc::clone(&self.shared_params), 44100);
            player.append(synth);
            player.play();
            self.audio_player = Some(player);
        }
        self.audio_device = sink;
    }

    pub fn toggle_panel(&mut self) {
        self.is_open = !self.is_open;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        self.settings.is_playing
    }

    pub fn toggle_play(&mut self) {
        self.settings.is_playing = !self.settings.is_playing;
        if self.settings.is_playing {
            self.init_audio_device();
            if let Some(ref p) = self.audio_player {
                p.play();
            }
        }
        self.sync_shared_params();
    }

    pub fn set_tts_active(&mut self, active: bool) {
        if let Ok(mut p) = self.shared_params.lock() {
            p.tts_is_active = active;
        }
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
        if let Some(m) = mins {
            self.timer_total_secs = Some(m * 60);
            self.timer_started_at = Some(Instant::now());
        } else {
            self.timer_total_secs = None;
            self.timer_started_at = None;
        }
    }

    pub fn remaining_timer_seconds(&self) -> Option<u64> {
        if let (Some(total), Some(started)) = (self.timer_total_secs, self.timer_started_at) {
            let elapsed = started.elapsed().as_secs();
            if elapsed < total {
                Some(total - elapsed)
            } else {
                Some(0)
            }
        } else {
            None
        }
    }

    pub fn tick(&mut self, ctx: &egui::Context) -> bool {
        let mut state_changed = false;
        if self.settings.is_playing {
            if let Some(rem) = self.remaining_timer_seconds() {
                if rem == 0 {
                    self.settings.is_playing = false;
                    self.timer_total_secs = None;
                    self.timer_started_at = None;
                    self.sync_shared_params();
                    state_changed = true;
                } else if self.is_open {
                    ctx.request_repaint_after(Duration::from_millis(500));
                }
            }
        }
        state_changed
    }

    pub fn sync_shared_params(&mut self) {
        if let Ok(mut p) = self.shared_params.lock() {
            p.is_playing = self.settings.is_playing;
            p.master_volume = self.settings.master_volume;
            p.tone_softness = self.settings.tone_softness;
            p.duck_during_tts = self.settings.duck_during_tts;
            p.brown_noise_vol = self.settings.brown_noise_vol;
            p.pink_noise_vol = self.settings.pink_noise_vol;
            p.rain_vol = self.settings.rain_vol;
            p.fireplace_vol = self.settings.fireplace_vol;
            p.lofi_vol = self.settings.lofi_vol;
            p.binaural_vol = self.settings.binaural_vol;
        }
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
                let is_off = self.timer_total_secs.is_none();
                if ui.selectable_label(is_off, t.ambient_timer_off).clicked() {
                    self.set_timer(None);
                }
                let is_15 = self.timer_total_secs == Some(15 * 60);
                if ui.selectable_label(is_15, t.ambient_timer_15m).clicked() {
                    self.set_timer(Some(15));
                }
                let is_30 = self.timer_total_secs == Some(30 * 60);
                if ui.selectable_label(is_30, t.ambient_timer_30m).clicked() {
                    self.set_timer(Some(30));
                }
                let is_60 = self.timer_total_secs == Some(60 * 60);
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
