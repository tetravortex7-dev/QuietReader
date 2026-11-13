mod controller;
mod panel;
mod types;
mod voices;
mod worker;

pub use self::controller::TtsController;
#[allow(unused_imports)]
pub(crate) use self::controller::clamp_tts_rate;
#[allow(unused_imports)]
pub use self::types::{TtsCommand, TtsEngineKind, TtsStatus, TtsVoiceInfo};
#[allow(unused_imports)]
pub use self::voices::voice_matches_lang;
#[allow(unused_imports)]
pub(crate) use self::worker::{
    apply_remote_edge_voices, compute_active_word_range, compute_word_intervals, edge_player_speed,
    edge_speech_rate_pct, step_paragraph_index,
};
