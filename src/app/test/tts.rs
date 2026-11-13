#[test]
fn test_tts_engine_init() {
    let tts_res = tts::Tts::default();
    if let Ok(mut tts) = tts_res {
        let features = tts.supported_features();
        let _ = tts.normal_rate();
        let _ = tts.min_rate();
        let _ = tts.max_rate();
        let _ = tts.voices();
        if features.utterance_callbacks {
            let _ = tts.on_utterance_end(Some(Box::new(|u| {
                let _ = u;
            })));
        }
        let _ = tts.stop();
    }
}

fn build_silent_mp3_bytes(frame_count: usize) -> Vec<u8> {
    const FRAME_LEN: usize = 417;
    let mut bytes = vec![0u8; frame_count * FRAME_LEN];
    for frame in 0..frame_count {
        let offset = frame * FRAME_LEN;
        bytes[offset] = 0xFF;
        bytes[offset + 1] = 0xFB;
        bytes[offset + 2] = 0x90;
        bytes[offset + 3] = 0xC4;
    }
    bytes
}

#[test]
fn test_edge_tts_offline_mp3_decode_and_rate_scaling() {
    use crate::app::tts::{edge_player_speed, edge_speech_rate_pct};
    use rodio::Source;

    let mp3_bytes = build_silent_mp3_bytes(38);
    let mut decoder =
        rodio::Decoder::new(std::io::Cursor::new(mp3_bytes)).expect("decode offline MP3");
    assert_eq!(decoder.channels().get(), 1);
    assert_eq!(decoder.sample_rate().get(), 44_100);
    let sample_count = (&mut decoder).count();
    assert!(sample_count >= 40_000);

    assert_eq!(edge_speech_rate_pct(0.5), -50);
    assert_eq!(edge_speech_rate_pct(1.0), 0);
    assert_eq!(edge_speech_rate_pct(1.5), 50);
    assert_eq!(edge_speech_rate_pct(2.0), 100);
    assert_eq!(edge_speech_rate_pct(3.0), 200);
    assert_eq!(edge_speech_rate_pct(0.1), -50);
    assert_eq!(edge_speech_rate_pct(10.0), 200);
    assert_eq!(edge_speech_rate_pct(f32::NAN), 0);

    assert!((edge_player_speed(1.5, 1.5) - 1.0).abs() < 1e-5);
    assert!((edge_player_speed(2.0, 2.0) - 1.0).abs() < 1e-5);
    assert!((edge_player_speed(1.5, 3.0) - 2.0).abs() < 1e-5);
    assert!((edge_player_speed(2.0, 1.0) - 0.5).abs() < 1e-5);
}

#[test]
fn test_tts_controller_engine_and_rate_switching() {
    use crate::app::tts::{TtsController, TtsEngineKind};
    use crate::settings::TtsSettings;

    let mut controller = TtsController::new(TtsSettings::default());
    let initial_status = controller.status();
    assert_eq!(initial_status.rate, 1.0);

    controller.set_rate(1.5);
    assert_eq!(controller.status().rate, 1.5);
    assert_eq!(controller.preferences().rate, 1.5);

    controller.set_engine(TtsEngineKind::Edge);
    assert_eq!(controller.status().engine, TtsEngineKind::Edge);
    assert_eq!(controller.preferences().engine, "edge");

    controller.set_voice("ru-RU-DmitryNeural".to_string());
    assert_eq!(
        controller.status().selected_voice_id.as_deref(),
        Some("ru-RU-DmitryNeural")
    );
    assert_eq!(
        controller.preferences().voice_id.as_deref(),
        Some("ru-RU-DmitryNeural")
    );

    controller.set_voice("ru-RU-SvetlanaNeural".to_string());
    assert_eq!(
        controller.status().selected_voice_id.as_deref(),
        Some("ru-RU-SvetlanaNeural")
    );

    controller.set_engine(TtsEngineKind::System);
    assert_eq!(controller.status().engine, TtsEngineKind::System);
    assert_eq!(controller.preferences().engine, "system");
}

#[test]
fn test_tts_rate_clamping_in_controller() {
    use crate::app::tts::{TtsController, clamp_tts_rate};
    use crate::settings::TtsSettings;

    assert_eq!(clamp_tts_rate(0.1), 0.5);
    assert_eq!(clamp_tts_rate(9.5), 3.0);
    assert_eq!(clamp_tts_rate(f32::NAN), 1.0);
    assert_eq!(clamp_tts_rate(f32::INFINITY), 1.0);

    let mut controller = TtsController::new(TtsSettings {
        engine: "edge".to_string(),
        voice_id: None,
        rate: 8.0,
    });
    assert_eq!(controller.preferences().rate, 3.0);
    assert_eq!(controller.status().rate, 3.0);

    controller.set_rate(0.2);
    assert_eq!(controller.preferences().rate, 0.5);
    assert_eq!(controller.status().rate, 0.5);

    controller.set_rate(f32::NAN);
    assert_eq!(controller.preferences().rate, 1.0);
    assert_eq!(controller.status().rate, 1.0);
}

#[test]
fn test_tts_preferences_are_initialized_from_settings() {
    use crate::app::tts::TtsController;
    use crate::settings::TtsSettings;

    let controller = TtsController::new(TtsSettings {
        engine: "edge".to_string(),
        voice_id: Some("ru-RU-DmitryNeural".to_string()),
        rate: 1.25,
    });

    let preferences = controller.preferences();
    assert_eq!(preferences.engine, "edge");
    assert_eq!(preferences.voice_id.as_deref(), Some("ru-RU-DmitryNeural"));
    assert_eq!(preferences.rate, 1.25);
}

#[test]
fn test_language_detection_and_voice_matching() {
    use crate::app::tts::voice_matches_lang;
    use crate::core::ReaderCore;

    let ru_text = "Здоровье моего тела всегда улучшается и обновляется с каждым новым днем.";
    assert_eq!(ReaderCore::detect_language_from_text(ru_text), "ru");

    let uk_text = "Здоров'я мого тіла завжди покращується та оновлюється з кожним новим днем.";
    assert_eq!(ReaderCore::detect_language_from_text(uk_text), "uk");

    let en_text = "The quick brown fox jumps over the lazy dog and that is with great joy.";
    assert_eq!(ReaderCore::detect_language_from_text(en_text), "en");

    let de_text = "Die Gesundheit meines Körpers verbessert sich und das ist wunderbar für mich.";
    assert_eq!(ReaderCore::detect_language_from_text(de_text), "de");

    let fr_text =
        "La santé de mon corps s'améliore toujours avec le temps et c'est pour une vie saine.";
    assert_eq!(ReaderCore::detect_language_from_text(fr_text), "fr");

    let es_text =
        "La salud de mi cuerpo siempre mejora y se renueva con los días para vivir mejor.";
    assert_eq!(ReaderCore::detect_language_from_text(es_text), "es");

    let zh_text = "阅读能够开阔视野并让我们体验不同的人生与思考方式。";
    assert_eq!(ReaderCore::detect_language_from_text(zh_text), "zh");

    let ja_text = "本を読むことは心を豊かにし、新しい世界を広げてくれます。";
    assert_eq!(ReaderCore::detect_language_from_text(ja_text), "ja");

    let ar_text = "القراءة تفتح آفاقاً جديدة وتغذي العقل بالمعرفة والحكمة.";
    assert_eq!(ReaderCore::detect_language_from_text(ar_text), "ar");

    let it_text = "La lettura della vita è anche questo viaggio magnifico con gli amici.";
    assert_eq!(ReaderCore::detect_language_from_text(it_text), "it");

    let pt_text = "A leitura não é apenas um hábito, mas uma paixão para toda a vida.";
    assert_eq!(ReaderCore::detect_language_from_text(pt_text), "pt");

    let tr_text = "Kitap okumak yeni dünyalar keşfetmek için bir kapı açar ve çok güzeldir.";
    assert_eq!(ReaderCore::detect_language_from_text(tr_text), "tr");

    let id_text = "Membaca buku adalah jendela dunia yang membuka pikiran dan wawasan kita.";
    assert_eq!(ReaderCore::detect_language_from_text(id_text), "id");

    assert!(voice_matches_lang("ru-RU", "ru"));
    assert!(voice_matches_lang("Russian", "ru"));
    assert!(voice_matches_lang("Microsoft Irina - Russian", "ru"));
    assert!(voice_matches_lang("en-US", "en"));
    assert!(voice_matches_lang("English", "en"));
    assert!(voice_matches_lang("uk-UA", "uk"));
    assert!(voice_matches_lang("de-DE", "de"));
    assert!(voice_matches_lang("fr-FR", "fr"));
    assert!(voice_matches_lang("es-ES", "es"));
    assert!(voice_matches_lang("zh-CN", "zh"));
    assert!(voice_matches_lang("ja-JP", "ja"));
    assert!(voice_matches_lang("ar-SA", "ar"));
    assert!(voice_matches_lang("it-IT", "it"));
    assert!(voice_matches_lang("pt-BR", "pt"));
    assert!(voice_matches_lang("tr-TR", "tr"));
    assert!(voice_matches_lang("id-ID", "id"));

    assert!(!voice_matches_lang("", "en"));
    assert!(!voice_matches_lang("en-US", ""));
    assert!(!voice_matches_lang("", ""));
    assert!(!voice_matches_lang("ru-RU", "en"));
    assert!(!voice_matches_lang("en-US", "ru"));
    assert!(!voice_matches_lang("de-DE", "ru"));
    assert!(!voice_matches_lang("es-AR", "ar"));
    assert!(!voice_matches_lang("French", "en"));
    assert!(!voice_matches_lang("Portuguese", "es"));
    assert!(!voice_matches_lang(
        "Microsoft Server Speech Text to Speech Voice (es-AR, Elena)",
        "ar"
    ));
}

#[test]
fn test_apply_remote_edge_voices_preserves_cache_across_engines() {
    use crate::app::tts::{TtsEngineKind, TtsStatus, TtsVoiceInfo, apply_remote_edge_voices};

    let mut cache = vec![TtsVoiceInfo {
        id: "default".to_string(),
        name: "Default".to_string(),
        language: "en-US".to_string(),
    }];
    let mut status = TtsStatus {
        engine: TtsEngineKind::System,
        voices: vec![TtsVoiceInfo {
            id: "sys-1".to_string(),
            name: "System Voice".to_string(),
            language: "en-US".to_string(),
        }],
        ..Default::default()
    };

    let remote = vec![
        TtsVoiceInfo {
            id: "fr-FR-DeniseNeural".to_string(),
            name: "Denise".to_string(),
            language: "fr-FR".to_string(),
        },
        TtsVoiceInfo {
            id: "en-US-AriaNeural".to_string(),
            name: "Aria".to_string(),
            language: "en-US".to_string(),
        },
        TtsVoiceInfo {
            id: "ru-RU-DmitryNeural".to_string(),
            name: "Dmitry".to_string(),
            language: "ru-RU".to_string(),
        },
    ];

    apply_remote_edge_voices(&mut cache, &mut status, remote);
    assert_eq!(cache.len(), 3);
    assert_eq!(cache[0].id, "ru-RU-DmitryNeural");
    assert_eq!(cache[1].id, "en-US-AriaNeural");
    assert_eq!(cache[2].id, "fr-FR-DeniseNeural");
    assert_eq!(status.voices.len(), 1);
    assert_eq!(status.voices[0].id, "sys-1");

    status.engine = TtsEngineKind::Edge;
    apply_remote_edge_voices(&mut cache, &mut status, vec![]);
    assert_eq!(cache.len(), 3);

    let remote_again = cache.clone();
    apply_remote_edge_voices(&mut cache, &mut status, remote_again);
    assert_eq!(status.voices.len(), 3);
    assert_eq!(status.voices[0].id, "ru-RU-DmitryNeural");
}

#[test]
fn test_step_paragraph_index_playing_vs_paused_and_stopped() {
    use crate::app::tts::step_paragraph_index;

    let paragraphs = vec![
        "First".to_string(),
        "".to_string(),
        "<b>  </b>".to_string(),
        "Second".to_string(),
        "Last".to_string(),
        "".to_string(),
    ];

    assert_eq!(step_paragraph_index(&paragraphs, 0, true, true), Some(3));
    assert_eq!(step_paragraph_index(&paragraphs, 4, true, true), Some(6));
    assert_eq!(step_paragraph_index(&paragraphs, 3, false, true), Some(0));
    assert_eq!(step_paragraph_index(&paragraphs, 0, false, true), Some(0));

    assert_eq!(step_paragraph_index(&paragraphs, 0, true, false), Some(3));
    assert_eq!(step_paragraph_index(&paragraphs, 4, true, false), None);
    assert_eq!(step_paragraph_index(&paragraphs, 3, false, false), Some(0));
    assert_eq!(step_paragraph_index(&paragraphs, 0, false, false), Some(0));

    let empty = vec!["".to_string(), "   ".to_string()];
    assert_eq!(step_paragraph_index(&empty, 0, true, false), None);
    assert_eq!(step_paragraph_index(&empty, 0, false, false), None);
}

#[test]
fn test_tts_paused_navigation_and_nonblocking_stop() {
    use crate::app::tts::{TtsController, TtsEngineKind};
    use crate::settings::TtsSettings;
    use std::time::{Duration, Instant};

    let mut controller = TtsController::new(TtsSettings {
        engine: "edge".to_string(),
        voice_id: None,
        rate: 1.5,
    });

    let wait_until_timeout =
        |ctrl: &TtsController,
         timeout: Duration,
         pred: &dyn Fn(&crate::app::tts::TtsStatus) -> bool| {
            let start = Instant::now();
            while start.elapsed() < timeout {
                let st = ctrl.status();
                if pred(&st) {
                    return Some(st);
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            None
        };
    let wait_until = |ctrl: &TtsController, pred: &dyn Fn(&crate::app::tts::TtsStatus) -> bool| {
        wait_until_timeout(ctrl, Duration::from_millis(800), pred)
    };

    let ready = wait_until_timeout(&controller, Duration::from_secs(15), &|st| {
        !st.voices.is_empty()
    });
    assert!(ready.is_some());

    let started = Instant::now();
    controller.set_engine(TtsEngineKind::Edge);
    controller.play_chapter(
        0,
        0,
        vec!["One.".to_string(), "Two.".to_string(), "Three.".to_string()],
    );
    controller.pause();

    let paused_or_stopped = wait_until(&controller, &|st| {
        (st.is_playing && st.is_paused) || st.playback_failed
    });
    assert!(started.elapsed() < Duration::from_secs(1));

    if let Some(st) = paused_or_stopped {
        if st.is_playing && st.is_paused {
            controller.next_paragraph();
            let after_next = wait_until(&controller, &|s| s.current_paragraph == 1);
            assert!(after_next.is_some());
            let after_next = after_next.unwrap();
            assert!(after_next.is_playing);
            assert!(after_next.is_paused);
            assert!(!after_next.needs_next_chapter);

            controller.next_paragraph();
            let at_end = wait_until(&controller, &|s| s.current_paragraph == 2);
            assert!(at_end.is_some());

            controller.next_paragraph();
            std::thread::sleep(Duration::from_millis(50));
            let past_end = controller.status();
            assert_eq!(past_end.current_paragraph, 2);
            assert!(past_end.is_paused);
            assert!(!past_end.needs_next_chapter);

            controller.prev_paragraph();
            let after_prev = wait_until(&controller, &|s| s.current_paragraph == 1);
            assert!(after_prev.is_some());
            assert!(after_prev.unwrap().is_paused);
        }
    }

    controller.stop();
    let stopped = wait_until(&controller, &|st| !st.is_playing && !st.is_paused);
    assert!(stopped.is_some());
}

#[test]
fn test_tts_word_intervals_and_active_word_range() {
    use crate::app::tts::{compute_active_word_range, compute_word_intervals};
    use std::time::Duration;

    let text = "Hello world, this is a test.";
    let intervals = compute_word_intervals(text);
    assert_eq!(intervals.len(), 6);

    let words: Vec<&str> = intervals.iter().map(|&(s, e, _)| &text[s..e]).collect();
    assert_eq!(words, vec!["Hello", "world,", "this", "is", "a", "test."]);

    let first_word = compute_active_word_range(
        &intervals,
        Duration::ZERO,
        1.0,
        Some(Duration::from_secs(6)),
    );
    assert_eq!(first_word, Some((0, 5)));

    let middle_word = compute_active_word_range(
        &intervals,
        Duration::from_secs(3),
        1.0,
        Some(Duration::from_secs(6)),
    );
    assert!(middle_word.is_some());
    let (s, e) = middle_word.unwrap();
    assert!(&text[s..e] == "this" || &text[s..e] == "is");

    let last_word = compute_active_word_range(
        &intervals,
        Duration::from_secs(10),
        1.0,
        Some(Duration::from_secs(6)),
    );
    assert_eq!(last_word, Some((23, 28)));

    let empty = compute_active_word_range(&[], Duration::from_secs(1), 1.0, None);
    assert_eq!(empty, None);
}

#[test]
fn test_tts_floating_panel_fits_within_compact_and_ultrawide_viewports() {
    use crate::app::theme::ReaderTheme;
    use crate::app::tts::TtsController;
    use crate::i18n::UiLanguage;
    use crate::settings::TtsSettings;
    use eframe::egui;

    for (w, h, ppp) in [
        (420.0_f32, 360.0_f32, 1.5_f32),
        (800.0, 600.0, 1.0),
        (3440.0, 1440.0, 2.0),
    ] {
        let mut ctrl = TtsController::new(TtsSettings::default());
        ctrl.show_panel = true;
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
        for _ in 0..2 {
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(screen);
            raw.viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(ppp);
            let mut on_play = false;
            let mut on_pro = None;
            let mut out = ctx.run_ui(raw, |ui| {
                ctrl.render_panel(
                    ui.ctx(),
                    &ReaderTheme::Light,
                    true,
                    Some("ru"),
                    &mut on_play,
                    UiLanguage::Ru,
                    true,
                    &mut on_pro,
                );
            });
            out.textures_delta.clear();
        }

        let rect = ctx
            .memory(|m| m.area_rect(egui::Id::unique("tts_floating_panel")))
            .expect("tts floating panel rect");
        assert!(rect.width() <= w + 1.0, "tts {w}x{h}@{ppp}: rect={rect:?}");
        assert!(
            rect.left() >= -1.0 && rect.right() <= w + 1.0,
            "tts {w}x{h}@{ppp}: rect={rect:?}"
        );
        assert!(
            rect.top() >= -1.0 && rect.bottom() <= h + 1.0,
            "tts {w}x{h}@{ppp}: rect={rect:?}"
        );
        if w >= 2000.0 {
            assert!(
                rect.width() <= 1020.0,
                "tts ultrawide {w}x{h}@{ppp}: rect={rect:?}"
            );
        }
    }
}
