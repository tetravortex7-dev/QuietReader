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
pub mod library_view;
pub mod license_view;
pub mod localized_document;
pub mod onboarding;
pub mod opds_covers;
pub mod opds_sidebar;
pub mod opds_view;
pub mod paged_common;
mod pdf_page;
pub mod pdf_toolbar;
pub mod pdf_view;
mod post_central;
pub mod reading_stats;
pub mod scan_processor;
pub mod search_view;
pub mod selection;
pub mod settings_system;
pub mod settings_view;
pub mod tabs;
pub mod text_images;
pub mod text_view;
pub mod theme;
pub mod toc_view;
mod top_bar_actions;
pub mod translate;
pub mod tts;

use crate::core::ReaderCore;
use crate::core::library::{LibraryStore, UiDesignMode};
use crate::core::license::ProFeature;
use crate::document::{Document, DocumentViewKind};
use crate::settings::AppSettings;
use eframe::egui::{self, RichText};
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
pub use self::library_view::{LibraryActions, LibraryViewState};
pub use self::license_view::{ProUpgradeDialog, ProUpgradeState};
pub use self::pdf_view::PdfViewState;
use self::post_central::PostCentralActions;
pub use self::search_view::{SearchState, SearchView};
pub use self::settings_view::SettingsView;
pub use self::tabs::{BookTab, TabBarAction, TabsBar};
pub use self::text_images::TextImageState;
pub use self::text_view::TextView;
pub use self::theme::ReaderTheme;
pub use self::toc_view::{SidebarTab, TocView, TocViewState};
pub use self::translate::{TranslateState, TranslateView};
pub use self::tts::TtsController;

pub(crate) fn rgba_byte_size(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
}

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
    pub progress_dirty: bool,
    pub last_progress_change: Instant,
    pub last_saved_scroll_y: f32,

    pub show_association_dialog: bool,
    pub dont_show_association_again: bool,
    pub show_recent_dialog: bool,
    pub show_settings_dialog: bool,
    pub settings_tab: SettingsTab,

    pub autoscroll: AutoscrollController,
    pub comic_view: ComicViewState,
    pub pdf_view: PdfViewState,
    pub text_images: TextImageState,
    pub document_loader: DocumentLoader,
    pub search: SearchState,
    pub tts: TtsController,
    pub last_tts_paragraph: Option<usize>,
    pub last_applied_theme: Option<ReaderTheme>,
    pub last_applied_design: UiDesignMode,
    pub library: LibraryStore,
    pub library_path: PathBuf,
    pub library_dirty: bool,
    pub last_library_save: Instant,
    pub last_reading_activity: Option<Instant>,
    pub reading_seconds_accumulator: f32,
    pub library_view: LibraryViewState,
    pub show_library: bool,
    pub updater: crate::updater::UpdateChecker,
    pub show_opds_dialog: bool,
    pub opds_state: crate::opds::OpdsBrowserState,
    pub translate: TranslateState,
    pub sidebar_tab: SidebarTab,
    pub bookmark_view_state: BookmarkViewState,
    pub toc_view: TocViewState,
    pub return_location: Option<(usize, f32, Option<usize>)>,
    pub ambient: AmbientController,
    pub pro_upgrade: ProUpgradeState,
    pub last_applied_font_family: String,
}

impl eframe::App for QuietReaderApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.theme.bg_color().to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let effective_font_family = self.settings.effective_font_family();
        if effective_font_family != self.last_applied_font_family {
            self::fonts::apply_font_family(&ctx, &effective_font_family);
            self.last_applied_font_family = effective_font_family;
        }
        let mut pro_required = None;
        let ambient_allowed = self.settings.can_use(ProFeature::AmbientAudio);
        let neural_tts_allowed = self.settings.can_use(ProFeature::NeuralTts);
        self.document_loader.update_ctx(&ctx);
        self.poll_document_loader();
        self.flush_debounced_settings();
        if self.settings_dirty {
            ctx.request_repaint_after(std::time::Duration::from_millis(1500));
        }
        if self.progress_dirty {
            ctx.request_repaint_after(std::time::Duration::from_secs(8));
        }

        crate::platform::register_primary_window_hwnd();
        crate::platform::sync_fullscreen_state(&mut self.is_fullscreen);
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
            self.mark_progress_dirty();
        }

        if (self.current_scroll_y - self.last_saved_scroll_y).abs() > 20.0 {
            self.last_saved_scroll_y = self.current_scroll_y;
            self.mark_progress_dirty();
        }

        let is_manga = self
            .comic_view
            .is_effective_manga(&self.document, &self.settings.comic);
        let view_kind = self.view_kind();

        let commands = poll_keyboard_commands(&ctx, is_manga);

        for cmd in &commands {
            if self.show_library
                && !matches!(
                    cmd,
                    crate::core::Command::ToggleSettings
                        | crate::core::Command::ToggleFullscreen
                        | crate::core::Command::ExitFullscreen
                )
            {
                continue;
            }
            self.execute_command(&ctx, cmd, view_kind);
        }

        if !self.show_library {
            match view_kind {
                DocumentViewKind::Text => {
                    if let Some(doc) = &self.document {
                        let total_chapters = doc.chapters().len();
                        let next_non_empty = ReaderCore::find_next_non_empty_chapter(
                            doc.as_ref(),
                            self.current_chapter,
                        );
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
        }

        if self.last_applied_theme.as_ref() != Some(&self.theme)
            || self.last_applied_design != self.library.design
        {
            ctx.set_visuals(self.theme.create_visuals_for(self.library.design));
            self.last_applied_theme = Some(self.theme.clone());
            self.last_applied_design = self.library.design;
        }

        self::top_bar_actions::render(self, ui, &ctx, &mut pro_required);

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

        if !self.show_library || status_text.is_some() {
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
        }

        if (!self.is_fullscreen || self.show_toc) && !self.show_library {
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
                &mut self.toc_view,
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
        let mut reload_text_action = false;
        let mut close_tab_action = false;
        let mut library_actions = LibraryActions::default();

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
                        &mut reload_text_action,
                        &self.updater,
                        &mut self.pro_upgrade,
                    );
                    if reload_text_action {
                        self.reload_current_text_encoding();
                    }
                } else if self.show_library || (self.document.is_none() && self.tabs.is_empty()) {
                    let settings = &self.settings;
                    let theme = &self.theme;
                    self.library_view.render(
                        ui,
                        settings,
                        &mut self.library,
                        theme,
                        settings.ui_language,
                        &mut library_actions,
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
                        && !self.show_library
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
                            let smart_scan_allowed =
                                self.settings.can_use(ProFeature::SmartScanProcessing);
                            let djvu_query = if self.search.is_open
                                && self
                                    .document
                                    .as_ref()
                                    .is_some_and(|d| d.format_name() == "DjVu")
                            {
                                Some(self.search.query.clone())
                            } else {
                                None
                            };
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
                                djvu_query.as_deref(),
                                &mut on_translate,
                                smart_scan_allowed,
                                &mut pro_required,
                            );
                        }
                        DocumentViewKind::Comic => {
                            let smart_scan_allowed =
                                self.settings.can_use(ProFeature::SmartScanProcessing);
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
                                smart_scan_allowed,
                                &mut pro_required,
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
                                &mut self.text_images,
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
                        if self.settings.can_use(ProFeature::Translation) {
                            let target_code = self.settings.ui_language.resolve().code();
                            self.translate.translate(
                                text,
                                target_code,
                                self.settings.translation.provider,
                                &self.settings.translation.deepl_api_key,
                            );
                        } else {
                            pro_required = Some(ProFeature::Translation);
                        }
                    }
                }
            });

        let mut post_central = PostCentralActions {
            library_actions,
            close_tab_action,
            on_open_file,
            on_open_folder,
            on_load_recent,
            save_settings_action,
            pro_required,
            ambient_allowed,
            neural_tts_allowed,
        };
        self::post_central::render(self, &ctx, &mut post_central);
        let save_settings_action = post_central.save_settings_action;

        if toggle_autoscroll_action {
            self.autoscroll.toggle(
                self.current_scroll_y,
                self.max_scroll_y,
                self.settings.auto_scroll.step_distance,
            );
        }
        if save_settings_action {
            self.flush_settings_now();
            if crate::settings::AppSettings::last_save_error().is_none()
                && crate::settings::AppSettings::last_secret_error().is_none()
            {
                self.status_message = Some(
                    self.settings
                        .ui_language
                        .t()
                        .status_settings_saved
                        .to_string(),
                );
            }
        }
        if register_assoc_action {
            self.register_file_association();
        }
        self.accrue_reading_time(&ctx);
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
