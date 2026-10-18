use eframe::egui::{self, Color32, Frame, Id, Order, RichText, Stroke, Vec2};
use std::io::Cursor;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tts::Tts;

use super::icons::{paint_icon, Icon, IconButton, IconLabel};
use super::theme::ReaderTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TtsEngineKind {
    #[default]
    System,
    Edge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtsVoiceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
}

#[derive(Debug, Clone, Default)]
pub struct TtsStatus {
    pub is_available: bool,
    pub is_playing: bool,
    pub is_paused: bool,
    pub current_chapter: usize,
    pub current_paragraph: usize,
    pub total_paragraphs: usize,
    pub rate: f32,
    pub engine: TtsEngineKind,
    pub voices: Vec<TtsVoiceInfo>,
    pub selected_voice_id: Option<String>,
    pub error: Option<String>,
    pub needs_next_chapter: bool,
}

pub enum TtsCommand {
    PlayChapter { chapter_index: usize, paragraph_index: usize, paragraphs: Vec<String> },
    PlayNextChapter { chapter_index: usize, paragraphs: Vec<String> },
    Pause,
    Resume,
    Stop,
    NextParagraph,
    PrevParagraph,
    SetRate(f32),
    SetVoice(String),
    SetEngine(TtsEngineKind),
}

fn default_edge_voices() -> Vec<TtsVoiceInfo> {
    const VOICES: &[(&str, &str, &str)] = &[
        ("ru-RU-SvetlanaNeural", "Светлана (Нейро)", "ru-RU"),
        ("ru-RU-DmitryNeural", "Дмитрий (Нейро)", "ru-RU"),
        ("en-US-JennyNeural", "Jenny (Neural)", "en-US"),
        ("en-US-GuyNeural", "Guy (Neural)", "en-US"),
        ("en-US-AriaNeural", "Aria (Neural)", "en-US"),
        ("uk-UA-PolinaNeural", "Поліна (Нейро)", "uk-UA"),
        ("uk-UA-OstapNeural", "Остап (Нейро)", "uk-UA"),
        ("de-DE-KatjaNeural", "Katja (Neural)", "de-DE"),
        ("fr-FR-DeniseNeural", "Denise (Neural)", "fr-FR"),
        ("es-ES-ElviraNeural", "Elvira (Neural)", "es-ES"),
    ];
    VOICES.iter().map(|&(id, name, lang)| TtsVoiceInfo { id: id.to_string(), name: name.to_string(), language: lang.to_string() }).collect()
}

fn format_voice_name(name: &str) -> String {
    let s = name.trim();
    let s = s.strip_prefix("Microsoft ").unwrap_or(s);
    let s = s.strip_suffix(" Desktop").unwrap_or(s);
    let s = s.strip_suffix(" Mobile").unwrap_or(s);
    if let Some(idx) = s.find(" Online (Natural)") {
        return s[..idx].trim().to_string();
    }
    s.trim().to_string()
}

pub fn voice_matches_lang(voice_lang: &str, book_lang: &str) -> bool {
    let vl = voice_lang.to_ascii_lowercase();
    let bl = book_lang.to_ascii_lowercase();
    if vl.starts_with(&bl) || bl.starts_with(&vl) {
        return true;
    }
    match bl.as_str() {
        "ru" => vl.contains("ru") || vl.contains("rus"),
        "en" => vl.contains("en") || vl.contains("eng"),
        "uk" => vl.contains("uk") || vl.contains("ukr"),
        "de" => vl.contains("de") || vl.contains("ger") || vl.contains("deu"),
        "fr" => vl.contains("fr") || vl.contains("fre") || vl.contains("fra"),
        "es" => vl.contains("es") || vl.contains("spa"),
        _ => false,
    }
}

pub struct TtsController {
    command_tx: Option<Sender<TtsCommand>>,
    status: Arc<Mutex<TtsStatus>>,
    pub show_panel: bool,
}

impl Default for TtsController {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsController {
    pub fn new() -> Self {
        let status = Arc::new(Mutex::new(TtsStatus {
            rate: 1.0,
            ..Default::default()
        }));

        let (tx, rx) = channel::<TtsCommand>();
        let status_clone = Arc::clone(&status);

        thread::spawn(move || {
            Self::worker_loop(rx, status_clone);
        });

        Self {
            command_tx: Some(tx),
            status,
            show_panel: false,
        }
    }

    pub fn status(&self) -> TtsStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }

    pub fn play_chapter(&mut self, chapter_index: usize, paragraph_index: usize, paragraphs: Vec<String>) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::PlayChapter { chapter_index, paragraph_index, paragraphs });
        }
    }

    pub fn play_next_chapter(&mut self, chapter_index: usize, paragraphs: Vec<String>) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::PlayNextChapter { chapter_index, paragraphs });
        }
    }

    pub fn pause(&mut self) {
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::Pause); }
    }

    pub fn resume(&mut self) {
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::Resume); }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::Stop); }
    }

    pub fn next_paragraph(&mut self) {
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::NextParagraph); }
    }

    pub fn prev_paragraph(&mut self) {
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::PrevParagraph); }
    }

    pub fn set_rate(&mut self, rate: f32) {
        if let Ok(mut st) = self.status.lock() { st.rate = rate; }
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::SetRate(rate)); }
    }

    pub fn set_voice(&mut self, voice_id: String) {
        if let Ok(mut st) = self.status.lock() { st.selected_voice_id = Some(voice_id.clone()); }
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::SetVoice(voice_id)); }
    }

    pub fn set_engine(&mut self, engine: TtsEngineKind) {
        if let Ok(mut st) = self.status.lock() { st.engine = engine; }
        if let Some(tx) = &self.command_tx { let _ = tx.send(TtsCommand::SetEngine(engine)); }
    }

    fn worker_loop(rx: Receiver<TtsCommand>, status: Arc<Mutex<TtsStatus>>) {
        let mut sys_tts = Tts::default().ok();
        let audio_device: Option<rodio::MixerDeviceSink> = rodio::DeviceSinkBuilder::open_default_sink().ok();
        let mut audio_player: Option<rodio::Player> = audio_device.as_ref().map(|handle| rodio::Player::connect_new(&handle.mixer()));

        let system_voices: Vec<TtsVoiceInfo> = sys_tts
            .as_ref()
            .and_then(|t| t.voices().ok())
            .unwrap_or_default()
            .into_iter()
            .map(|v| TtsVoiceInfo {
                id: v.id(),
                name: v.name(),
                language: v.language().to_string(),
            })
            .collect();

        let edge_voices = default_edge_voices();

        let mut current_sys_voice_id = if let Some(ru_voice) = system_voices
            .iter()
            .find(|v| v.language.to_ascii_lowercase().starts_with("ru"))
        {
            Some(ru_voice.id.clone())
        } else {
            system_voices.first().map(|v| v.id.clone())
        };

        let mut current_edge_voice_id = Some("ru-RU-SvetlanaNeural".to_string());

        if let (Some(tts), Some(id)) = (sys_tts.as_mut(), &current_sys_voice_id) {
            if let Ok(all_voices) = tts.voices() {
                if let Some(target) = all_voices.iter().find(|v| v.id() == *id) {
                    let _ = tts.set_voice(target);
                }
            }
        }

        let mut current_engine = if sys_tts.is_some() {
            TtsEngineKind::System
        } else {
            TtsEngineKind::Edge
        };

        {
            if let Ok(mut st) = status.lock() {
                st.is_available = sys_tts.is_some() || audio_device.is_some();
                st.engine = current_engine;
                st.rate = 1.0;
                st.voices = if current_engine == TtsEngineKind::Edge {
                    edge_voices.clone()
                } else {
                    system_voices.clone()
                };
                st.selected_voice_id = if current_engine == TtsEngineKind::Edge {
                    current_edge_voice_id.clone()
                } else {
                    current_sys_voice_id.clone()
                };
            }
        }

        let status_for_remote = Arc::clone(&status);
        thread::spawn(move || {
            if let Ok(remote_voices) = msedge_tts::voice::get_voices_list() {
                let mut mapped: Vec<TtsVoiceInfo> = remote_voices
                    .into_iter()
                    .filter_map(|v| {
                        let id = v.short_name?;
                        let name = v.friendly_name.unwrap_or_else(|| id.clone());
                        let language = v.locale.unwrap_or_default();
                        Some(TtsVoiceInfo { id, name, language })
                    })
                    .collect();

                if !mapped.is_empty() {
                    mapped.sort_by(|a, b| {
                        b.language.starts_with("ru").cmp(&a.language.starts_with("ru"))
                            .then_with(|| b.language.starts_with("en").cmp(&a.language.starts_with("en")))
                            .then_with(|| a.language.cmp(&b.language))
                            .then_with(|| a.name.cmp(&b.name))
                    });

                    if let Ok(mut st) = status_for_remote.lock() {
                        if st.engine == TtsEngineKind::Edge {
                            st.voices = mapped;
                        }
                    }
                }
            }
        });

        let mut current_chapter = 0;
        let mut paragraphs: Vec<String> = Vec::new();
        let mut current_para = 0;
        let mut is_playing = false;
        let mut is_paused = false;
        let mut rate = 1.0f32;
        let mut utterance_started_at: Option<Instant> = None;
        let mut edge_client: Option<msedge_tts::tts::client::MSEdgeTTSClient<std::net::TcpStream>> = None;

        let find_clean_para = |paras: &[String], idx: usize| -> Option<(usize, String)> {
            let mut i = idx;
            while i < paras.len() {
                let clean = crate::document::strip_formatting_tags(&paras[i]);
                let trimmed = clean.trim();
                if !trimmed.is_empty() {
                    return Some((i, trimmed.to_string()));
                }
                i += 1;
            }
            None
        };

        let restart_player = |
            device: &Option<rodio::MixerDeviceSink>,
            player: &mut Option<rodio::Player>,
        | {
            if let Some(p) = player.take() {
                p.stop();
            }
            if let Some(dev) = device {
                *player = Some(rodio::Player::connect_new(&dev.mixer()));
            }
        };

        let stop_all = |
            sys_tts: &mut Option<Tts>,
            device: &Option<rodio::MixerDeviceSink>,
            player: &mut Option<rodio::Player>,
        | {
            if let Some(t) = sys_tts.as_mut() {
                let _ = t.stop();
            }
            restart_player(device, player);
        };

        let pause_all = |
            sys_tts: &mut Option<Tts>,
            player: &Option<rodio::Player>,
        | {
            if let Some(t) = sys_tts.as_mut() {
                let _ = t.stop();
            }
            if let Some(p) = player {
                p.pause();
            }
        };

        let speak_para = |
            engine: TtsEngineKind,
            text: &str,
            cur_rate: f32,
            sys_tts: &mut Option<Tts>,
            edge_client: &mut Option<msedge_tts::tts::client::MSEdgeTTSClient<std::net::TcpStream>>,
            edge_voice_id: &Option<String>,
            device: &Option<rodio::MixerDeviceSink>,
            player: &mut Option<rodio::Player>,
        | -> bool {
            match engine {
                TtsEngineKind::System => {
                    restart_player(device, player);
                    if let Some(t) = sys_tts.as_mut() {
                        let _ = t.set_rate(cur_rate);
                        t.speak(text, true).is_ok()
                    } else {
                        false
                    }
                }
                TtsEngineKind::Edge => {
                    if device.is_none() {
                        return false;
                    }
                    let voice_name = edge_voice_id
                        .clone()
                        .unwrap_or_else(|| "ru-RU-SvetlanaNeural".to_string());
                    let rate_pct = ((cur_rate - 1.0) * 100.0).round() as i32;
                    let config = msedge_tts::tts::SpeechConfig {
                        voice_name,
                        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
                        pitch: 0,
                        rate: rate_pct,
                        volume: 0,
                    };

                    let mut attempt = 0;
                    let mut success = false;
                    while attempt < 2 && !success {
                        if edge_client.is_none() {
                            *edge_client = msedge_tts::tts::client::connect().ok();
                        }
                        if let Some(client) = edge_client.as_mut() {
                            match client.synthesize(text, &config) {
                                Ok(audio) => {
                                    if let Ok(decoder) = rodio::Decoder::new(Cursor::new(audio.audio_bytes)) {
                                        restart_player(device, player);
                                        if let Some(p) = player.as_ref() {
                                            p.append(decoder);
                                            p.set_speed(cur_rate);
                                            p.play();
                                            success = true;
                                        }
                                    }
                                }
                                Err(_) => {
                                    *edge_client = None;
                                }
                            }
                        }
                        attempt += 1;
                    }
                    success
                }
            }
        };

        loop {
            let cmd = if is_playing && !is_paused {
                rx.recv_timeout(Duration::from_millis(60)).ok()
            } else {
                rx.recv().ok()
            };

            if let Some(command) = cmd {
                match command {
                    TtsCommand::PlayChapter {
                        chapter_index,
                        paragraph_index,
                        paragraphs: new_paras,
                    } => {
                        stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                        current_chapter = chapter_index;
                        paragraphs = new_paras;
                        is_playing = true;
                        is_paused = false;

                        if let Some((idx, text)) = find_clean_para(&paragraphs, paragraph_index) {
                            current_para = idx;
                            let ok = speak_para(
                                current_engine,
                                &text,
                                rate,
                                &mut sys_tts,
                                &mut edge_client,
                                &current_edge_voice_id,
                                &audio_device,
                                &mut audio_player,
                            );
                            if ok {
                                utterance_started_at = Some(Instant::now());
                                if let Ok(mut st) = status.lock() {
                                    st.error = None;
                                }
                            } else {
                                is_playing = false;
                                if let Ok(mut st) = status.lock() {
                                    st.error = Some("Ошибка воспроизведения речи".to_string());
                                }
                            }
                        } else {
                            current_para = paragraphs.len();
                        }
                    }
                    TtsCommand::PlayNextChapter {
                        chapter_index,
                        paragraphs: new_paras,
                    } => {
                        stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                        current_chapter = chapter_index;
                        paragraphs = new_paras;
                        is_playing = true;
                        is_paused = false;

                        if let Some((idx, text)) = find_clean_para(&paragraphs, 0) {
                            current_para = idx;
                            let ok = speak_para(
                                current_engine,
                                &text,
                                rate,
                                &mut sys_tts,
                                &mut edge_client,
                                &current_edge_voice_id,
                                &audio_device,
                                &mut audio_player,
                            );
                            if ok {
                                utterance_started_at = Some(Instant::now());
                            } else {
                                is_playing = false;
                            }
                        } else {
                            current_para = paragraphs.len();
                        }
                    }
                    TtsCommand::Pause => {
                        pause_all(&mut sys_tts, &audio_player);
                        is_paused = true;
                        utterance_started_at = None;
                    }
                    TtsCommand::Resume => {
                        if is_paused && current_para < paragraphs.len() {
                            if current_engine == TtsEngineKind::Edge {
                                if let Some(s) = &audio_player {
                                    if !s.empty() {
                                        s.play();
                                        is_paused = false;
                                        utterance_started_at = Some(Instant::now());
                                    } else if let Some((idx, text)) = find_clean_para(&paragraphs, current_para) {
                                        current_para = idx;
                                        if speak_para(
                                            current_engine,
                                            &text,
                                            rate,
                                            &mut sys_tts,
                                            &mut edge_client,
                                            &current_edge_voice_id,
                                            &audio_device,
                                            &mut audio_player,
                                        ) {
                                            is_paused = false;
                                            utterance_started_at = Some(Instant::now());
                                        }
                                    }
                                }
                            } else if let Some((idx, text)) = find_clean_para(&paragraphs, current_para) {
                                current_para = idx;
                                if speak_para(
                                    current_engine,
                                    &text,
                                    rate,
                                    &mut sys_tts,
                                    &mut edge_client,
                                    &current_edge_voice_id,
                                    &audio_device,
                                    &mut audio_player,
                                ) {
                                    is_paused = false;
                                    utterance_started_at = Some(Instant::now());
                                }
                            }
                        }
                    }
                    TtsCommand::Stop => {
                        stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                        is_playing = false;
                        is_paused = false;
                        utterance_started_at = None;
                    }
                    TtsCommand::NextParagraph => {
                        stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                        if let Some((idx, text)) = find_clean_para(&paragraphs, current_para + 1) {
                            current_para = idx;
                            if speak_para(
                                current_engine,
                                &text,
                                rate,
                                &mut sys_tts,
                                &mut edge_client,
                                &current_edge_voice_id,
                                &audio_device,
                                &mut audio_player,
                            ) {
                                utterance_started_at = Some(Instant::now());
                            }
                        } else {
                            current_para = paragraphs.len();
                        }
                    }
                    TtsCommand::PrevParagraph => {
                        stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                        let target_idx = current_para.saturating_sub(1);
                        let mut back_idx = target_idx;
                        while back_idx > 0 {
                            let clean = crate::document::strip_formatting_tags(&paragraphs[back_idx]);
                            if !clean.trim().is_empty() {
                                break;
                            }
                            back_idx -= 1;
                        }
                        if let Some((idx, text)) = find_clean_para(&paragraphs, back_idx) {
                            current_para = idx;
                            if speak_para(
                                current_engine,
                                &text,
                                rate,
                                &mut sys_tts,
                                &mut edge_client,
                                &current_edge_voice_id,
                                &audio_device,
                                &mut audio_player,
                            ) {
                                utterance_started_at = Some(Instant::now());
                            }
                        }
                    }
                    TtsCommand::SetRate(new_rate) => {
                        rate = new_rate.clamp(0.5, 3.0);
                        if let Ok(mut st) = status.lock() {
                            st.rate = rate;
                        }
                        if current_engine == TtsEngineKind::Edge {
                            if let Some(p) = &audio_player {
                                p.set_speed(rate);
                            }
                        } else if is_playing && !is_paused && current_para < paragraphs.len() {
                            stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                            if let Some((idx, text)) = find_clean_para(&paragraphs, current_para) {
                                current_para = idx;
                                if speak_para(
                                    current_engine,
                                    &text,
                                    rate,
                                    &mut sys_tts,
                                    &mut edge_client,
                                    &current_edge_voice_id,
                                    &audio_device,
                                    &mut audio_player,
                                ) {
                                    utterance_started_at = Some(Instant::now());
                                }
                            }
                        }
                    }
                    TtsCommand::SetVoice(voice_id) => {
                        let voice_changed = if current_engine == TtsEngineKind::Edge {
                            let changed = current_edge_voice_id.as_deref() != Some(&voice_id);
                            current_edge_voice_id = Some(voice_id.clone());
                            changed
                        } else {
                            let changed = current_sys_voice_id.as_deref() != Some(&voice_id);
                            current_sys_voice_id = Some(voice_id.clone());
                            if let (Some(tts), Some(id)) = (sys_tts.as_mut(), &current_sys_voice_id) {
                                if let Ok(all_voices) = tts.voices() {
                                    if let Some(target) = all_voices.iter().find(|v| v.id() == *id) {
                                        let _ = tts.set_voice(target);
                                    }
                                }
                            }
                            changed
                        };
                        if voice_changed && current_engine == TtsEngineKind::Edge {
                            edge_client = None;
                        }
                        if let Ok(mut st) = status.lock() {
                            st.selected_voice_id = Some(voice_id);
                        }
                        if is_playing && !is_paused && current_para < paragraphs.len() {
                            stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                            if let Some((idx, text)) = find_clean_para(&paragraphs, current_para) {
                                current_para = idx;
                                if speak_para(
                                    current_engine,
                                    &text,
                                    rate,
                                    &mut sys_tts,
                                    &mut edge_client,
                                    &current_edge_voice_id,
                                    &audio_device,
                                    &mut audio_player,
                                ) {
                                    utterance_started_at = Some(Instant::now());
                                    if let Ok(mut st) = status.lock() {
                                        st.error = None;
                                    }
                                } else {
                                    is_playing = false;
                                    utterance_started_at = None;
                                    if let Ok(mut st) = status.lock() {
                                        st.error = Some("Ошибка воспроизведения речи".to_string());
                                    }
                                }
                            }
                        }
                    }
                    TtsCommand::SetEngine(new_engine) => {
                        if current_engine != new_engine {
                            stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                            current_engine = new_engine;
                            edge_client = None;
                            let new_selected_voice = if current_engine == TtsEngineKind::Edge {
                                current_edge_voice_id.clone()
                            } else {
                                current_sys_voice_id.clone()
                            };
                            let new_voices = if current_engine == TtsEngineKind::Edge {
                                edge_voices.clone()
                            } else {
                                system_voices.clone()
                            };
                            if let Ok(mut st) = status.lock() {
                                st.engine = current_engine;
                                st.voices = new_voices;
                                st.selected_voice_id = new_selected_voice;
                                st.error = None;
                            }
                            if is_playing && !is_paused && current_para < paragraphs.len() {
                                if let Some((idx, text)) = find_clean_para(&paragraphs, current_para) {
                                    current_para = idx;
                                    if speak_para(
                                        current_engine,
                                        &text,
                                        rate,
                                        &mut sys_tts,
                                        &mut edge_client,
                                        &current_edge_voice_id,
                                        &audio_device,
                                        &mut audio_player,
                                    ) {
                                        utterance_started_at = Some(Instant::now());
                                    } else {
                                        is_playing = false;
                                        utterance_started_at = None;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if is_playing && !is_paused {
                let speaking = match current_engine {
                    TtsEngineKind::System => {
                        sys_tts.as_mut().and_then(|t| t.is_speaking().ok()).unwrap_or(false)
                    }
                    TtsEngineKind::Edge => {
                        audio_player.as_ref().map(|s| !s.empty() && !s.is_paused()).unwrap_or(false)
                    }
                };

                let grace_passed = utterance_started_at
                    .map(|t| t.elapsed() > Duration::from_millis(300))
                    .unwrap_or(true);

                if !speaking && grace_passed {
                    if let Some((idx, text)) = find_clean_para(&paragraphs, current_para + 1) {
                        current_para = idx;
                        if speak_para(
                            current_engine,
                            &text,
                            rate,
                            &mut sys_tts,
                            &mut edge_client,
                            &current_edge_voice_id,
                            &audio_device,
                            &mut audio_player,
                        ) {
                            utterance_started_at = Some(Instant::now());
                        } else {
                            is_playing = false;
                            utterance_started_at = None;
                        }
                    } else {
                        is_playing = false;
                        utterance_started_at = None;
                    }
                }
            }

            let needs_next = is_playing && current_para >= paragraphs.len() && !paragraphs.is_empty();

            if let Ok(mut st) = status.lock() {
                st.is_playing = is_playing;
                st.is_paused = is_paused;
                st.current_chapter = current_chapter;
                st.current_paragraph = current_para;
                st.total_paragraphs = paragraphs.len();
                st.rate = rate;
                st.engine = current_engine;
                st.selected_voice_id = if current_engine == TtsEngineKind::Edge {
                    current_edge_voice_id.clone()
                } else {
                    current_sys_voice_id.clone()
                };
                st.needs_next_chapter = needs_next;
            }
        }
    }

    pub fn render_panel(
        &mut self,
        ctx: &egui::Context,
        theme: &ReaderTheme,
        current_book_has_text: bool,
        book_language: Option<&str>,
        on_play_requested: &mut bool,
    ) {
        if !self.show_panel {
            return;
        }

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
                        IconLabel::new(Icon::Speech, "Чтение вслух")
                            .size(13.5)
                            .strong()
                            .color(theme.text_color()),
                    );

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    let is_sys = st.engine == TtsEngineKind::System;
                    let is_edge = st.engine == TtsEngineKind::Edge;

                    if ui.selectable_label(is_sys, "Системный").on_hover_text("Локальный офлайн-синтезатор Windows (Ирина, Павел)").clicked() && !is_sys {
                        self.set_engine(TtsEngineKind::System);
                    }
                    if ui.selectable_label(is_edge, "Edge Нейро").clicked() && !is_edge {
                        self.set_engine(TtsEngineKind::Edge);
                    }

                    let (q_rect, q_resp) = ui.allocate_exact_size(Vec2::new(18.0, 20.0), egui::Sense::hover());
                    let icon_rect = egui::Rect::from_center_size(q_rect.center(), Vec2::splat(14.0));
                    let q_color = if q_resp.hovered() {
                        theme.accent_color()
                    } else {
                        theme.secondary_text_color().gamma_multiply(0.7)
                    };
                    paint_icon(ui.painter(), Icon::Question, icon_rect, q_color);
                    q_resp
                        .on_hover_cursor(egui::CursorIcon::Help)
                        .on_hover_text("Для качественного нейросетевого озвучивания требуется подключение к интернету");

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    if !st.is_available {
                        ui.label(
                            RichText::new("Синтезатор речи недоступен")
                                .color(Color32::RED)
                                .size(12.0),
                        );
                    } else if !current_book_has_text {
                        ui.label(
                            RichText::new("Откройте текстовую книгу для озвучивания")
                                .color(theme.secondary_text_color())
                                .size(12.0),
                        );
                    } else {
                        let prev_btn = ui.add(
                            IconButton::new(Icon::ChevronUp, "")
                                .min_size(Vec2::new(26.0, 24.0)),
                        ).on_hover_text("Предыдущий абзац");
                        if prev_btn.clicked() {
                            self.prev_paragraph();
                        }

                        if st.is_playing && !st.is_paused {
                            let pause_btn = ui.add(
                                IconButton::new(Icon::Pause, "Пауза")
                                    .min_size(Vec2::new(76.0, 24.0)),
                            );
                            if pause_btn.clicked() {
                                self.pause();
                            }
                        } else {
                            let play_icon = Icon::Play;
                            let play_label = if st.is_paused { "Продолжить" } else { "Слушать" };
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

                        let next_btn = ui.add(
                            IconButton::new(Icon::ChevronDown, "")
                                .min_size(Vec2::new(26.0, 24.0)),
                        ).on_hover_text("Следующий абзац");
                        if next_btn.clicked() {
                            self.next_paragraph();
                        }

                        if st.is_playing || st.is_paused {
                            let stop_btn = ui.add(
                                IconButton::new(Icon::Close, "Стоп")
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

                        let matched_voices: Vec<&TtsVoiceInfo> = if let Some(lang) = book_language {
                            let m: Vec<&TtsVoiceInfo> = st.voices.iter().filter(|v| voice_matches_lang(&v.language, lang)).collect();
                            if m.is_empty() { st.voices.iter().collect() } else { m }
                        } else {
                            st.voices.iter().collect()
                        };

                        if !matched_voices.is_empty() {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);

                            if !matched_voices.iter().any(|v| Some(&v.id) == st.selected_voice_id.as_ref()) {
                                if let Some(first) = matched_voices.first() {
                                    self.set_voice(first.id.clone());
                                }
                            }

                            let current_voice_name = st
                                .selected_voice_id
                                .as_ref()
                                .and_then(|id| matched_voices.iter().find(|v| v.id == *id))
                                .map(|v| format_voice_name(&v.name))
                                .unwrap_or_else(|| "Выбор голоса".to_string());

                            egui::ComboBox::from_id_salt("tts_voice_combo")
                                .selected_text(current_voice_name)
                                .width(150.0)
                                .show_ui(ui, |ui| {
                                    for v in &matched_voices {
                                        let is_sel = st.selected_voice_id.as_deref() == Some(&v.id);
                                        let clean_name = format_voice_name(&v.name);
                                        let label = format!("{} ({})", clean_name, v.language);
                                        if ui.selectable_label(is_sel, label).clicked() {
                                            self.set_voice(v.id.clone());
                                        }
                                    }
                                });
                        }

                        if let Some(ref err) = st.error {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(err)
                                    .size(11.5)
                                    .color(Color32::RED),
                            );
                        } else if st.is_playing || st.is_paused {
                            ui.add_space(6.0);
                            let status_str = format!(
                                "Гл. {}, абзац {}/{}",
                                st.current_chapter + 1,
                                st.current_paragraph + 1,
                                st.total_paragraphs
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
                            .add(IconButton::new(Icon::Close, "").min_size(Vec2::new(20.0, 20.0)))
                            .on_hover_text("Скрыть панель озвучивания")
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
