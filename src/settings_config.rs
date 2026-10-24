use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpdsCatalog {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(skip)]
    pub password: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

impl std::fmt::Debug for OpdsCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpdsCatalog")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("url", &self.url)
            .field("username", &self.username)
            .field(
                "password",
                &if self
                    .password
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())
                {
                    "<redacted>"
                } else {
                    ""
                },
            )
            .field("is_default", &self.is_default)
            .finish()
    }
}

pub fn default_opds_catalogs() -> Vec<OpdsCatalog> {
    vec![
        OpdsCatalog {
            id: "standard_ebooks".into(),
            name: "Standard Ebooks".into(),
            url: "https://standardebooks.org/opds/all-records".into(),
            username: None,
            password: None,
            is_default: true,
        },
        OpdsCatalog {
            id: "gutenberg".into(),
            name: "Project Gutenberg".into(),
            url: "https://m.gutenberg.org/ebooks.opds/".into(),
            username: None,
            password: None,
            is_default: true,
        },
        OpdsCatalog {
            id: "feedbooks".into(),
            name: "Feedbooks Free".into(),
            url: "https://openbooks.feedbooks.com/catalog.atom".into(),
            username: None,
            password: None,
            is_default: true,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TtsSettings {
    pub engine: String,
    pub voice_id: Option<String>,
    pub rate: f32,
}

impl Default for TtsSettings {
    fn default() -> Self {
        Self {
            engine: "system".to_string(),
            voice_id: None,
            rate: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientSettings {
    #[serde(default)]
    pub is_playing: bool,
    #[serde(default)]
    pub resume_on_startup: bool,
    #[serde(default = "default_master_vol")]
    pub master_volume: f32,
    #[serde(default = "default_tone_softness")]
    pub tone_softness: f32,
    #[serde(default = "default_true")]
    pub duck_during_tts: bool,
    #[serde(default = "default_duck_amount")]
    pub duck_amount: f32,
    #[serde(default = "default_preset")]
    pub selected_preset: Option<String>,
    #[serde(default)]
    pub timer_end_unix: Option<u64>,
    #[serde(default = "default_brown_vol")]
    pub brown_noise_vol: f32,
    #[serde(default)]
    pub pink_noise_vol: f32,
    #[serde(default)]
    pub rain_vol: f32,
    #[serde(default)]
    pub fireplace_vol: f32,
    #[serde(default)]
    pub lofi_vol: f32,
    #[serde(default = "default_binaural_vol")]
    pub binaural_vol: f32,
}

fn default_master_vol() -> f32 {
    0.65
}

fn default_tone_softness() -> f32 {
    0.50
}

fn default_true() -> bool {
    true
}

fn default_duck_amount() -> f32 {
    0.52
}

fn default_preset() -> Option<String> {
    Some("laser_focus".to_string())
}

fn default_brown_vol() -> f32 {
    0.75
}

fn default_binaural_vol() -> f32 {
    0.35
}

impl Default for AmbientSettings {
    fn default() -> Self {
        Self {
            is_playing: false,
            resume_on_startup: false,
            master_volume: default_master_vol(),
            tone_softness: default_tone_softness(),
            duck_during_tts: default_true(),
            duck_amount: default_duck_amount(),
            selected_preset: default_preset(),
            timer_end_unix: None,
            brown_noise_vol: default_brown_vol(),
            pink_noise_vol: 0.0,
            rain_vol: 0.0,
            fireplace_vol: 0.0,
            lofi_vol: 0.0,
            binaural_vol: default_binaural_vol(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct BookProgress {
    pub chapter: usize,
    pub scroll_y: f32,
    pub paragraph: Option<usize>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct RecentBook {
    pub path: PathBuf,
    pub title: String,
    pub author: String,
    pub chapter: usize,
    pub scroll_y: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WindowState {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1050.0,
            height: 750.0,
            maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AutoScrollSettings {
    pub pause_mode: bool,
    pub continuous_speed: f32,
    pub step_distance: f32,
    pub step_duration: f32,
    pub pause_duration: f32,
}

impl Default for AutoScrollSettings {
    fn default() -> Self {
        Self {
            pause_mode: true,
            continuous_speed: 40.0,
            step_distance: 90.0,
            step_duration: 0.8,
            pause_duration: 3.5,
        }
    }
}

fn default_crop_tolerance() -> u8 {
    24
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ComicSettings {
    pub reading_mode: String,
    pub page_layout: String,
    pub fit_mode: String,
    pub zoom_percent: f32,
    #[serde(default)]
    pub smart_dark: bool,
    #[serde(default)]
    pub auto_crop: bool,
    #[serde(default = "default_crop_tolerance")]
    pub auto_crop_tolerance: u8,
}

impl Default for ComicSettings {
    fn default() -> Self {
        Self {
            reading_mode: "Auto".to_string(),
            page_layout: "Single".to_string(),
            fit_mode: "FitWindow".to_string(),
            zoom_percent: 100.0,
            smart_dark: false,
            auto_crop: false,
            auto_crop_tolerance: default_crop_tolerance(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PdfSettings {
    pub layout: String,
    pub fit_mode: String,
    pub zoom_percent: f32,
    pub rotation: i32,
    #[serde(default)]
    pub smart_dark: bool,
    #[serde(default)]
    pub auto_crop: bool,
    #[serde(default = "default_crop_tolerance")]
    pub auto_crop_tolerance: u8,
}

impl Default for PdfSettings {
    fn default() -> Self {
        Self {
            layout: "Single".to_string(),
            fit_mode: "FitWindow".to_string(),
            zoom_percent: 100.0,
            rotation: 0,
            smart_dark: false,
            auto_crop: false,
            auto_crop_tolerance: default_crop_tolerance(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TranslationProvider {
    #[default]
    Google,
    DeepL,
}

impl TranslationProvider {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Google => "Google Translate",
            Self::DeepL => "DeepL",
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct TranslationSettings {
    pub provider: TranslationProvider,
    #[serde(skip)]
    pub deepl_api_key: String,
}

impl std::fmt::Debug for TranslationSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranslationSettings")
            .field("provider", &self.provider)
            .field(
                "deepl_api_key",
                &if self.deepl_api_key.is_empty() {
                    ""
                } else {
                    "<redacted>"
                },
            )
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SavedWord {
    #[serde(default)]
    pub word: String,
    #[serde(default)]
    pub definition: String,
    #[serde(default)]
    pub pack: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub book: String,
    #[serde(default)]
    pub created_at: u64,
}
