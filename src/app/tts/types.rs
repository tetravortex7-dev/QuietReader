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
    pub playback_failed: bool,
    pub needs_next_chapter: bool,
}

pub enum TtsCommand {
    PlayChapter {
        chapter_index: usize,
        paragraph_index: usize,
        paragraphs: Vec<String>,
    },
    PlayNextChapter {
        chapter_index: usize,
        paragraphs: Vec<String>,
    },
    Pause,
    Resume,
    Stop,
    NextParagraph,
    PrevParagraph,
    SetRate(f32),
    SetVoice(String),
    SetEngine(TtsEngineKind),
}
