use rodio::Source;
use std::io::Cursor;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tts::Tts;

use super::controller::{TtsController, clamp_tts_rate};
use super::types::{TtsCommand, TtsEngineKind, TtsStatus, TtsVoiceInfo};
use super::voices::default_edge_voices;
use crate::document::strip_formatting_tags;
use crate::settings::TtsSettings;

struct EdgeSynthResult {
    generation: u64,
    paragraph_index: usize,
    synthesized_rate: f32,
    audio_bytes: Option<Vec<u8>>,
}

struct TtsWorkerState {
    status: Arc<Mutex<TtsStatus>>,
    sys_tts: Option<Tts>,
    audio_device: Option<rodio::MixerDeviceSink>,
    audio_player: Option<rodio::Player>,
    system_voices: Vec<TtsVoiceInfo>,
    edge_voices: Arc<Mutex<Vec<TtsVoiceInfo>>>,
    edge_client: Arc<Mutex<Option<msedge_tts::tts::client::MSEdgeTTSClient<TcpStream>>>>,
    synth_generation: Arc<AtomicU64>,
    synth_tx: Sender<EdgeSynthResult>,
    synth_rx: Receiver<EdgeSynthResult>,
    current_sys_voice_id: Option<String>,
    current_edge_voice_id: Option<String>,
    current_engine: TtsEngineKind,
    current_chapter: usize,
    paragraphs: Vec<String>,
    current_para: usize,
    is_playing: bool,
    is_paused: bool,
    rate: f32,
    edge_synthesized_rate: f32,
    pending_edge_gen: Option<u64>,
    utterance_started_at: Option<Instant>,
    utterance_resumed_at: Option<Instant>,
    utterance_elapsed: Duration,
    current_para_intervals: Vec<(usize, usize, u32)>,
    current_para_duration: Option<Duration>,
}

impl TtsWorkerState {
    fn new(status: Arc<Mutex<TtsStatus>>, initial: TtsSettings) -> Self {
        let mut sys_tts = Tts::default().ok();
        let audio_device = rodio::DeviceSinkBuilder::open_default_sink().ok();
        let audio_player = audio_device
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

        let initial_edge_voices = default_edge_voices();
        let initial_voice = initial.voice_id.clone();
        let initial_rate = clamp_tts_rate(initial.rate);

        let current_sys_voice_id = initial_voice
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

        let current_edge_voice_id = initial_voice
            .as_ref()
            .filter(|id| {
                initial_edge_voices.iter().any(|voice| &voice.id == *id) || id.contains("Neural")
            })
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
        let current_engine = if requested_edge && audio_device.is_some() {
            TtsEngineKind::Edge
        } else if sys_tts.is_some() {
            TtsEngineKind::System
        } else {
            TtsEngineKind::Edge
        };

        if let Ok(mut st) = status.lock() {
            st.is_available = sys_tts.is_some() || audio_device.is_some();
            st.engine = current_engine;
            st.rate = initial_rate;
            st.voices = if current_engine == TtsEngineKind::Edge {
                initial_edge_voices.clone()
            } else {
                system_voices.clone()
            };
            st.selected_voice_id = if current_engine == TtsEngineKind::Edge {
                current_edge_voice_id.clone()
            } else {
                current_sys_voice_id.clone()
            };
        }

        let edge_voices = Arc::new(Mutex::new(initial_edge_voices));
        let edge_voices_for_remote = Arc::clone(&edge_voices);
        let status_for_remote = Arc::clone(&status);
        let _ = thread::Builder::new()
            .name("tts-edge-voices".to_string())
            .spawn(move || {
                let _ = rustls::crypto::ring::default_provider().install_default();
                if let Ok(remote_voices) = msedge_tts::voice::get_voices_list() {
                    let mapped: Vec<TtsVoiceInfo> = remote_voices
                        .into_iter()
                        .filter_map(|v| {
                            let id = v.short_name?;
                            let name = v.friendly_name.unwrap_or_else(|| id.clone());
                            let language = v.locale.unwrap_or_default();
                            Some(TtsVoiceInfo { id, name, language })
                        })
                        .collect();

                    if !mapped.is_empty() {
                        if let (Ok(mut cache), Ok(mut st)) =
                            (edge_voices_for_remote.lock(), status_for_remote.lock())
                        {
                            apply_remote_edge_voices(&mut cache, &mut st, mapped);
                        }
                    }
                }
            });

        let (synth_tx, synth_rx) = channel();

        Self {
            status,
            sys_tts,
            audio_device,
            audio_player,
            system_voices,
            edge_voices,
            edge_client: Arc::new(Mutex::new(None)),
            synth_generation: Arc::new(AtomicU64::new(0)),
            synth_tx,
            synth_rx,
            current_sys_voice_id,
            current_edge_voice_id,
            current_engine,
            current_chapter: 0,
            paragraphs: Vec::new(),
            current_para: 0,
            is_playing: false,
            is_paused: false,
            rate: initial_rate,
            edge_synthesized_rate: initial_rate,
            pending_edge_gen: None,
            utterance_started_at: None,
            utterance_resumed_at: None,
            utterance_elapsed: Duration::ZERO,
            current_para_intervals: Vec::new(),
            current_para_duration: None,
        }
    }

    fn cancel_pending_edge_synth(&mut self) {
        self.synth_generation.fetch_add(1, Ordering::AcqRel);
        self.pending_edge_gen = None;
    }

    fn restart_player(&mut self) {
        if let Some(p) = self.audio_player.take() {
            p.stop();
        }
        if let Some(dev) = &self.audio_device {
            self.audio_player = Some(rodio::Player::connect_new(&dev.mixer()));
        }
    }

    fn stop_audio(&mut self) {
        if let Some(t) = self.sys_tts.as_mut() {
            let _ = t.stop();
        }
        self.restart_player();
    }

    fn pause_audio(&mut self) {
        if let Some(t) = self.sys_tts.as_mut() {
            let _ = t.stop();
        }
        if let Some(p) = &self.audio_player {
            p.pause();
        }
    }

    fn clear_utterance_timing(&mut self) {
        self.utterance_started_at = None;
        self.utterance_resumed_at = None;
        self.utterance_elapsed = Duration::ZERO;
        self.current_para_duration = None;
    }

    fn current_player_speed(&self) -> f32 {
        if self.current_engine == TtsEngineKind::Edge && self.current_para_duration.is_some() {
            edge_player_speed(self.edge_synthesized_rate, self.rate)
        } else {
            1.0
        }
    }

    fn flush_elapsed_audio_time(&mut self) {
        if let Some(t) = self.utterance_resumed_at.take() {
            let speed = self.current_player_speed();
            self.utterance_elapsed += t.elapsed().mul_f32(speed);
        }
    }

    fn mark_playback_started(&mut self) {
        let now = Instant::now();
        self.utterance_started_at = Some(now);
        self.utterance_resumed_at = Some(now);
        self.utterance_elapsed = Duration::ZERO;
        if let Ok(mut st) = self.status.lock() {
            st.error = None;
            st.playback_failed = false;
        }
    }

    fn handle_playback_failure(&mut self) {
        self.is_playing = false;
        self.pending_edge_gen = None;
        self.clear_utterance_timing();
        self.current_para_intervals.clear();
        if let Ok(mut st) = self.status.lock() {
            st.error = None;
            st.playback_failed = true;
        }
    }

    fn speak_clean_paragraph(&mut self, text: &str) {
        self.cancel_pending_edge_synth();
        self.stop_audio();
        self.clear_utterance_timing();
        self.current_para_intervals = compute_word_intervals(text);

        match self.current_engine {
            TtsEngineKind::System => {
                let ok = if let Some(t) = self.sys_tts.as_mut() {
                    let _ = t.set_rate(self.rate);
                    t.speak(text, true).is_ok()
                } else {
                    false
                };
                if ok {
                    self.mark_playback_started();
                } else {
                    self.handle_playback_failure();
                }
            }
            TtsEngineKind::Edge => {
                if self.audio_device.is_none() {
                    self.handle_playback_failure();
                    return;
                }
                let generation = self.synth_generation.fetch_add(1, Ordering::AcqRel) + 1;
                self.pending_edge_gen = Some(generation);
                self.spawn_edge_synthesis(generation, self.current_para, text.to_string());
            }
        }
    }

    fn spawn_edge_synthesis(&self, generation: u64, paragraph_index: usize, text: String) {
        let synth_generation = Arc::clone(&self.synth_generation);
        let edge_client = Arc::clone(&self.edge_client);
        let synth_tx = self.synth_tx.clone();
        let voice_name = self
            .current_edge_voice_id
            .clone()
            .unwrap_or_else(|| "ru-RU-SvetlanaNeural".to_string());
        let synthesized_rate = clamp_tts_rate(self.rate);
        let config = msedge_tts::tts::SpeechConfig {
            voice_name,
            audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
            pitch: 0,
            rate: edge_speech_rate_pct(synthesized_rate),
            volume: 0,
        };

        let spawned = thread::Builder::new()
            .name("tts-edge-synth".to_string())
            .spawn(move || {
                let _ = rustls::crypto::ring::default_provider().install_default();
                let mut audio_bytes = None;
                let mut attempt = 0;
                while attempt < 2 && audio_bytes.is_none() {
                    if synth_generation.load(Ordering::Acquire) != generation {
                        return;
                    }
                    let mut client_opt = edge_client
                        .try_lock()
                        .ok()
                        .and_then(|mut guard| guard.take());
                    if client_opt.is_none() {
                        client_opt = msedge_tts::tts::client::connect().ok();
                    }
                    if synth_generation.load(Ordering::Acquire) != generation {
                        return;
                    }
                    if let Some(mut client) = client_opt {
                        if let Ok(audio) = client.synthesize(&text, &config) {
                            audio_bytes = Some(audio.audio_bytes);
                            if synth_generation.load(Ordering::Acquire) == generation {
                                if let Ok(mut guard) = edge_client.lock() {
                                    *guard = Some(client);
                                }
                            }
                        }
                    }
                    attempt += 1;
                }
                if synth_generation.load(Ordering::Acquire) == generation {
                    let _ = synth_tx.send(EdgeSynthResult {
                        generation,
                        paragraph_index,
                        synthesized_rate,
                        audio_bytes,
                    });
                }
            });

        if spawned.is_err() {
            let _ = self.synth_tx.send(EdgeSynthResult {
                generation,
                paragraph_index,
                synthesized_rate,
                audio_bytes: None,
            });
        }
    }

    fn process_synth_results(&mut self) {
        while let Ok(result) = self.synth_rx.try_recv() {
            if self.pending_edge_gen != Some(result.generation)
                || !self.is_playing
                || self.is_paused
                || self.current_engine != TtsEngineKind::Edge
                || self.current_para != result.paragraph_index
            {
                continue;
            }
            self.pending_edge_gen = None;

            let Some(bytes) = result.audio_bytes else {
                self.handle_playback_failure();
                continue;
            };

            if self.audio_device.is_none() {
                self.handle_playback_failure();
                continue;
            }

            if let Ok(decoder) = rodio::Decoder::new(Cursor::new(bytes)) {
                self.current_para_duration = decoder.total_duration();
                self.edge_synthesized_rate = result.synthesized_rate;
                let speed = edge_player_speed(self.edge_synthesized_rate, self.rate);
                self.restart_player();
                if let Some(p) = &self.audio_player {
                    p.append(decoder);
                    p.set_speed(speed);
                    p.play();
                    self.mark_playback_started();
                } else {
                    self.handle_playback_failure();
                }
            } else {
                self.handle_playback_failure();
            }
        }
    }

    fn start_chapter(
        &mut self,
        chapter_index: usize,
        paragraph_index: usize,
        new_paras: Vec<String>,
    ) {
        self.cancel_pending_edge_synth();
        self.stop_audio();
        self.current_chapter = chapter_index;
        self.paragraphs = new_paras;
        self.is_playing = true;
        self.is_paused = false;

        if let Some((idx, text)) = find_clean_para(&self.paragraphs, paragraph_index) {
            self.current_para = idx;
            self.speak_clean_paragraph(&text);
        } else {
            self.current_para = self.paragraphs.len();
            self.clear_utterance_timing();
            self.current_para_intervals.clear();
        }
    }

    fn step_paragraph(&mut self, forward: bool) {
        let allow_chapter_end = self.is_playing && !self.is_paused;
        let Some(target_idx) = step_paragraph_index(
            &self.paragraphs,
            self.current_para,
            forward,
            allow_chapter_end,
        ) else {
            return;
        };

        self.cancel_pending_edge_synth();
        self.stop_audio();
        self.current_para = target_idx;
        self.clear_utterance_timing();

        if target_idx < self.paragraphs.len() {
            let clean = strip_formatting_tags(&self.paragraphs[target_idx]);
            let trimmed = clean.trim().to_string();
            self.current_para_intervals = compute_word_intervals(&trimmed);
            if self.is_playing && !self.is_paused {
                self.speak_clean_paragraph(&trimmed);
            }
        } else {
            self.current_para_intervals.clear();
        }
    }

    fn handle_command(&mut self, command: TtsCommand) {
        match command {
            TtsCommand::PlayChapter {
                chapter_index,
                paragraph_index,
                paragraphs,
            } => {
                self.start_chapter(chapter_index, paragraph_index, paragraphs);
            }
            TtsCommand::PlayNextChapter {
                chapter_index,
                paragraphs,
            } => {
                self.start_chapter(chapter_index, 0, paragraphs);
            }
            TtsCommand::Pause => {
                self.cancel_pending_edge_synth();
                self.flush_elapsed_audio_time();
                self.pause_audio();
                if self.is_playing {
                    self.is_paused = true;
                }
            }
            TtsCommand::Resume => {
                if self.is_paused && self.current_para < self.paragraphs.len() {
                    if self.current_engine == TtsEngineKind::Edge {
                        let has_buffered = self
                            .audio_player
                            .as_ref()
                            .map(|p| !p.empty())
                            .unwrap_or(false);
                        if has_buffered {
                            if let Some(p) = &self.audio_player {
                                let speed =
                                    edge_player_speed(self.edge_synthesized_rate, self.rate);
                                p.set_speed(speed);
                                p.play();
                                self.is_paused = false;
                                self.utterance_resumed_at = Some(Instant::now());
                            }
                        } else if let Some((idx, text)) =
                            find_clean_para(&self.paragraphs, self.current_para)
                        {
                            self.current_para = idx;
                            self.is_paused = false;
                            self.speak_clean_paragraph(&text);
                        }
                    } else if let Some((idx, text)) =
                        find_clean_para(&self.paragraphs, self.current_para)
                    {
                        self.current_para = idx;
                        self.is_paused = false;
                        self.speak_clean_paragraph(&text);
                    }
                }
            }
            TtsCommand::Stop => {
                self.cancel_pending_edge_synth();
                self.stop_audio();
                self.is_playing = false;
                self.is_paused = false;
                self.clear_utterance_timing();
                self.current_para_intervals.clear();
            }
            TtsCommand::NextParagraph => {
                self.step_paragraph(true);
            }
            TtsCommand::PrevParagraph => {
                self.step_paragraph(false);
            }
            TtsCommand::SetRate(new_rate) => {
                let new_rate = clamp_tts_rate(new_rate);
                if self.is_playing && !self.is_paused {
                    self.flush_elapsed_audio_time();
                    if self.utterance_started_at.is_some() {
                        self.utterance_resumed_at = Some(Instant::now());
                    }
                }
                self.rate = new_rate;
                if let Ok(mut st) = self.status.lock() {
                    st.rate = self.rate;
                }
                if self.current_engine == TtsEngineKind::Edge {
                    if self.pending_edge_gen.is_some()
                        && self.is_playing
                        && !self.is_paused
                        && self.current_para < self.paragraphs.len()
                    {
                        if let Some((idx, text)) =
                            find_clean_para(&self.paragraphs, self.current_para)
                        {
                            self.current_para = idx;
                            self.speak_clean_paragraph(&text);
                        }
                    } else if let Some(p) = &self.audio_player {
                        p.set_speed(edge_player_speed(self.edge_synthesized_rate, self.rate));
                    }
                } else if self.is_playing
                    && !self.is_paused
                    && self.current_para < self.paragraphs.len()
                {
                    self.stop_audio();
                    if let Some((idx, text)) = find_clean_para(&self.paragraphs, self.current_para)
                    {
                        self.current_para = idx;
                        self.speak_clean_paragraph(&text);
                    }
                }
            }
            TtsCommand::SetVoice(voice_id) => {
                let voice_changed = if self.current_engine == TtsEngineKind::Edge {
                    let changed = self.current_edge_voice_id.as_deref() != Some(&voice_id);
                    self.current_edge_voice_id = Some(voice_id.clone());
                    changed
                } else {
                    let changed = self.current_sys_voice_id.as_deref() != Some(&voice_id);
                    self.current_sys_voice_id = Some(voice_id.clone());
                    if let (Some(tts), Some(id)) =
                        (self.sys_tts.as_mut(), &self.current_sys_voice_id)
                    {
                        if let Ok(all_voices) = tts.voices() {
                            if let Some(target) = all_voices.iter().find(|v| v.id() == *id) {
                                let _ = tts.set_voice(target);
                            }
                        }
                    }
                    changed
                };
                if voice_changed && self.current_engine == TtsEngineKind::Edge {
                    self.cancel_pending_edge_synth();
                    if let Ok(mut guard) = self.edge_client.lock() {
                        *guard = None;
                    }
                    if self.is_paused {
                        self.stop_audio();
                        self.clear_utterance_timing();
                    }
                }
                if let Ok(mut st) = self.status.lock() {
                    st.selected_voice_id = Some(voice_id);
                }
                if self.is_playing && !self.is_paused && self.current_para < self.paragraphs.len() {
                    if let Some((idx, text)) = find_clean_para(&self.paragraphs, self.current_para)
                    {
                        self.current_para = idx;
                        self.speak_clean_paragraph(&text);
                    }
                }
            }
            TtsCommand::SetEngine(new_engine) => {
                if self.current_engine != new_engine {
                    self.cancel_pending_edge_synth();
                    self.stop_audio();
                    self.current_engine = new_engine;
                    if let Ok(mut guard) = self.edge_client.lock() {
                        *guard = None;
                    }
                    self.clear_utterance_timing();

                    let new_selected_voice = if self.current_engine == TtsEngineKind::Edge {
                        self.current_edge_voice_id.clone()
                    } else {
                        self.current_sys_voice_id.clone()
                    };
                    let new_voices = if self.current_engine == TtsEngineKind::Edge {
                        self.edge_voices
                            .lock()
                            .map(|v| v.clone())
                            .unwrap_or_else(|_| default_edge_voices())
                    } else {
                        self.system_voices.clone()
                    };
                    if let Ok(mut st) = self.status.lock() {
                        st.engine = self.current_engine;
                        st.voices = new_voices;
                        st.selected_voice_id = new_selected_voice;
                        st.error = None;
                        st.playback_failed = false;
                    }
                    if self.is_playing
                        && !self.is_paused
                        && self.current_para < self.paragraphs.len()
                    {
                        if let Some((idx, text)) =
                            find_clean_para(&self.paragraphs, self.current_para)
                        {
                            self.current_para = idx;
                            self.speak_clean_paragraph(&text);
                        }
                    }
                }
            }
        }
    }

    fn is_currently_speaking(&mut self) -> bool {
        if !self.is_playing || self.is_paused {
            return false;
        }
        match self.current_engine {
            TtsEngineKind::System => self
                .sys_tts
                .as_mut()
                .and_then(|t| t.is_speaking().ok())
                .unwrap_or(false),
            TtsEngineKind::Edge => self
                .audio_player
                .as_ref()
                .map(|s| !s.empty() && !s.is_paused())
                .unwrap_or(false),
        }
    }

    fn advance_if_finished(&mut self, speaking: bool) {
        if !self.is_playing || self.is_paused || self.pending_edge_gen.is_some() {
            return;
        }
        let grace_passed = self
            .utterance_started_at
            .map(|t| t.elapsed() > Duration::from_millis(300))
            .unwrap_or(true);

        if !speaking && grace_passed {
            if let Some((idx, text)) = find_clean_para(&self.paragraphs, self.current_para + 1) {
                self.current_para = idx;
                self.speak_clean_paragraph(&text);
            } else {
                self.is_playing = false;
                self.clear_utterance_timing();
                self.current_para_intervals.clear();
            }
        }
    }

    fn publish_status(&mut self, speaking: bool) {
        let active_word_range =
            if self.is_playing && self.pending_edge_gen.is_none() && (speaking || self.is_paused) {
                let speed = self.current_player_speed();
                let stream_elapsed = self.utterance_elapsed
                    + self
                        .utterance_resumed_at
                        .map(|t| t.elapsed().mul_f32(speed))
                        .unwrap_or(Duration::ZERO);
                let highlight_rate = if self.current_para_duration.is_some() {
                    1.0
                } else {
                    self.rate
                };
                compute_active_word_range(
                    &self.current_para_intervals,
                    stream_elapsed,
                    highlight_rate,
                    self.current_para_duration,
                )
            } else {
                None
            };

        let needs_next = self.is_playing
            && !self.is_paused
            && self.current_para >= self.paragraphs.len()
            && !self.paragraphs.is_empty();

        if let Ok(mut st) = self.status.lock() {
            st.is_playing = self.is_playing;
            st.is_paused = self.is_paused;
            st.current_chapter = self.current_chapter;
            st.current_paragraph = self.current_para;
            st.total_paragraphs = self.paragraphs.len();
            st.rate = self.rate;
            st.engine = self.current_engine;
            st.selected_voice_id = if self.current_engine == TtsEngineKind::Edge {
                self.current_edge_voice_id.clone()
            } else {
                self.current_sys_voice_id.clone()
            };
            st.needs_next_chapter = needs_next;
            st.current_word_range = active_word_range;
        }
    }

    fn needs_fast_poll(&self) -> bool {
        self.pending_edge_gen.is_some() || (self.is_playing && !self.is_paused)
    }

    fn shutdown(&mut self) {
        self.cancel_pending_edge_synth();
        self.stop_audio();
    }
}

impl TtsController {
    pub(super) fn worker_loop(
        rx: Receiver<TtsCommand>,
        status: Arc<Mutex<TtsStatus>>,
        initial: TtsSettings,
    ) {
        let mut state = TtsWorkerState::new(status, initial);

        loop {
            let received = if state.needs_fast_poll() {
                match rx.recv_timeout(Duration::from_millis(30)) {
                    Ok(cmd) => Some(cmd),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match rx.recv() {
                    Ok(cmd) => Some(cmd),
                    Err(_) => break,
                }
            };

            if let Some(command) = received {
                state.handle_command(command);
            }

            state.process_synth_results();

            let speaking = state.is_currently_speaking();
            state.advance_if_finished(speaking);
            state.publish_status(speaking);
        }

        state.shutdown();
    }
}

fn find_clean_para(paras: &[String], idx: usize) -> Option<(usize, String)> {
    let mut i = idx;
    while i < paras.len() {
        let clean = strip_formatting_tags(&paras[i]);
        let trimmed = clean.trim();
        if !trimmed.is_empty() {
            return Some((i, trimmed.to_string()));
        }
        i += 1;
    }
    None
}

fn find_prev_clean_para(paras: &[String], before_idx: usize) -> Option<(usize, String)> {
    if paras.is_empty() || before_idx == 0 {
        return None;
    }
    let mut i = before_idx.min(paras.len());
    while i > 0 {
        i -= 1;
        let clean = strip_formatting_tags(&paras[i]);
        let trimmed = clean.trim();
        if !trimmed.is_empty() {
            return Some((i, trimmed.to_string()));
        }
    }
    None
}

pub(crate) fn step_paragraph_index(
    paragraphs: &[String],
    current_para: usize,
    forward: bool,
    allow_chapter_end: bool,
) -> Option<usize> {
    if paragraphs.is_empty() {
        return None;
    }
    if forward {
        if let Some((idx, _)) = find_clean_para(paragraphs, current_para.saturating_add(1)) {
            Some(idx)
        } else if allow_chapter_end {
            Some(paragraphs.len())
        } else {
            None
        }
    } else if let Some((idx, _)) = find_prev_clean_para(paragraphs, current_para) {
        Some(idx)
    } else {
        find_clean_para(paragraphs, 0).map(|(idx, _)| idx)
    }
}

pub(crate) fn edge_speech_rate_pct(rate: f32) -> i32 {
    let clamped = clamp_tts_rate(rate);
    ((clamped - 1.0) * 100.0).round().clamp(-50.0, 200.0) as i32
}

pub(crate) fn edge_player_speed(synthesized_rate: f32, target_rate: f32) -> f32 {
    let synth = clamp_tts_rate(synthesized_rate);
    let target = clamp_tts_rate(target_rate);
    (target / synth).clamp(0.1, 6.0)
}

pub(crate) fn sort_edge_voices(voices: &mut [TtsVoiceInfo]) {
    voices.sort_by(|a, b| {
        let a_lang = a.language.to_ascii_lowercase();
        let b_lang = b.language.to_ascii_lowercase();
        b_lang
            .starts_with("ru")
            .cmp(&a_lang.starts_with("ru"))
            .then_with(|| b_lang.starts_with("en").cmp(&a_lang.starts_with("en")))
            .then_with(|| a.language.cmp(&b.language))
            .then_with(|| a.name.cmp(&b.name))
    });
}

pub(crate) fn apply_remote_edge_voices(
    edge_voices: &mut Vec<TtsVoiceInfo>,
    status: &mut TtsStatus,
    mut mapped: Vec<TtsVoiceInfo>,
) {
    if mapped.is_empty() {
        return;
    }
    sort_edge_voices(&mut mapped);
    *edge_voices = mapped.clone();
    if status.engine == TtsEngineKind::Edge {
        status.voices = mapped;
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
