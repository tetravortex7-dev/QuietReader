mod controller;
mod panel;
mod types;
mod voices;
mod worker;

pub use self::controller::TtsController;
#[allow(unused_imports)]
pub use self::types::{TtsCommand, TtsEngineKind, TtsStatus, TtsVoiceInfo};
#[allow(unused_imports)]
pub use self::voices::voice_matches_lang;
#[allow(unused_imports)]
pub(crate) use self::worker::{compute_active_word_range, compute_word_intervals};
