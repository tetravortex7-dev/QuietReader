use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const CURRENT_LIBRARY_SCHEMA_VERSION: u32 = 3;
pub const MAX_FOLDER_SCAN_DEPTH: usize = 6;
pub const MAX_FOLDER_SCAN_ENTRIES: usize = 2_000;
pub const MAX_FOLDER_SCAN_DIRS: usize = 2_000;
pub const DEFAULT_READING_WPM: u32 = 220;
const SECONDS_PER_DAY: u64 = 86_400;
const CHARS_PER_WORD: usize = 5;
const MAX_CALIBRE_METADATA_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LibraryMode {
    #[default]
    Shelf,
    Folder,
    ReaderOnly,
}

impl LibraryMode {
    pub const ALL: [LibraryMode; 3] = [
        LibraryMode::Shelf,
        LibraryMode::Folder,
        LibraryMode::ReaderOnly,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiDesignMode {
    #[default]
    Cozy,
    Studio,
}

impl UiDesignMode {
    pub const ALL: [UiDesignMode; 2] = [UiDesignMode::Cozy, UiDesignMode::Studio];

    pub fn corner_radius(self) -> u8 {
        match self {
            UiDesignMode::Cozy => 12,
            UiDesignMode::Studio => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShelfViewMode {
    Mini,
    #[default]
    Standard,
    Maxi,
}

impl ShelfViewMode {
    pub const ALL: [ShelfViewMode; 3] = [
        ShelfViewMode::Mini,
        ShelfViewMode::Standard,
        ShelfViewMode::Maxi,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LibraryReadingStatus {
    #[default]
    Unread,
    Reading,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryEntry {
    pub path: PathBuf,
    pub title: String,
    pub author: String,
    pub added_at_unix: u64,
    pub last_opened_unix: u64,
    pub status: LibraryReadingStatus,
    pub current_unit: usize,
    pub total_units: usize,
}

impl Default for LibraryEntry {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            title: String::new(),
            author: String::new(),
            added_at_unix: 0,
            last_opened_unix: 0,
            status: LibraryReadingStatus::Unread,
            current_unit: 0,
            total_units: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryStore {
    pub schema_version: u32,
    pub mode: LibraryMode,
    pub design: UiDesignMode,
    pub shelf_view: ShelfViewMode,
    pub library_folder: Option<PathBuf>,
    pub recent_imported: bool,
    pub entries: Vec<LibraryEntry>,
    #[serde(default)]
    pub daily_reading_seconds: BTreeMap<u64, u64>,
}

impl Default for LibraryStore {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_LIBRARY_SCHEMA_VERSION,
            mode: LibraryMode::Shelf,
            design: UiDesignMode::Cozy,
            shelf_view: ShelfViewMode::Standard,
            library_folder: None,
            recent_imported: false,
            entries: Vec::new(),
            daily_reading_seconds: BTreeMap::new(),
        }
    }
}

impl LibraryStore {
    pub fn path_beside(settings_path: &Path) -> PathBuf {
        settings_path.with_file_name("library.json")
    }

    pub fn entry(&self, path: &Path) -> Option<&LibraryEntry> {
        self.entries.iter().find(|entry| entry.path == path)
    }

    pub fn entry_mut(&mut self, path: &Path) -> Option<&mut LibraryEntry> {
        self.entries.iter_mut().find(|entry| entry.path == path)
    }

    pub fn import_folder_books(&mut self, books: &[FolderBook], now_unix: u64) -> bool {
        let mut changed = false;
        for book in books {
            changed |= self.ensure_book(&book.path, &book.title, &book.author, now_unix);
        }
        changed
    }

    pub fn ensure_book(&mut self, path: &Path, title: &str, author: &str, now_unix: u64) -> bool {
        if let Some(entry) = self.entry_mut(path) {
            let mut changed = false;
            if !title.trim().is_empty() && entry.title != title {
                entry.title = title.to_string();
                changed = true;
            }
            if !author.trim().is_empty() && entry.author != author {
                entry.author = author.to_string();
                changed = true;
            }
            if changed {
                entry.last_opened_unix = now_unix;
            }
            return changed;
        }
        self.entries.push(LibraryEntry {
            path: path.to_path_buf(),
            title: title.to_string(),
            author: author.to_string(),
            added_at_unix: now_unix,
            last_opened_unix: now_unix,
            status: LibraryReadingStatus::Unread,
            current_unit: 0,
            total_units: 0,
        });
        true
    }

    pub fn record_progress(
        &mut self,
        path: &Path,
        title: &str,
        author: &str,
        current_unit: usize,
        total_units: usize,
        now_unix: u64,
    ) -> bool {
        let existed = self.entry(path).is_some();
        let metadata_changed = self.ensure_book(path, title, author, now_unix);
        let Some(entry) = self.entry_mut(path) else {
            return false;
        };
        let progress_changed =
            entry.current_unit != current_unit || entry.total_units != total_units;
        let mut changed = !existed || metadata_changed;
        if entry.current_unit != current_unit {
            entry.current_unit = current_unit;
            changed = true;
        }
        if entry.total_units != total_units {
            entry.total_units = total_units;
            changed = true;
        }
        if progress_changed {
            entry.last_opened_unix = now_unix;
        }
        if entry.status == LibraryReadingStatus::Unread && (current_unit > 0 || total_units > 0) {
            entry.status = LibraryReadingStatus::Reading;
            changed = true;
        }
        changed
    }

    pub fn touch(&mut self, path: &Path, now_unix: u64) -> bool {
        let Some(entry) = self.entry_mut(path) else {
            return false;
        };
        if entry.last_opened_unix == now_unix {
            return false;
        }
        entry.last_opened_unix = now_unix;
        true
    }

    pub fn set_status(&mut self, path: &Path, status: LibraryReadingStatus) -> bool {
        let Some(entry) = self.entry_mut(path) else {
            return false;
        };
        if entry.status == status {
            return false;
        }
        entry.status = status;
        true
    }

    pub fn record_reading_seconds(&mut self, day_index: u64, seconds: u64) -> bool {
        if seconds == 0 {
            return false;
        }
        let total = self.daily_reading_seconds.entry(day_index).or_insert(0);
        let before = *total;
        *total = total.saturating_add(seconds);
        *total != before
    }

    pub fn total_reading_seconds(&self) -> u64 {
        self.daily_reading_seconds
            .values()
            .fold(0u64, |total, seconds| total.saturating_add(*seconds))
    }

    pub fn total_reading_minutes(&self) -> u64 {
        self.total_reading_seconds() / 60
    }

    pub fn completed_books_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.status == LibraryReadingStatus::Finished)
            .count()
    }

    pub fn current_streak_days(&self, today_day_index: u64) -> u32 {
        let today_seconds = self
            .daily_reading_seconds
            .get(&today_day_index)
            .copied()
            .unwrap_or(0);
        let mut day = if today_seconds > 0 {
            Some(today_day_index)
        } else {
            today_day_index
                .checked_sub(1)
                .filter(|day| self.daily_reading_seconds.get(day).copied().unwrap_or(0) > 0)
        };
        let mut streak = 0u32;
        while let Some(current) = day {
            if self
                .daily_reading_seconds
                .get(&current)
                .copied()
                .unwrap_or(0)
                == 0
            {
                break;
            }
            streak = streak.saturating_add(1);
            day = current.checked_sub(1);
        }
        streak
    }

    #[allow(dead_code)]
    pub fn last_7_days_activity(&self, today_day_index: u64) -> Vec<(u64, u64)> {
        (0..7)
            .rev()
            .map(|offset| {
                let day = today_day_index.saturating_sub(offset);
                (
                    day,
                    self.daily_reading_seconds.get(&day).copied().unwrap_or(0),
                )
            })
            .collect()
    }

    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.path != path);
        self.entries.len() != before
    }

    pub fn sorted_entries(&self) -> Vec<&LibraryEntry> {
        let mut entries: Vec<&LibraryEntry> = self.entries.iter().collect();
        entries.sort_by(|a, b| {
            b.last_opened_unix
                .cmp(&a.last_opened_unix)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        entries
    }

    pub fn load_from_path(path: &Path) -> Self {
        let backup_path = backup_path(path);
        if let Some(store) = read_store(path) {
            return store.migrated();
        }

        if path.exists() {
            let corrupted_path = path.with_extension("corrupted");
            if !corrupted_path.exists() {
                let _ = fs::copy(path, &corrupted_path);
            }
        }

        if let Some(store) = read_store(&backup_path) {
            let store = store.migrated();
            let _ = write_store(path, &store, false);
            return store;
        }

        Self::default()
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        write_store(path, self, true)
    }

    fn migrated(mut self) -> Self {
        if self.schema_version < CURRENT_LIBRARY_SCHEMA_VERSION {
            self.schema_version = CURRENT_LIBRARY_SCHEMA_VERSION;
        }
        self
    }
}

pub fn unix_day_index(unix_seconds: u64) -> u64 {
    unix_seconds / SECONDS_PER_DAY
}

pub fn estimate_minutes_left_in_chapter(
    word_count_or_chars: usize,
    scroll_progress: f32,
    wpm: u32,
) -> u32 {
    if word_count_or_chars == 0 {
        return 0;
    }
    let progress = if scroll_progress.is_finite() {
        scroll_progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let remaining_chars = (word_count_or_chars as f64 * (1.0 - progress as f64)).ceil() as usize;
    if remaining_chars == 0 {
        return 0;
    }
    let words = remaining_chars.div_ceil(CHARS_PER_WORD);
    let effective_wpm = if wpm == 0 { DEFAULT_READING_WPM } else { wpm };
    let minutes = (words as u64).div_ceil(effective_wpm as u64);
    minutes.min(u32::MAX as u64) as u32
}

fn backup_path(path: &Path) -> PathBuf {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!("{value}.bak"))
        .unwrap_or_else(|| "bak".to_string());
    let mut backup = path.to_path_buf();
    backup.set_extension(ext);
    backup
}

fn read_store(path: &Path) -> Option<LibraryStore> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_store(path: &Path, store: &LibraryStore, backup_existing: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create library directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let content = serde_json::to_string_pretty(store)
        .map_err(|error| format!("failed to serialize library: {error}"))?;
    let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));

    let write_result = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!(
            "failed to write temporary library file {}: {error}",
            tmp_path.display()
        ));
    }

    if backup_existing && path.exists() {
        let backup_path = backup_path(path);
        if let Err(error) = fs::copy(path, &backup_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "failed to back up library {}: {error}",
                path.display()
            ));
        }
    }

    let mut rename_error = None;
    for _ in 0..5 {
        match fs::rename(&tmp_path, path) {
            Ok(()) => {
                rename_error = None;
                break;
            }
            Err(error) => {
                rename_error = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }
    }

    if let Some(error) = rename_error {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!(
            "failed to save library {}: {error}",
            path.display()
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderBook {
    pub path: PathBuf,
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalibreMetadata {
    pub title: String,
    pub author: String,
}

pub fn read_calibre_metadata(book_path: &Path) -> Option<CalibreMetadata> {
    let directory = book_path.parent()?;
    let metadata_path = find_calibre_metadata_path(directory)?;
    let metadata = fs::metadata(&metadata_path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_CALIBRE_METADATA_BYTES {
        return None;
    }
    let bytes = fs::read(&metadata_path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    parse_calibre_metadata(&text)
}

fn find_calibre_metadata_path(directory: &Path) -> Option<PathBuf> {
    let direct = directory.join("metadata.opf");
    if direct.is_file() {
        return Some(direct);
    }
    let entries = fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("metadata.opf")
        {
            return Some(entry.path());
        }
    }
    None
}

fn parse_calibre_metadata(xml: &str) -> Option<CalibreMetadata> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut title = String::new();
    let mut creators = Vec::new();
    let mut current_creator = String::new();
    let mut in_title = false;
    let mut in_creator = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref event)) => {
                let name = event.local_name();
                let name = name.as_ref();
                if name.eq_ignore_ascii_case("title") {
                    in_title = true;
                } else if name.eq_ignore_ascii_case("creator") {
                    in_creator = true;
                    current_creator.clear();
                }
            }
            Ok(Event::Text(ref event)) => {
                let raw = event.as_ref();
                let text = quick_xml::escape::unescape(raw)
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                if in_title {
                    title.push_str(&text);
                } else if in_creator {
                    current_creator.push_str(&text);
                }
            }
            Ok(Event::CData(ref event)) => {
                let text = event.as_ref();
                if in_title {
                    title.push_str(text);
                } else if in_creator {
                    current_creator.push_str(text);
                }
            }
            Ok(Event::GeneralRef(ref event)) => {
                let code = event.as_ref();
                let replacement = match code {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    value if value.starts_with("#x") || value.starts_with("#X") => {
                        u32::from_str_radix(&value[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                    }
                    value if value.starts_with('#') => {
                        value[1..].parse::<u32>().ok().and_then(char::from_u32)
                    }
                    _ => None,
                };
                if let Some(character) = replacement {
                    if in_title {
                        title.push(character);
                    } else if in_creator {
                        current_creator.push(character);
                    }
                }
            }
            Ok(Event::End(ref event)) => {
                let name = event.local_name();
                let name = name.as_ref();
                if name.eq_ignore_ascii_case("title") {
                    in_title = false;
                } else if name.eq_ignore_ascii_case("creator") {
                    in_creator = false;
                    let creator = current_creator.trim();
                    if !creator.is_empty() {
                        creators.push(creator.to_string());
                    }
                    current_creator.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buffer.clear();
    }

    let title = title.trim();
    if title.is_empty() {
        return None;
    }
    Some(CalibreMetadata {
        title: title.to_string(),
        author: creators.join(", "),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderScan {
    pub books: Vec<FolderBook>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderScanError {
    NotDirectory,
    Io(String),
}

pub fn scan_folder<F>(
    root: &Path,
    max_depth: usize,
    max_entries: usize,
    is_supported: F,
) -> Result<FolderScan, FolderScanError>
where
    F: Fn(&Path) -> bool,
{
    let metadata =
        fs::symlink_metadata(root).map_err(|error| FolderScanError::Io(error.to_string()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(FolderScanError::NotDirectory);
    }

    let mut books = Vec::new();
    let mut truncated = false;
    let mut directories_seen = 0usize;
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut calibre_metadata: HashMap<PathBuf, Option<CalibreMetadata>> = HashMap::new();

    while let Some((directory, depth)) = stack.pop() {
        if books.len() >= max_entries || directories_seen >= MAX_FOLDER_SCAN_DIRS {
            truncated = true;
            break;
        }

        let read_dir = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                if directory == root {
                    return Err(FolderScanError::Io(error.to_string()));
                }
                continue;
            }
        };

        for entry in read_dir.flatten() {
            if books.len() >= max_entries || directories_seen >= MAX_FOLDER_SCAN_DIRS {
                truncated = true;
                break;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "__MACOSX" {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }

            let path = entry.path();
            if file_type.is_dir() {
                if depth < max_depth {
                    directories_seen += 1;
                    stack.push((path, depth + 1));
                } else {
                    truncated = true;
                }
                continue;
            }

            if !file_type.is_file() || !is_supported(&path) {
                continue;
            }

            let metadata = path.parent().and_then(|parent| {
                calibre_metadata
                    .entry(parent.to_path_buf())
                    .or_insert_with(|| read_calibre_metadata(&path))
                    .clone()
            });
            let (title, author) = match metadata {
                Some(metadata) => (metadata.title, metadata.author),
                None => (folder_book_title(&path), String::new()),
            };
            books.push(FolderBook {
                title,
                author,
                path,
            });
        }
    }

    books.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(FolderScan { books, truncated })
}

fn folder_book_title(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".fb2.zip") {
        return name[..name.len() - 8].to_string();
    }
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name)
        .to_string()
}
