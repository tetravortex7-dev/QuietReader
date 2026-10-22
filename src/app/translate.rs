use crate::app::icons::{Icon, IconButton, IconLabel};
use crate::app::theme::ReaderTheme;
use crate::i18n::UiLanguage;
use crate::settings::{AppSettings, TranslationProvider};
use eframe::egui::{self, Align2, Color32, Frame, RichText, ScrollArea, Stroke, Vec2};
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn map_lang_code(code: &str) -> &str {
    match code {
        "zh" => "zh-CN",
        _ => code,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslationError {
    MissingApiKey,
    InvalidKey,
    QuotaExceeded,
    TooManyRequests,
    NoTranslation,
    EmptyTranslation,
    Provider(String),
}

fn format_translation_error(err: &TranslationError, lang: crate::i18n::UiLanguage) -> String {
    let t = lang.t();
    match err {
        TranslationError::MissingApiKey => t.extra.translate_missing_api_key.to_string(),
        TranslationError::InvalidKey => t.extra.translate_invalid_key.to_string(),
        TranslationError::QuotaExceeded => t.extra.translate_quota_exceeded.to_string(),
        TranslationError::TooManyRequests => t.extra.translate_too_many_requests.to_string(),
        TranslationError::NoTranslation => t.extra.translate_no_translation.to_string(),
        TranslationError::EmptyTranslation => t.extra.translate_empty_translation.to_string(),
        TranslationError::Provider(message) => {
            crate::i18n::fill(t.extra.translate_provider_error, &[("{error}", message)])
        }
    }
}

fn fetch_google_translation(
    text: &str,
    target_lang: &str,
) -> Result<(String, String), TranslationError> {
    let tl = map_lang_code(target_lang);
    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl={}&dt=t&q={}",
        url_encode(tl),
        url_encode(text)
    );

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent();

    let mut resp = agent
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .call()
        .map_err(|e| TranslationError::Provider(e.to_string()))?;

    let mut body = String::new();
    resp.body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|e| TranslationError::Provider(e.to_string()))?;

    let val: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| TranslationError::Provider(e.to_string()))?;

    let mut translated = String::new();
    if let Some(arr) = val.get(0).and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(part) = item.get(0).and_then(|v| v.as_str()) {
                translated.push_str(part);
            }
        }
    }

    if translated.is_empty() {
        return Err(TranslationError::Provider(
            "Empty translation returned".to_string(),
        ));
    }

    let detected = val
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("auto")
        .to_string();

    Ok((translated, detected))
}

fn map_deepl_target_lang(lang: &str) -> &str {
    match lang.to_ascii_lowercase().as_str() {
        "ru" => "RU",
        "en" => "EN-US",
        "fr" => "FR",
        "es" => "ES",
        "zh" | "zh-cn" => "ZH",
        "ja" => "JA",
        "pt" => "PT-PT",
        "it" => "IT",
        "tr" => "TR",
        "uk" => "UK",
        "ar" => "AR",
        "id" => "ID",
        _ => "EN-US",
    }
}

fn fetch_deepl_translation(
    text: &str,
    target_lang: &str,
    api_key: &str,
) -> Result<(String, String), TranslationError> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err(TranslationError::MissingApiKey);
    }

    let is_free = key.ends_with(":fx");
    let endpoint = if is_free {
        "https://api-free.deepl.com/v2/translate"
    } else {
        "https://api.deepl.com/v2/translate"
    };

    let target = map_deepl_target_lang(target_lang);

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(12)))
        .build()
        .new_agent();

    let payload = serde_json::json!({
        "text": [text],
        "target_lang": target
    });
    let payload_bytes =
        serde_json::to_vec(&payload).map_err(|e| TranslationError::Provider(e.to_string()))?;

    let mut resp = agent
        .post(endpoint)
        .header("Authorization", &format!("DeepL-Auth-Key {key}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", "QuietReader/0.7.1")
        .send(&payload_bytes)
        .map_err(|e| TranslationError::Provider(e.to_string()))?;

    if resp.status().as_u16() == 403 {
        return Err(TranslationError::InvalidKey);
    } else if resp.status().as_u16() == 456 {
        return Err(TranslationError::QuotaExceeded);
    } else if resp.status().as_u16() == 429 {
        return Err(TranslationError::TooManyRequests);
    } else if !resp.status().is_success() {
        let mut err_body = String::new();
        let _ = resp.body_mut().as_reader().read_to_string(&mut err_body);
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&err_body) {
            if let Some(msg) = val.get("message").and_then(|m| m.as_str()) {
                return Err(TranslationError::Provider(format!("DeepL: {msg}")));
            }
        }
        return Err(TranslationError::Provider(format!(
            "DeepL HTTP {}",
            resp.status()
        )));
    }

    let mut body = String::new();
    resp.body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|e| TranslationError::Provider(e.to_string()))?;

    let val: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| TranslationError::Provider(e.to_string()))?;
    if let Some(err_msg) = val.get("message").and_then(|m| m.as_str()) {
        return Err(TranslationError::Provider(err_msg.to_string()));
    }

    let trans = val
        .get("translations")
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .ok_or(TranslationError::NoTranslation)?;

    let translated_text = trans
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_string();
    let detected_lang = trans
        .get("detected_source_language")
        .and_then(|t| t.as_str())
        .unwrap_or("auto")
        .to_lowercase();

    if translated_text.is_empty() {
        return Err(TranslationError::EmptyTranslation);
    }

    Ok((translated_text, detected_lang))
}

struct WorkerTask {
    text: String,
    target_lang: String,
    provider: TranslationProvider,
    deepl_api_key: String,
}

struct WorkerResponse {
    original: String,
    _target_lang: String,
    result: Result<(String, String), TranslationError>,
}

pub struct TranslateState {
    pub is_open: bool,
    pub original_text: String,
    pub translated_text: Option<String>,
    pub detected_lang: String,
    pub target_lang: String,
    pub provider: TranslationProvider,
    pub is_loading: bool,
    pub error: Option<TranslationError>,
    pub copy_feedback_timer: Option<std::time::Instant>,
    pub editing_api_key: bool,
    pub temp_api_key: String,
    task_tx: Option<Sender<WorkerTask>>,
    resp_rx: Option<Receiver<WorkerResponse>>,
}

impl Default for TranslateState {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslateState {
    pub fn new() -> Self {
        let (task_tx, task_rx) = channel::<WorkerTask>();
        let (resp_tx, resp_rx) = channel::<WorkerResponse>();

        thread::spawn(move || {
            while let Ok(task) = task_rx.recv() {
                let res = match task.provider {
                    TranslationProvider::Google => {
                        fetch_google_translation(&task.text, &task.target_lang)
                    }
                    TranslationProvider::DeepL => {
                        fetch_deepl_translation(&task.text, &task.target_lang, &task.deepl_api_key)
                    }
                };
                let resp = WorkerResponse {
                    original: task.text,
                    _target_lang: task.target_lang,
                    result: res,
                };
                let _ = resp_tx.send(resp);
            }
        });

        Self {
            is_open: false,
            original_text: String::new(),
            translated_text: None,
            detected_lang: "auto".to_string(),
            target_lang: "ru".to_string(),
            provider: TranslationProvider::Google,
            is_loading: false,
            error: None,
            copy_feedback_timer: None,
            editing_api_key: false,
            temp_api_key: String::new(),
            task_tx: Some(task_tx),
            resp_rx: Some(resp_rx),
        }
    }

    pub fn translate(
        &mut self,
        text: String,
        target_lang: &str,
        provider: TranslationProvider,
        deepl_key: &str,
    ) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        self.original_text = trimmed.to_string();
        self.translated_text = None;
        self.error = None;
        self.is_loading = true;
        self.target_lang = target_lang.to_string();
        self.provider = provider;
        self.temp_api_key = deepl_key.to_string();
        self.editing_api_key =
            provider == TranslationProvider::DeepL && deepl_key.trim().is_empty();
        self.is_open = true;

        if let Some(tx) = &self.task_tx {
            let _ = tx.send(WorkerTask {
                text: self.original_text.clone(),
                target_lang: self.target_lang.clone(),
                provider,
                deepl_api_key: deepl_key.to_string(),
            });
        }
    }

    pub fn retranslate(&mut self, deepl_key: &str) {
        if self.original_text.trim().is_empty() {
            return;
        }
        self.translated_text = None;
        self.error = None;
        self.is_loading = true;

        if let Some(tx) = &self.task_tx {
            let _ = tx.send(WorkerTask {
                text: self.original_text.clone(),
                target_lang: self.target_lang.clone(),
                provider: self.provider,
                deepl_api_key: deepl_key.to_string(),
            });
        }
    }

    pub fn poll(&mut self) {
        if let Some(rx) = &self.resp_rx {
            while let Ok(resp) = rx.try_recv() {
                if resp.original == self.original_text {
                    self.is_loading = false;
                    match resp.result {
                        Ok((trans, detected)) => {
                            self.translated_text = Some(trans);
                            self.detected_lang = detected;
                            self.error = None;
                        }
                        Err(e) => {
                            self.error = Some(e);
                        }
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn close(&mut self) {
        self.is_open = false;
        self.is_loading = false;
    }
}

pub struct TranslateView;

impl TranslateView {
    pub fn render(
        state: &mut TranslateState,
        ctx: &egui::Context,
        theme: &ReaderTheme,
        settings: &mut AppSettings,
        on_speak_text: &mut Option<String>,
        on_save_settings: &mut bool,
    ) {
        state.poll();

        if !state.is_open {
            return;
        }

        let t = settings.ui_language.t();
        let is_rtl = settings.ui_language.is_rtl();

        let mut is_open = state.is_open;

        egui::Window::new(t.translate_title)
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .min_width(340.0)
            .max_width(740.0)
            .default_height(350.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        IconLabel::new(Icon::Globe, "")
                            .size(16.0)
                            .color(theme.accent_color()),
                    );
                    let source_badge = state.detected_lang.to_uppercase();
                    ui.label(
                        RichText::new(&source_badge)
                            .strong()
                            .color(theme.text_color()),
                    );
                    ui.add(
                        IconLabel::new(Icon::ArrowRight, "")
                            .size(14.0)
                            .color(theme.secondary_text_color()),
                    );

                    let cur_target = state.target_lang.clone();
                    egui::ComboBox::from_id_salt("translate_target_lang")
                        .selected_text(cur_target.to_uppercase())
                        .show_ui(ui, |ui| {
                            for ui_lang in UiLanguage::CONCRETE {
                                let code = ui_lang.code();
                                let is_sel = state.target_lang == code;
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        format!(
                                            "{} ({})",
                                            ui_lang.native_name(),
                                            code.to_uppercase()
                                        ),
                                    )
                                    .clicked()
                                    && !is_sel
                                {
                                    state.target_lang = code.to_string();
                                    state.retranslate(&settings.translation.deepl_api_key);
                                }
                            }
                        });

                    if state.is_loading {
                        ui.spinner();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let is_deepl = settings.translation.provider == TranslationProvider::DeepL;
                        let is_google =
                            settings.translation.provider == TranslationProvider::Google;

                        if is_deepl {
                            let key_icon = if state.editing_api_key {
                                Icon::Close
                            } else {
                                Icon::Key
                            };
                            let key_btn = ui
                                .add(IconButton::new(key_icon, "").min_size(Vec2::new(22.0, 22.0)))
                                .on_hover_text(t.deepl_api_key);
                            if key_btn.clicked() {
                                state.editing_api_key = !state.editing_api_key;
                                if state.editing_api_key {
                                    state.temp_api_key = settings.translation.deepl_api_key.clone();
                                }
                            }
                        }

                        if ui
                            .selectable_label(is_deepl, "DeepL")
                            .on_hover_text(t.deepl_desc)
                            .clicked()
                            && !is_deepl
                        {
                            settings.translation.provider = TranslationProvider::DeepL;
                            *on_save_settings = true;
                            state.provider = TranslationProvider::DeepL;
                            if settings.translation.deepl_api_key.trim().is_empty() {
                                state.editing_api_key = true;
                                state.temp_api_key = String::new();
                            } else {
                                state.retranslate(&settings.translation.deepl_api_key);
                            }
                        }

                        if ui.selectable_label(is_google, "Google").clicked() && !is_google {
                            settings.translation.provider = TranslationProvider::Google;
                            *on_save_settings = true;
                            state.provider = TranslationProvider::Google;
                            state.retranslate("");
                        }
                    });
                });

                if state.editing_api_key {
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme.panel_bg_color())
                        .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.4)))
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(t.deepl_api_key)
                                        .size(12.0)
                                        .color(theme.text_color()),
                                );
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut state.temp_api_key)
                                        .password(true)
                                        .hint_text(t.deepl_key_hint)
                                        .desired_width(200.0),
                                );
                                if ui.button(t.save_key).clicked()
                                    || (resp.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                {
                                    settings.translation.deepl_api_key =
                                        state.temp_api_key.trim().to_string();
                                    *on_save_settings = true;
                                    state.editing_api_key = false;
                                    state.retranslate(&settings.translation.deepl_api_key);
                                }
                            });
                        });
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                let orig_frame = Frame::canvas(ui.style())
                    .fill(theme.panel_bg_color().gamma_multiply(0.6))
                    .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.2)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(10, 8));

                orig_frame.show(ui, |ui| {
                    ui.set_max_height(85.0);
                    ScrollArea::vertical()
                        .id_salt("translate_orig_scroll")
                        .max_height(85.0)
                        .show(ui, |ui| {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &state.original_text,
                                0.0,
                                egui::TextFormat {
                                    font_id: egui::FontId::proportional(13.0),
                                    color: theme.secondary_text_color(),
                                    ..Default::default()
                                },
                            );
                            ui.add(egui::Label::new(job).wrap().selectable(true));
                        });
                });

                ui.add_space(8.0);

                let trans_frame = Frame::canvas(ui.style())
                    .fill(theme.bg_color())
                    .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.4)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 10));

                let error_msg = state.error.clone();
                let mut retry = false;
                let mut open_key_edit = false;

                trans_frame.show(ui, |ui| {
                    ui.set_min_height(95.0);
                    if state.is_loading && state.translated_text.is_none() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(25.0);
                            ui.spinner();
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(t.translating).color(theme.secondary_text_color()),
                            );
                        });
                    } else if let Some(err) = &error_msg {
                        ui.vertical_centered(|ui| {
                            ui.add_space(15.0);
                            ui.label(
                                RichText::new(format!(
                                    "{}: {}",
                                    t.translate_error,
                                    format_translation_error(err, settings.ui_language)
                                ))
                                .color(Color32::RED),
                            );
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button(t.translate).clicked() {
                                    retry = true;
                                }
                                if state.provider == TranslationProvider::DeepL {
                                    if ui.button(t.deepl_api_key).clicked() {
                                        open_key_edit = true;
                                    }
                                }
                            });
                        });
                    } else if let Some(trans) = &state.translated_text {
                        ScrollArea::vertical()
                            .id_salt("translate_trans_scroll")
                            .max_height(200.0)
                            .show(ui, |ui| {
                                let mut job = egui::text::LayoutJob::default();
                                if is_rtl {
                                    job.halign = egui::Align::RIGHT;
                                }
                                job.append(
                                    trans,
                                    0.0,
                                    egui::TextFormat {
                                        font_id: egui::FontId::proportional(15.5),
                                        color: theme.text_color(),
                                        ..Default::default()
                                    },
                                );
                                ui.add(egui::Label::new(job).wrap().selectable(true));
                            });
                    }
                });

                if open_key_edit {
                    state.editing_api_key = true;
                    state.temp_api_key = settings.translation.deepl_api_key.clone();
                }

                if retry {
                    state.retranslate(&settings.translation.deepl_api_key);
                }

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if let Some(trans) = &state.translated_text {
                        let is_recently_copied = state
                            .copy_feedback_timer
                            .map(|time| time.elapsed().as_secs_f32() < 2.0)
                            .unwrap_or(false);

                        let (copy_icon, copy_btn_text) = if is_recently_copied {
                            (Icon::Check, t.copy)
                        } else {
                            (Icon::Copy, t.copy_translation)
                        };

                        if ui
                            .add(
                                IconButton::new(copy_icon, copy_btn_text)
                                    .min_size(Vec2::new(140.0, 26.0)),
                            )
                            .clicked()
                        {
                            ui.ctx().copy_text(trans.clone());
                            state.copy_feedback_timer = Some(std::time::Instant::now());
                        }

                        if ui
                            .add(
                                IconButton::new(Icon::Speech, t.speech)
                                    .min_size(Vec2::new(90.0, 26.0)),
                            )
                            .clicked()
                        {
                            *on_speak_text = Some(trans.clone());
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t.close_tab).clicked() {
                            state.is_open = false;
                        }
                    });
                });
            });

        state.is_open = is_open;
    }
}
