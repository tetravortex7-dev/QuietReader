use super::ambient::AmbientController;
use super::autoscroll::AutoscrollController;
use super::bookmarks_view::BookmarkViewState;
use super::comic_view::ComicViewState;
use super::dialogs::SettingsTab;
use super::fonts::apply_font_family;
use super::pdf_view::PdfViewState;
use super::search_view::SearchState;
use super::tabs::BookTab;
use super::theme::ReaderTheme;
use super::toc_view::SidebarTab;
use super::translate::TranslateState;
use super::tts::TtsController;
use super::{Dialogs, DocumentLoader, QuietReaderApp};
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

        let ambient = AmbientController::new(settings.ambient.clone());

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
            last_saved_scroll_y: 0.0,
            show_association_dialog,
            dont_show_association_again: false,
            show_recent_dialog: false,
            show_settings_dialog: false,
            settings_tab: SettingsTab::Fonts,
            autoscroll: AutoscrollController::default(),
            comic_view: ComicViewState::default(),
            pdf_view: PdfViewState::default(),
            document_loader: DocumentLoader::default(),
            search: SearchState::default(),
            tts: TtsController::new(),
            last_tts_paragraph: None,
            last_applied_theme: None,
            updater,
            show_opds_dialog: false,
            opds_state: crate::opds::OpdsBrowserState::new(),
            translate: TranslateState::new(),
            sidebar_tab: SidebarTab::Toc,
            bookmark_view_state: BookmarkViewState::default(),
            return_location: None,
            ambient,
        }
    }

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        initial_file: Option<PathBuf>,
        settings: AppSettings,
    ) -> Self {
        apply_font_family(&cc.egui_ctx, &settings.font_family);
        let saved_tabs = settings.open_tabs.clone();
        let saved_active_idx = settings.active_tab_index;
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

        app
    }

    pub fn mark_settings_dirty(&mut self) {
        self.settings_dirty = true;
        self.last_settings_change = Instant::now();
    }

    pub fn flush_debounced_settings(&mut self) {
        if self.settings_dirty && self.last_settings_change.elapsed() >= Duration::from_millis(1500)
        {
            self.flush_settings_now();
        }
    }

    pub fn flush_settings_now(&mut self) {
        self.save_active_to_tab();
        self.sync_settings_fields();
        if let Err(e) = self.settings.save() {
            self.status_message = Some(format!("Ошибка сохранения: {e}"));
        }
        self.settings_dirty = false;
        self.last_settings_change = Instant::now();
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
        match crate::platform::register_file_associations(&exts) {
            Ok(msg) => {
                self.status_message = Some(msg);
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка ассоциации: {e}"));
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

        let request_id = self
            .document_loader
            .request_load(path.clone(), self.settings.ui_language);
        let tab = BookTab::new_loading(path, request_id);
        self.tabs.push(tab);
        let new_idx = self.tabs.len() - 1;
        self.apply_tab_state(new_idx);
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
                        self.current_chapter = loaded.chapter;
                        self.last_rendered_chapter = loaded.chapter;
                        self.pending_scroll_y = Some(loaded.scroll_y);
                        self.current_scroll_y = loaded.scroll_y;
                        self.comic_view.clear_cache();
                        self.comic_view.current_page = loaded.comic_page;
                        self.pdf_view.clear_cache();
                        self.pdf_view.current_page = loaded.comic_page;
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
        #[cfg(windows)]
        {
            let hwnd = crate::platform::get_primary_window_hwnd();
            if crate::platform::set_window_fullscreen(hwnd, fullscreen) {
                self.is_fullscreen = fullscreen;
                return;
            }
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
