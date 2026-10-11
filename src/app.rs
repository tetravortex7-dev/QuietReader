use std::path::PathBuf;
use eframe::egui::{self, Color32, Panel, CentralPanel, RichText, ScrollArea, Vec2};
use crate::document::{Document, Fb2Document, TocItem};
use crate::settings::AppSettings;

#[derive(PartialEq, Clone, Copy)]
pub enum ReaderTheme {
    Dark,
    Light,
    Sepia,
}

impl ReaderTheme {
    pub fn bg_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(26, 26, 28),
            ReaderTheme::Light => Color32::from_rgb(248, 249, 250),
            ReaderTheme::Sepia => Color32::from_rgb(244, 237, 222),
        }
    }

    pub fn text_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(220, 220, 220),
            ReaderTheme::Light => Color32::from_rgb(33, 37, 41),
            ReaderTheme::Sepia => Color32::from_rgb(60, 48, 36),
        }
    }

    pub fn panel_bg_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(35, 35, 38),
            ReaderTheme::Light => Color32::from_rgb(238, 240, 244),
            ReaderTheme::Sepia => Color32::from_rgb(235, 227, 210),
        }
    }

    pub fn accent_color(&self) -> Color32 {
        match self {
            ReaderTheme::Dark => Color32::from_rgb(100, 160, 255),
            ReaderTheme::Light => Color32::from_rgb(0, 102, 204),
            ReaderTheme::Sepia => Color32::from_rgb(140, 90, 40),
        }
    }
}

pub struct QuietReaderApp {
    document: Option<Box<dyn Document>>,
    current_file_path: Option<PathBuf>,
    current_chapter: usize,
    last_rendered_chapter: usize,
    pending_scroll_y: Option<f32>,
    current_scroll_y: f32,
    viewport_height: f32,
    max_scroll_y: f32,
    font_size: f32,
    theme: ReaderTheme,
    show_toc: bool,
    keep_screen_on: bool,
    show_association_dialog: bool,
    show_recent_dialog: bool,
    settings: AppSettings,
    status_message: Option<String>,
}

impl Default for QuietReaderApp {
    fn default() -> Self {
        let settings = AppSettings::load();
        let is_default = crate::platform::check_fb2_association_status() == crate::platform::AssociationStatus::Default;
        let dismissed = crate::platform::is_association_prompt_dismissed();
        let theme = match settings.theme.as_str() {
            "Dark" => ReaderTheme::Dark,
            "Light" => ReaderTheme::Light,
            _ => ReaderTheme::Sepia,
        };
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
            show_association_dialog: !is_default && !dismissed,
            show_recent_dialog: false,
            settings,
            status_message: None,
        }
    }
}

impl Drop for QuietReaderApp {
    fn drop(&mut self) {
        crate::platform::set_keep_screen_on(false);
        self.sync_and_save_settings();
    }
}

impl QuietReaderApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>, settings: AppSettings) -> Self {
        let is_default = crate::platform::check_fb2_association_status() == crate::platform::AssociationStatus::Default;
        let dismissed = crate::platform::is_association_prompt_dismissed();
        let theme = match settings.theme.as_str() {
            "Dark" => ReaderTheme::Dark,
            "Light" => ReaderTheme::Light,
            _ => ReaderTheme::Sepia,
        };

        let mut app = Self {
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
            show_association_dialog: !is_default && !dismissed,
            show_recent_dialog: false,
            settings,
            status_message: None,
        };

        crate::platform::set_keep_screen_on(app.keep_screen_on);

        if let Some(path) = initial_file {
            if path.exists() && path.is_file() {
                app.load_file(path);
            }
        } else if let Some(last_path) = app.settings.last_opened_book.clone() {
            // Если открывается само приложение — открываем последнюю книгу на том же месте
            if last_path.exists() && last_path.is_file() {
                app.load_file(last_path);
            }
        }

        app
    }

    pub fn sync_and_save_settings(&mut self) {
        if let (Some(doc), Some(path)) = (&self.document, &self.current_file_path) {
            let meta = doc.metadata();
            self.settings.record_book_progress(
                path,
                &meta.title,
                &meta.author,
                self.current_chapter,
                self.current_scroll_y,
            );
        }
        self.settings.font_size = self.font_size;
        self.settings.keep_screen_on = self.keep_screen_on;
        self.settings.show_toc = self.show_toc;
        self.settings.theme = match self.theme {
            ReaderTheme::Dark => "Dark",
            ReaderTheme::Light => "Light",
            ReaderTheme::Sepia => "Sepia",
        }.to_string();
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

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Книги FB2 (*.fb2)", &["fb2", "xml"])
            .pick_file()
        {
            self.load_file(path);
        }
    }

    pub fn load_file(&mut self, path: PathBuf) {
        match Fb2Document::from_file(&path) {
            Ok(doc) => {
                let title = doc.metadata().title.clone();
                let author = doc.metadata().author.clone();
                let total_chapters = doc.chapters().len();

                let (ch, scroll_y) = if let Some((saved_ch, saved_scroll)) = self.settings.get_book_progress(&path) {
                    (saved_ch.min(total_chapters.saturating_sub(1)), saved_scroll)
                } else {
                    (0, 0.0)
                };

                self.current_file_path = Some(path.clone());
                self.document = Some(Box::new(doc));
                self.current_chapter = ch;
                self.last_rendered_chapter = ch;
                self.pending_scroll_y = Some(scroll_y);
                self.current_scroll_y = scroll_y;

                self.settings.record_book_progress(&path, &title, &author, ch, scroll_y);
                let _ = self.settings.save();

                let status = if ch > 0 || scroll_y > 10.0 {
                    format!("Открыта книга: {title} (продолжение с раздела {})", ch + 1)
                } else {
                    format!("Открыта книга: {title}")
                };
                self.status_message = Some(status);
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка открытия файла: {e}"));
            }
        }
    }
}

impl eframe::App for QuietReaderApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Track window position and size for persistence
        ctx.input(|i| {
            let rect = i.viewport().outer_rect.or(i.viewport().inner_rect);
            let is_maximized = i.viewport().maximized.unwrap_or(false);
            if let Some(r) = rect {
                self.settings.update_window_state(
                    Some(r.min.x),
                    Some(r.min.y),
                    r.width(),
                    r.height(),
                    is_maximized,
                );
            }
        });

        // Handle drag & drop
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        for dropped in dropped_files {
            let path = dropped.path();
            if path.exists() {
                self.load_file(path.to_path_buf());
                break;
            }
        }

        // Auto scroll to top when chapter changes
        if self.current_chapter != self.last_rendered_chapter {
            self.pending_scroll_y = Some(0.0);
            self.last_rendered_chapter = self.current_chapter;
        }

        let total_chapters = self.document.as_ref().map(|d| d.chapters().len()).unwrap_or(0);

        // Handle keyboard navigation:
        // PageDown / Space -> scroll down by page
        // PageUp / Shift+Space -> scroll up by page
        // ArrowDown / ArrowUp -> scroll by line
        // Ctrl+ArrowRight / Ctrl+ArrowLeft -> jump to next / previous chapter
        let (page_down, page_up, line_down, line_up, next_chapter, prev_chapter) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command || i.modifiers.alt;
            let shift = i.modifiers.shift;

            let page_down = i.key_pressed(egui::Key::PageDown) || (!shift && i.key_pressed(egui::Key::Space));
            let page_up = i.key_pressed(egui::Key::PageUp) || (shift && i.key_pressed(egui::Key::Space));

            let line_down = i.key_pressed(egui::Key::ArrowDown);
            let line_up = i.key_pressed(egui::Key::ArrowUp);

            let next_ch = (ctrl && i.key_pressed(egui::Key::ArrowRight)) || (ctrl && i.key_pressed(egui::Key::ArrowDown));
            let prev_ch = (ctrl && i.key_pressed(egui::Key::ArrowLeft)) || (ctrl && i.key_pressed(egui::Key::ArrowUp));

            (page_down, page_up, line_down, line_up, next_ch, prev_ch)
        });

        let page_step = (self.viewport_height * 0.85).max(200.0);
        let line_step = 60.0;

        if next_chapter && self.current_chapter + 1 < total_chapters {
            self.current_chapter += 1;
            self.pending_scroll_y = Some(0.0);
            self.last_rendered_chapter = self.current_chapter;
        } else if prev_chapter && self.current_chapter > 0 {
            self.current_chapter -= 1;
            self.pending_scroll_y = Some(0.0);
            self.last_rendered_chapter = self.current_chapter;
        } else if page_down {
            if self.current_scroll_y + 15.0 >= self.max_scroll_y && self.current_chapter + 1 < total_chapters {
                // When reached the end of the chapter, pressing PageDown moves to next chapter
                self.current_chapter += 1;
                self.pending_scroll_y = Some(0.0);
                self.last_rendered_chapter = self.current_chapter;
            } else {
                let target = (self.current_scroll_y + page_step).min(self.max_scroll_y);
                self.pending_scroll_y = Some(target);
            }
        } else if page_up {
            if self.current_scroll_y <= 5.0 && self.current_chapter > 0 {
                self.current_chapter -= 1;
                self.pending_scroll_y = Some(0.0);
                self.last_rendered_chapter = self.current_chapter;
            } else {
                let target = (self.current_scroll_y - page_step).max(0.0);
                self.pending_scroll_y = Some(target);
            }
        } else if line_down {
            let target = (self.current_scroll_y + line_step).min(self.max_scroll_y);
            self.pending_scroll_y = Some(target);
        } else if line_up {
            let target = (self.current_scroll_y - line_step).max(0.0);
            self.pending_scroll_y = Some(target);
        }

        // Apply theme colors
        let mut visuals = if self.theme == ReaderTheme::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.theme.panel_bg_color();
        ctx.set_visuals(visuals);

        // Top Control Bar
        Panel::top("top_bar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("📂 Открыть FB2...").clicked() {
                    self.open_file_dialog();
                }

                if ui.button("📚 Недавние книги...").on_hover_text("Список недавно открытых книг").clicked() {
                    self.show_recent_dialog = !self.show_recent_dialog;
                }

                ui.separator();

                if self.document.is_some() {
                    let toc_text = if self.show_toc { "📖 Скрыть оглавление" } else { "📖 Оглавление" };
                    if ui.button(toc_text).clicked() {
                        self.show_toc = !self.show_toc;
                    }
                    ui.separator();
                }

                // Font size controls
                ui.label("Размер шрифта:");
                if ui.button("➖").clicked() && self.font_size > 12.0 {
                    self.font_size -= 1.0;
                }
                ui.label(format!("{:.0}", self.font_size));
                if ui.button("➕").clicked() && self.font_size < 36.0 {
                    self.font_size += 1.0;
                }

                ui.separator();

                // Theme picker
                ui.label("Тема:");
                let _ = ui.selectable_value(&mut self.theme, ReaderTheme::Sepia, "Сепия");
                let _ = ui.selectable_value(&mut self.theme, ReaderTheme::Light, "Светлая");
                let _ = ui.selectable_value(&mut self.theme, ReaderTheme::Dark, "Тёмная");

                ui.separator();

                if ui.checkbox(&mut self.keep_screen_on, "🔆 Не выключать экран")
                    .on_hover_text("Предотвращает переход экрана в спящий режим во время чтения")
                    .changed()
                {
                    crate::platform::set_keep_screen_on(self.keep_screen_on);
                }

                ui.separator();

                let assoc_status = crate::platform::check_fb2_association_status();
                let (assoc_btn_text, assoc_tooltip) = match assoc_status {
                    crate::platform::AssociationStatus::Default => (
                        "✔ FB2 по умолчанию",
                        "QuietReader назначен читалкой по умолчанию для файлов .fb2 в Windows",
                    ),
                    _ => (
                        "📌 Сделать по умолчанию (.fb2)",
                        "Зарегистрировать QuietReader для файлов .fb2 и открыть параметры Windows",
                    ),
                };

                if ui.button(assoc_btn_text).on_hover_text(assoc_tooltip).clicked() {
                    self.register_file_association();
                }

                // Book Title in top bar
                if let Some(doc) = &self.document {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let meta = doc.metadata();
                        let title_summary = if meta.author.is_empty() {
                            meta.title.clone()
                        } else {
                            format!("{} — {}", meta.author, meta.title)
                        };
                        ui.label(RichText::new(title_summary).strong());
                    });
                }
            });
        });

        // Bottom Status Bar
        Panel::bottom("bottom_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref msg) = self.status_message {
                    ui.label(msg);
                } else {
                    ui.label("Готов к чтению");
                }

                if let Some(doc) = &self.document {
                    let total = doc.chapters().len();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Раздел {} из {} (Клавиши: PageDown / PageUp, Ctrl+Влево / Ctrl+Вправо)", self.current_chapter + 1, total));
                    });
                }
            });
        });

        // Left Panel: Table of Contents (TOC)
        if self.show_toc {
            if let Some(doc) = &self.document {
                let available_w = ui.available_width();
                let max_toc_w = (available_w * 0.45).clamp(200.0, 480.0);
                let mut close_toc = false;

                Panel::left("toc_panel")
                    .resizable(true)
                    .show_separator_line(true)
                    .min_size(140.0)
                    .max_size(max_toc_w)
                    .default_size(260.0)
                    .show(ui, |ui| {
                        ui.style_mut().interaction.resize_grab_radius_side = 8.0;
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        ui.horizontal(|ui| {
                            ui.heading("Оглавление");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("×").on_hover_text("Скрыть оглавление").clicked() {
                                    close_toc = true;
                                }
                            });
                        });
                        ui.add_space(4.0);

                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_max_width(ui.available_width());
                                let toc = doc.toc();
                                if toc.is_empty() {
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new("В этой книге нет оглавления")
                                            .italics()
                                            .color(Color32::from_gray(140)),
                                    );
                                } else {
                                    for item in toc {
                                        render_toc_item(
                                            ui,
                                            item,
                                            self.current_chapter,
                                            &mut self.current_chapter,
                                            self.theme.accent_color(),
                                        );
                                    }
                                }
                            });
                    });

                if close_toc {
                    self.show_toc = false;
                }
            }
        }

        // Central Reading Area
        CentralPanel::default()
            .frame(egui::Frame::default().fill(self.theme.bg_color()).inner_margin(16.0))
            .show(ui, |ui| {
                if let Some(doc) = &self.document {
                    let total_chapters = doc.chapters().len();
                    let chapter = doc.chapters().get(self.current_chapter);

                    let mut scroll_area = ScrollArea::vertical()
                        .auto_shrink([false, false]);

                    if let Some(offset) = self.pending_scroll_y.take() {
                        scroll_area = scroll_area.vertical_scroll_offset(offset);
                    }

                    let scroll_output = scroll_area.show(ui, |ui| {
                        // Centered readable column container (max width 760px)
                        ui.vertical_centered(|ui| {
                            ui.set_max_width(760.0);

                            if let Some(ch) = chapter {
                                ui.add_space(20.0);
                                if !ch.title.is_empty() {
                                    let title_heading = RichText::new(&ch.title)
                                        .size(self.font_size * 1.35)
                                        .strong()
                                        .color(self.theme.text_color());
                                    ui.label(title_heading);
                                    ui.add_space(24.0);
                                }

                                // Render paragraphs
                                for paragraph in &ch.paragraphs {
                                    if paragraph.is_empty() {
                                        ui.add_space(self.font_size * 0.7);
                                    } else {
                                        let text = RichText::new(format!("    {paragraph}"))
                                            .size(self.font_size)
                                            .color(self.theme.text_color());
                                        ui.add(egui::Label::new(text).wrap());
                                        ui.add_space(self.font_size * 0.4);
                                    }
                                }

                                ui.add_space(40.0);

                                // Bottom Chapter Navigation
                                ui.horizontal(|ui| {
                                    if self.current_chapter > 0 {
                                        if ui.button("« Предыдущий раздел").clicked() {
                                            self.current_chapter -= 1;
                                            self.pending_scroll_y = Some(0.0);
                                            self.last_rendered_chapter = self.current_chapter;
                                        }
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if self.current_chapter + 1 < total_chapters {
                                            if ui.button("Следующий раздел »").clicked() {
                                                self.current_chapter += 1;
                                                self.pending_scroll_y = Some(0.0);
                                                self.last_rendered_chapter = self.current_chapter;
                                            }
                                        }
                                    });
                                });

                                ui.add_space(30.0);
                            }
                        });
                    });

                    self.current_scroll_y = scroll_output.state.offset.y;
                    self.viewport_height = scroll_output.inner_rect.height();
                    self.max_scroll_y = (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);

                    // Update reading progress in memory
                    if let (Some(doc), Some(path)) = (&self.document, &self.current_file_path) {
                        let meta = doc.metadata();
                        self.settings.record_book_progress(
                            path,
                            &meta.title,
                            &meta.author,
                            self.current_chapter,
                            self.current_scroll_y,
                        );
                    }
                } else {
                    // Welcome screen when no book is opened
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.heading(
                            RichText::new("📖 QuietReader")
                                .size(36.0)
                                .color(self.theme.accent_color()),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new("Минималистичная, портативная читалка книг")
                                .size(18.0)
                                .color(self.theme.text_color()),
                        );
                        ui.add_space(30.0);

                        let open_btn = egui::Button::new(
                            RichText::new("📂 Выбрать книгу FB2")
                                .size(18.0)
                        ).min_size(Vec2::new(240.0, 48.0));

                        if ui.add(open_btn).clicked() {
                            self.open_file_dialog();
                        }

                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("или просто перетащите FB2-файл в это окно")
                                .size(14.0)
                                .color(Color32::from_gray(140)),
                        );

                        if !self.settings.recent_books.is_empty() {
                            ui.add_space(28.0);
                            ui.separator();
                            ui.add_space(12.0);
                            ui.label(RichText::new("Недавние книги:").strong().size(16.0));
                            ui.add_space(10.0);

                            let mut recent_path_to_open = None;
                            for book in self.settings.recent_books.iter().take(4) {
                                let label = if book.author.is_empty() {
                                    format!("📖 {} (Раздел {})", book.title, book.chapter + 1)
                                } else {
                                    format!("📖 {} — {} (Раздел {})", book.author, book.title, book.chapter + 1)
                                };
                                if ui.button(RichText::new(label).size(14.0)).clicked() {
                                    recent_path_to_open = Some(book.path.clone());
                                }
                                ui.add_space(4.0);
                            }

                            if let Some(path) = recent_path_to_open {
                                self.load_file(path);
                            }
                        }
                    });
                }
            });

        // Unobtrusive prompt for default application association
        if self.show_association_dialog {
            egui::Window::new("📌 Ассоциация файлов FB2")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.set_max_width(420.0);
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Сделать QuietReader основной читалкой для файлов .fb2?")
                            .size(15.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        "QuietReader зарегистрируется в Windows для открытия файлов .fb2 по двойному щелчку и добавится в меню «Открыть с помощью». Также откроются параметры Windows для подтверждения выбора по умолчанию.",
                    );
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("✔ Сделать по умолчанию").strong()).clicked() {
                            self.register_file_association();
                            crate::platform::set_association_prompt_dismissed(true);
                            self.show_association_dialog = false;
                        }
                        if ui.button("✖ Не сейчас").clicked() {
                            crate::platform::set_association_prompt_dismissed(true);
                            self.show_association_dialog = false;
                        }
                    });
                    ui.add_space(4.0);
                });
        }

        // Recent Books Dialog
        if self.show_recent_dialog {
            let mut book_to_open = None;
            let mut clear_recent = false;

            egui::Window::new("📚 Недавние книги")
                .collapsible(false)
                .resizable(true)
                .default_size([520.0, 360.0])
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("История чтения");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                self.show_recent_dialog = false;
                            }
                            if !self.settings.recent_books.is_empty() {
                                if ui.button("🗑 Очистить").on_hover_text("Очистить список недавних книг").clicked() {
                                    clear_recent = true;
                                }
                            }
                        });
                    });
                    ui.separator();

                    if self.settings.recent_books.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Список недавних книг пуст").italics().color(Color32::from_gray(140)));
                        });
                    } else {
                        ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            for book in &self.settings.recent_books {
                                let is_current = self.current_file_path.as_ref() == Some(&book.path);
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            let mut title_text = RichText::new(&book.title).strong().size(15.0);
                                            if is_current {
                                                title_text = title_text.color(self.theme.accent_color());
                                            }
                                            ui.label(title_text);

                                            if !book.author.is_empty() {
                                                ui.label(RichText::new(&book.author).size(13.0).color(Color32::from_gray(160)));
                                            }

                                            let file_name = book.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                            ui.label(
                                                RichText::new(format!("Раздел {} • {}", book.chapter + 1, file_name))
                                                    .size(12.0)
                                                    .color(Color32::from_gray(130)),
                                            );
                                        });

                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let btn_text = if is_current { "Открыта" } else { "Читать »" };
                                            if ui.button(btn_text).clicked() {
                                                book_to_open = Some(book.path.clone());
                                            }
                                        });
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        });
                    }
                });

            if clear_recent {
                self.settings.recent_books.clear();
                self.settings.last_opened_book = None;
                let _ = self.settings.save();
            }

            if let Some(path) = book_to_open {
                self.show_recent_dialog = false;
                self.load_file(path);
            }
        }
    }
}

fn render_toc_item(
    ui: &mut egui::Ui,
    item: &TocItem,
    current_chapter: usize,
    selected_chapter: &mut usize,
    accent_color: Color32,
) {
    let is_selected = current_chapter == item.chapter_index;
    let title = if item.title.trim().is_empty() {
        "—"
    } else {
        item.title.as_str()
    };

    if item.children.is_empty() {
        let mut rich = RichText::new(title);
        if is_selected {
            rich = rich.strong().color(accent_color);
        }
        let btn = egui::Button::selectable(is_selected, rich).wrap();
        if ui.add(btn).clicked() {
            *selected_chapter = item.chapter_index;
        }
    } else {
        let header_id = ui.make_persistent_id(format!("toc_{}_{}", item.chapter_index, item.title));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), header_id, true)
            .show_header(ui, |ui| {
                let mut rich = RichText::new(title);
                if is_selected {
                    rich = rich.strong().color(accent_color);
                }
                let btn = egui::Button::selectable(is_selected, rich).wrap();
                if ui.add(btn).clicked() {
                    *selected_chapter = item.chapter_index;
                }
            })
            .body(|ui| {
                ui.indent(header_id.with("indent"), |ui| {
                    for child in &item.children {
                        render_toc_item(ui, child, current_chapter, selected_chapter, accent_color);
                    }
                });
            });
    }
}

