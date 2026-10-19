pub mod ambient;
pub mod ambient_synth;
pub mod app_lifecycle;
pub mod autoscroll;
pub mod bars;
pub mod bionic;
pub mod bookmarks_view;
pub mod comic_view;
pub mod commands;
pub mod dialogs;
pub mod document_loader;
pub mod fonts;
pub mod icons;
pub mod input;
pub mod opds_sidebar;
pub mod opds_view;
pub mod paged_common;
pub mod pdf_toolbar;
pub mod pdf_view;
pub mod scan_processor;
pub mod search_view;
pub mod selection;
pub mod settings_system;
pub mod settings_view;
pub mod tabs;
pub mod text_view;
pub mod theme;
pub mod toc_view;
pub mod translate;
pub mod tts;

use crate::core::ReaderCore;
use crate::document::{Document, DocumentViewKind};
use crate::settings::AppSettings;
use eframe::egui::{self, Panel, RichText};
use std::path::PathBuf;
use std::time::Instant;

pub use self::ambient::AmbientController;
pub use self::autoscroll::AutoscrollController;
pub use self::bars::{render_bottom_bar, render_top_bar};
pub use self::bookmarks_view::BookmarkViewState;
pub use self::comic_view::ComicViewState;
pub use self::dialogs::{Dialogs, SettingsTab};
pub use self::document_loader::DocumentLoader;
pub use self::input::poll_keyboard_commands;
pub use self::pdf_view::PdfViewState;
pub use self::search_view::{SearchState, SearchView};
pub use self::settings_view::SettingsView;
pub use self::tabs::{BookTab, TabBarAction, TabsBar};
pub use self::text_view::TextView;
pub use self::theme::ReaderTheme;
pub use self::toc_view::{SidebarTab, TocView};
pub use self::translate::{TranslateState, TranslateView};
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
    pub status_time: Option<Instant>,
    pub last_status_text: Option<String>,
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
    pub updater: crate::updater::UpdateChecker,
    pub show_opds_dialog: bool,
    pub opds_state: crate::opds::OpdsBrowserState,
    pub translate: TranslateState,
    pub sidebar_tab: SidebarTab,
    pub bookmark_view_state: BookmarkViewState,
    pub return_location: Option<(usize, f32, Option<usize>)>,
    pub ambient: AmbientController,
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

        let (key_o, key_p, ctrl_w, next_tab_key, prev_tab_key) = if ctx.egui_wants_keyboard_input()
        {
            (false, false, false, false, false)
        } else {
            ctx.input(|i| {
                let cmd_mod = i.modifiers.ctrl || i.modifiers.command;
                let no_alt = !i.modifiers.alt;
                let shift = i.modifiers.shift;
                (
                    (cmd_mod || no_alt) && i.key_pressed(egui::Key::O),
                    (cmd_mod || no_alt) && i.key_pressed(egui::Key::P),
                    cmd_mod && i.key_pressed(egui::Key::W),
                    cmd_mod && !shift && i.key_pressed(egui::Key::Tab),
                    cmd_mod && shift && i.key_pressed(egui::Key::Tab),
                )
            })
        };
        if key_o {
            self.open_file_dialog();
        }
        if key_p {
            self.open_folder_dialog();
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
        let update_tag = match self.updater.status() {
            crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
                Some(info.latest_version.clone())
            }
            _ => None,
        };
        if !self.is_fullscreen || ptr_top || self.show_settings_dialog {
            let actions = render_top_bar(
                ui,
                self.document.is_some(),
                self.is_fullscreen,
                self.show_toc,
                self.settings.reading_ruler,
                self.settings.ui_language,
                update_tag.as_deref(),
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
            if actions.toggle_opds {
                self.show_opds_dialog = !self.show_opds_dialog;
            }
            if actions.toggle_toc {
                self.show_toc = !self.show_toc;
                if self.show_toc {
                    self.sidebar_tab = SidebarTab::Toc;
                }
            }
            if actions.toggle_bookmark {
                self.toggle_quick_bookmark();
            }
            if actions.toggle_bookmarks_panel {
                if !self.show_toc {
                    self.show_toc = true;
                    self.sidebar_tab = SidebarTab::Bookmarks;
                } else if self.sidebar_tab == SidebarTab::Bookmarks {
                    self.show_toc = false;
                } else {
                    self.sidebar_tab = SidebarTab::Bookmarks;
                }
            }
            if actions.toggle_search {
                if self.view_kind() == DocumentViewKind::Text && self.document.is_some() {
                    self.search.toggle();
                }
            }
            if actions.toggle_tts {
                self.tts.toggle_panel();
                if self.tts.show_panel && !self.tts.status().is_playing {
                    if let Some(doc) = &self.document {
                        let visible_para = crate::core::ReaderCore::estimate_visible_paragraph(
                            doc.as_ref(),
                            self.current_chapter,
                            self.current_scroll_y,
                            self.font_size,
                        );
                        self.last_tts_paragraph = Some(visible_para);
                    }
                }
            }
            if actions.toggle_reading_ruler {
                self.settings.reading_ruler = !self.settings.reading_ruler;
                self.mark_settings_dirty();
            }
            if actions.toggle_ambient {
                self.ambient.toggle_panel();
            }
            if actions.toggle_fullscreen {
                self.set_fullscreen_mode(&ctx, !self.is_fullscreen);
            }
            if actions.toggle_settings {
                self.show_settings_dialog = !self.show_settings_dialog;
            }
            if actions.open_update {
                if let crate::updater::UpdateStatus::UpdateAvailable(ref info) =
                    self.updater.status()
                {
                    ctx.open_url(egui::OpenUrl::new_tab(&info.release_url));
                }
            }

            if !self.tabs.is_empty() {
                let mut tab_action = TabBarAction::None;
                Panel::top("tabs_bar").show(ui, |ui| {
                    tab_action = TabsBar::render(
                        ui,
                        &self.tabs,
                        self.active_tab,
                        &self.theme,
                        self.settings.ui_language,
                    );
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
        if self.status_message != self.last_status_text {
            self.last_status_text = self.status_message.clone();
            self.status_time = self.status_message.as_ref().map(|_| Instant::now());
            ctx.request_repaint();
        }

        let (status_text, status_opacity) = if let Some(ref msg) = self.status_message {
            if let Some(time) = self.status_time {
                let elapsed = time.elapsed().as_secs_f32();
                if elapsed >= 5.0 {
                    self.status_message = None;
                    self.status_time = None;
                    self.last_status_text = None;
                    (None, 0.0)
                } else {
                    let opacity = if elapsed < 3.5 {
                        ctx.request_repaint_after(std::time::Duration::from_millis(50));
                        1.0
                    } else {
                        ctx.request_repaint();
                        ((5.0 - elapsed) / 1.5).clamp(0.0, 1.0)
                    };
                    (Some(msg.as_str()), opacity)
                }
            } else {
                (Some(msg.as_str()), 1.0)
            }
        } else {
            (None, 0.0)
        };

        render_bottom_bar(
            ui,
            &self.document,
            &self.theme,
            status_text,
            status_opacity,
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
            let mut on_add_bookmark = false;
            TocView::render(
                ui,
                &self.document,
                &mut self.current_chapter,
                current_page_ref,
                &mut self.show_toc,
                &mut self.sidebar_tab,
                &mut self.settings,
                self.current_file_path.as_deref(),
                self.current_scroll_y,
                &mut self.pending_scroll_y,
                &mut self.last_rendered_chapter,
                &self.theme,
                &mut self.status_message,
                &mut self.return_location,
                &mut self.bookmark_view_state,
                &mut on_add_bookmark,
            );
            if on_add_bookmark {
                self.toggle_quick_bookmark();
            }
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
                        &self.updater,
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

                    let mut on_translate = None;

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
                                &mut on_translate,
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
                                        let scroll_y =
                                            crate::core::ReaderCore::estimate_paragraph_scroll_y(
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
                                            self.tts
                                                .play_next_chapter(next_ch, ch.paragraphs.clone());
                                        } else {
                                            self.tts.stop();
                                        }
                                    }
                                }
                            }

                            let search_highlight =
                                if self.search.is_open && !self.search.query.trim().is_empty() {
                                    let active_para = self
                                        .search
                                        .current_hit()
                                        .filter(|h| h.chapter_index == self.current_chapter)
                                        .and_then(|h| h.paragraph_index);
                                    Some((self.search.query.as_str(), active_para))
                                } else {
                                    None
                                };

                            let tts_highlight = if (tts_st.is_playing || tts_st.is_paused)
                                && tts_st.current_chapter == self.current_chapter
                            {
                                Some(tts_st.current_paragraph)
                            } else {
                                None
                            };
                            let mut on_start_tts = None;
                            let mut on_open_opds = false;
                            let mut on_save_quote = None;

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
                                &mut on_open_opds,
                                &mut on_translate,
                                &mut on_save_quote,
                            );

                            if let Some(text) = on_save_quote {
                                self.save_quote_bookmark(text);
                            }

                            if on_open_opds {
                                self.show_opds_dialog = true;
                            }

                            if let Some(para_idx) = on_start_tts {
                                self.start_tts_at_paragraph(para_idx);
                            }
                        }
                    }

                    if let Some(text) = on_translate {
                        let target_code = self.settings.ui_language.resolve().code();
                        self.translate.translate(
                            text,
                            target_code,
                            self.settings.translation.provider,
                            &self.settings.translation.deepl_api_key,
                        );
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

        let has_text_content = self
            .document
            .as_ref()
            .map(|d| d.text_content().is_some())
            .unwrap_or(false);
        let book_lang = self
            .document
            .as_deref()
            .map(crate::core::ReaderCore::detect_document_language);
        let mut on_tts_play = false;
        self.tts.render_panel(
            &ctx,
            &self.theme,
            has_text_content,
            book_lang,
            &mut on_tts_play,
            self.settings.ui_language,
        );
        if on_tts_play {
            let start_para = self.last_tts_paragraph.unwrap_or_else(|| {
                if let Some(doc) = &self.document {
                    crate::core::ReaderCore::estimate_visible_paragraph(
                        doc.as_ref(),
                        self.current_chapter,
                        self.current_scroll_y,
                        self.font_size,
                    )
                } else {
                    0
                }
            });
            self.start_tts_at_paragraph(start_para);
        }

        self.ambient.set_tts_active(self.tts.status().is_playing);
        if self.ambient.tick(&ctx) {
            self.settings.ambient = self.ambient.settings.clone();
            self.mark_settings_dirty();
        }
        if self
            .ambient
            .render(&ctx, &self.theme, self.settings.ui_language)
        {
            self.settings.ambient = self.ambient.settings.clone();
            self.mark_settings_dirty();
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

        self.opds_state.poll(&mut self.settings);
        let mut opds_file_to_open = None;
        self.opds_state.render(
            &ctx,
            &mut self.show_opds_dialog,
            &mut self.settings,
            &self.theme,
            &mut opds_file_to_open,
        );
        if let Some(path) = opds_file_to_open {
            self.load_file(path);
            self.status_message = Some(
                self.settings
                    .ui_language
                    .t()
                    .opds_book_downloaded_opened
                    .to_string(),
            );
        }

        let mut on_speak_translated = None;
        TranslateView::render(
            &mut self.translate,
            &ctx,
            &self.theme,
            &mut self.settings,
            &mut on_speak_translated,
            &mut save_settings_action,
        );
        if let Some(text) = on_speak_translated {
            self.tts.show_panel = true;
            self.tts.play_chapter(self.current_chapter, 0, vec![text]);
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
