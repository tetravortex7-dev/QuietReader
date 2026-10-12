pub mod autoscroll;
pub mod comic_view;
pub mod dialogs;
pub mod document_loader;
pub mod fonts;
pub mod settings_view;
pub mod text_view;
pub mod theme;
pub mod toc_view;

use crate::core::{Command, ReaderCore};
use crate::document::Document;
use crate::settings::AppSettings;
use eframe::egui::{self, Panel, RichText};
use std::path::PathBuf;

pub use self::autoscroll::AutoscrollController;
pub use self::comic_view::ComicViewState;
pub use self::dialogs::{Dialogs, SettingsTab};
pub use self::fonts::apply_font_family;
pub use self::settings_view::SettingsView;
pub use self::text_view::TextView;
pub use self::theme::ReaderTheme;
pub use self::toc_view::TocView;

/// Главное состояние и координатор приложения QuietReader
pub struct QuietReaderApp {
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

    // Диалоговые окна
    pub show_association_dialog: bool,
    pub dont_show_association_again: bool,
    pub show_recent_dialog: bool,
    pub show_settings_dialog: bool,
    pub settings_tab: SettingsTab,

    // Модули
    pub autoscroll: AutoscrollController,
    pub comic_view: ComicViewState,
    #[allow(dead_code)]
    pub core: ReaderCore,
}

impl Default for QuietReaderApp {
    fn default() -> Self {
        Self::from_settings(AppSettings::load())
    }
}

impl Drop for QuietReaderApp {
    fn drop(&mut self) {
        crate::platform::set_keep_screen_on(false);
        self.sync_and_save_settings();
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
            _ => ReaderTheme::Sepia,
        };

        let show_association_dialog =
            crate::platform::supports_file_association() && !is_already_associated && !dismissed;

        Self {
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
            show_association_dialog,
            dont_show_association_again: false,
            show_recent_dialog: false,
            show_settings_dialog: false,
            settings_tab: SettingsTab::Fonts,
            autoscroll: AutoscrollController::default(),
            comic_view: ComicViewState::default(),
            core: ReaderCore::default(),
        }
    }

    pub fn new(
        cc: &eframe::CreationContext<'_>,
        initial_file: Option<PathBuf>,
        settings: AppSettings,
    ) -> Self {
        apply_font_family(&cc.egui_ctx, &settings.font_family);
        let mut app = Self::from_settings(settings);
        crate::platform::set_keep_screen_on(app.keep_screen_on);

        if let Some(path) = initial_file {
            if path.exists() && path.is_file() {
                app.load_file(path);
            }
        } else if let Some(last_path) = app.settings.last_opened_book.clone()
            && last_path.exists()
            && last_path.is_file()
        {
            app.load_file(last_path);
        }

        app
    }

    pub fn is_comic(&self) -> bool {
        self.document
            .as_ref()
            .and_then(|d| d.page_content())
            .is_some()
    }

    pub fn sync_and_save_settings(&mut self) {
        ReaderCore::record_progress(
            &mut self.settings,
            &self.document,
            &self.current_file_path,
            self.current_chapter,
            self.comic_view.current_page,
            self.current_scroll_y,
        );
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
        }
        .to_string();
        let _ = self.settings.save();
    }

    pub fn register_file_association(&mut self) {
        match crate::platform::register_fb2_association() {
            Ok(msg) => {
                self.status_message = Some(msg);
                crate::platform::open_default_apps_settings();
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка ассоциации: {e}"));
            }
        }
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Все поддерживаемые документы",
                &[
                    "pdf", "fb2", "epub", "txt", "md", "markdown", "log", "cbz", "cbr", "cb7",
                    "zip", "rar", "7z", "xml",
                ],
            )
            .add_filter("Документы PDF (*.pdf)", &["pdf"])
            .add_filter(
                "Комиксы и манга (*.cbz, *.cbr, *.cb7, *.zip)",
                &["cbz", "cbr", "cb7", "zip", "rar", "7z"],
            )
            .add_filter("Книги EPUB (*.epub)", &["epub"])
            .add_filter("Книги FB2 (*.fb2)", &["fb2", "xml"])
            .add_filter(
                "Текстовые файлы (*.txt, *.md, *.log)",
                &["txt", "md", "markdown", "log"],
            )
            .pick_file()
        {
            self.load_file(path);
        }
    }

    pub fn open_folder_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Выберите папку с изображениями комикса или манги")
            .pick_folder()
        {
            self.load_file(path);
        }
    }

    pub fn load_file(&mut self, path: PathBuf) {
        match document_loader::DocumentLoader::load_file(
            path,
            &mut self.settings,
            &mut self.comic_view,
        ) {
            Ok(state) => {
                self.current_file_path = Some(state.file_path);
                self.document = Some(state.document);
                self.current_chapter = state.chapter;
                self.last_rendered_chapter = state.chapter;
                self.pending_scroll_y = Some(state.scroll_y);
                self.current_scroll_y = state.scroll_y;
                self.comic_view.current_page = state.comic_page;
                self.status_message = Some(state.status_message);
            }
            Err(e) => {
                self.status_message = Some(e);
            }
        }
    }

    pub fn set_fullscreen_mode(&mut self, ctx: &egui::Context, fullscreen: bool) {
        if self.is_fullscreen == fullscreen {
            return;
        }
        self.is_fullscreen = fullscreen;

        if !crate::platform::set_window_fullscreen(fullscreen) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(fullscreen));
        }

        ctx.request_repaint();
    }
}

impl eframe::App for QuietReaderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if !self.is_fullscreen {
            if !ctx.input(|i| i.pointer.any_down()) && crate::platform::ensure_window_on_screen() {
                ctx.request_repaint();
            }
            let close_requested = ctx.input(|i| {
                let vp = i.viewport();
                if !vp.minimized.unwrap_or(false)
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
                self.sync_and_save_settings();
            }
        }

        for dropped in ctx.input(|i| i.raw.dropped_files.clone()) {
            let path = dropped.path();
            if !path.as_os_str().is_empty() {
                self.load_file(path.to_path_buf());
                break;
            }
        }

        if self.current_chapter != self.last_rendered_chapter {
            self.pending_scroll_y = Some(0.0);
            self.last_rendered_chapter = self.current_chapter;
        }

        let is_manga = self
            .comic_view
            .is_effective_manga(&self.document, &self.settings.comic);
        let is_comic = self.is_comic();

        let commands = ctx.input(|i| {
            let mut cmds = Vec::new();
            let ctrl = i.modifiers.ctrl || i.modifiers.command || i.modifiers.alt;
            let cmd_mod = i.modifiers.ctrl || i.modifiers.command;
            let shift = i.modifiers.shift;

            if cmd_mod && i.key_pressed(egui::Key::A) {
                cmds.push(Command::CopyChapterText);
            } else if !ctrl && i.key_pressed(egui::Key::A) {
                cmds.push(Command::ToggleAutoscroll);
            }
            if cmd_mod && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
                cmds.push(Command::ZoomIn);
            } else if cmd_mod && i.key_pressed(egui::Key::Minus) {
                cmds.push(Command::ZoomOut);
            } else if cmd_mod && i.key_pressed(egui::Key::Num0) {
                cmds.push(Command::ResetZoom);
            }
            if !ctrl && i.key_pressed(egui::Key::F) {
                cmds.push(Command::ToggleFullscreen);
            } else if i.key_pressed(egui::Key::Escape) {
                cmds.push(Command::ExitFullscreen);
            }
            if ctrl && (i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown))
            {
                cmds.push(Command::NextChapter);
            } else if ctrl
                && (i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp))
            {
                cmds.push(Command::PrevChapter);
            }

            if i.key_pressed(egui::Key::PageDown) || (!shift && i.key_pressed(egui::Key::Space)) {
                cmds.push(Command::PageDown);
            } else if i.key_pressed(egui::Key::PageUp) || (shift && i.key_pressed(egui::Key::Space))
            {
                cmds.push(Command::PageUp);
            } else if i.key_pressed(egui::Key::ArrowDown) {
                cmds.push(Command::LineDown);
            } else if i.key_pressed(egui::Key::ArrowUp) {
                cmds.push(Command::LineUp);
            } else if i.key_pressed(egui::Key::Home) {
                cmds.push(Command::Home);
            } else if i.key_pressed(egui::Key::End) {
                cmds.push(Command::End);
            } else if !ctrl && i.key_pressed(egui::Key::ArrowLeft) {
                cmds.push(if is_manga {
                    Command::NextPage
                } else {
                    Command::PrevPage
                });
            } else if !ctrl && i.key_pressed(egui::Key::ArrowRight) {
                cmds.push(if is_manga {
                    Command::PrevPage
                } else {
                    Command::NextPage
                });
            }
            cmds
        });

        for cmd in &commands {
            match cmd {
                Command::CopyChapterText => {
                    if let Some(doc) = &self.document
                        && let Some(text) =
                            ReaderCore::extract_chapter_text(doc.as_ref(), self.current_chapter)
                    {
                        ctx.copy_text(text);
                        self.status_message =
                            Some("Весь раздел скопирован в буфер обмена (Ctrl+A)".to_string());
                    }
                }
                Command::ZoomIn | Command::ZoomOut | Command::ResetZoom => {
                    self.font_size = match cmd {
                        Command::ZoomIn => (self.font_size + 1.0).clamp(12.0, 36.0),
                        Command::ZoomOut => (self.font_size - 1.0).clamp(12.0, 36.0),
                        _ => 19.0,
                    };
                    self.sync_and_save_settings();
                }
                Command::ToggleAutoscroll => {
                    if self.document.is_some() {
                        self.autoscroll.toggle(
                            self.current_scroll_y,
                            self.max_scroll_y,
                            self.settings.auto_scroll.step_distance,
                        );
                    }
                }
                Command::ToggleFullscreen => {
                    self.set_fullscreen_mode(&ctx, !self.is_fullscreen);
                }
                Command::ExitFullscreen => {
                    if self.autoscroll.is_active() {
                        self.autoscroll.stop();
                    } else if self.is_fullscreen {
                        self.set_fullscreen_mode(&ctx, false);
                    } else if self.show_settings_dialog {
                        self.show_settings_dialog = false;
                    }
                }
                nav_cmd => {
                    if matches!(
                        nav_cmd,
                        Command::PageDown
                            | Command::PageUp
                            | Command::LineDown
                            | Command::LineUp
                            | Command::NextChapter
                            | Command::PrevChapter
                    ) {
                        self.autoscroll.stop();
                    }
                    if is_comic {
                        if ReaderCore::apply_paged_command(
                            nav_cmd,
                            &self.document,
                            &self.settings.comic,
                            &mut self.comic_view.current_page,
                            self.current_scroll_y,
                            self.max_scroll_y,
                            self.viewport_height,
                            &mut self.pending_scroll_y,
                        ) {
                            self.sync_and_save_settings();
                        }
                    } else if let Some(doc) = &self.document {
                        ReaderCore::apply_text_command(
                            nav_cmd,
                            doc.as_ref(),
                            &mut self.current_chapter,
                            &mut self.last_rendered_chapter,
                            self.current_scroll_y,
                            self.max_scroll_y,
                            self.viewport_height,
                            &mut self.pending_scroll_y,
                        );
                    }
                }
            }
        }

        if !is_comic && let Some(doc) = &self.document {
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

        ctx.set_visuals(self.theme.create_visuals());

        let ptr_top = ctx
            .input(|i| i.pointer.latest_pos().map(|p| p.y < 36.0))
            .unwrap_or(false);
        if !self.is_fullscreen || ptr_top || self.show_settings_dialog {
            Panel::top("top_bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📂 Открыть...").on_hover_text("Открыть файл книги, PDF или комикса (PDF, FB2, EPUB, TXT, MD, LOG, CBZ, CBR, CB7, ZIP, RAR, 7Z)").clicked() {
                        self.open_file_dialog();
                    }
                    if ui.button("📁 Папка...").on_hover_text("Открыть каталог с изображениями манги или комикса").clicked() {
                        self.open_folder_dialog();
                    }
                    if ui.button("📚 Недавние...").on_hover_text("Список недавно открытых книг").clicked() {
                        self.show_recent_dialog = !self.show_recent_dialog;
                    }
                    if self.document.is_some() {
                        ui.separator();
                        let toc_text = if self.show_toc { "📖 Скрыть оглавление" } else { "📖 Оглавление" };
                        if ui.button(toc_text).clicked() {
                            self.show_toc = !self.show_toc;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let fs_text = if self.is_fullscreen { "🗗 Оконный (F)" } else { "⛶ На весь экран (F)" };
                        if ui.button(fs_text).on_hover_text("Полноэкранный режим чтения (горячая клавиша: F, выход: F или Esc)").clicked() {
                            self.set_fullscreen_mode(&ctx, !self.is_fullscreen);
                        }
                        if ui.button("⚙ Настройки...").on_hover_text("Шрифты, темы оформления, автопрокрутка, комиксы и горячие клавиши").clicked() {
                            self.show_settings_dialog = !self.show_settings_dialog;
                        }
                    });
                });
            });

            Panel::bottom("bottom_bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(doc) = &self.document {
                        let meta = doc.metadata();
                        let book_info = if meta.author.is_empty() {
                            meta.title.clone()
                        } else {
                            format!("{} — {}", meta.author, meta.title)
                        };
                        ui.label(RichText::new(format!("📖 {book_info}")).strong());
                        if let Some(ref msg) = self.status_message {
                            ui.separator();
                            ui.label(RichText::new(msg).color(self.theme.accent_color()));
                        }
                    } else if let Some(ref msg) = self.status_message {
                        ui.label(msg);
                    } else {
                        ui.label("Готов к чтению");
                    }

                    if self.is_comic() {
                        let total = self.comic_view.page_count(&self.document);
                        let cur = self.comic_view.current_page + 1;
                        let pct = if total > 0 {
                            (cur as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
                        } else {
                            0.0
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("Страница {cur} из {total}  •  {pct:.1}%"))
                                    .strong(),
                            );
                        });
                    } else if let Some(doc) = &self.document {
                        let total = doc.chapters().len();
                        let scroll_frac = if self.max_scroll_y > 1.0 {
                            (self.current_scroll_y / self.max_scroll_y).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        let pct = if total > 0 {
                            ((self.current_chapter as f32 + scroll_frac) / total as f32 * 100.0)
                                .clamp(0.0, 100.0)
                        } else {
                            0.0
                        };
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Раздел {} из {total}  •  {pct:.1}%",
                                    self.current_chapter + 1
                                ))
                                .strong(),
                            );
                        });
                    }
                });
            });
        }

        if !self.is_fullscreen || self.show_toc {
            TocView::render(
                ui,
                &self.document,
                &self.settings.comic,
                &mut self.current_chapter,
                &mut self.comic_view.current_page,
                &mut self.show_toc,
                &self.theme,
            );
        }

        // Центральная область чтения или настроек
        let mut on_open_file = false;
        let mut on_open_folder = false;
        let mut on_load_recent = None;
        let mut toggle_autoscroll_action = false;
        let mut save_settings_action = false;
        let mut register_assoc_action = false;

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
                } else {
                    if self.document.is_some()
                        && (!self.is_comic() || self.settings.comic.reading_mode == "Webtoon")
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

                    if self.is_comic() {
                        self.comic_view.render(
                            ui,
                            &ctx,
                            &self.document,
                            &self.settings.comic,
                            &self.theme,
                            &mut self.pending_scroll_y,
                            &mut self.current_scroll_y,
                            &mut self.viewport_height,
                            &mut self.max_scroll_y,
                            &mut self.status_message,
                        );
                    } else {
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
                            &mut self.status_message,
                            &mut on_open_file,
                            &mut on_open_folder,
                            &mut on_load_recent,
                        );
                    }
                }
            });

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
            self.sync_and_save_settings();
            self.status_message = Some("Настройки сохранены".to_string());
        }
        if register_assoc_action {
            self.register_file_association();
        }
    }
}
