use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

use super::types::{TtsCommand, TtsEngineKind, TtsStatus};
use crate::settings::TtsSettings;

pub struct TtsController {
    command_tx: Option<Sender<TtsCommand>>,
    status: Arc<Mutex<TtsStatus>>,
    preferred: TtsSettings,
    pub show_panel: bool,
}

impl Default for TtsController {
    fn default() -> Self {
        Self::new(TtsSettings::default())
    }
}

impl TtsController {
    pub fn new(initial: TtsSettings) -> Self {
        let mut initial = initial;
        initial.rate = initial.rate.clamp(0.5, 3.0);
        let status = Arc::new(Mutex::new(TtsStatus {
            rate: initial.rate,
            ..Default::default()
        }));

        let (tx, rx) = channel::<TtsCommand>();
        let status_clone = Arc::clone(&status);
        let worker_initial = initial.clone();

        thread::spawn(move || {
            Self::worker_loop(rx, status_clone, worker_initial);
        });

        Self {
            command_tx: Some(tx),
            status,
            preferred: initial,
            show_panel: false,
        }
    }

    pub fn preferences(&self) -> TtsSettings {
        self.preferred.clone()
    }

    pub fn status(&self) -> TtsStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn toggle_panel(&mut self) {
        self.show_panel = !self.show_panel;
    }

    pub fn play_chapter(
        &mut self,
        chapter_index: usize,
        paragraph_index: usize,
        paragraphs: Vec<String>,
    ) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::PlayChapter {
                chapter_index,
                paragraph_index,
                paragraphs,
            });
        }
    }

    pub fn play_next_chapter(&mut self, chapter_index: usize, paragraphs: Vec<String>) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::PlayNextChapter {
                chapter_index,
                paragraphs,
            });
        }
    }

    pub fn pause(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::Pause);
        }
    }

    pub fn resume(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::Resume);
        }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::Stop);
        }
    }

    pub fn next_paragraph(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::NextParagraph);
        }
    }

    pub fn prev_paragraph(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::PrevParagraph);
        }
    }

    pub fn set_rate(&mut self, rate: f32) {
        self.preferred.rate = rate.clamp(0.5, 3.0);
        if let Ok(mut st) = self.status.lock() {
            st.rate = rate;
        }
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::SetRate(rate));
        }
    }

    pub fn set_voice(&mut self, voice_id: String) {
        self.preferred.voice_id = Some(voice_id.clone());
        if let Ok(mut st) = self.status.lock() {
            st.selected_voice_id = Some(voice_id.clone());
        }
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::SetVoice(voice_id));
        }
    }

    pub fn set_engine(&mut self, engine: TtsEngineKind) {
        self.preferred.engine = match engine {
            TtsEngineKind::System => "system".to_string(),
            TtsEngineKind::Edge => "edge".to_string(),
        };
        if let Ok(mut st) = self.status.lock() {
            st.engine = engine;
        }
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(TtsCommand::SetEngine(engine));
        }
    }
}
