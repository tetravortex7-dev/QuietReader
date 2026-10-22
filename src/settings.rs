use crate::opds::{OpdsCatalog, default_opds_catalogs};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const DEEPL_SECRET_KEY: &str = "deepl_api_key";

static LAST_SAVE_ERROR: Mutex<Option<String>> = Mutex::new(None);
static LAST_SECRET_ERROR: Mutex<Option<String>> = Mutex::new(None);
static LAST_SECRET_SNAPSHOT: Mutex<Option<SecretSnapshot>> = Mutex::new(None);

#[derive(Clone, PartialEq, Eq, Default)]
struct SecretSnapshot {
    deepl_api_key: Option<String>,
    opds_passwords: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct BookProgress {
    pub chapter: usize,
    pub scroll_y: f32,
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

fn default_comic_zoom() -> f32 {
    100.0
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

pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 2;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub font_size: f32,
    pub font_family: String,
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
    pub ambient: crate::app::ambient::AmbientSettings,
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
            font_size: 19.0,
            font_family: default_font_family(),
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
            ambient: crate::app::ambient::AmbientSettings::default(),
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

    pub fn last_secret_error() -> Option<String> {
        LAST_SECRET_ERROR
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    fn set_secret_error(error: String) {
        if let Ok(mut guard) = LAST_SECRET_ERROR.lock() {
            *guard = Some(error);
        }
    }

    fn clear_secret_error() {
        if let Ok(mut guard) = LAST_SECRET_ERROR.lock() {
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
        let legacy = Self::read_legacy_secret_entries(&path)
            .or_else(|| Self::read_legacy_secret_entries(&bak_path))
            .unwrap_or_default();
        let mut settings = Self::load_from_path(&path);
        settings.apply_secrets(&legacy);
        if legacy.is_empty() {
            if let Ok(mut guard) = LAST_SECRET_SNAPSHOT.lock() {
                *guard = Some(settings.secret_snapshot());
            }
        } else {
            match settings.store_secrets() {
                Ok(()) => {
                    Self::clear_secret_error();
                    let _ = settings.save_to_path(&path);
                }
                Err(error) => Self::set_secret_error(error),
            }
        }
        Self::sanitize_auxiliary_files(&path);
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

    fn opds_secret_key(catalog_id: &str) -> String {
        format!("opds_password:{catalog_id}")
    }

    fn extract_legacy_secret_entries(content: &str) -> Vec<(String, String)> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        if let Some(key) = value
            .get("translation")
            .and_then(|translation| translation.get("deepl_api_key"))
            .and_then(|key| key.as_str())
            && !key.is_empty()
        {
            entries.push((DEEPL_SECRET_KEY.to_string(), key.to_string()));
        }
        if let Some(catalogs) = value
            .get("opds_catalogs")
            .and_then(|catalogs| catalogs.as_array())
        {
            for catalog in catalogs {
                let Some(id) = catalog.get("id").and_then(|id| id.as_str()) else {
                    continue;
                };
                let Some(password) = catalog
                    .get("password")
                    .and_then(|password| password.as_str())
                else {
                    continue;
                };
                if !id.is_empty() && !password.is_empty() {
                    entries.push((Self::opds_secret_key(id), password.to_string()));
                }
            }
        }
        entries
    }

    fn sanitize_secret_fields(content: &str) -> Option<String> {
        let mut value = match serde_json::from_str::<serde_json::Value>(content) {
            Ok(value) => value,
            Err(_) => return Self::redact_secret_text(content),
        };
        let mut changed = false;
        if let Some(translation) = value
            .get_mut("translation")
            .and_then(|translation| translation.as_object_mut())
            && translation.remove("deepl_api_key").is_some()
        {
            changed = true;
        }
        if let Some(catalogs) = value
            .get_mut("opds_catalogs")
            .and_then(|catalogs| catalogs.as_array_mut())
        {
            for catalog in catalogs {
                if let Some(object) = catalog.as_object_mut()
                    && object.remove("password").is_some()
                {
                    changed = true;
                }
            }
        }
        if !changed {
            return None;
        }
        serde_json::to_string_pretty(&value).ok()
    }

    fn redact_secret_text(content: &str) -> Option<String> {
        let mut result = String::with_capacity(content.len());
        let mut cursor = 0;
        let mut changed = false;

        while let Some(relative) = content[cursor..].find("\"deepl_api_key\"") {
            let key_start = cursor + relative;
            let key_end = key_start + "\"deepl_api_key\"".len();
            let Some((value_start, value_end)) = Self::json_string_value_bounds(content, key_end)
            else {
                cursor = key_end;
                continue;
            };
            result.push_str(&content[cursor..value_start + 1]);
            result.push('"');
            cursor = value_end;
            changed = true;
        }
        result.push_str(&content[cursor..]);

        let mut second = String::with_capacity(result.len());
        let mut cursor = 0;
        while let Some(relative) = result[cursor..].find("\"password\"") {
            let key_start = cursor + relative;
            let key_end = key_start + "\"password\"".len();
            let Some((value_start, value_end)) = Self::json_string_value_bounds(&result, key_end)
            else {
                cursor = key_end;
                continue;
            };
            second.push_str(&result[cursor..value_start + 1]);
            second.push('"');
            cursor = value_end;
            changed = true;
        }
        second.push_str(&result[cursor..]);

        if changed { Some(second) } else { None }
    }

    fn json_string_value_bounds(content: &str, key_end: usize) -> Option<(usize, usize)> {
        let bytes = content.as_bytes();
        let mut index = key_end;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b':') {
            return None;
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes.get(index) != Some(&b'"') {
            return None;
        }
        let value_start = index;
        index += 1;
        let mut escaped = false;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' if !escaped => escaped = true,
                b'"' if !escaped => return Some((value_start, index + 1)),
                _ => escaped = false,
            }
            index += 1;
        }
        None
    }

    fn write_settings_file(path: &Path, content: &str) -> std::io::Result<()> {
        std::fs::write(path, content)?;
        crate::platform::permissions::restrict_file(path);
        Ok(())
    }

    fn write_secret_safe_copy(source: &Path, destination: &Path) -> std::io::Result<()> {
        let content = std::fs::read_to_string(source)?;
        if let Some(sanitized) = Self::sanitize_secret_fields(&content) {
            Self::write_settings_file(destination, &sanitized)
        } else {
            std::fs::copy(source, destination)?;
            crate::platform::permissions::restrict_file(destination);
            Ok(())
        }
    }

    fn sanitize_existing_file(path: &Path) {
        let Ok(content) = std::fs::read_to_string(path) else {
            return;
        };
        let Some(sanitized) = Self::sanitize_secret_fields(&content) else {
            return;
        };
        let tmp_path = path.with_extension(format!("sanitize.{}", std::process::id()));
        if Self::write_settings_file(&tmp_path, &sanitized).is_ok() {
            let _ = std::fs::rename(&tmp_path, path);
        } else {
            let _ = std::fs::remove_file(&tmp_path);
        }
        crate::platform::permissions::restrict_file(path);
    }

    fn sanitize_auxiliary_files(path: &Path) {
        Self::sanitize_existing_file(&Self::make_backup_path(path));
        Self::sanitize_existing_file(&path.with_extension("corrupted"));
    }

    fn read_legacy_secret_entries(path: &Path) -> Option<Vec<(String, String)>> {
        let content = std::fs::read_to_string(path).ok()?;
        let entries = Self::extract_legacy_secret_entries(&content);
        if entries.is_empty() {
            None
        } else {
            Some(entries)
        }
    }

    fn secret_snapshot(&self) -> SecretSnapshot {
        SecretSnapshot {
            deepl_api_key: if self.translation.deepl_api_key.is_empty() {
                None
            } else {
                Some(self.translation.deepl_api_key.clone())
            },
            opds_passwords: self
                .opds_catalogs
                .iter()
                .filter_map(|catalog| {
                    catalog
                        .password
                        .as_ref()
                        .filter(|password| !password.is_empty())
                        .map(|password| (catalog.id.clone(), password.clone()))
                })
                .collect(),
        }
    }

    fn store_secrets(&self) -> Result<(), String> {
        let snapshot = self.secret_snapshot();
        let previous = LAST_SECRET_SNAPSHOT
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if previous.as_ref() == Some(&snapshot) {
            return Ok(());
        }
        let previous = previous.unwrap_or_default();

        if snapshot.deepl_api_key != previous.deepl_api_key {
            match snapshot.deepl_api_key.as_deref() {
                Some(value) => crate::platform::secrets::store(DEEPL_SECRET_KEY, value)?,
                None => crate::platform::secrets::delete(DEEPL_SECRET_KEY)?,
            }
        }

        for (catalog_id, password) in &snapshot.opds_passwords {
            if previous.opds_passwords.get(catalog_id) != Some(password) {
                crate::platform::secrets::store(&Self::opds_secret_key(catalog_id), password)?;
            }
        }

        for catalog_id in previous.opds_passwords.keys() {
            if !snapshot.opds_passwords.contains_key(catalog_id) {
                crate::platform::secrets::delete(&Self::opds_secret_key(catalog_id))?;
            }
        }

        if let Ok(mut guard) = LAST_SECRET_SNAPSHOT.lock() {
            *guard = Some(snapshot);
        }
        Ok(())
    }

    fn apply_secrets(&mut self, legacy: &[(String, String)]) {
        let mut first_error = None;
        let legacy_deepl = legacy
            .iter()
            .find(|(key, _)| key == DEEPL_SECRET_KEY)
            .map(|(_, value)| value.clone());
        match crate::platform::secrets::load(DEEPL_SECRET_KEY) {
            Ok(Some(value)) => self.translation.deepl_api_key = value,
            Ok(None) => {
                if let Some(value) = legacy_deepl {
                    self.translation.deepl_api_key = value;
                }
            }
            Err(error) => {
                first_error = Some(error);
                if let Some(value) = legacy_deepl {
                    self.translation.deepl_api_key = value;
                }
            }
        }

        for catalog in &mut self.opds_catalogs {
            let key = Self::opds_secret_key(&catalog.id);
            let legacy_password = legacy
                .iter()
                .find(|(entry_key, _)| entry_key == &key)
                .map(|(_, value)| value.clone());
            match crate::platform::secrets::load(&key) {
                Ok(Some(value)) => catalog.password = Some(value),
                Ok(None) => {
                    if let Some(value) = legacy_password {
                        catalog.password = Some(value);
                    }
                }
                Err(error) => {
                    first_error = Some(error);
                    if let Some(value) = legacy_password {
                        catalog.password = Some(value);
                    }
                }
            }
        }

        if let Some(error) = first_error {
            Self::set_secret_error(error);
        }
    }

    pub fn migrate_from_file(&mut self, path: &Path) {
        if self.schema_version < CURRENT_SETTINGS_SCHEMA_VERSION {
            let bak_path = Self::make_backup_path(path);
            if path.exists() {
                let _ = Self::write_secret_safe_copy(path, &bak_path);
            }

            for book in &self.recent_books {
                self.reading_progress
                    .entry(book.path.clone())
                    .or_insert(BookProgress {
                        chapter: book.chapter,
                        scroll_y: book.scroll_y,
                        timestamp: book.timestamp,
                    });
            }

            if self.font_family.trim().is_empty() {
                self.font_family = default_font_family();
            }
            self.font_size = self.font_size.clamp(10.0, 48.0);
            self.schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;

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
                            let _ = Self::write_secret_safe_copy(path, &bak_path);
                        }
                        let corrupted_path = path.with_extension("corrupted");
                        let _ = Self::write_secret_safe_copy(path, &corrupted_path);
                    }
                }
            } else {
                let corrupted_path = path.with_extension("corrupted");
                let _ = Self::write_secret_safe_copy(path, &corrupted_path);
            }
        }

        if bak_path.exists()
            && let Ok(bak_content) = std::fs::read_to_string(&bak_path)
            && let Ok(mut bak_settings) = serde_json::from_str::<AppSettings>(&bak_content)
        {
            bak_settings.migrate_from_file(path);
            let _ = bak_settings.save_to_path(path);
            return bak_settings;
        }

        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_settings_path();
        let secret_result = self.store_secrets();
        let save_result = self.save_to_path(&path);
        Self::sanitize_auxiliary_files(&path);
        match secret_result {
            Ok(()) => Self::clear_secret_error(),
            Err(error) => Self::set_secret_error(error),
        }
        save_result
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
        Self::sanitize_auxiliary_files(path);
        if let Ok(mut lock) = LAST_SAVE_ERROR.lock() {
            *lock = None;
        }
        Ok(())
    }

    pub fn record_book_progress(
        &mut self,
        path: &Path,
        title: &str,
        author: &str,
        chapter: usize,
        scroll_y: f32,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.reading_progress.insert(
            path.to_path_buf(),
            BookProgress {
                chapter,
                scroll_y,
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

#[cfg(all(test, feature = "enable_tests"))]
mod test;
