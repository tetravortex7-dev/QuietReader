use super::ambient::AmbientController;
use super::autoscroll::AutoscrollController;
use super::bookmarks_view::BookmarkViewState;
use super::comic_view::ComicViewState;
use super::dialogs::SettingsTab;
use super::fonts::apply_font_family;
use super::license_view::ProUpgradeState;
use super::pdf_view::PdfViewState;
use super::search_view::SearchState;
use super::tabs::BookTab;
use super::theme::ReaderTheme;
use super::toc_view::{SidebarTab, TocViewState};
use super::translate::TranslateState;
use super::tts::TtsController;
use super::{Dialogs, DocumentLoader, LibraryViewState, QuietReaderApp, onboarding};
use crate::core::library::{LibraryMode, LibraryStore};
use crate::core::license::ProFeature;
use crate::document::DocumentViewKind;
use crate::settings::AppSettings;
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

impl Default for QuietReaderApp {
    fn default() -> Self {
        Self::from_settings(AppSettings::load())
    }
}

impl Drop for QuietReaderApp {
    fn drop(&mut self) {
        crate::platform::set_keep_screen_on(false);
        self.flush_settings_now();
    }
}

impl QuietReaderApp {
    pub fn from_settings(settings: AppSettings) -> Self {
        let assoc_status = crate::platform::check_fb2_association_status();
        let is_already_associated = assoc_status == crate::platform::AssociationStatus::Default
            || assoc_status == crate::platform::AssociationStatus::Registered;
        let dismissed = settings.dismiss_association_prompt
            || crate::platform::is_association_prompt_dismissed();
        let theme = match settings.theme.as_str() {
            "Dark" => ReaderTheme::Dark,
            "Light" => ReaderTheme::Light,
            "Oled" => ReaderTheme::Oled,
            "Warm" => ReaderTheme::Warm,
            "Mint" => ReaderTheme::Mint,
            "Twilight" => ReaderTheme::Twilight,
            _ => ReaderTheme::Sepia,
        };

        let show_association_dialog =
            crate::platform::supports_file_association() && !is_already_associated && !dismissed;

        let updater = crate::updater::UpdateChecker::new();
        if settings.check_updates_on_startup {
            updater.check_now();
        }

        let mut ambient_settings = settings.ambient.clone();
        if !settings.can_use(ProFeature::AmbientAudio) {
            ambient_settings.is_playing = false;
        }
        let ambient = AmbientController::new(ambient_settings);
        let tts_settings = settings.tts.clone();
        let effective_font_family = settings.effective_font_family();
        let library_path = LibraryStore::path_beside(&AppSettings::get_settings_path());
        let library_existed = library_path.exists();
        let mut library = LibraryStore::load_from_path(&library_path);
        let mut library_dirty = false;
        if !library.recent_imported {
            let now = crate::core::license::unix_now();
            for book in &settings.recent_books {
                library.ensure_book(&book.path, &book.title, &book.author, now);
                if let Some((chapter, _)) = settings.get_book_progress(&book.path) {
                    library.record_progress(&book.path, &book.title, &book.author, chapter, 0, now);
                }
            }
            if let Some(path) = &settings.last_opened_book {
                library.ensure_book(path, "", "", now);
            }
            library.recent_imported = true;
            if library.save_to_path(&library_path).is_err() {
                library_dirty = true;
            }
        }
        if !library_existed && onboarding::ensure_welcome_book(&library_path, &mut library) {
            library_dirty = library.save_to_path(&library_path).is_err();
        }
        let show_library = library.mode != LibraryMode::ReaderOnly;
        let last_applied_design = library.design;
        let cover_cache_dir = library_path
            .parent()
            .map(|parent| parent.join("covers"))
            .unwrap_or_else(|| std::env::temp_dir().join("QuietReader").join("covers"));
        let mut library_view = LibraryViewState::default();
        library_view.set_cover_cache_dir(cover_cache_dir);

        Self {
            tabs: Vec::new(),
            active_tab: 0,
            document: None,
            current_file_path: None,
            current_chapter: 0,
            last_rendered_chapter: 0,
            pending_scroll_y: None,
            current_scroll_y: 0.0,
            viewport_height: 600.0,
            max_scroll_y: 0.0,
            font_size: settings.font_size,
            theme,
            show_toc: settings.show_toc,
            keep_screen_on: settings.keep_screen_on,
            is_fullscreen: false,
            status_message: None,
            status_time: None,
            last_status_text: None,
            settings,
            settings_dirty: false,
            last_settings_change: Instant::now(),
            progress_dirty: false,
            last_progress_change: Instant::now(),
            last_saved_scroll_y: 0.0,
            show_association_dialog,
            dont_show_association_again: false,
            show_recent_dialog: false,
            show_settings_dialog: false,
            settings_tab: SettingsTab::Fonts,
            autoscroll: AutoscrollController::default(),
            comic_view: ComicViewState::default(),
            pdf_view: PdfViewState::default(),
            text_images: Default::default(),
            document_loader: DocumentLoader::default(),
            search: SearchState::default(),
            tts: TtsController::new(tts_settings),
            last_tts_paragraph: None,
            last_applied_theme: None,
            last_applied_design,
            library,
            library_path,
            library_dirty,
            last_library_save: Instant::now(),
            last_reading_activity: None,
            reading_seconds_accumulator: 0.0,
            library_view,
            show_library,
            updater,
            show_opds_dialog: false,
            opds_state: crate::opds::OpdsBrowserState::new(),
            translate: TranslateState::new(),
            sidebar_tab: SidebarTab::Toc,
            bookmark_view_state: BookmarkViewState::default(),
            toc_view: TocViewState::default(),
            return_location: None,
            ambient,
            pro_upgrade: ProUpgradeState::default(),
            last_applied_font_family: effective_font_family,
        }
    }

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        initial_file: Option<PathBuf>,
        settings: AppSettings,
    ) -> Self {
        let effective_font_family = settings.effective_font_family();
        apply_font_family(&cc.egui_ctx, &effective_font_family);
        let saved_tabs = settings.open_tabs.clone();
        let saved_active_idx = settings.active_tab_index;
        let has_initial_file = initial_file.is_some();
        let mut app = Self::from_settings(settings);
        app.document_loader.update_ctx(&cc.egui_ctx);
        crate::platform::set_keep_screen_on(app.keep_screen_on);

        cc.egui_ctx
            .add_plugin(super::selection::SelectionCapturePlugin);

        if !saved_tabs.is_empty() {
            for tab_path in saved_tabs {
                if tab_path.exists() {
                    app.load_file(tab_path);
                }
            }
            if !app.tabs.is_empty() {
                let target_idx = saved_active_idx.min(app.tabs.len() - 1);
                app.activate_tab(target_idx);
            }
        } else if let Some(last_path) = app.settings.last_opened_book.clone()
            && last_path.exists()
        {
            app.load_file(last_path);
        }

        if let Some(path) = initial_file
            && path.exists()
        {
            app.load_file(path);
        }

        app.show_library = if has_initial_file {
            false
        } else {
            app.library.mode != LibraryMode::ReaderOnly
        };
        app
    }

    pub fn mark_settings_dirty(&mut self) {
        self.settings_dirty = true;
        self.last_settings_change = Instant::now();
    }

    pub fn mark_progress_dirty(&mut self) {
        self.progress_dirty = true;
        self.last_progress_change = Instant::now();
    }

    pub fn flush_debounced_settings(&mut self) {
        let settings_ready = self.settings_dirty
            && self.last_settings_change.elapsed() >= Duration::from_millis(1500);
        let progress_ready =
            self.progress_dirty && self.last_progress_change.elapsed() >= Duration::from_secs(8);
        if settings_ready || progress_ready {
            self.flush_settings_now();
        }
    }

    pub fn flush_settings_now(&mut self) {
        self.save_active_to_tab();
        self.sync_settings_fields();
        let settings_result = self.settings.save();
        if self.library_dirty {
            self.save_library_now();
        }
        if let Err(e) = settings_result {
            let t = self.settings.ui_language.t();
            self.status_message = Some(t.save_settings_error.replace("{}", &e));
        }
        self.settings_dirty = false;
        self.last_settings_change = Instant::now();
        self.progress_dirty = false;
        self.last_progress_change = Instant::now();
    }

    pub fn save_library_now(&mut self) {
        match self.library.save_to_path(&self.library_path) {
            Ok(()) => {
                self.library_dirty = false;
            }
            Err(error) => {
                let t = self.settings.ui_language.t();
                self.status_message = Some(t.library_save_error.replace("{error}", &error));
            }
        }
        self.last_library_save = Instant::now();
    }

    fn sync_settings_fields(&mut self) {
        self.settings.open_tabs = self.tabs.iter().map(|t| t.file_path.clone()).collect();
        self.settings.active_tab_index = self.active_tab;
        self.settings.font_size = self.font_size;
        self.settings.keep_screen_on = self.keep_screen_on;
        self.settings.show_toc = self.show_toc;
        self.settings.theme = match self.theme {
            ReaderTheme::Dark => "Dark",
            ReaderTheme::Light => "Light",
            ReaderTheme::Sepia => "Sepia",
            ReaderTheme::Oled => "Oled",
            ReaderTheme::Warm => "Warm",
            ReaderTheme::Mint => "Mint",
            ReaderTheme::Twilight => "Twilight",
        }
        .to_string();
    }

    pub fn start_tts_at_paragraph(&mut self, paragraph_idx: usize) {
        if let Some(doc) = &self.document {
            if let Some(text_content) = doc.text_content() {
                if let Some(ch) = text_content.chapters().get(self.current_chapter) {
                    self.tts.show_panel = true;
                    self.tts.play_chapter(
                        self.current_chapter,
                        paragraph_idx,
                        ch.paragraphs.clone(),
                    );
                    self.last_tts_paragraph = Some(paragraph_idx);
                    let t = self.settings.ui_language.t();
                    self.set_status(format!(
                        "{} (№ {})",
                        t.read_aloud_from_here,
                        paragraph_idx + 1
                    ));
                }
            }
        }
    }

    pub fn register_file_association(&mut self) {
        let exts: Vec<&str> = self
            .settings
            .associated_extensions
            .iter()
            .map(|s| s.as_str())
            .collect();
        match crate::platform::register_file_associations(&exts, self.settings.ui_language) {
            Ok(msg) => {
                self.status_message = Some(msg);
            }
            Err(e) => {
                let t = self.settings.ui_language.t();
                self.status_message = Some(t.extra.status_assoc_error.replace("{error}", &e));
            }
        }
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(paths) = Dialogs::pick_document_files(self.settings.ui_language) {
            for path in paths {
                self.load_file(path);
            }
        }
    }

    pub fn open_folder_dialog(&mut self) {
        if let Some(path) = Dialogs::pick_comic_folder(self.settings.ui_language) {
            self.load_file(path);
        }
    }

    pub fn load_file(&mut self, path: PathBuf) {
        if let Some(existing_idx) = self.tabs.iter().position(|t| t.file_path == path) {
            self.activate_tab(existing_idx);
            return;
        }

        self.flush_settings_now();
        self.autoscroll.stop();

        let request_id = self.document_loader.request_load(
            path.clone(),
            self.settings.ui_language,
            self.settings.text_encoding_override().map(str::to_string),
        );
        let tab = BookTab::new_loading(path, request_id, self.settings.ui_language);
        self.tabs.push(tab);
        let new_idx = self.tabs.len() - 1;
        self.apply_tab_state(new_idx);
    }

    pub fn reload_current_text_encoding(&mut self) {
        let Some(path) = self.current_file_path.clone() else {
            return;
        };
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_lowercase())
            .unwrap_or_default();
        if !matches!(extension.as_str(), "txt" | "md" | "markdown" | "log") {
            return;
        }
        self.save_active_to_tab();
        let request_id = self.document_loader.request_load(
            path,
            self.settings.ui_language,
            self.settings.text_encoding_override().map(str::to_string),
        );
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.is_loading = true;
            tab.request_id = request_id;
            tab.load_error = None;
            tab.document = None;
        }
        self.document = None;
        self.text_images.clear();
        self.show_settings_dialog = false;
    }

    pub fn poll_document_loader(&mut self) {
        let responses = self.document_loader.poll_responses();
        for resp in responses {
            let matching_idx = self
                .tabs
                .iter()
                .position(|t| t.file_path == resp.path && t.request_id == resp.request_id);
            let Some(idx) = matching_idx else {
                continue;
            };
            match resp.result {
                Ok(doc) => {
                    let loaded = DocumentLoader::finalize_loaded_document(
                        doc,
                        resp.path,
                        &mut self.settings,
                    );
                    let meta = loaded.document.metadata();
                    let total_units = if let Some(pages) = loaded.document.page_content() {
                        pages.page_count()
                    } else {
                        loaded.document.chapters().len()
                    };
                    let current_unit = if loaded.document.page_content().is_some() {
                        loaded.comic_page
                    } else {
                        loaded.chapter
                    };
                    if self.library.record_progress(
                        &loaded.file_path,
                        &meta.title,
                        &meta.author,
                        current_unit,
                        total_units,
                        crate::core::license::unix_now(),
                    ) {
                        self.library_dirty = true;
                    }
                    if let Some(tab) = self.tabs.get_mut(idx) {
                        tab.update_loaded(
                            loaded.document.clone(),
                            loaded.chapter,
                            loaded.scroll_y,
                            loaded.comic_page,
                        );
                    }
                    if self.active_tab == idx {
                        self.document = Some(loaded.document);
                        self.current_file_path = Some(loaded.file_path);
                        self.last_reading_activity = Some(Instant::now());
                        self.reading_seconds_accumulator = 0.0;
                        self.current_chapter = loaded.chapter;
                        self.last_rendered_chapter = loaded.chapter;
                        self.pending_scroll_y = Some(loaded.scroll_y);
                        self.current_scroll_y = loaded.scroll_y;
                        self.comic_view.clear_cache();
                        self.comic_view.current_page = loaded.comic_page;
                        self.pdf_view.clear_cache();
                        self.pdf_view.current_page = loaded.comic_page;
                        self.text_images.clear();
                        self.search.reset_for_document();
                        self.toc_view.filter.clear();
                        self.status_message = Some(loaded.status_message);
                        self.mark_settings_dirty();
                    }
                }
                Err(err) => {
                    if let Some(tab) = self.tabs.get_mut(idx) {
                        tab.is_loading = false;
                        tab.load_error = Some(err.clone());
                    }
                    if self.active_tab == idx {
                        self.document = None;
                        self.status_message = Some(err);
                    }
                }
            }
        }
    }

    pub fn set_fullscreen_mode(&mut self, ctx: &egui::Context, fullscreen: bool) {
        if self.is_fullscreen == fullscreen {
            return;
        }
        if crate::platform::try_set_native_fullscreen(fullscreen) {
            self.is_fullscreen = fullscreen;
            return;
        }
        self.is_fullscreen = fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
    }

    pub fn view_kind(&self) -> DocumentViewKind {
        self.document
            .as_ref()
            .map(|d| d.view_kind())
            .unwrap_or(DocumentViewKind::Text)
    }

    #[allow(dead_code)]
    pub fn is_comic(&self) -> bool {
        self.view_kind() == DocumentViewKind::Comic
    }

    #[allow(dead_code)]
    pub fn is_pdf(&self) -> bool {
        self.view_kind() == DocumentViewKind::Pdf
    }

    #[allow(dead_code)]
    pub fn set_status(&mut self, msg: impl Into<String>) {
        let s = msg.into();
        self.last_status_text = Some(s.clone());
        self.status_message = Some(s);
        self.status_time = Some(Instant::now());
    }
}
