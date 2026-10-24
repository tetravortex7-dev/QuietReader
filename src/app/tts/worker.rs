use std::io::Cursor;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tts::Tts;

use super::controller::TtsController;
use super::types::{TtsCommand, TtsEngineKind, TtsStatus, TtsVoiceInfo};
use super::voices::default_edge_voices;
use crate::settings::TtsSettings;

impl TtsController {
    pub(super) fn worker_loop(
        rx: Receiver<TtsCommand>,
        status: Arc<Mutex<TtsStatus>>,
        initial: TtsSettings,
    ) {
        let mut sys_tts = Tts::default().ok();
        let audio_device: Option<rodio::MixerDeviceSink> =
            rodio::DeviceSinkBuilder::open_default_sink().ok();
        let mut audio_player: Option<rodio::Player> = audio_device
            .as_ref()
            .map(|handle| rodio::Player::connect_new(&handle.mixer()));

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
        let initial_voice = initial.voice_id.clone();
        let initial_rate = initial.rate.clamp(0.5, 3.0);

        let mut current_sys_voice_id = initial_voice
            .as_ref()
            .filter(|id| system_voices.iter().any(|voice| &voice.id == *id))
            .cloned()
            .or_else(|| {
                system_voices
                    .iter()
                    .find(|v| v.language.to_ascii_lowercase().starts_with("ru"))
                    .map(|v| v.id.clone())
            })
            .or_else(|| system_voices.first().map(|v| v.id.clone()));

        let mut current_edge_voice_id = initial_voice
            .as_ref()
            .filter(|id| edge_voices.iter().any(|voice| &voice.id == *id))
            .cloned()
            .or_else(|| Some("ru-RU-SvetlanaNeural".to_string()));

        if let (Some(tts), Some(id)) = (sys_tts.as_mut(), &current_sys_voice_id) {
            if let Ok(all_voices) = tts.voices() {
                if let Some(target) = all_voices.iter().find(|v| v.id() == *id) {
                    let _ = tts.set_voice(target);
                }
            }
        }

        let requested_edge = initial.engine.eq_ignore_ascii_case("edge");
        let mut current_engine = if requested_edge && audio_device.is_some() {
            TtsEngineKind::Edge
        } else if sys_tts.is_some() {
            TtsEngineKind::System
        } else {
            TtsEngineKind::Edge
        };

        {
            if let Ok(mut st) = status.lock() {
                st.is_available = sys_tts.is_some() || audio_device.is_some();
                st.engine = current_engine;
                st.rate = initial_rate;
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
                        b.language
                            .starts_with("ru")
                            .cmp(&a.language.starts_with("ru"))
                            .then_with(|| {
                                b.language
                                    .starts_with("en")
                                    .cmp(&a.language.starts_with("en"))
                            })
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
        let mut rate = initial_rate;
        let mut utterance_started_at: Option<Instant> = None;
        let mut edge_client: Option<msedge_tts::tts::client::MSEdgeTTSClient<std::net::TcpStream>> =
            None;

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

        let restart_player = |device: &Option<rodio::MixerDeviceSink>,
                              player: &mut Option<rodio::Player>| {
            if let Some(p) = player.take() {
                p.stop();
            }
            if let Some(dev) = device {
                *player = Some(rodio::Player::connect_new(&dev.mixer()));
            }
        };

        let stop_all = |sys_tts: &mut Option<Tts>,
                        device: &Option<rodio::MixerDeviceSink>,
                        player: &mut Option<rodio::Player>| {
            if let Some(t) = sys_tts.as_mut() {
                let _ = t.stop();
            }
            restart_player(device, player);
        };

        let pause_all = |sys_tts: &mut Option<Tts>, player: &Option<rodio::Player>| {
            if let Some(t) = sys_tts.as_mut() {
                let _ = t.stop();
            }
            if let Some(p) = player {
                p.pause();
            }
        };

        let speak_para = |engine: TtsEngineKind,
                          text: &str,
                          cur_rate: f32,
                          sys_tts: &mut Option<Tts>,
                          edge_client: &mut Option<
            msedge_tts::tts::client::MSEdgeTTSClient<std::net::TcpStream>,
        >,
                          edge_voice_id: &Option<String>,
                          device: &Option<rodio::MixerDeviceSink>,
                          player: &mut Option<rodio::Player>|
         -> bool {
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
                                    if let Ok(decoder) =
                                        rodio::Decoder::new(Cursor::new(audio.audio_bytes))
                                    {
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
                                    st.playback_failed = false;
                                }
                            } else {
                                is_playing = false;
                                if let Ok(mut st) = status.lock() {
                                    st.error = None;
                                    st.playback_failed = true;
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
                                    } else if let Some((idx, text)) =
                                        find_clean_para(&paragraphs, current_para)
                                    {
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
                            } else if let Some((idx, text)) =
                                find_clean_para(&paragraphs, current_para)
                            {
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
                            let clean =
                                crate::document::strip_formatting_tags(&paragraphs[back_idx]);
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
                            if let (Some(tts), Some(id)) = (sys_tts.as_mut(), &current_sys_voice_id)
                            {
                                if let Ok(all_voices) = tts.voices() {
                                    if let Some(target) = all_voices.iter().find(|v| v.id() == *id)
                                    {
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
                                        st.playback_failed = false;
                                    }
                                } else {
                                    is_playing = false;
                                    utterance_started_at = None;
                                    if let Ok(mut st) = status.lock() {
                                        st.error = None;
                                        st.playback_failed = true;
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
                                st.playback_failed = false;
                            }
                            if is_playing && !is_paused && current_para < paragraphs.len() {
                                if let Some((idx, text)) =
                                    find_clean_para(&paragraphs, current_para)
                                {
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
                    TtsEngineKind::System => sys_tts
                        .as_mut()
                        .and_then(|t| t.is_speaking().ok())
                        .unwrap_or(false),
                    TtsEngineKind::Edge => audio_player
                        .as_ref()
                        .map(|s| !s.empty() && !s.is_paused())
                        .unwrap_or(false),
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

            let needs_next =
                is_playing && current_para >= paragraphs.len() && !paragraphs.is_empty();

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
}
