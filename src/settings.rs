use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentBook {
    pub path: PathBuf,
    pub title: String,
    pub author: String,
    pub chapter: usize,
    pub scroll_y: f32,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
pub struct AutoScrollSettings {
    /// Включен ли режим с паузами (true) или плавное непрерывное движение (false)
    pub pause_mode: bool,
    /// Скорость непрерывной прокрутки в пикселях в секунду (от 10.0 до 300.0)
    pub continuous_speed: f32,
    /// Шаг прокрутки при режиме с паузами в пикселях (от 20.0 до 500.0)
    pub step_distance: f32,
    /// Длительность активного движения при шаге в секундах (от 0.2 до 5.0)
    pub step_duration: f32,
    /// Длительность паузы между движениями в секундах (от 0.5 до 30.0)
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComicSettings {
    /// Режим чтения: "Manga" (справа-налево), "Comic" (слева-направо), "Webtoon" (вертикальная непрерывная лента)
    pub reading_mode: String,
    /// Расположение страниц: "Single" (одиночная), "Double" (разворот)
    pub page_layout: String,
    /// Масштабирование: "FitWindow" (по окну), "FitWidth" (по ширине), "FitHeight" (по высоте), "Original" (100%)
    pub fit_mode: String,
}

impl Default for ComicSettings {
    fn default() -> Self {
        Self {
            reading_mode: "Auto".to_string(),
            page_layout: "Single".to_string(),
            fit_mode: "FitWindow".to_string(),
        }
    }
}

pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 1;

fn default_font_family() -> String {
    "Georgia".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
    #[serde(default)]
    pub schema_version: u32,
    pub window: WindowState,
    pub recent_books: Vec<RecentBook>,
    pub last_opened_book: Option<PathBuf>,
    #[serde(default)]
    pub open_tabs: Vec<PathBuf>,
    #[serde(default)]
    pub active_tab_index: usize,
    pub font_size: f32,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    pub theme: String,
    pub keep_screen_on: bool,
    pub show_toc: bool,
    #[serde(default)]
    pub auto_scroll: AutoScrollSettings,
    #[serde(default)]
    pub dismiss_association_prompt: bool,
    #[serde(default)]
    pub comic: ComicSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
            window: WindowState::default(),
            recent_books: Vec::new(),
            last_opened_book: None,
            open_tabs: Vec::new(),
            active_tab_index: 0,
            font_size: 19.0,
            font_family: "Georgia".to_string(),
            theme: "Sepia".to_string(),
            keep_screen_on: true,
            show_toc: true,
            auto_scroll: AutoScrollSettings::default(),
            dismiss_association_prompt: false,
            comic: ComicSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn get_settings_path() -> PathBuf {
        // Portable mode: if quietreader_settings.json exists next to exe, use it
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(parent) = exe_path.parent()
        {
            let local_settings = parent.join("quietreader_settings.json");
            if local_settings.exists() {
                return local_settings;
            }
        }

        // Standard location on Windows: %APPDATA%/QuietReader/settings.json
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let dir = PathBuf::from(app_data).join("QuietReader");
            let _ = std::fs::create_dir_all(&dir);
            return dir.join("settings.json");
        }

        // Standard location on Linux / macOS via XDG_CONFIG_HOME or HOME
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

        // Fallback: current directory
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

    /// Выполняет миграцию настроек со старых версий схемы на актуальную с сохранением резервной копии
    pub fn migrate_from_file(&mut self, path: &Path) {
        if self.schema_version < CURRENT_SETTINGS_SCHEMA_VERSION {
            let bak_path = Self::make_backup_path(path);
            let _ = std::fs::copy(path, &bak_path);

            if self.font_family.trim().is_empty() {
                self.font_family = default_font_family();
            }
            self.font_size = self.font_size.clamp(10.0, 48.0);
            self.schema_version = CURRENT_SETTINGS_SCHEMA_VERSION;

            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, content);
            }
        }
    }

    pub fn load_from_path(path: &Path) -> Self {
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(path)
        {
            match serde_json::from_str::<AppSettings>(&content) {
                Ok(mut settings) => {
                    settings.migrate_from_file(path);
                    return settings;
                }
                Err(_) => {
                    let bak_path = Self::make_backup_path(path);
                    let _ = std::fs::copy(path, &bak_path);
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Ошибка сериализации настроек: {e}"))?;
        let mut tmp_path = path.clone();
        tmp_path.set_extension("json.tmp");
        std::fs::write(&tmp_path, &content).map_err(|e| {
            format!(
                "Ошибка записи временного файла настроек {}: {e}",
                tmp_path.display()
            )
        })?;
        if std::fs::rename(&tmp_path, &path).is_err() {
            std::fs::write(&path, &content)
                .map_err(|e| format!("Ошибка записи файла настроек {}: {e}", path.display()))?;
            let _ = std::fs::remove_file(&tmp_path);
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

        // Remove existing entry for this file path if any
        self.recent_books.retain(|b| b.path != path);

        // Insert at beginning of list
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

        // Keep maximum 25 recent books
        if self.recent_books.len() > 25 {
            self.recent_books.truncate(25);
        }

        self.last_opened_book = Some(path.to_path_buf());
    }

    pub fn get_book_progress(&self, path: &Path) -> Option<(usize, f32)> {
        self.recent_books
            .iter()
            .find(|b| b.path == path)
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
