use crate::app::ambient::{AmbientController, AmbientSettings};
use crate::app::ambient_synth::{AmbientSynth, AmbientSynthParams};
use crate::app::input::process_keyboard_events;
use crate::core::Command;
use crate::i18n::UiLanguage;
use eframe::egui::{Key, Modifiers};
use std::sync::{Arc, Mutex};

#[test]
fn test_ambient_settings_default() {
    let settings = AmbientSettings::default();
    assert!(!settings.is_playing);
    assert!((settings.master_volume - 0.65).abs() < 1e-4);
    assert!((settings.tone_softness - 0.50).abs() < 1e-4);
    assert!(settings.duck_during_tts);
    assert_eq!(settings.selected_preset.as_deref(), Some("laser_focus"));
    assert!((settings.brown_noise_vol - 0.75).abs() < 1e-4);
    assert!((settings.binaural_vol - 0.35).abs() < 1e-4);
}

#[test]
fn test_ambient_presets() {
    let mut ctrl = AmbientController::new(AmbientSettings::default());

    ctrl.apply_preset("laser_focus");
    assert_eq!(
        ctrl.settings.selected_preset.as_deref(),
        Some("laser_focus")
    );
    assert!(ctrl.settings.brown_noise_vol > 0.5);
    assert!(ctrl.settings.binaural_vol > 0.2);
    assert_eq!(ctrl.settings.rain_vol, 0.0);

    ctrl.apply_preset("rainy_evening");
    assert_eq!(
        ctrl.settings.selected_preset.as_deref(),
        Some("rainy_evening")
    );
    assert!(ctrl.settings.rain_vol > 0.5);
    assert!(ctrl.settings.fireplace_vol > 0.4);
    assert_eq!(ctrl.settings.brown_noise_vol, 0.0);

    ctrl.apply_preset("night_lofi");
    assert_eq!(ctrl.settings.selected_preset.as_deref(), Some("night_lofi"));
    assert!(ctrl.settings.lofi_vol > 0.5);
    assert!(ctrl.settings.rain_vol > 0.2);

    ctrl.apply_preset("deep_space");
    assert_eq!(ctrl.settings.selected_preset.as_deref(), Some("deep_space"));
    assert!(ctrl.settings.binaural_vol > 0.5);
    assert!(ctrl.settings.brown_noise_vol > 0.3);
}

#[test]
fn test_ambient_timer() {
    let mut ctrl = AmbientController::new(AmbientSettings::default());
    ctrl.set_timer(None);
    assert_eq!(ctrl.remaining_timer_seconds(), None);

    ctrl.set_timer(Some(15));
    let rem = ctrl.remaining_timer_seconds();
    assert!(rem.is_some());
    assert!(rem.unwrap() <= 900 && rem.unwrap() >= 898);

    ctrl.set_timer(Some(30));
    let rem30 = ctrl.remaining_timer_seconds();
    assert!(rem30.unwrap() <= 1800 && rem30.unwrap() >= 1798);

    ctrl.set_timer(Some(60));
    let rem60 = ctrl.remaining_timer_seconds();
    assert!(rem60.unwrap() <= 3600 && rem60.unwrap() >= 3598);

    ctrl.set_timer(None);
    assert_eq!(ctrl.remaining_timer_seconds(), None);
}

#[test]
fn test_procedural_synth_generation_bounds() {
    let params = Arc::new(Mutex::new(AmbientSynthParams {
        is_playing: true,
        master_volume: 1.0,
        tone_softness: 0.5,
        duck_during_tts: false,
        tts_is_active: false,
        brown_noise_vol: 0.8,
        pink_noise_vol: 0.8,
        rain_vol: 0.8,
        fireplace_vol: 0.8,
        lofi_vol: 0.8,
        binaural_vol: 0.8,
    }));

    let mut synth = AmbientSynth::new(params, 44100);

    for _ in 0..88200 {
        let sample = synth.next();
        assert!(sample.is_some());
        let val = sample.unwrap();
        assert!(!val.is_nan());
        assert!(!val.is_infinite());
        assert!(val >= -1.5 && val <= 1.5);
    }
}

#[test]
fn test_ducking_tts_state() {
    let mut ctrl = AmbientController::new(AmbientSettings::default());
    ctrl.set_tts_active(true);
    ctrl.set_tts_active(false);
}

#[test]
fn test_hotkey_m_triggers_toggle_ambient() {
    let cmds = process_keyboard_events(false, |k| k == Key::M, Modifiers::default(), false);
    assert!(cmds.contains(&Command::ToggleAmbient));

    let cmds_ctrl = process_keyboard_events(
        false,
        |k| k == Key::M,
        Modifiers {
            ctrl: true,
            ..Default::default()
        },
        false,
    );
    assert!(!cmds_ctrl.contains(&Command::ToggleAmbient));
}

#[test]
fn test_translations_coverage() {
    for &lang in &UiLanguage::CONCRETE {
        let t = lang.t();
        assert!(!t.ambient.is_empty());
        assert!(!t.ambient_tooltip.is_empty());
        assert!(!t.ambient_title.is_empty());
        assert!(!t.ambient_play.is_empty());
        assert!(!t.ambient_pause.is_empty());
        assert!(!t.ambient_master_volume.is_empty());
        assert!(!t.ambient_tone_softness.is_empty());
        assert!(!t.ambient_timer_off.is_empty());
        assert!(!t.ambient_timer_15m.is_empty());
        assert!(!t.ambient_timer_30m.is_empty());
        assert!(!t.ambient_timer_60m.is_empty());
        assert!(!t.preset_laser_focus.is_empty());
        assert!(!t.preset_rainy_evening.is_empty());
        assert!(!t.preset_night_lofi.is_empty());
        assert!(!t.preset_deep_space.is_empty());
        assert!(!t.layer_brown_noise.is_empty());
        assert!(!t.layer_brown_noise_desc.is_empty());
        assert!(!t.layer_pink_noise.is_empty());
        assert!(!t.layer_pink_noise_desc.is_empty());
        assert!(!t.layer_rain.is_empty());
        assert!(!t.layer_rain_desc.is_empty());
        assert!(!t.layer_fireplace.is_empty());
        assert!(!t.layer_fireplace_desc.is_empty());
        assert!(!t.layer_lofi.is_empty());
        assert!(!t.layer_lofi_desc.is_empty());
        assert!(!t.layer_binaural.is_empty());
        assert!(!t.layer_binaural_desc.is_empty());
        assert!(!t.ambient_duck_tts.is_empty());
    }
}
