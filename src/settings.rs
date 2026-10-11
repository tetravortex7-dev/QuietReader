use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

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
pub struct AppSettings {
    pub window: WindowState,
    pub recent_books: Vec<RecentBook>,
    pub last_opened_book: Option<PathBuf>,
    pub font_size: f32,
    pub theme: String,
    pub keep_screen_on: bool,
    pub show_toc: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            window: WindowState::default(),
            recent_books: Vec::new(),
            last_opened_book: None,
            font_size: 19.0,
            theme: "Sepia".to_string(),
            keep_screen_on: true,
            show_toc: true,
        }
    }
}

impl AppSettings {
    pub fn get_settings_path() -> PathBuf {
        // Portable mode: if quietreader_settings.json exists next to exe, use it
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let local_settings = parent.join("quietreader_settings.json");
                if local_settings.exists() {
                    return local_settings;
                }
            }
        }

        // Standard location: %APPDATA%/QuietReader/settings.json
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let dir = PathBuf::from(app_data).join("QuietReader");
            let _ = std::fs::create_dir_all(&dir);
            return dir.join("settings.json");
        }

        // Fallback: temp or current directory
        PathBuf::from("quietreader_settings.json")
    }

    pub fn load() -> Self {
        let path = Self::get_settings_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
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
        std::fs::write(&path, content)
            .map_err(|e| format!("Ошибка записи файла настроек {}: {e}", path.display()))?;
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
        self.window.x = x;
        self.window.y = y;
        if !maximized && width >= 400.0 && height >= 300.0 {
            self.window.width = width;
            self.window.height = height;
        }
        self.window.maximized = maximized;
    }
}
