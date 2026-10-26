use rodio::Source;
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
        let mut utterance_resumed_at: Option<Instant> = None;
        let mut utterance_elapsed = Duration::ZERO;
        let mut current_para_intervals: Vec<(usize, usize, u32)> = Vec::new();
        let mut current_para_duration: Option<Duration> = None;
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
                          player: &mut Option<rodio::Player>,
                          current_para_duration: &mut Option<Duration>,
                          current_para_intervals: &mut Vec<(usize, usize, u32)>|
         -> bool {
            *current_para_intervals = compute_word_intervals(text);
            *current_para_duration = None;
            let success = match engine {
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
                                        *current_para_duration = decoder.total_duration();
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
            };
            if !success {
                current_para_intervals.clear();
                *current_para_duration = None;
            }
            success
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
                                &mut current_para_duration,
                                &mut current_para_intervals,
                            );
                            if ok {
                                let now = Instant::now();
                                utterance_started_at = Some(now);
                                utterance_resumed_at = Some(now);
                                utterance_elapsed = Duration::ZERO;
                                if let Ok(mut st) = status.lock() {
                                    st.error = None;
                                    st.playback_failed = false;
                                }
                            } else {
                                is_playing = false;
                                utterance_started_at = None;
                                utterance_resumed_at = None;
                                utterance_elapsed = Duration::ZERO;
                                current_para_intervals.clear();
                                current_para_duration = None;
                                if let Ok(mut st) = status.lock() {
                                    st.error = None;
                                    st.playback_failed = true;
                                }
                            }
                        } else {
                            current_para = paragraphs.len();
                            utterance_started_at = None;
                            utterance_resumed_at = None;
                            utterance_elapsed = Duration::ZERO;
                            current_para_intervals.clear();
                            current_para_duration = None;
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
                                &mut current_para_duration,
                                &mut current_para_intervals,
                            );
                            if ok {
                                let now = Instant::now();
                                utterance_started_at = Some(now);
                                utterance_resumed_at = Some(now);
                                utterance_elapsed = Duration::ZERO;
                            } else {
                                is_playing = false;
                                utterance_started_at = None;
                                utterance_resumed_at = None;
                                utterance_elapsed = Duration::ZERO;
                                current_para_intervals.clear();
                                current_para_duration = None;
                            }
                        } else {
                            current_para = paragraphs.len();
                            utterance_started_at = None;
                            utterance_resumed_at = None;
                            utterance_elapsed = Duration::ZERO;
                            current_para_intervals.clear();
                            current_para_duration = None;
                        }
                    }
                    TtsCommand::Pause => {
                        pause_all(&mut sys_tts, &audio_player);
                        is_paused = true;
                        if let Some(t) = utterance_resumed_at.take() {
                            utterance_elapsed += t.elapsed();
                        }
                    }
                    TtsCommand::Resume => {
                        if is_paused && current_para < paragraphs.len() {
                            if current_engine == TtsEngineKind::Edge {
                                if let Some(s) = &audio_player {
                                    if !s.empty() {
                                        s.play();
                                        is_paused = false;
                                        utterance_resumed_at = Some(Instant::now());
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
                                            &mut current_para_duration,
                                            &mut current_para_intervals,
                                        ) {
                                            is_paused = false;
                                            let now = Instant::now();
                                            utterance_started_at = Some(now);
                                            utterance_resumed_at = Some(now);
                                            utterance_elapsed = Duration::ZERO;
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
                                    &mut current_para_duration,
                                    &mut current_para_intervals,
                                ) {
                                    is_paused = false;
                                    let now = Instant::now();
                                    utterance_started_at = Some(now);
                                    utterance_resumed_at = Some(now);
                                    utterance_elapsed = Duration::ZERO;
                                }
                            }
                        }
                    }
                    TtsCommand::Stop => {
                        stop_all(&mut sys_tts, &audio_device, &mut audio_player);
                        is_playing = false;
                        is_paused = false;
                        utterance_started_at = None;
                        utterance_resumed_at = None;
                        utterance_elapsed = Duration::ZERO;
                        current_para_intervals.clear();
                        current_para_duration = None;
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
                                &mut current_para_duration,
                                &mut current_para_intervals,
                            ) {
                                let now = Instant::now();
                                utterance_started_at = Some(now);
                                utterance_resumed_at = Some(now);
                                utterance_elapsed = Duration::ZERO;
                            } else {
                                current_para_intervals.clear();
                                current_para_duration = None;
                                utterance_started_at = None;
                                utterance_resumed_at = None;
                                utterance_elapsed = Duration::ZERO;
                            }
                        } else {
                            current_para = paragraphs.len();
                            current_para_intervals.clear();
                            current_para_duration = None;
                            utterance_started_at = None;
                            utterance_resumed_at = None;
                            utterance_elapsed = Duration::ZERO;
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
                                &mut current_para_duration,
                                &mut current_para_intervals,
                            ) {
                                let now = Instant::now();
                                utterance_started_at = Some(now);
                                utterance_resumed_at = Some(now);
                                utterance_elapsed = Duration::ZERO;
                            } else {
                                current_para_intervals.clear();
                                current_para_duration = None;
                                utterance_started_at = None;
                                utterance_resumed_at = None;
                                utterance_elapsed = Duration::ZERO;
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
                                    &mut current_para_duration,
                                    &mut current_para_intervals,
                                ) {
                                    let now = Instant::now();
                                    utterance_started_at = Some(now);
                                    utterance_resumed_at = Some(now);
                                    utterance_elapsed = Duration::ZERO;
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
                                    &mut current_para_duration,
                                    &mut current_para_intervals,
                                ) {
                                    let now = Instant::now();
                                    utterance_started_at = Some(now);
                                    utterance_resumed_at = Some(now);
                                    utterance_elapsed = Duration::ZERO;
                                    if let Ok(mut st) = status.lock() {
                                        st.error = None;
                                        st.playback_failed = false;
                                    }
                                } else {
                                    is_playing = false;
                                    utterance_started_at = None;
                                    utterance_resumed_at = None;
                                    utterance_elapsed = Duration::ZERO;
                                    current_para_intervals.clear();
                                    current_para_duration = None;
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
                                        &mut current_para_duration,
                                        &mut current_para_intervals,
                                    ) {
                                        let now = Instant::now();
                                        utterance_started_at = Some(now);
                                        utterance_resumed_at = Some(now);
                                        utterance_elapsed = Duration::ZERO;
                                    } else {
                                        is_playing = false;
                                        utterance_started_at = None;
                                        utterance_resumed_at = None;
                                        utterance_elapsed = Duration::ZERO;
                                        current_para_intervals.clear();
                                        current_para_duration = None;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let speaking = if is_playing && !is_paused {
                match current_engine {
                    TtsEngineKind::System => sys_tts
                        .as_mut()
                        .and_then(|t| t.is_speaking().ok())
                        .unwrap_or(false),
                    TtsEngineKind::Edge => audio_player
                        .as_ref()
                        .map(|s| !s.empty() && !s.is_paused())
                        .unwrap_or(false),
                }
            } else {
                false
            };

            if is_playing && !is_paused {
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
                            &mut current_para_duration,
                            &mut current_para_intervals,
                        ) {
                            let now = Instant::now();
                            utterance_started_at = Some(now);
                            utterance_resumed_at = Some(now);
                            utterance_elapsed = Duration::ZERO;
                        } else {
                            is_playing = false;
                            utterance_started_at = None;
                            utterance_resumed_at = None;
                            utterance_elapsed = Duration::ZERO;
                            current_para_intervals.clear();
                            current_para_duration = None;
                        }
                    } else {
                        is_playing = false;
                        utterance_started_at = None;
                        utterance_resumed_at = None;
                        utterance_elapsed = Duration::ZERO;
                        current_para_intervals.clear();
                        current_para_duration = None;
                    }
                }
            }

            let active_word_range = if is_playing && (speaking || is_paused) {
                let elapsed = utterance_elapsed
                    + utterance_resumed_at
                        .map(|t| t.elapsed())
                        .unwrap_or(Duration::ZERO);
                compute_active_word_range(
                    &current_para_intervals,
                    elapsed,
                    rate,
                    current_para_duration,
                )
            } else {
                None
            };

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
                st.current_word_range = active_word_range;
            }
        }
    }
}

pub(crate) fn compute_word_intervals(text: &str) -> Vec<(usize, usize, u32)> {
    let mut intervals = Vec::new();
    let mut word_start = None;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = word_start.take() {
                let end = idx;
                let word = &text[start..end];
                let weight = compute_word_weight(word);
                intervals.push((start, end, weight));
            }
        } else if word_start.is_none() {
            word_start = Some(idx);
        }
    }
    if let Some(start) = word_start {
        let end = text.len();
        let word = &text[start..end];
        let weight = compute_word_weight(word);
        intervals.push((start, end, weight));
    }
    intervals
}

fn compute_word_weight(word: &str) -> u32 {
    let char_count = word.chars().count() as u32;
    let extra =
        if word.ends_with('.') || word.ends_with('!') || word.ends_with('?') || word.ends_with(';')
        {
            6
        } else if word.ends_with(',')
            || word.ends_with(':')
            || word.ends_with('—')
            || word.ends_with('-')
        {
            3
        } else {
            1
        };
    (char_count.max(1) + extra).max(2)
}

pub(crate) fn compute_active_word_range(
    intervals: &[(usize, usize, u32)],
    elapsed: Duration,
    rate: f32,
    total_audio_duration: Option<Duration>,
) -> Option<(usize, usize)> {
    if intervals.is_empty() {
        return None;
    }
    let total_weight: u32 = intervals.iter().map(|(_, _, w)| *w).sum();
    if total_weight == 0 {
        return None;
    }

    let elapsed_secs = elapsed.as_secs_f32();
    let total_secs = match total_audio_duration {
        Some(d) => (d.as_secs_f32() / rate.max(0.2)).max(0.1),
        None => (total_weight as f32 * 0.055 / rate.max(0.2)).max(0.1),
    };

    if elapsed_secs >= total_secs {
        return intervals.last().map(|&(s, e, _)| (s, e));
    }

    let progress = (elapsed_secs / total_secs).clamp(0.0, 1.0);
    let target_weight = (progress * total_weight as f32) as u32;

    let mut accumulated = 0;
    for &(start, end, weight) in intervals {
        accumulated += weight;
        if target_weight <= accumulated {
            return Some((start, end));
        }
    }

    intervals.last().map(|&(s, e, _)| (s, e))
}
