use crate::app::theme::ReaderTheme;
use crate::app::translate::{
    TranslateState, TranslateView, TranslationError, WorkerResponse, read_provider_body,
    translation_user_agent,
};
use crate::settings::{AppSettings, TranslationProvider};
use eframe::egui;
use std::io::Cursor;

#[test]
fn test_read_provider_body_accepts_exact_limit() {
    let mut data = Cursor::new(b"hello".to_vec());
    assert_eq!(read_provider_body(&mut data, 5).unwrap(), "hello");
}

#[test]
fn test_read_provider_body_rejects_oversized_response() {
    let mut data = Cursor::new(b"hello!".to_vec());
    assert!(read_provider_body(&mut data, 5).is_err());
}

#[test]
fn test_read_provider_body_rejects_invalid_utf8() {
    let mut data = Cursor::new(vec![0xff, 0xfe]);
    assert!(read_provider_body(&mut data, 10).is_err());
}

#[test]
fn test_translation_user_agent_uses_dynamic_app_version() {
    let ua = translation_user_agent();
    assert_eq!(ua, crate::net::user_agent(Some("Translator")));
    assert!(ua.starts_with("QuietReader/"));
    assert!(ua.contains(env!("CARGO_PKG_VERSION")));
    assert!(!ua.contains("Mozilla/"));
}

#[test]
fn test_translate_state_rejects_stale_target_lang_provider_and_request_id() {
    let (mut state, resp_tx, task_rx) = TranslateState::new_offline();
    state.translate("hello".to_string(), "ru", TranslationProvider::Google, "");
    let first_task = task_rx.try_recv().unwrap();
    assert_eq!(first_task.target_lang, "ru");
    assert_eq!(first_task.provider, TranslationProvider::Google);

    state.set_target_lang("fr", "");
    let second_task = task_rx.try_recv().unwrap();
    assert_ne!(second_task.request_id, first_task.request_id);
    assert_eq!(second_task.target_lang, "fr");
    assert!(state.is_loading);
    assert!(state.translated_text.is_none());

    resp_tx
        .send(WorkerResponse {
            request_id: first_task.request_id,
            original: "hello".to_string(),
            target_lang: "ru".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("привет".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert!(state.is_loading);
    assert!(state.translated_text.is_none());

    state.set_provider(TranslationProvider::DeepL, "secret-key");
    let third_task = task_rx.try_recv().unwrap();
    assert_ne!(third_task.request_id, second_task.request_id);
    assert_eq!(third_task.provider, TranslationProvider::DeepL);
    assert_eq!(third_task.deepl_api_key, "secret-key");

    resp_tx
        .send(WorkerResponse {
            request_id: second_task.request_id,
            original: "hello".to_string(),
            target_lang: "fr".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("bonjour".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert!(state.is_loading);
    assert!(state.translated_text.is_none());

    resp_tx
        .send(WorkerResponse {
            request_id: third_task.request_id,
            original: "hello".to_string(),
            target_lang: "fr".to_string(),
            provider: TranslationProvider::DeepL,
            result: Ok(("bonjour (deepl)".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert!(!state.is_loading);
    assert_eq!(state.translated_text.as_deref(), Some("bonjour (deepl)"));
    assert_eq!(state.detected_lang, "en");
}

#[test]
fn test_translate_cache_separates_target_lang_and_provider() {
    let (mut state, resp_tx, task_rx) = TranslateState::new_offline();

    state.translate("hello".to_string(), "ru", TranslationProvider::Google, "");
    let task_google_ru = task_rx.try_recv().unwrap();
    resp_tx
        .send(WorkerResponse {
            request_id: task_google_ru.request_id,
            original: "hello".to_string(),
            target_lang: "ru".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("привет".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert_eq!(state.cache_len(), 1);

    state.set_target_lang("fr", "");
    assert!(state.is_loading);
    assert!(state.translated_text.is_none());
    let task_google_fr = task_rx.try_recv().unwrap();
    resp_tx
        .send(WorkerResponse {
            request_id: task_google_fr.request_id,
            original: "hello".to_string(),
            target_lang: "fr".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("bonjour".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert_eq!(state.cache_len(), 2);

    state.set_provider(TranslationProvider::DeepL, "deepl-key");
    assert!(state.is_loading);
    assert!(state.translated_text.is_none());
    let task_deepl_fr = task_rx.try_recv().unwrap();
    assert_eq!(task_deepl_fr.target_lang, "fr");
    assert_eq!(task_deepl_fr.provider, TranslationProvider::DeepL);
    resp_tx
        .send(WorkerResponse {
            request_id: task_deepl_fr.request_id,
            original: "hello".to_string(),
            target_lang: "fr".to_string(),
            provider: TranslationProvider::DeepL,
            result: Ok(("salut".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert_eq!(state.cache_len(), 3);

    state.set_provider(TranslationProvider::Google, "");
    state.set_target_lang("ru", "");
    assert!(!state.is_loading);
    assert!(state.pending_request_id().is_none());
    assert!(task_rx.try_recv().is_err());
    assert_eq!(state.translated_text.as_deref(), Some("привет"));

    state.retranslate("");
    assert!(state.is_loading);
    assert!(state.translated_text.is_none());
    let refresh_task = task_rx.try_recv().unwrap();
    assert_eq!(refresh_task.target_lang, "ru");
    assert_eq!(refresh_task.provider, TranslationProvider::Google);
}

#[test]
fn test_translate_switch_to_deepl_without_key_clears_result_and_prompts_key() {
    let (mut state, resp_tx, task_rx) = TranslateState::new_offline();

    state.translate("hello".to_string(), "ru", TranslationProvider::Google, "");
    let google_task = task_rx.try_recv().unwrap();
    resp_tx
        .send(WorkerResponse {
            request_id: google_task.request_id,
            original: "hello".to_string(),
            target_lang: "ru".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("привет".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert_eq!(state.translated_text.as_deref(), Some("привет"));

    state.sync_with_settings(TranslationProvider::DeepL, "", true);
    assert_eq!(state.provider, TranslationProvider::DeepL);
    assert!(state.translated_text.is_none());
    assert!(!state.is_loading);
    assert!(state.editing_api_key);
    assert_eq!(state.error, Some(TranslationError::MissingApiKey));
    assert!(state.pending_request_id().is_none());
    assert!(task_rx.try_recv().is_err());

    state.set_provider(TranslationProvider::Google, "");
    assert!(!state.editing_api_key);
    assert!(state.error.is_none());
    assert_eq!(state.translated_text.as_deref(), Some("привет"));
}

#[test]
fn test_translate_close_and_direct_mutation_invalidate_stale_state() {
    let (mut state, resp_tx, task_rx) = TranslateState::new_offline();

    state.translate("hello".to_string(), "ru", TranslationProvider::Google, "");
    let task = task_rx.try_recv().unwrap();
    state.close();
    assert!(!state.is_open);
    assert!(!state.is_loading);
    assert!(state.pending_request_id().is_none());

    resp_tx
        .send(WorkerResponse {
            request_id: task.request_id,
            original: "hello".to_string(),
            target_lang: "ru".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("привет".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert!(state.translated_text.is_none());

    state.translate("hello".to_string(), "ru", TranslationProvider::Google, "");
    let task2 = task_rx.try_recv().unwrap();
    resp_tx
        .send(WorkerResponse {
            request_id: task2.request_id,
            original: "hello".to_string(),
            target_lang: "ru".to_string(),
            provider: TranslationProvider::Google,
            result: Ok(("привет".to_string(), "en".to_string())),
        })
        .unwrap();
    state.poll();
    assert_eq!(state.translated_text.as_deref(), Some("привет"));

    state.target_lang = "de".to_string();
    state.ensure_result_matches_current();
    assert!(state.translated_text.is_none());
    assert!(state.error.is_none());
}

#[test]
fn test_translate_view_syncs_provider_change_and_bounds_cache() {
    let (mut state, resp_tx, task_rx) = TranslateState::new_offline();

    for i in 0..70 {
        let text = format!("phrase_{i}");
        state.translate(text.clone(), "ru", TranslationProvider::Google, "");
        let task = task_rx.try_recv().unwrap();
        resp_tx
            .send(WorkerResponse {
                request_id: task.request_id,
                original: text,
                target_lang: "ru".to_string(),
                provider: TranslationProvider::Google,
                result: Ok((format!("tr_{i}"), "en".to_string())),
            })
            .unwrap();
        state.poll();
    }
    assert_eq!(state.cache_len(), 64);

    let mut settings = AppSettings::default();
    settings.translation.provider = TranslationProvider::DeepL;
    settings.translation.deepl_api_key.clear();

    let ctx = egui::Context::default();
    let mut on_speak = None;
    let mut on_save = false;
    let mut on_pro = None;
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        TranslateView::render(
            &mut state,
            ui.ctx(),
            &ReaderTheme::Light,
            &mut settings,
            &mut on_speak,
            &mut on_save,
            true,
            &mut on_pro,
        );
    });
    out.textures_delta.clear();

    assert_eq!(state.provider, TranslationProvider::DeepL);
    assert!(state.translated_text.is_none());
    assert!(state.editing_api_key);
    assert_eq!(state.error, Some(TranslationError::MissingApiKey));
}
