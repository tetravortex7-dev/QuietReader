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

#[test]
fn test_edge_tts_synth() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let config = msedge_tts::tts::SpeechConfig {
        voice_name: "ru-RU-SvetlanaNeural".to_string(),
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: 0,
        rate: 0,
        volume: 0,
    };
    if let Ok(mut client) = msedge_tts::tts::client::connect() {
        if let Ok(audio) = client.synthesize("Привет", &config) {
            assert!(!audio.audio_bytes.is_empty());
            let cursor = std::io::Cursor::new(audio.audio_bytes);
            let decoder = rodio::Decoder::new(cursor);
            assert!(decoder.is_ok());
        }
    }
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

    assert!(!voice_matches_lang("ru-RU", "en"));
    assert!(!voice_matches_lang("en-US", "ru"));
    assert!(!voice_matches_lang("de-DE", "ru"));
}
