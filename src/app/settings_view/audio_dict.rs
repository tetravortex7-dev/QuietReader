use super::widgets::{
    SectionBadge, SettingsPalette, metric_slider, pill_badge, row_card, section_card,
    segmented_control, themed_button, toggle_switch,
};
use crate::app::ambient::AmbientController;
use crate::app::dictionary::DictionaryState;
use crate::app::license_view::ProUpgradeState;
use crate::app::tts::{TtsController, TtsEngineKind};
use crate::core::license::ProFeature;
use crate::i18n::{Translations, fill};
use crate::settings::{AppSettings, TranslationProvider};
use eframe::egui::{self, RichText, Stroke, Vec2};

pub(super) fn render_ambience_tts_tab(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    ambient: &mut AmbientController,
    tts: &mut TtsController,
    on_save_settings: &mut bool,
    on_open_ambient: &mut bool,
    on_open_tts: &mut bool,
    pro: &mut ProUpgradeState,
) {
    render_ambient_section(
        ui,
        p,
        t,
        settings,
        ambient,
        on_save_settings,
        on_open_ambient,
        pro,
    );
    ui.add_space(14.0);
    render_tts_section(ui, p, t, settings, tts, on_save_settings, on_open_tts, pro);
}

pub(super) fn render_ambient_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    ambient: &mut AmbientController,
    on_save_settings: &mut bool,
    on_open_ambient: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let allowed = settings.can_use(ProFeature::AmbientAudio);
    section_card(
        ui,
        p,
        t.extra.settings_ambient_section_title,
        Some(SectionBadge::Pro(t.license_pro_badge)),
        Some(t.ambient_headphones_hint),
        |ui| {
            let mut master_vol = (ambient.settings.master_volume.clamp(0.0, 1.0) * 100.0).round();
            let vol_readout = format!("{:.0}%", master_vol);
            let mut tone_soft = (ambient.settings.tone_softness.clamp(0.0, 1.0) * 100.0).round();
            let tone_readout = format!("{:.0}%", tone_soft);

            if ui.available_width() >= 520.0 {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.columns(2, |cols| {
                        let (ch1, cm1) = metric_slider(
                            &mut cols[0],
                            &mut master_vol,
                            0.0..=100.0,
                            Some(1.0),
                            t.ambient_master_volume,
                            &vol_readout,
                            None,
                            p,
                        );
                        if ch1 || cm1 {
                            ambient.settings.master_volume = (master_vol / 100.0).clamp(0.0, 1.0);
                            ambient.sync_shared_params();
                            settings.ambient = ambient.settings.clone();
                            *on_save_settings = true;
                        }

                        let (ch2, cm2) = metric_slider(
                            &mut cols[1],
                            &mut tone_soft,
                            0.0..=100.0,
                            Some(1.0),
                            t.ambient_tone_softness,
                            &tone_readout,
                            None,
                            p,
                        );
                        if ch2 || cm2 {
                            ambient.settings.tone_softness = (tone_soft / 100.0).clamp(0.0, 1.0);
                            ambient.sync_shared_params();
                            settings.ambient = ambient.settings.clone();
                            *on_save_settings = true;
                        }
                    });
                });
            } else {
                let (ch1, cm1) = metric_slider(
                    ui,
                    &mut master_vol,
                    0.0..=100.0,
                    Some(1.0),
                    t.ambient_master_volume,
                    &vol_readout,
                    None,
                    p,
                );
                if ch1 || cm1 {
                    ambient.settings.master_volume = (master_vol / 100.0).clamp(0.0, 1.0);
                    ambient.sync_shared_params();
                    settings.ambient = ambient.settings.clone();
                    *on_save_settings = true;
                }
                ui.add_space(8.0);
                let (ch2, cm2) = metric_slider(
                    ui,
                    &mut tone_soft,
                    0.0..=100.0,
                    Some(1.0),
                    t.ambient_tone_softness,
                    &tone_readout,
                    None,
                    p,
                );
                if ch2 || cm2 {
                    ambient.settings.tone_softness = (tone_soft / 100.0).clamp(0.0, 1.0);
                    ambient.sync_shared_params();
                    settings.ambient = ambient.settings.clone();
                    *on_save_settings = true;
                }
            }

            ui.add_space(10.0);
            row_card(ui, p, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.ambient_resume_on_startup)
                            .size(12.5)
                            .strong()
                            .color(p.text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut val = ambient.settings.resume_on_startup;
                        if toggle_switch(ui, &mut val, p).changed() {
                            if val && !allowed {
                                pro.request(ProFeature::AmbientAudio);
                            } else {
                                ambient.settings.resume_on_startup = val;
                                settings.ambient = ambient.settings.clone();
                                *on_save_settings = true;
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);
            row_card(ui, p, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.ambient_duck_tts)
                            .size(12.5)
                            .strong()
                            .color(p.text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut duck = ambient.settings.duck_during_tts;
                        if toggle_switch(ui, &mut duck, p).changed() {
                            ambient.settings.duck_during_tts = duck;
                            ambient.sync_shared_params();
                            settings.ambient = ambient.settings.clone();
                            *on_save_settings = true;
                        }
                    });
                });
                if ambient.settings.duck_during_tts {
                    ui.add_space(8.0);
                    let mut duck_pct =
                        (ambient.settings.duck_amount.clamp(0.1, 1.0) * 100.0).round();
                    let readout = format!("{:.0}%", duck_pct);
                    let (ch, cm) = metric_slider(
                        ui,
                        &mut duck_pct,
                        10.0..=100.0,
                        Some(2.0),
                        t.ambient_duck_amount,
                        &readout,
                        None,
                        p,
                    );
                    if ch || cm {
                        ambient.settings.duck_amount = (duck_pct / 100.0).clamp(0.1, 1.0);
                        ambient.sync_shared_params();
                        settings.ambient = ambient.settings.clone();
                        *on_save_settings = true;
                    }
                }
            });

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                let presets = [
                    ("laser_focus", t.preset_laser_focus),
                    ("rainy_evening", t.preset_rainy_evening),
                    ("night_lofi", t.preset_night_lofi),
                    ("deep_space", t.preset_deep_space),
                ];
                for (id, label) in presets {
                    let is_sel = ambient.settings.selected_preset.as_deref() == Some(id);
                    if themed_button(ui, label, is_sel, p).clicked() {
                        if !allowed {
                            pro.request(ProFeature::AmbientAudio);
                        } else {
                            ambient.apply_preset(id);
                            settings.ambient = ambient.settings.clone();
                            *on_save_settings = true;
                        }
                    }
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if themed_button(ui, t.extra.settings_open_ambient, true, p).clicked() {
                    if !allowed {
                        pro.request(ProFeature::AmbientAudio);
                    } else {
                        *on_open_ambient = true;
                    }
                }
            });
        },
    );
}

pub(super) fn render_tts_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    tts: &mut TtsController,
    on_save_settings: &mut bool,
    on_open_tts: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let neural_allowed = settings.can_use(ProFeature::NeuralTts);
    section_card(
        ui,
        p,
        t.extra.settings_tts_section_title,
        None,
        Some(t.tts_title),
        |ui| {
            let mut prefs = tts.preferences();
            let mut kind = if prefs.engine == "edge" && neural_allowed {
                TtsEngineKind::Edge
            } else {
                TtsEngineKind::System
            };

            row_card(ui, p, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(t.tts_voice).size(12.5).strong().color(p.text));
                    if !neural_allowed {
                        pill_badge(ui, p.secondary_bg, p.secondary_fg, t.license_pro_badge, 9.5);
                    }
                });
                ui.add_space(6.0);
                let items = [
                    (TtsEngineKind::System, t.tts_system),
                    (TtsEngineKind::Edge, t.tts_edge),
                ];
                if segmented_control(ui, &mut kind, &items, p, false) {
                    if kind == TtsEngineKind::Edge && !neural_allowed {
                        pro.request(ProFeature::NeuralTts);
                    } else {
                        tts.set_engine(kind);
                        prefs = tts.preferences();
                        settings.tts = prefs.clone();
                        *on_save_settings = true;
                    }
                }
                ui.add_space(6.0);
                let tip = match kind {
                    TtsEngineKind::System => t.tts_system_tip,
                    TtsEngineKind::Edge => t.tts_internet_tip,
                };
                ui.label(RichText::new(tip).size(11.0).color(p.text_variant));
            });

            ui.add_space(8.0);
            row_card(ui, p, |ui| {
                let mut rate = prefs.rate.clamp(0.5, 2.5);
                let readout = format!("{:.2}×", rate);
                let (changed, commit) = metric_slider(
                    ui,
                    &mut rate,
                    0.5..=2.5,
                    Some(0.05),
                    t.tts_speed,
                    &readout,
                    None,
                    p,
                );
                if changed || commit {
                    tts.set_rate(rate);
                    settings.tts = tts.preferences();
                    *on_save_settings = true;
                }
            });

            ui.add_space(10.0);
            if themed_button(ui, t.extra.settings_open_tts, false, p).clicked() {
                *on_open_tts = true;
            }
        },
    );
}

pub(super) fn render_dict_translate_tab(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    dictionary: &DictionaryState,
    on_save_settings: &mut bool,
    on_open_dictionary: &mut bool,
    on_open_vocabulary: &mut bool,
    pro: &mut ProUpgradeState,
) {
    render_translation_section(ui, p, t, settings, on_save_settings, pro);
    ui.add_space(14.0);
    render_dictionary_vocab_section(
        ui,
        p,
        t,
        settings,
        dictionary,
        on_open_dictionary,
        on_open_vocabulary,
    );
}

pub(super) fn render_translation_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let translate_allowed = settings.can_use(ProFeature::Translation);
    section_card(
        ui,
        p,
        t.translation_provider,
        Some(SectionBadge::Pro(t.license_pro_badge)),
        Some(t.deepl_desc),
        |ui| {
            let mut provider = settings.translation.provider;
            let items = [
                (
                    TranslationProvider::Google,
                    TranslationProvider::Google.name(),
                ),
                (
                    TranslationProvider::DeepL,
                    TranslationProvider::DeepL.name(),
                ),
            ];
            if segmented_control(ui, &mut provider, &items, p, false) {
                if provider == TranslationProvider::DeepL && !translate_allowed {
                    pro.request(ProFeature::Translation);
                } else {
                    settings.translation.provider = provider;
                    *on_save_settings = true;
                }
            }

            if settings.translation.provider == TranslationProvider::DeepL {
                ui.add_space(10.0);
                row_card(ui, p, |ui| {
                    ui.label(
                        RichText::new(t.deepl_api_key)
                            .size(12.5)
                            .strong()
                            .color(p.text),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut settings.translation.deepl_api_key)
                                .password(true)
                                .hint_text(t.deepl_key_hint)
                                .desired_width((ui.available_width() - 120.0).max(160.0)),
                        );
                        if resp.changed() {
                            *on_save_settings = true;
                        }
                        if themed_button(ui, t.save_key, true, p).clicked() {
                            *on_save_settings = true;
                        }
                    });
                });
            }
        },
    );
}

pub(super) fn render_dictionary_vocab_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &AppSettings,
    dictionary: &DictionaryState,
    on_open_dictionary: &mut bool,
    on_open_vocabulary: &mut bool,
) {
    section_card(
        ui,
        p,
        t.extra.settings_dict_section_title,
        None,
        None,
        |ui| {
            let installed_text = fill(
                t.extra.settings_dict_installed_fmt,
                &[("{count}", &dictionary.installed.len().to_string())],
            );
            let saved_text = fill(
                t.extra.settings_vocab_saved_fmt,
                &[("{count}", &settings.saved_words.len().to_string())],
            );

            let avail = ui.available_width();
            if avail >= 500.0 {
                ui.columns(2, |cols| {
                    render_summary_action_card(
                        &mut cols[0],
                        p,
                        t.dictionary_title,
                        &installed_text,
                        t.extra.settings_open_dictionary,
                        on_open_dictionary,
                    );
                    render_summary_action_card(
                        &mut cols[1],
                        p,
                        t.vocabulary_title,
                        &saved_text,
                        t.extra.settings_open_vocabulary,
                        on_open_vocabulary,
                    );
                });
            } else {
                render_summary_action_card(
                    ui,
                    p,
                    t.dictionary_title,
                    &installed_text,
                    t.extra.settings_open_dictionary,
                    on_open_dictionary,
                );
                ui.add_space(8.0);
                render_summary_action_card(
                    ui,
                    p,
                    t.vocabulary_title,
                    &saved_text,
                    t.extra.settings_open_vocabulary,
                    on_open_vocabulary,
                );
            }
        },
    );
}

fn render_summary_action_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    title: &str,
    stat_text: &str,
    btn_label: &str,
    on_click: &mut bool,
) {
    row_card(ui, p, |ui| {
        ui.label(RichText::new(title).size(13.0).strong().color(p.text));
        ui.add_space(2.0);
        ui.label(RichText::new(stat_text).size(11.5).color(p.text_variant));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let btn = ui.add(
                egui::Button::new(RichText::new(btn_label).size(11.5).strong().color(p.text))
                    .fill(p.surface_container)
                    .stroke(Stroke::new(1.0, p.border_subtle))
                    .corner_radius(p.corner_sm)
                    .min_size(Vec2::new(0.0, 28.0)),
            );
            if btn.clicked() {
                *on_click = true;
            }
        });
    });
}
