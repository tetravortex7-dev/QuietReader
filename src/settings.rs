use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static LAST_SAVE_ERROR: Mutex<Option<String>> = Mutex::new(None);

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ComicSettings {
    pub reading_mode: String,
    pub page_layout: String,
    pub fit_mode: String,
    pub zoom_percent: f32,
}

impl Default for ComicSettings {
    fn default() -> Self {
        Self {
            reading_mode: "Auto".to_string(),
            page_layout: "Single".to_string(),
            fit_mode: "FitWindow".to_string(),
            zoom_percent: 100.0,
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
}

impl Default for PdfSettings {
    fn default() -> Self {
        Self {
            layout: "Single".to_string(),
            fit_mode: "FitWindow".to_string(),
            zoom_percent: 100.0,
            rotation: 0,
        }
    }
}

pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 2;

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
    crate::i18n::UiLanguage::Ru
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
        }
    }
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
            let _ = std::fs::create_dir_all(&dir);
            return dir.join("settings.json");
        }

        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            let dir = PathBuf::from(xdg).join("QuietReader");
            let _ = std::fs::create_dir_all(&dir);
            return dir.join("settings.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            let dir = if cfg!(target_os = "macos") {
                PathBuf::from(home).join("Library/Application Support/QuietReader")
            } else {
                PathBuf::from(home).join(".config/QuietReader")
            };
            let _ = std::fs::create_dir_all(&dir);
            return dir.join("settings.json");
        }

        PathBuf::from("quietreader_settings.json")
    }

    pub fn load() -> Self {
        let path = Self::get_settings_path();
        Self::load_from_path(&path)
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
                let _ = std::fs::copy(path, &bak_path);
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
                        settings.migrate_from_file(path);
                        return settings;
                    }
                    Err(_) => {
                        if !bak_path.exists() {
                            let _ = std::fs::copy(path, &bak_path);
                        }
                        let corrupted_path = path.with_extension("corrupted");
                        let _ = std::fs::copy(path, &corrupted_path);
                    }
                }
            } else {
                let corrupted_path = path.with_extension("corrupted");
                let _ = std::fs::copy(path, &corrupted_path);
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
        self.save_to_path(&path)
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            let err = format!("Ошибка сериализации настроек: {e}");
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
            Ok(())
        })();

        if let Err(e) = write_res {
            let _ = std::fs::remove_file(&tmp_path);
            let err = format!("Ошибка записи временного файла настроек: {e}");
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
            let err = format!("Ошибка сохранения настроек {}: {e}", path.display());
            if let Ok(mut lock) = LAST_SAVE_ERROR.lock() {
                *lock = Some(err.clone());
            }
            return Err(err);
        }

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
                        .unwrap_or_else(|| "Без названия".to_string())
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
