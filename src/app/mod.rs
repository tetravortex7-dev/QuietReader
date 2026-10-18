pub mod autoscroll;
pub mod bars;
pub mod comic_view;
pub mod commands;
pub mod dialogs;
pub mod document_loader;
pub mod fonts;
pub mod icons;
pub mod input;
pub mod paged_common;
pub mod pdf_toolbar;
pub mod pdf_view;
pub mod search_view;
pub mod settings_view;
pub mod tabs;
pub mod text_view;
pub mod theme;
pub mod toc_view;
pub mod tts;

use crate::core::ReaderCore;
use crate::document::{Document, DocumentViewKind};
use crate::settings::AppSettings;
use eframe::egui::{self, Panel, RichText};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub use self::autoscroll::AutoscrollController;
pub use self::bars::{render_bottom_bar, render_top_bar};
pub use self::comic_view::ComicViewState;
pub use self::dialogs::{Dialogs, SettingsTab};
pub use self::document_loader::DocumentLoader;
pub use self::fonts::apply_font_family;
pub use self::input::poll_keyboard_commands;
pub use self::pdf_view::PdfViewState;
pub use self::search_view::{SearchState, SearchView};
pub use self::settings_view::SettingsView;
pub use self::tabs::{BookTab, TabBarAction, TabsBar};
pub use self::text_view::TextView;
pub use self::theme::ReaderTheme;
pub use self::toc_view::TocView;
pub use self::tts::TtsController;

pub struct QuietReaderApp {
    pub tabs: Vec<BookTab>,
    pub active_tab: usize,
    pub document: Option<std::sync::Arc<dyn Document>>,
    pub current_file_path: Option<PathBuf>,
    pub current_chapter: usize,
    pub last_rendered_chapter: usize,
    pub pending_scroll_y: Option<f32>,
    pub current_scroll_y: f32,
    pub viewport_height: f32,
    pub max_scroll_y: f32,
    pub font_size: f32,
    pub theme: ReaderTheme,
    pub show_toc: bool,
    pub keep_screen_on: bool,
    pub is_fullscreen: bool,
    pub status_message: Option<String>,
    pub settings: AppSettings,

    pub settings_dirty: bool,
    pub last_settings_change: Instant,
    pub last_saved_scroll_y: f32,

    pub show_association_dialog: bool,
    pub dont_show_association_again: bool,
    pub show_recent_dialog: bool,
    pub show_settings_dialog: bool,
    pub settings_tab: SettingsTab,

    pub autoscroll: AutoscrollController,
    pub comic_view: ComicViewState,
    pub pdf_view: PdfViewState,
    pub document_loader: DocumentLoader,
    pub search: SearchState,
    pub tts: TtsController,
    pub last_tts_paragraph: Option<usize>,
    pub last_applied_theme: Option<ReaderTheme>,
}

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
        if let Some(paths) = Dialogs::pick_document_files() {
            for path in paths {
                self.load_file(path);
            }
        }
    }

    pub fn open_folder_dialog(&mut self) {
        if let Some(path) = Dialogs::pick_comic_folder() {
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

        let request_id = self.document_loader.request_load(path.clone());
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
}

impl eframe::App for QuietReaderApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.theme.bg_color().to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.document_loader.update_ctx(&ctx);
        self.poll_document_loader();
        self.flush_debounced_settings();

        crate::platform::register_primary_window_hwnd();
        #[cfg(windows)]
        {
            self.is_fullscreen = crate::platform::is_window_fullscreen();
        }
        let (should_focus, ipc_files) = crate::platform::poll_single_instance_requests();
        if should_focus {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            for path in ipc_files {
                self.load_file(path);
            }
            ctx.request_repaint();
        }

        let close_requested = ctx.input(|i| {
            let vp = i.viewport();
            let is_fs = self.is_fullscreen
                || crate::platform::is_window_fullscreen()
                || vp.fullscreen.unwrap_or(false);
            if !is_fs
                && !vp.minimized.unwrap_or(false)
                && let Some(inner) = vp.inner_rect
            {
                let pos = vp.outer_rect.map(|r| (r.min.x, r.min.y));
                self.settings.update_window_state(
                    pos.map(|p| p.0),
                    pos.map(|p| p.1),
                    inner.width(),
                    inner.height(),
                    vp.maximized.unwrap_or(false),
                );
            }
            vp.close_requested()
        });
        if close_requested {
            self.flush_settings_now();
        }

        for dropped in ctx.input(|i| i.raw.dropped_files.clone()) {
            let path = dropped.path();
            if !path.as_os_str().is_empty() {
                self.load_file(path.to_path_buf());
            }
        }

        let (ctrl_o, ctrl_w, next_tab_key, prev_tab_key) = if ctx.egui_wants_keyboard_input() {
            (false, false, false, false)
        } else {
            ctx.input(|i| {
                let cmd_mod = i.modifiers.ctrl || i.modifiers.command;
                let shift = i.modifiers.shift;
                (
                    cmd_mod && i.key_pressed(egui::Key::O),
                    cmd_mod && i.key_pressed(egui::Key::W),
                    cmd_mod && !shift && i.key_pressed(egui::Key::Tab),
                    cmd_mod && shift && i.key_pressed(egui::Key::Tab),
                )
            })
        };
        if ctrl_o {
            self.open_file_dialog();
        }
        if ctrl_w && !self.tabs.is_empty() {
            self.close_tab(self.active_tab);
        } else if next_tab_key {
            self.next_tab();
        } else if prev_tab_key {
            self.prev_tab();
        }

        if self.current_chapter != self.last_rendered_chapter {
            self.pending_scroll_y = Some(0.0);
            self.current_scroll_y = 0.0;
            self.last_saved_scroll_y = 0.0;
            self.last_rendered_chapter = self.current_chapter;
            self.mark_settings_dirty();
        }

        if (self.current_scroll_y - self.last_saved_scroll_y).abs() > 20.0 {
            self.last_saved_scroll_y = self.current_scroll_y;
            self.mark_settings_dirty();
        }

        let is_manga = self
            .comic_view
            .is_effective_manga(&self.document, &self.settings.comic);
        let view_kind = self.view_kind();

        let commands = poll_keyboard_commands(&ctx, is_manga);

        for cmd in &commands {
            self.execute_command(&ctx, cmd, view_kind);
        }

        match view_kind {
            DocumentViewKind::Text => {
                if let Some(doc) = &self.document {
                    let total_chapters = doc.chapters().len();
                    let next_non_empty =
                        ReaderCore::find_next_non_empty_chapter(doc.as_ref(), self.current_chapter);
                    self.autoscroll.update(
                        &ctx,
                        &self.settings.auto_scroll,
                        self.current_scroll_y,
                        self.max_scroll_y,
                        total_chapters,
                        &mut self.current_chapter,
                        &mut self.pending_scroll_y,
                        &mut self.last_rendered_chapter,
                        next_non_empty,
                    );
                }
            }
            DocumentViewKind::Pdf => {
                if self.settings.pdf.layout == "Continuous" {
                    self.autoscroll.update_continuous(
                        &ctx,
                        &self.settings.auto_scroll,
                        self.current_scroll_y,
                        self.max_scroll_y,
                        &mut self.pending_scroll_y,
                    );
                } else {
                    let total_pages = self.pdf_view.page_count(&self.document);
                    let cur_page = self.pdf_view.current_page;
                    if self.autoscroll.update_paged(
                        &ctx,
                        &self.settings.auto_scroll,
                        cur_page,
                        total_pages,
                    ) {
                        self.pdf_view.next_page(total_pages, &self.settings.pdf);
                        self.mark_settings_dirty();
                    }
                }
            }
            DocumentViewKind::Comic => {
                if self.settings.comic.reading_mode == "Webtoon" {
                    self.autoscroll.update_continuous(
                        &ctx,
                        &self.settings.auto_scroll,
                        self.current_scroll_y,
                        self.max_scroll_y,
                        &mut self.pending_scroll_y,
                    );
                } else {
                    let total_pages = self.comic_view.page_count(&self.document);
                    let cur_page = self.comic_view.current_page;
                    if self.autoscroll.update_paged(
                        &ctx,
                        &self.settings.auto_scroll,
                        cur_page,
                        total_pages,
                    ) {
                        self.comic_view.next_page(total_pages, &self.settings.comic);
                        self.mark_settings_dirty();
                    }
                }
            }
        }

        if self.last_applied_theme.as_ref() != Some(&self.theme) {
            ctx.set_visuals(self.theme.create_visuals());
            self.last_applied_theme = Some(self.theme.clone());
        }

        let ptr_top = ctx
            .input(|i| i.pointer.latest_pos().map(|p| p.y < 72.0))
            .unwrap_or(false);
        if !self.is_fullscreen || ptr_top || self.show_settings_dialog {
            let actions = render_top_bar(
                ui,
                self.document.is_some(),
                self.is_fullscreen,
                self.show_toc,
                self.settings.ui_language,
            );
            if actions.open_file {
                self.open_file_dialog();
            }
            if actions.open_folder {
                self.open_folder_dialog();
            }
            if actions.toggle_recent {
                self.show_recent_dialog = !self.show_recent_dialog;
            }
            if actions.toggle_toc {
                self.show_toc = !self.show_toc;
            }
            if actions.toggle_search {
                if self.view_kind() == DocumentViewKind::Text && self.document.is_some() {
                    self.search.toggle();
                }
            }
            if actions.toggle_tts {
                self.tts.toggle_panel();
            }
            if actions.toggle_fullscreen {
                self.set_fullscreen_mode(&ctx, !self.is_fullscreen);
            }
            if actions.toggle_settings {
                self.show_settings_dialog = !self.show_settings_dialog;
            }

            if !self.tabs.is_empty() {
                let mut tab_action = TabBarAction::None;
                Panel::top("tabs_bar").show(ui, |ui| {
                    tab_action = TabsBar::render(ui, &self.tabs, self.active_tab, &self.theme, self.settings.ui_language);
                });
                match tab_action {
                    TabBarAction::None => {}
                    TabBarAction::Activate(idx) => self.activate_tab(idx),
                    TabBarAction::Close(idx) => self.close_tab(idx),
                    TabBarAction::OpenNewFile => self.open_file_dialog(),
                }
            }
        }

        let pdf_info = (
            self.pdf_view.current_page,
            self.pdf_view.page_count(&self.document),
        );
        let comic_info = (
            self.comic_view.current_page,
            self.comic_view.page_count(&self.document),
        );
        let text_info = (
            self.current_chapter,
            self.document
                .as_ref()
                .map(|d| d.chapters().len())
                .unwrap_or(0),
            self.current_scroll_y,
            self.max_scroll_y,
        );
        render_bottom_bar(
            ui,
            &self.document,
            &self.theme,
            self.status_message.as_deref(),
            view_kind,
            pdf_info,
            comic_info,
            text_info,
            self.settings.ui_language,
        );

        if !self.is_fullscreen || self.show_toc {
            let prev_pdf_page = self.pdf_view.current_page;
            let current_page_ref = match view_kind {
                DocumentViewKind::Pdf => &mut self.pdf_view.current_page,
                _ => &mut self.comic_view.current_page,
            };
            TocView::render(
                ui,
                &self.document,
                &self.settings.comic,
                &mut self.current_chapter,
                current_page_ref,
                &mut self.show_toc,
                &self.theme,
                self.settings.ui_language,
            );
            if view_kind == DocumentViewKind::Pdf && self.pdf_view.current_page != prev_pdf_page {
                self.pdf_view.go_to_page(self.pdf_view.current_page);
            }
        }

        let mut on_open_file = false;
        let mut on_open_folder = false;
        let mut on_load_recent = None;
        let mut toggle_autoscroll_action = false;
        let mut save_settings_action = false;
        let mut register_assoc_action = false;
        let mut close_tab_action = false;

        let active_tab_info = self
            .tabs
            .get(self.active_tab)
            .map(|t| (t.is_loading, t.title.clone(), t.load_error.clone()));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(self.theme.bg_color())
                    .inner_margin(16.0),
            )
            .show(ui, |ui| {
                if self.show_settings_dialog {
                    let mut is_autoscrolling_flag = self.autoscroll.is_active();
                    SettingsView::render(
                        ui,
                        &ctx,
                        &mut self.show_settings_dialog,
                        &mut self.settings_tab,
                        &mut self.settings,
                        &mut self.font_size,
                        &mut self.theme,
                        &mut self.keep_screen_on,
                        &mut is_autoscrolling_flag,
                        &mut toggle_autoscroll_action,
                        &mut save_settings_action,
                        &mut register_assoc_action,
                    );
                } else if let Some((true, title, _)) = active_tab_info {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.35);
                        ui.spinner();
                        let t = self.settings.ui_language.t();
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(t.loading_doc.replace("{}", &title))
                                .size(18.0)
                                .color(self.theme.text_color()),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(t.please_wait)
                                .size(13.0)
                                .color(self.theme.secondary_text_color()),
                        );
                    });
                } else if let Some((false, _, Some(err))) = active_tab_info {
                    let t = self.settings.ui_language.t();
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.35);
                        ui.add(
                            self::icons::IconLabel::new(
                                self::icons::Icon::Warning,
                                t.failed_open_doc,
                            )
                            .size(20.0)
                            .color(egui::Color32::RED)
                            .strong(),
                        );
                        ui.add_space(12.0);
                        ui.label(RichText::new(err).size(14.0).color(self.theme.text_color()));
                        ui.add_space(16.0);
                        if ui.button(t.close_tab).clicked() {
                            close_tab_action = true;
                        }
                    });
                } else {
                    if self.document.is_some()
                        && (view_kind == DocumentViewKind::Text
                            || (view_kind == DocumentViewKind::Comic
                                && self.settings.comic.reading_mode == "Webtoon"))
                        && !self.show_recent_dialog
                        && !self.show_association_dialog
                    {
                        self.autoscroll.handle_middle_click_scroll(
                            &ctx,
                            self.show_toc,
                            self.current_scroll_y,
                            self.max_scroll_y,
                            &mut self.pending_scroll_y,
                        );
                    }

                    match view_kind {
                        DocumentViewKind::Pdf => {
                            self.pdf_view.render(
                                ui,
                                &ctx,
                                &self.document,
                                &mut self.settings.pdf,
                                &self.theme,
                                &mut self.pending_scroll_y,
                                &mut self.current_scroll_y,
                                &mut self.viewport_height,
                                &mut self.max_scroll_y,
                                &mut self.status_message,
                                self.settings.ui_language,
                            );
                        }
                        DocumentViewKind::Comic => {
                            self.comic_view.render(
                                ui,
                                &ctx,
                                &self.document,
                                &mut self.settings.comic,
                                &self.theme,
                                &mut self.pending_scroll_y,
                                &mut self.current_scroll_y,
                                &mut self.viewport_height,
                                &mut self.max_scroll_y,
                                &mut self.status_message,
                                self.settings.ui_language,
                            );
                        }
                        DocumentViewKind::Text => {
                            let tts_st = self.tts.status();
                            if tts_st.is_playing && !tts_st.is_paused {
                                if self.current_chapter != tts_st.current_chapter {
                                    self.current_chapter = tts_st.current_chapter;
                                    self.last_rendered_chapter = tts_st.current_chapter;
                                }
                                if self.last_tts_paragraph != Some(tts_st.current_paragraph) {
                                    if let Some(doc) = &self.document {
                                        let scroll_y = crate::core::ReaderCore::estimate_paragraph_scroll_y(
                                            doc.as_ref(),
                                            tts_st.current_chapter,
                                            Some(tts_st.current_paragraph),
                                            self.font_size,
                                        );
                                        self.pending_scroll_y = Some(scroll_y);
                                    }
                                    self.last_tts_paragraph = Some(tts_st.current_paragraph);
                                }
                            }
                            if tts_st.needs_next_chapter {
                                if let Some(doc) = &self.document {
                                    let next_ch = tts_st.current_chapter + 1;
                                    if let Some(text_content) = doc.text_content() {
                                        if let Some(ch) = text_content.chapters().get(next_ch) {
                                            self.current_chapter = next_ch;
                                            self.last_rendered_chapter = next_ch;
                                            self.pending_scroll_y = Some(0.0);
                                            self.tts.play_next_chapter(next_ch, ch.paragraphs.clone());
                                        } else {
                                            self.tts.stop();
                                        }
                                    }
                                }
                            }

                            let search_highlight = if self.search.is_open && !self.search.query.trim().is_empty() {
                                let active_para = self
                                    .search
                                    .current_hit()
                                    .filter(|h| h.chapter_index == self.current_chapter)
                                    .and_then(|h| h.paragraph_index);
                                Some((self.search.query.as_str(), active_para))
                            } else {
                                None
                            };

                            let tts_highlight = if (tts_st.is_playing || tts_st.is_paused) && tts_st.current_chapter == self.current_chapter {
                                Some(tts_st.current_paragraph)
                            } else {
                                None
                            };
                            let mut on_start_tts = None;

                            TextView::render(
                                ui,
                                &ctx,
                                &self.document,
                                &self.current_file_path,
                                &mut self.current_chapter,
                                &mut self.last_rendered_chapter,
                                &mut self.pending_scroll_y,
                                &mut self.current_scroll_y,
                                &mut self.viewport_height,
                                &mut self.max_scroll_y,
                                self.font_size,
                                &self.theme,
                                &mut self.settings,
                                &mut on_open_file,
                                &mut on_open_folder,
                                &mut on_load_recent,
                                search_highlight,
                                tts_highlight,
                                &mut on_start_tts,
                            );

                            if let Some(para_idx) = on_start_tts {
                                self.start_tts_at_paragraph(para_idx);
                            }
                        }
                    }
                }
            });

        if self.view_kind() == DocumentViewKind::Text && self.document.is_some() {
            SearchView::render(
                &ctx,
                &mut self.search,
                self.document.as_deref(),
                &mut self.current_chapter,
                &mut self.last_rendered_chapter,
                &mut self.pending_scroll_y,
                self.font_size,
                &self.theme,
                self.settings.ui_language,
            );
        }

        let has_text_content = self.document.as_ref().map(|d| d.text_content().is_some()).unwrap_or(false);
        let book_lang = self.document.as_deref().map(crate::core::ReaderCore::detect_document_language);
        let mut on_tts_play = false;
        self.tts.render_panel(&ctx, &self.theme, has_text_content, book_lang, &mut on_tts_play, self.settings.ui_language);
        if on_tts_play {
            let start_para = self.last_tts_paragraph.unwrap_or(0);
            self.start_tts_at_paragraph(start_para);
        }

        if close_tab_action {
            self.close_tab(self.active_tab);
        }
        if on_open_file {
            self.open_file_dialog();
        }
        if on_open_folder {
            self.open_folder_dialog();
        }
        if let Some(path) = on_load_recent {
            self.load_file(path);
        }

        let mut on_register_assoc = false;
        Dialogs::render_association_dialog(
            &ctx,
            &mut self.show_association_dialog,
            &mut self.dont_show_association_again,
            &mut self.settings,
            &self.theme,
            &mut on_register_assoc,
            &mut save_settings_action,
        );
        if on_register_assoc {
            self.register_file_association();
        }

        let mut recent_file_to_open = None;
        Dialogs::render_recent_dialog(
            &ctx,
            &mut self.show_recent_dialog,
            &self.current_file_path,
            &mut self.settings,
            &self.theme,
            &mut recent_file_to_open,
            &mut save_settings_action,
        );
        if let Some(path) = recent_file_to_open {
            self.load_file(path);
        }

        if toggle_autoscroll_action {
            self.autoscroll.toggle(
                self.current_scroll_y,
                self.max_scroll_y,
                self.settings.auto_scroll.step_distance,
            );
        }
        if save_settings_action {
            self.flush_settings_now();
            if crate::settings::AppSettings::last_save_error().is_none() {
                self.status_message = Some("Настройки сохранены".to_string());
            }
        }
        if register_assoc_action {
            self.register_file_association();
        }
    }
}
