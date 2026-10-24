pub use crate::settings_config::{
    AmbientSettings, AutoScrollSettings, BookProgress, ComicSettings, OpdsCatalog, PdfSettings,
    RecentBook, SavedWord, TranslationProvider, TranslationSettings, TtsSettings, WindowState,
    default_opds_catalogs,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const OPEN_DYSLEXIC_FONT: &str = "OpenDyslexic";

static LAST_SAVE_ERROR: Mutex<Option<String>> = Mutex::new(None);

pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 3;

fn default_schema_version() -> u32 {
    0
}

fn default_font_family() -> String {
    "Georgia".to_string()
}

fn default_associated_extensions() -> Vec<String> {
    vec!["fb2".to_string(), "epub".to_string()]
}

fn default_ui_language() -> crate::i18n::UiLanguage {
    crate::i18n::UiLanguage::Auto
}

fn default_check_updates_on_startup() -> bool {
    true
}

fn default_reading_ruler_lines() -> u32 {
    3
}

fn default_reading_ruler_opacity() -> f32 {
    0.45
}

fn default_text_encoding() -> String {
    "auto".to_string()
}

fn default_line_height_scale() -> f32 {
    1.45
}

fn default_max_column_width() -> f32 {
    760.0
}

fn default_reading_brightness() -> f32 {
    1.0
}

fn default_trackpad_swipe_pages() -> bool {
    true
}

fn default_text_paged_scroll() -> bool {
    false
}

fn default_text_two_columns() -> bool {
    false
}

fn default_rsvp_wpm() -> u32 {
    300
}

fn default_library_pin_hash() -> Option<String> {
    None
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppSettings {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub window: WindowState,
    pub recent_books: Vec<RecentBook>,
    pub reading_progress: HashMap<PathBuf, BookProgress>,
    pub last_opened_book: Option<PathBuf>,
    pub open_tabs: Vec<PathBuf>,
    pub active_tab_index: usize,
    #[serde(default = "default_text_encoding")]
    pub text_encoding: String,
    pub font_size: f32,
    pub font_family: String,
    #[serde(default = "default_line_height_scale")]
    pub line_height_scale: f32,
    #[serde(default = "default_max_column_width")]
    pub max_column_width: f32,
    #[serde(default = "default_reading_brightness")]
    pub reading_brightness: f32,
    #[serde(default = "default_trackpad_swipe_pages")]
    pub trackpad_swipe_pages: bool,
    #[serde(default = "default_text_paged_scroll")]
    pub text_paged_scroll: bool,
    #[serde(default = "default_text_two_columns")]
    pub text_two_columns: bool,
    #[serde(default)]
    pub text_paragraph_mode: bool,
    #[serde(default = "default_rsvp_wpm")]
    pub rsvp_wpm: u32,
    #[serde(default = "default_library_pin_hash")]
    pub library_pin_hash: Option<String>,
    #[serde(default)]
    pub saved_words: Vec<SavedWord>,
    pub theme: String,
    pub keep_screen_on: bool,
    pub show_toc: bool,
    pub auto_scroll: AutoScrollSettings,
    pub dismiss_association_prompt: bool,
    #[serde(default = "default_associated_extensions")]
    pub associated_extensions: Vec<String>,
    pub comic: ComicSettings,
    pub pdf: PdfSettings,
    #[serde(default = "default_ui_language")]
    pub ui_language: crate::i18n::UiLanguage,
    #[serde(default = "default_check_updates_on_startup")]
    pub check_updates_on_startup: bool,
    #[serde(default)]
    pub bionic_reading: bool,
    #[serde(default)]
    pub reading_ruler: bool,
    #[serde(default = "default_reading_ruler_lines")]
    pub reading_ruler_lines: u32,
    #[serde(default = "default_reading_ruler_opacity")]
    pub reading_ruler_opacity: f32,
    #[serde(default = "default_opds_catalogs")]
    pub opds_catalogs: Vec<OpdsCatalog>,
    #[serde(default)]
    pub translation: TranslationSettings,
    #[serde(default)]
    pub bookmarks: Vec<crate::core::Bookmark>,
    #[serde(default)]
    pub ambient: AmbientSettings,
    #[serde(default)]
    pub tts: TtsSettings,
    #[serde(default)]
    pub trial_started_at_unix: Option<u64>,
    #[serde(skip)]
    pub license_key: String,
}

impl std::fmt::Debug for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "license_key".to_string(),
                serde_json::Value::String(if self.license_key.is_empty() {
                    String::new()
                } else {
                    "<redacted>".to_string()
                }),
            );
        }
        f.debug_struct("AppSettings").field("data", &value).finish()
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
            window: WindowState::default(),
            recent_books: Vec::new(),
            reading_progress: HashMap::new(),
            last_opened_book: None,
            open_tabs: Vec::new(),
            active_tab_index: 0,
            text_encoding: default_text_encoding(),
            font_size: 19.0,
            font_family: default_font_family(),
            line_height_scale: default_line_height_scale(),
            max_column_width: default_max_column_width(),
            reading_brightness: default_reading_brightness(),
            trackpad_swipe_pages: default_trackpad_swipe_pages(),
            text_paged_scroll: default_text_paged_scroll(),
            text_two_columns: default_text_two_columns(),
            text_paragraph_mode: false,
            rsvp_wpm: default_rsvp_wpm(),
            library_pin_hash: default_library_pin_hash(),
            saved_words: Vec::new(),
            theme: "Sepia".to_string(),
            keep_screen_on: true,
            show_toc: true,
            auto_scroll: AutoScrollSettings::default(),
            dismiss_association_prompt: false,
            associated_extensions: default_associated_extensions(),
            comic: ComicSettings::default(),
            pdf: PdfSettings::default(),
            ui_language: default_ui_language(),
            check_updates_on_startup: true,
            bionic_reading: false,
            reading_ruler: false,
            reading_ruler_lines: 3,
            reading_ruler_opacity: 0.45,
            opds_catalogs: default_opds_catalogs(),
            translation: TranslationSettings::default(),
            bookmarks: Vec::new(),
            ambient: AmbientSettings::default(),
            tts: TtsSettings::default(),
            trial_started_at_unix: None,
            license_key: String::new(),
        }
    }
}

pub(crate) fn fallback_settings_path(base: PathBuf) -> PathBuf {
    let dir = base.join("QuietReader");
    crate::platform::permissions::create_private_dir(&dir);
    dir.join("settings.json")
}

impl AppSettings {
    pub fn last_save_error() -> Option<String> {
        LAST_SAVE_ERROR.lock().ok().and_then(|guard| guard.clone())
    }

    pub fn clear_save_error() {
        if let Ok(mut guard) = LAST_SAVE_ERROR.lock() {
            *guard = None;
        }
    }

    pub fn get_settings_path() -> PathBuf {
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(parent) = exe_path.parent()
        {
            let local_settings = parent.join("quietreader_settings.json");
            if local_settings.exists() {
                return local_settings;
            }
        }

        if let Some(app_data) = std::env::var_os("APPDATA") {
            let dir = PathBuf::from(app_data).join("QuietReader");
            crate::platform::permissions::create_private_dir(&dir);
            return dir.join("settings.json");
        }

        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            let dir = PathBuf::from(xdg).join("QuietReader");
            crate::platform::permissions::create_private_dir(&dir);
            return dir.join("settings.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            let dir = if cfg!(target_os = "macos") {
                PathBuf::from(home).join("Library/Application Support/QuietReader")
            } else {
                PathBuf::from(home).join(".config/QuietReader")
            };
            crate::platform::permissions::create_private_dir(&dir);
            return dir.join("settings.json");
        }

        fallback_settings_path(std::env::temp_dir())
    }

    pub fn load() -> Self {
        Self::clear_secret_error();
        let path = Self::get_settings_path();
        let bak_path = Self::make_backup_path(&path);
        let legacy = secrets::read_legacy_secret_entries(&path)
            .or_else(|| secrets::read_legacy_secret_entries(&bak_path))
            .unwrap_or_default();
        let mut settings = Self::load_from_path(&path);
        secrets::initialize(&mut settings, &path, legacy);
        settings
    }

    fn make_backup_path(path: &Path) -> PathBuf {
        let mut bak_path = path.to_path_buf();
        let ext = bak_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}.bak"))
            .unwrap_or_else(|| "bak".to_string());
        bak_path.set_extension(ext);
        bak_path
    }

    pub fn migrate_from_file(&mut self, path: &Path) {
        if self.schema_version < CURRENT_SETTINGS_SCHEMA_VERSION {
            let bak_path = Self::make_backup_path(path);
            if path.exists() {
                let _ = secrets::write_secret_safe_copy(path, &bak_path);
            }

            self.apply_migrations();
            let _ = self.save_to_path(path);
        } else {
            let mut need_save = false;
            if self.reading_progress.is_empty() && !self.recent_books.is_empty() {
                for book in &self.recent_books {
                    self.reading_progress
                        .entry(book.path.clone())
                        .or_insert(BookProgress {
                            chapter: book.chapter,
                            scroll_y: book.scroll_y,
                            paragraph: None,
                            timestamp: book.timestamp,
                        });
                }
                need_save = true;
            }
            if need_save {
                let _ = self.save_to_path(path);
            }
        }
    }

    fn apply_migrations(&mut self) {
        for book in &self.recent_books {
            self.reading_progress
                .entry(book.path.clone())
                .or_insert(BookProgress {
                    chapter: book.chapter,
                    scroll_y: book.scroll_y,
                    paragraph: None,
                    timestamp: book.timestamp,
                });
        }
        if self.font_family.trim().is_empty() {
            self.font_family = default_font_family();
        }
        self.font_size = self.font_size.clamp(10.0, 48.0);
        self.line_height_scale = self.line_height_scale.clamp(1.1, 2.0);
        self.max_column_width = self.max_column_width.clamp(520.0, 1200.0);
        self.rsvp_wpm = self.rsvp_wpm.clamp(100, 1000);
        self.schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;
    }

    pub fn load_from_path(path: &Path) -> Self {
        let bak_path = Self::make_backup_path(path);

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                match serde_json::from_str::<AppSettings>(&content) {
                    Ok(mut settings) => {
                        crate::platform::permissions::restrict_file(path);
                        settings.migrate_from_file(path);
                        return settings;
                    }
                    Err(_) => {
                        if !bak_path.exists() {
                            let _ = secrets::write_secret_safe_copy(path, &bak_path);
                        }
                        let corrupted_path = path.with_extension("corrupted");
                        let _ = secrets::write_secret_safe_copy(path, &corrupted_path);
                    }
                }
            } else {
                let corrupted_path = path.with_extension("corrupted");
                let _ = secrets::write_secret_safe_copy(path, &corrupted_path);
            }
        }

        if bak_path.exists()
            && let Ok(bak_content) = std::fs::read_to_string(&bak_path)
            && let Ok(mut bak_settings) = serde_json::from_str::<AppSettings>(&bak_content)
        {
            bak_settings.apply_migrations();
            let _ = bak_settings.save_to_path(path);
            return bak_settings;
        }

        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_settings_path();
        let secret_result = self.store_secrets();
        self.save_with_secret_result(&path, secret_result)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            let err = format!("failed to serialize settings: {e}");
            if let Ok(mut lock) = LAST_SAVE_ERROR.lock() {
                *lock = Some(err.clone());
            }
            err
        })?;

        let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));

        let write_res = (|| -> std::io::Result<()> {
            use std::io::Write;
            let mut file = std::fs::File::create(&tmp_path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            drop(file);
            crate::platform::permissions::restrict_file(&tmp_path);
            Ok(())
        })();

        if let Err(e) = write_res {
            let _ = std::fs::remove_file(&tmp_path);
            let err = format!("failed to write temporary settings file: {e}");
            if let Ok(mut lock) = LAST_SAVE_ERROR.lock() {
                *lock = Some(err.clone());
            }
            return Err(err);
        }

        let mut rename_err = None;
        for _ in 0..5 {
            match std::fs::rename(&tmp_path, path) {
                Ok(()) => {
                    rename_err = None;
                    break;
                }
                Err(e) => {
                    rename_err = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }
            }
        }

        if let Some(e) = rename_err {
            let _ = std::fs::remove_file(&tmp_path);
            let err = format!("failed to save settings {}: {e}", path.display());
            if let Ok(mut lock) = LAST_SAVE_ERROR.lock() {
                *lock = Some(err.clone());
            }
            return Err(err);
        }

        crate::platform::permissions::restrict_file(path);
        secrets::sanitize_auxiliary_files(path);
        if let Ok(mut lock) = LAST_SAVE_ERROR.lock() {
            *lock = None;
        }
        Ok(())
    }

    pub fn license_status_at(&self, now_unix: u64) -> crate::core::license::LicenseStatus {
        crate::core::license::LicenseStatus::from_stored(
            Some(&self.license_key),
            self.trial_started_at_unix,
            now_unix,
        )
    }

    pub fn license_status(&self) -> crate::core::license::LicenseStatus {
        self.license_status_at(crate::core::license::unix_now())
    }

    pub fn can_use(&self, feature: crate::core::license::ProFeature) -> bool {
        self.license_status().can_use(feature)
    }

    pub fn start_trial(&mut self, now_unix: u64) -> Result<(), crate::core::license::TrialError> {
        if self.license_status_at(now_unix).is_pro_active() {
            return Err(crate::core::license::TrialError::AlreadyStarted);
        }
        crate::core::license::start_trial(&mut self.trial_started_at_unix, now_unix)
    }

    pub fn activate_license(
        &mut self,
        key: &str,
    ) -> Result<crate::core::license::LicenseClaims, crate::core::license::LicenseError> {
        let claims = crate::core::license::verify_license_key(key)?;
        self.license_key = key.trim().to_string();
        Ok(claims)
    }

    pub fn effective_bionic_reading(&self) -> bool {
        self.bionic_reading && self.can_use(crate::core::license::ProFeature::FocusTools)
    }

    pub fn has_library_pin(&self) -> bool {
        self.library_pin_hash
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    }

    pub fn set_library_pin(&mut self, pin: &str) {
        if pin.is_empty() {
            self.library_pin_hash = None;
            return;
        }
        let salt = format!("{:016x}{:016x}", salt_seed(), salt_seed());
        self.library_pin_hash = Some(format!("{salt}:{}", hash_pin(&salt, pin)));
    }

    pub fn clear_library_pin(&mut self) {
        self.library_pin_hash = None;
    }

    pub fn verify_library_pin(&self, pin: &str) -> bool {
        let Some(stored) = self.library_pin_hash.as_deref() else {
            return true;
        };
        let Some((salt, expected)) = stored.split_once(':') else {
            return false;
        };
        hash_pin(salt, pin).eq_ignore_ascii_case(expected)
    }

    pub fn save_word(&mut self, word: SavedWord) -> bool {
        let normalized = word.word.trim().to_lowercase();
        if normalized.is_empty() {
            return false;
        }
        if self.saved_words.iter().any(|existing| {
            existing.word.trim().to_lowercase() == normalized
                && existing.pack.eq_ignore_ascii_case(&word.pack)
        }) {
            return false;
        }
        self.saved_words.push(SavedWord {
            word: word.word.trim().to_string(),
            ..word
        });
        true
    }

    pub fn effective_reading_ruler(&self) -> bool {
        self.reading_ruler && self.can_use(crate::core::license::ProFeature::FocusTools)
    }

    pub fn effective_pdf_auto_crop(&self) -> bool {
        self.pdf.auto_crop && self.can_use(crate::core::license::ProFeature::SmartScanProcessing)
    }

    pub fn effective_font_family(&self) -> String {
        if self.font_family == OPEN_DYSLEXIC_FONT
            && !self.can_use(crate::core::license::ProFeature::FocusTools)
        {
            return default_font_family();
        }
        self.font_family.clone()
    }

    pub fn effective_line_height_scale(&self) -> f32 {
        self.line_height_scale.clamp(1.1, 2.0)
    }

    pub fn effective_max_column_width(&self) -> f32 {
        self.max_column_width.clamp(520.0, 1200.0)
    }

    pub fn text_encoding_override(&self) -> Option<&str> {
        if self.text_encoding == "auto" || self.text_encoding.trim().is_empty() {
            None
        } else {
            Some(self.text_encoding.as_str())
        }
    }

    pub fn record_book_progress(
        &mut self,
        path: &Path,
        title: &str,
        author: &str,
        chapter: usize,
        scroll_y: f32,
    ) {
        self.record_book_progress_anchored(path, title, author, chapter, scroll_y, None);
    }

    pub fn record_book_progress_anchored(
        &mut self,
        path: &Path,
        title: &str,
        author: &str,
        chapter: usize,
        scroll_y: f32,
        paragraph: Option<usize>,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let existing_paragraph = self
            .reading_progress
            .get(path)
            .filter(|progress| progress.chapter == chapter)
            .and_then(|progress| progress.paragraph);
        self.reading_progress.insert(
            path.to_path_buf(),
            BookProgress {
                chapter,
                scroll_y,
                paragraph: paragraph.or(existing_paragraph),
                timestamp: now,
            },
        );

        if self.reading_progress.len() > 10_000 {
            let mut entries: Vec<_> = self
                .reading_progress
                .iter()
                .map(|(p, b)| (p.clone(), b.timestamp))
                .collect();
            entries.sort_by_key(|&(_, t)| t);
            for (old_path, _) in entries.into_iter().take(2_000) {
                self.reading_progress.remove(&old_path);
            }
        }

        self.recent_books.retain(|b| b.path != path);
        self.recent_books.insert(
            0,
            RecentBook {
                path: path.to_path_buf(),
                title: if title.is_empty() {
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                } else {
                    title.to_string()
                },
                author: author.to_string(),
                chapter,
                scroll_y,
                timestamp: now,
            },
        );

        if self.recent_books.len() > 25 {
            self.recent_books.truncate(25);
        }

        self.last_opened_book = Some(path.to_path_buf());
    }

    pub fn get_book_progress(&self, path: &Path) -> Option<(usize, f32)> {
        if let Some(prog) = self.reading_progress.get(path) {
            return Some((prog.chapter, prog.scroll_y));
        }
        let path_str = path.to_string_lossy();
        for (p, prog) in &self.reading_progress {
            if p.to_string_lossy().eq_ignore_ascii_case(&path_str) {
                return Some((prog.chapter, prog.scroll_y));
            }
        }
        self.recent_books
            .iter()
            .find(|b| b.path == path || b.path.to_string_lossy().eq_ignore_ascii_case(&path_str))
            .map(|b| (b.chapter, b.scroll_y))
    }

    pub fn get_book_paragraph(&self, path: &Path) -> Option<usize> {
        if let Some(progress) = self.reading_progress.get(path) {
            return progress.paragraph;
        }
        let path_str = path.to_string_lossy();
        self.reading_progress
            .iter()
            .find(|(candidate, _)| candidate.to_string_lossy().eq_ignore_ascii_case(&path_str))
            .and_then(|(_, progress)| progress.paragraph)
    }

    pub fn update_window_state(
        &mut self,
        x: Option<f32>,
        y: Option<f32>,
        width: f32,
        height: f32,
        maximized: bool,
    ) {
        if !maximized {
            if let (Some(px), Some(py)) = (x, y)
                && px.is_finite()
                && py.is_finite()
                && px > -15000.0
                && py > -15000.0
            {
                self.window.x = Some(px);
                self.window.y = Some(py);
            }
            if width.is_finite() && height.is_finite() && width >= 400.0 && height >= 300.0 {
                self.window.width = width;
                self.window.height = height;
            }
        }
        self.window.maximized = maximized;
    }
}

mod secrets;

#[cfg(all(test, feature = "enable_tests"))]
mod test;

fn salt_seed() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    hasher.write_u128(nanos);
    hasher.finish()
}

fn hash_pin(salt: &str, pin: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(pin.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}
