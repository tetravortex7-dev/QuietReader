use super::icons::{Icon, IconButton, IconLabel, icon_button};
use super::theme::ReaderTheme;
use crate::core::library::{
    FolderBook, FolderScan, FolderScanError, LibraryEntry, LibraryMode, LibraryReadingStatus,
    LibraryStore, MAX_FOLDER_SCAN_DEPTH, MAX_FOLDER_SCAN_ENTRIES, UiDesignMode, scan_folder,
    unix_day_index,
};
use crate::document::DecodedImage;
use crate::i18n::{Translations, UiLanguage};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, StrokeKind, TextureHandle, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

mod shelf;

const MAX_COVER_WORKERS: usize = 3;
const MAX_COVER_TEXTURES: usize = 256;
const MAX_COVER_BYTES: usize = 128 * 1024 * 1024;

pub(crate) fn cover_cache_needs_eviction(count: usize, total: usize, incoming: usize) -> bool {
    count >= MAX_COVER_TEXTURES || total.saturating_add(incoming) > MAX_COVER_BYTES
}

struct CoverTextureEntry {
    texture: TextureHandle,
    last_access: u64,
    byte_size: usize,
}

struct CoverLoader {
    cache_dir: PathBuf,
    textures: HashMap<PathBuf, CoverTextureEntry>,
    total_bytes: usize,
    access_counter: u64,
    missing: HashSet<PathBuf>,
    pending: HashSet<PathBuf>,
    sender: Sender<(PathBuf, Option<DecodedImage>)>,
    receiver: Receiver<(PathBuf, Option<DecodedImage>)>,
}

impl Default for CoverLoader {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            cache_dir: PathBuf::new(),
            textures: HashMap::new(),
            total_bytes: 0,
            access_counter: 0,
            missing: HashSet::new(),
            pending: HashSet::new(),
            sender,
            receiver,
        }
    }
}

impl CoverLoader {
    fn set_cache_dir(&mut self, cache_dir: PathBuf) {
        self.cache_dir = cache_dir;
    }

    fn queue(&mut self, path: &Path) {
        if self.cache_dir.as_os_str().is_empty()
            || self.pending.len() >= MAX_COVER_WORKERS
            || self.textures.contains_key(path)
            || self.missing.contains(path)
            || self.pending.contains(path)
        {
            return;
        }
        self.pending.insert(path.to_path_buf());
        let sender = self.sender.clone();
        let book_path = path.to_path_buf();
        let cache_dir = self.cache_dir.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::document::cover::load_or_create_cover_thumbnail(&cache_dir, &book_path)
            }))
            .ok()
            .flatten();
            let _ = sender.send((book_path, result));
        });
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((path, image)) = self.receiver.try_recv() {
            self.pending.remove(&path);
            let Some(image) = image else {
                self.missing.insert(path);
                continue;
            };
            let expected = (image.width as usize)
                .checked_mul(image.height as usize)
                .and_then(|value| value.checked_mul(4));
            if image.width == 0 || image.height == 0 || expected != Some(image.rgba.len()) {
                self.missing.insert(path);
                continue;
            }
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [image.width as usize, image.height as usize],
                &image.rgba,
            );
            let texture = ctx.load_texture(
                format!("library-cover:{}", path.display()),
                color,
                egui::TextureOptions::LINEAR,
            );
            let byte_size = super::rgba_byte_size(image.width, image.height);
            if let Some(old) = self.textures.remove(&path) {
                self.total_bytes = self.total_bytes.saturating_sub(old.byte_size);
            }
            while cover_cache_needs_eviction(self.textures.len(), self.total_bytes, byte_size) {
                let Some((oldest_key, _)) = self
                    .textures
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_access)
                else {
                    break;
                };
                let oldest_key = oldest_key.clone();
                if let Some(old) = self.textures.remove(&oldest_key) {
                    self.total_bytes = self.total_bytes.saturating_sub(old.byte_size);
                }
            }
            self.access_counter = self.access_counter.wrapping_add(1);
            let last_access = self.access_counter;
            self.textures.insert(
                path,
                CoverTextureEntry {
                    texture,
                    last_access,
                    byte_size,
                },
            );
            self.total_bytes = self.total_bytes.saturating_add(byte_size);
        }
    }

    fn texture(&mut self, path: &Path) -> Option<TextureHandle> {
        self.access_counter = self.access_counter.wrapping_add(1);
        let last_access = self.access_counter;
        self.textures.get_mut(path).map(|entry| {
            entry.last_access = last_access;
            entry.texture.clone()
        })
    }

    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    #[default]
    All,
    Reading,
    Finished,
}

#[derive(Default)]
pub struct LibraryActions {
    pub open_file: bool,
    pub open_folder: bool,
    pub pick_library_folder: bool,
    pub open_path: Option<PathBuf>,
    pub save_library: bool,
}

pub struct LibraryViewState {
    pub search_query: String,
    pub filter: LibraryFilter,
    folder_books: Vec<FolderBook>,
    folder_loading: bool,
    folder_error: Option<FolderScanError>,
    folder_truncated: bool,
    scan_rx: Option<Receiver<(u64, Result<FolderScan, FolderScanError>)>>,
    scan_generation: u64,
    last_scanned_folder: Option<PathBuf>,
    cover_loader: CoverLoader,
}

impl Default for LibraryViewState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            filter: LibraryFilter::All,
            folder_books: Vec::new(),
            folder_loading: false,
            folder_error: None,
            folder_truncated: false,
            scan_rx: None,
            scan_generation: 0,
            last_scanned_folder: None,
            cover_loader: CoverLoader::default(),
        }
    }
}

impl LibraryViewState {
    pub fn set_cover_cache_dir(&mut self, cache_dir: PathBuf) {
        self.cover_loader.set_cache_dir(cache_dir);
    }

    pub fn request_scan(&mut self, folder: PathBuf) {
        self.scan_generation = self.scan_generation.wrapping_add(1);
        let generation = self.scan_generation;
        self.folder_loading = true;
        self.folder_error = None;
        self.folder_truncated = false;
        self.folder_books.clear();
        self.last_scanned_folder = Some(folder.clone());

        let (sender, receiver) = mpsc::channel();
        self.scan_rx = Some(receiver);
        std::thread::spawn(move || {
            let result = scan_folder(
                &folder,
                MAX_FOLDER_SCAN_DEPTH,
                MAX_FOLDER_SCAN_ENTRIES,
                crate::document::registry::is_supported_document_path,
            );
            let _ = sender.send((generation, result));
        });
    }

    fn poll_scan(&mut self, library: &mut LibraryStore) -> bool {
        let Some(receiver) = self.scan_rx.take() else {
            return false;
        };
        let mut imported = false;
        match receiver.try_recv() {
            Ok((generation, result)) => {
                if generation == self.scan_generation {
                    self.folder_loading = false;
                    match result {
                        Ok(scan) => {
                            imported = library
                                .import_folder_books(&scan.books, crate::core::license::unix_now());
                            self.folder_books = scan.books;
                            self.folder_truncated = scan.truncated;
                            self.folder_error = None;
                        }
                        Err(error) => {
                            self.folder_books.clear();
                            self.folder_truncated = false;
                            self.folder_error = Some(error);
                        }
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.scan_rx = Some(receiver);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.folder_loading = false;
            }
        }
        imported
    }

    fn ensure_scan(&mut self, library: &LibraryStore) {
        if library.mode != LibraryMode::Folder {
            return;
        }
        let Some(folder) = library.library_folder.clone() else {
            return;
        };
        if self.last_scanned_folder.as_ref() != Some(&folder) && self.scan_rx.is_none() {
            self.request_scan(folder);
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        library: &mut LibraryStore,
        theme: &ReaderTheme,
        lang: UiLanguage,
        actions: &mut LibraryActions,
    ) {
        if self.poll_scan(library) {
            actions.save_library = true;
        }
        self.ensure_scan(library);
        self.cover_loader.poll(ui.ctx());
        if self.folder_loading {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        let t = lang.t();
        egui::Frame::new()
            .fill(theme.bg_color())
            .inner_margin(egui::Margin::symmetric(18, 14))
            .show(ui, |ui| {
                self.render_header(ui, library, theme, t, actions);
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                match library.mode {
                    LibraryMode::Shelf => self.render_shelf(ui, library, theme, t, actions),
                    LibraryMode::Folder => {
                        self.render_folder(ui, settings, library, theme, t, actions)
                    }
                    LibraryMode::ReaderOnly => self.render_reader_only(ui, theme, t, actions),
                }
            });
        if self.cover_loader.has_pending() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }

    fn render_header(
        &mut self,
        ui: &mut egui::Ui,
        library: &mut LibraryStore,
        theme: &ReaderTheme,
        t: &Translations,
        actions: &mut LibraryActions,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                IconLabel::new(Icon::Library, t.library_title)
                    .size(22.0)
                    .strong()
                    .color(theme.accent_color()),
            );
            ui.add_space(8.0);
            ui.separator();

            for mode in LibraryMode::ALL {
                let (icon, label) = mode_button(mode, t);
                let response = ui
                    .add(
                        IconButton::new(icon, label)
                            .selected(library.mode == mode)
                            .font_size(13.0),
                    )
                    .on_hover_text(t.library_mode_tooltip);
                if response.clicked() && library.mode != mode {
                    library.mode = mode;
                    actions.save_library = true;
                    if mode == LibraryMode::Folder {
                        self.last_scanned_folder = None;
                    }
                }
            }

            ui.separator();
            for design in UiDesignMode::ALL {
                let (icon, label) = design_button(design, t);
                let response = ui
                    .add(
                        IconButton::new(icon, label)
                            .selected(library.design == design)
                            .font_size(13.0),
                    )
                    .on_hover_text(t.library_design_tooltip);
                if response.clicked() && library.design != design {
                    library.design = design;
                    actions.save_library = true;
                }
            }

            if library.mode == LibraryMode::Shelf && library.design == UiDesignMode::Cozy {
                ui.separator();
                let today = unix_day_index(crate::core::license::unix_now());
                let streak = library.current_streak_days(today);
                let total_minutes = library.total_reading_minutes();
                let hours = total_minutes / 60;
                let minutes = total_minutes % 60;
                let time_value = t
                    .library_stats_time_value
                    .replace("{hours}", &hours.to_string())
                    .replace("{minutes}", &minutes.to_string());
                let completed = library.completed_books_count();
                for (label, value) in [
                    (t.library_stats_streak, streak.to_string()),
                    (t.library_stats_reading_time, time_value),
                    (t.library_stats_completed, completed.to_string()),
                ] {
                    ui.label(
                        RichText::new(format!("{label}: {value}"))
                            .size(11.0)
                            .color(theme.secondary_text_color()),
                    );
                }
            }
        });
    }

    fn render_folder(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        library: &LibraryStore,
        theme: &ReaderTheme,
        t: &Translations,
        actions: &mut LibraryActions,
    ) {
        let folder = library.library_folder.clone();
        ui.horizontal_wrapped(|ui| {
            ui.add(
                IconLabel::new(Icon::Folder, t.library_mode_folder)
                    .size(18.0)
                    .strong()
                    .color(theme.accent_color()),
            );
            if let Some(folder) = &folder {
                ui.label(
                    RichText::new(folder.display().to_string())
                        .size(11.0)
                        .color(theme.secondary_text_color()),
                );
            }
            if ui
                .add(IconButton::new(Icon::Folder, t.library_choose_folder))
                .clicked()
            {
                actions.pick_library_folder = true;
            }
            if let Some(folder) = &folder
                && ui
                    .add(IconButton::new(Icon::Refresh, t.library_refresh_folder))
                    .clicked()
            {
                self.request_scan(folder.clone());
            }
        });
        ui.add_space(10.0);

        if folder.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(70.0);
                ui.label(
                    RichText::new(t.library_no_folder)
                        .size(13.0)
                        .color(theme.secondary_text_color()),
                );
                ui.add_space(12.0);
                if ui
                    .add(IconButton::new(Icon::Folder, t.library_choose_folder))
                    .clicked()
                {
                    actions.pick_library_folder = true;
                }
            });
            return;
        }

        if self.folder_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(
                    RichText::new(t.library_folder_loading)
                        .size(12.0)
                        .color(theme.secondary_text_color()),
                );
            });
            return;
        }

        if let Some(error) = &self.folder_error {
            ui.add(
                IconLabel::new(Icon::Warning, &format_scan_error(error, t))
                    .size(13.0)
                    .color(Color32::from_rgb(220, 120, 80)),
            );
            return;
        }

        let folder = folder.unwrap_or_default();
        let recent_paths: HashSet<PathBuf> = settings
            .recent_books
            .iter()
            .map(|book| book.path.clone())
            .collect();
        let folder_books: Vec<&FolderBook> = self
            .folder_books
            .iter()
            .filter(|book| !recent_paths.contains(&book.path))
            .collect();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if !settings.recent_books.is_empty() {
                    ui.label(
                        RichText::new(t.library_recent_section)
                            .size(13.0)
                            .strong()
                            .color(theme.text_color()),
                    );
                    for book in settings.recent_books.iter().take(8) {
                        ui.horizontal(|ui| {
                            ui.add(IconLabel::new(Icon::Recent, book.title.as_str()).size(12.0));
                            ui.add_space(4.0);
                            if ui.small_button(t.library_open_book).clicked() {
                                actions.open_path = Some(book.path.clone());
                            }
                        });
                    }
                    ui.add_space(10.0);
                }

                if !folder_books.is_empty() {
                    ui.label(
                        RichText::new(t.library_folder_section)
                            .size(13.0)
                            .strong()
                            .color(theme.text_color()),
                    );
                    for book in folder_books {
                        let relative = book.path.strip_prefix(&folder).unwrap_or(&book.path);
                        ui.horizontal(|ui| {
                            ui.add(IconLabel::new(Icon::Book, book.title.as_str()).size(12.0));
                            ui.label(
                                RichText::new(relative.display().to_string())
                                    .size(10.0)
                                    .color(theme.secondary_text_color()),
                            );
                            if ui.small_button(t.library_open_book).clicked() {
                                actions.open_path = Some(book.path.clone());
                            }
                        });
                    }
                } else if self.folder_books.is_empty() {
                    ui.label(
                        RichText::new(t.library_folder_empty)
                            .size(12.0)
                            .color(theme.secondary_text_color()),
                    );
                }

                if self.folder_truncated {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(
                            t.library_folder_truncated
                                .replace("{count}", &self.folder_books.len().to_string()),
                        )
                        .size(11.0)
                        .color(Color32::from_rgb(220, 160, 80)),
                    );
                }
            });
    }

    fn render_reader_only(
        &self,
        ui: &mut egui::Ui,
        theme: &ReaderTheme,
        t: &Translations,
        actions: &mut LibraryActions,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(90.0);
            ui.add(
                IconLabel::new(Icon::Book, t.library_reader_title)
                    .size(34.0)
                    .strong()
                    .color(theme.accent_color()),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(t.library_reader_desc)
                    .size(15.0)
                    .color(theme.text_color()),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(t.drag_drop_hint)
                    .size(12.0)
                    .color(theme.secondary_text_color()),
            );
            ui.add_space(22.0);
            ui.horizontal(|ui| {
                if ui
                    .add(IconButton::new(Icon::OpenFile, t.open_file))
                    .clicked()
                {
                    actions.open_file = true;
                }
                if ui
                    .add(IconButton::new(Icon::Folder, t.open_folder))
                    .clicked()
                {
                    actions.open_folder = true;
                }
            });
        });
    }

    fn matches_filter(&self, entry: &LibraryEntry) -> bool {
        match self.filter {
            LibraryFilter::All => true,
            LibraryFilter::Reading => entry.status == LibraryReadingStatus::Reading,
            LibraryFilter::Finished => entry.status == LibraryReadingStatus::Finished,
        }
    }

    fn matches_query(&self, entry: &LibraryEntry, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        if entry.title.to_lowercase().contains(query) || entry.author.to_lowercase().contains(query)
        {
            return true;
        }
        entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_lowercase().contains(query))
            .unwrap_or(false)
    }
}

fn mode_button(mode: LibraryMode, t: &Translations) -> (Icon, &'static str) {
    match mode {
        LibraryMode::Shelf => (Icon::Library, t.library_mode_shelf),
        LibraryMode::Folder => (Icon::Folder, t.library_mode_folder),
        LibraryMode::ReaderOnly => (Icon::Book, t.library_mode_reader),
    }
}

fn design_button(design: UiDesignMode, t: &Translations) -> (Icon, &'static str) {
    match design {
        UiDesignMode::Cozy => (Icon::Cozy, t.library_design_cozy),
        UiDesignMode::Studio => (Icon::Studio, t.library_design_studio),
    }
}

fn format_scan_error(error: &FolderScanError, t: &Translations) -> String {
    let detail = match error {
        FolderScanError::NotDirectory => t.library_folder_invalid.to_string(),
        FolderScanError::Io(error) => error.clone(),
    };
    t.library_folder_error.replace("{error}", &detail)
}
