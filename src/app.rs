use std::path::PathBuf;
use eframe::egui::{self, Color32, Panel, CentralPanel, RichText, ScrollArea, Vec2};
use crate::document::{Document, TocItem};
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
    show_settings_dialog: bool,
    is_fullscreen: bool,
    // Autoscroll runtime state
    is_autoscrolling: bool,
    autoscroll_phase_elapsed: f32,
    autoscroll_in_pause: bool,
    step_start_scroll_y: f32,
    target_step_scroll_y: f32,
    // Middle-click fast scrolling
    middle_drag_anchor: Option<egui::Pos2>,
    settings: AppSettings,
    dont_show_association_again: bool,
    status_message: Option<String>,
    // Comic & Manga runtime state
    current_page: usize,
    comic_textures: std::collections::HashMap<usize, (egui::TextureHandle, u64)>,
    comic_access_counter: u64,
}

impl Default for QuietReaderApp {
    fn default() -> Self {
        let settings = AppSettings::load();
        let assoc_status = crate::platform::check_fb2_association_status();
        let is_already_associated = assoc_status == crate::platform::AssociationStatus::Default
            || assoc_status == crate::platform::AssociationStatus::Registered;
        let dismissed = settings.dismiss_association_prompt || crate::platform::is_association_prompt_dismissed();
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
            show_association_dialog: !is_already_associated && !dismissed,
            show_recent_dialog: false,
            show_settings_dialog: false,
            is_fullscreen: false,
            is_autoscrolling: false,
            autoscroll_phase_elapsed: 0.0,
            autoscroll_in_pause: false,
            step_start_scroll_y: 0.0,
            target_step_scroll_y: 0.0,
            middle_drag_anchor: None,
            settings,
            dont_show_association_again: false,
            status_message: None,
            current_page: 0,
            comic_textures: std::collections::HashMap::new(),
            comic_access_counter: 0,
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
        let assoc_status = crate::platform::check_fb2_association_status();
        let is_already_associated = assoc_status == crate::platform::AssociationStatus::Default
            || assoc_status == crate::platform::AssociationStatus::Registered;
        let dismissed = settings.dismiss_association_prompt || crate::platform::is_association_prompt_dismissed();
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
            show_association_dialog: !is_already_associated && !dismissed,
            show_recent_dialog: false,
            show_settings_dialog: false,
            is_fullscreen: false,
            is_autoscrolling: false,
            autoscroll_phase_elapsed: 0.0,
            autoscroll_in_pause: false,
            step_start_scroll_y: 0.0,
            target_step_scroll_y: 0.0,
            middle_drag_anchor: None,
            settings,
            dont_show_association_again: false,
            status_message: None,
            current_page: 0,
            comic_textures: std::collections::HashMap::new(),
            comic_access_counter: 0,
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
            let saved_ch = if doc.as_comic().is_some() {
                self.current_page
            } else {
                self.current_chapter
            };
            self.settings.record_book_progress(
                path,
                &meta.title,
                &meta.author,
                saved_ch,
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

    pub fn is_comic(&self) -> bool {
        self.document.as_ref().and_then(|d| d.as_comic()).is_some()
    }

    pub fn comic_page_count(&self) -> usize {
        self.document.as_ref().and_then(|d| d.as_comic()).map(|c| c.page_count()).unwrap_or(0)
    }

    pub fn next_comic_page(&mut self) {
        let total = self.comic_page_count();
        if total == 0 {
            return;
        }
        let is_double = self.settings.comic.page_layout == "Double" && self.settings.comic.reading_mode != "Webtoon";
        let step = if is_double { 2 } else { 1 };
        self.current_page = (self.current_page + step).min(total.saturating_sub(1));
        self.sync_and_save_settings();
    }

    pub fn prev_comic_page(&mut self) {
        let is_double = self.settings.comic.page_layout == "Double" && self.settings.comic.reading_mode != "Webtoon";
        let step = if is_double { 2 } else { 1 };
        self.current_page = self.current_page.saturating_sub(step);
        self.sync_and_save_settings();
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Все поддерживаемые книги и комиксы", &["fb2", "epub", "cbz", "cbr", "cb7", "zip", "rar", "7z", "xml"])
            .add_filter("Комиксы и манга (*.cbz, *.cbr, *.cb7, *.zip)", &["cbz", "cbr", "cb7", "zip", "rar", "7z"])
            .add_filter("Книги EPUB (*.epub)", &["epub"])
            .add_filter("Книги FB2 (*.fb2)", &["fb2", "xml"])
            .pick_file()
        {
            self.load_file(path);
        }
    }

    fn open_folder_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Выберите папку с изображениями комикса или манги")
            .pick_folder()
        {
            self.load_file(path);
        }
    }

    fn load_page_texture(&mut self, ctx: &egui::Context, page_idx: usize) -> Option<egui::TextureHandle> {
        self.comic_access_counter += 1;
        let counter = self.comic_access_counter;

        if let Some(entry) = self.comic_textures.get_mut(&page_idx) {
            entry.1 = counter;
            return Some(entry.0.clone());
        }

        let doc = self.document.as_ref()?;
        let comic = doc.as_comic()?;

        match comic.load_page_image(page_idx) {
            Ok(img) => {
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [img.width as usize, img.height as usize],
                    &img.rgba,
                );
                let texture = ctx.load_texture(
                    format!("comic_page_{}", page_idx),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );

                const MAX_CACHED_TEXTURES: usize = 16;
                if self.comic_textures.len() >= MAX_CACHED_TEXTURES {
                    if let Some((&oldest_key, _)) = self.comic_textures.iter().min_by_key(|(_, (_, acc))| *acc) {
                        self.comic_textures.remove(&oldest_key);
                    }
                }

                self.comic_textures.insert(page_idx, (texture.clone(), counter));
                Some(texture)
            }
            Err(e) => {
                self.status_message = Some(format!("Ошибка загрузки страницы {}: {e}", page_idx + 1));
                None
            }
        }
    }

    fn render_comic_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (page_count, _is_manga_doc) = if let Some(doc) = &self.document {
            if let Some(comic) = doc.as_comic() {
                (comic.page_count(), comic.is_manga())
            } else {
                return;
            }
        } else {
            return;
        };

        if page_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("В этом комиксе/манге нет страниц").color(self.theme.text_color()));
            });
            return;
        }

        self.current_page = self.current_page.min(page_count.saturating_sub(1));
        let reading_mode = self.settings.comic.reading_mode.clone();
        let is_webtoon = reading_mode == "Webtoon";
        let is_manga = reading_mode == "Manga";
        let is_double = self.settings.comic.page_layout == "Double" && !is_webtoon;

        if is_webtoon {
            // Webtoon mode: continuous vertical scroll
            let mut scroll_area = ScrollArea::vertical()
                .auto_shrink([false, false]);

            if let Some(offset) = self.pending_scroll_y.take() {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
            }

            let scroll_output = scroll_area.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let avail_w = ui.available_width().min(900.0);
                    for p in 0..page_count {
                        if let Some(tex) = self.load_page_texture(ctx, p) {
                            let tex_size = tex.size_vec2();
                            let aspect = if tex_size.x > 0.0 { tex_size.y / tex_size.x } else { 1.4 };
                            let display_w = avail_w;
                            let display_h = display_w * aspect;
                            let img_widget = egui::Image::from_texture(&tex)
                                .fit_to_exact_size(Vec2::new(display_w, display_h));
                            ui.add(img_widget);
                            ui.add_space(2.0);
                        } else {
                            ui.label(format!("Загрузка страницы {}...", p + 1));
                        }
                    }
                });
            });

            self.current_scroll_y = scroll_output.state.offset.y;
            self.viewport_height = scroll_output.inner_rect.height();
            self.max_scroll_y = (scroll_output.content_size.y - self.viewport_height).max(0.0);
        } else {
            // Single or Double page mode
            let avail_size = ui.available_size();

            let pages_to_show: Vec<usize> = if is_double && self.current_page + 1 < page_count {
                if is_manga {
                    // Right-to-Left: right page is current, left page is current + 1
                    vec![self.current_page + 1, self.current_page]
                } else {
                    // Left-to-Right: left page is current, right page is current + 1
                    vec![self.current_page, self.current_page + 1]
                }
            } else {
                vec![self.current_page]
            };

            // Preload next and prev pages into memory
            if self.current_page + 1 < page_count {
                let _ = self.load_page_texture(ctx, self.current_page + 1);
            }
            if self.current_page > 0 {
                let _ = self.load_page_texture(ctx, self.current_page - 1);
            }

            let mut prev_clicked = false;
            let mut next_clicked = false;

            ui.horizontal_centered(|ui| {
                let num_pages = pages_to_show.len() as f32;
                let target_w_per_page = (avail_size.x - 20.0) / num_pages;

                for &p in &pages_to_show {
                    if let Some(tex) = self.load_page_texture(ctx, p) {
                        let tex_size = tex.size_vec2();
                        let (display_w, display_h) = match self.settings.comic.fit_mode.as_str() {
                            "FitHeight" => {
                                let h = (avail_size.y - 16.0).max(100.0);
                                let w = h * (tex_size.x / tex_size.y);
                                (w, h)
                            }
                            "FitWidth" => {
                                let w = (target_w_per_page - 8.0).max(100.0);
                                let h = w * (tex_size.y / tex_size.x);
                                (w, h)
                            }
                            "Original" => (tex_size.x, tex_size.y),
                            _ /* FitWindow */ => {
                                let max_w = (target_w_per_page - 8.0).max(100.0);
                                let max_h = (avail_size.y - 16.0).max(100.0);
                                let scale = (max_w / tex_size.x).min(max_h / tex_size.y);
                                (tex_size.x * scale, tex_size.y * scale)
                            }
                        };

                        let img_widget = egui::Image::from_texture(&tex)
                            .fit_to_exact_size(Vec2::new(display_w, display_h))
                            .sense(egui::Sense::click());
                        let img_resp = ui.add(img_widget);

                        if img_resp.clicked() {
                            if let Some(pos) = img_resp.interact_pointer_pos() {
                                let is_left = pos.x < img_resp.rect.center().x;
                                if is_manga {
                                    if is_left { next_clicked = true; } else { prev_clicked = true; }
                                } else {
                                    if is_left { prev_clicked = true; } else { next_clicked = true; }
                                }
                            }
                        }
                    } else {
                        ui.label(format!("Загрузка страницы {}...", p + 1));
                    }
                }
            });

            if next_clicked {
                self.next_comic_page();
            } else if prev_clicked {
                self.prev_comic_page();
            }
        }
    }

    pub fn load_file(&mut self, path: PathBuf) {
        match crate::document::open_document(&path) {
            Ok(doc) => {
                let title = doc.metadata().title.clone();
                let author = doc.metadata().author.clone();
                let total_chapters = doc.chapters().len();
                let is_comic = doc.as_comic().is_some();
                let comic_page_count = doc.as_comic().map(|c| c.page_count()).unwrap_or(0);

                let (ch, scroll_y) = if let Some((saved_ch, saved_scroll)) = self.settings.get_book_progress(&path) {
                    (saved_ch, saved_scroll)
                } else {
                    (0, 0.0)
                };

                self.comic_textures.clear();
                self.current_page = if is_comic && comic_page_count > 0 {
                    ch.min(comic_page_count.saturating_sub(1))
                } else {
                    0
                };

                let effective_ch = if is_comic {
                    0
                } else {
                    ch.min(total_chapters.saturating_sub(1))
                };

                if let Some(comic) = doc.as_comic() {
                    if comic.is_manga() {
                        self.settings.comic.reading_mode = "Manga".to_string();
                    }
                }

                self.current_file_path = Some(path.clone());
                self.document = Some(doc);
                self.current_chapter = effective_ch;
                self.last_rendered_chapter = effective_ch;
                self.pending_scroll_y = Some(scroll_y);
                self.current_scroll_y = scroll_y;

                self.settings.record_book_progress(
                    &path,
                    &title,
                    &author,
                    if is_comic { self.current_page } else { effective_ch },
                    scroll_y,
                );
                let _ = self.settings.save();

                let status = if is_comic {
                    if self.current_page > 0 {
                        format!("Открыт комикс/манга: {title} (стр. {}/{})", self.current_page + 1, comic_page_count)
                    } else {
                        format!("Открыт комикс/манга: {title} (всего {} стр.)", comic_page_count)
                    }
                } else if effective_ch > 0 || scroll_y > 10.0 {
                    format!("Открыта книга: {title} (продолжение с раздела {})", effective_ch + 1)
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

        // Set native window title based on current document
        let window_title = if let Some(doc) = &self.document {
            let meta = doc.metadata();
            if meta.author.is_empty() {
                format!("{} — QuietReader", meta.title)
            } else {
                format!("{} — {} — QuietReader", meta.title, meta.author)
            }
        } else {
            "QuietReader".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title));

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
        // F -> toggle fullscreen
        // Escape -> exit fullscreen (or stop autoscroll if active)
        // A -> toggle autoscroll
        // PageDown / Space -> scroll down by page
        // PageUp / Shift+Space -> scroll up by page
        // ArrowDown / ArrowUp -> scroll by line
        // Ctrl+ArrowRight / Ctrl+ArrowLeft -> jump to next / previous chapter
        let (toggle_fs, exit_fs, toggle_autoscroll, page_down, page_up, line_down, line_up, next_chapter, prev_chapter, arrow_left, arrow_right, home_key, end_key, mouse_wheel_y, ctrl_a) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command || i.modifiers.alt;
            let shift = i.modifiers.shift;

            let toggle_fs = !ctrl && i.key_pressed(egui::Key::F);
            let exit_fs = i.key_pressed(egui::Key::Escape);
            let toggle_autoscroll = !ctrl && i.key_pressed(egui::Key::A);

            let page_down = i.key_pressed(egui::Key::PageDown) || (!shift && i.key_pressed(egui::Key::Space));
            let page_up = i.key_pressed(egui::Key::PageUp) || (shift && i.key_pressed(egui::Key::Space));

            let line_down = i.key_pressed(egui::Key::ArrowDown);
            let line_up = i.key_pressed(egui::Key::ArrowUp);
            let arrow_left = !ctrl && i.key_pressed(egui::Key::ArrowLeft);
            let arrow_right = !ctrl && i.key_pressed(egui::Key::ArrowRight);
            let home_key = i.key_pressed(egui::Key::Home);
            let end_key = i.key_pressed(egui::Key::End);
            let mouse_wheel_y = i.smooth_scroll_delta.y;

            let next_ch = (ctrl && i.key_pressed(egui::Key::ArrowRight)) || (ctrl && i.key_pressed(egui::Key::ArrowDown));
            let prev_ch = (ctrl && i.key_pressed(egui::Key::ArrowLeft)) || (ctrl && i.key_pressed(egui::Key::ArrowUp));
            let ctrl_a = (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::A);

            (toggle_fs, exit_fs, toggle_autoscroll, page_down, page_up, line_down, line_up, next_ch, prev_ch, arrow_left, arrow_right, home_key, end_key, mouse_wheel_y, ctrl_a)
        });

        if ctrl_a {
            if let Some(doc) = &self.document {
                if let Some(ch) = doc.chapters().get(self.current_chapter) {
                    let mut s = String::new();
                    if !ch.title.is_empty() {
                        s.push_str(&ch.title);
                        s.push_str("\n\n");
                    }
                    for (i, p) in ch.paragraphs.iter().enumerate() {
                        if i > 0 {
                            s.push_str("\n\n");
                        }
                        s.push_str(p);
                    }
                    ctx.copy_text(s);
                    self.status_message = Some("Весь раздел скопирован в буфер обмена (Ctrl+A)".to_string());
                }
            }
        }

        if toggle_autoscroll && self.document.is_some() {
            self.is_autoscrolling = !self.is_autoscrolling;
            self.autoscroll_phase_elapsed = 0.0;
            self.autoscroll_in_pause = false;
            self.step_start_scroll_y = self.current_scroll_y;
            self.target_step_scroll_y = (self.current_scroll_y + self.settings.auto_scroll.step_distance).min(self.max_scroll_y);
        }

        if toggle_fs {
            self.is_fullscreen = !self.is_fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
        } else if exit_fs {
            if self.is_autoscrolling {
                self.is_autoscrolling = false;
            } else if self.is_fullscreen {
                self.is_fullscreen = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
        }

        let page_step = (self.viewport_height * 0.85).max(200.0);
        let line_step = 60.0;

        // Any manual navigation stops autoscroll to give control to the user
        let manual_scroll = page_down || page_up || line_down || line_up || next_chapter || prev_chapter;
        if manual_scroll {
            self.is_autoscrolling = false;
        }

        if self.is_comic() {
            let is_webtoon = self.settings.comic.reading_mode == "Webtoon";
            let is_manga = self.settings.comic.reading_mode == "Manga";
            let page_count = self.comic_page_count();

            if is_webtoon {
                // In Webtoon mode: ArrowDown/Up scroll by line, PageDown/Up scroll by page
                if page_down {
                    let target = (self.current_scroll_y + page_step).min(self.max_scroll_y);
                    self.pending_scroll_y = Some(target);
                } else if page_up {
                    let target = (self.current_scroll_y - page_step).max(0.0);
                    self.pending_scroll_y = Some(target);
                } else if line_down {
                    let target = (self.current_scroll_y + line_step).min(self.max_scroll_y);
                    self.pending_scroll_y = Some(target);
                } else if line_up {
                    let target = (self.current_scroll_y - line_step).max(0.0);
                    self.pending_scroll_y = Some(target);
                } else if home_key {
                    self.pending_scroll_y = Some(0.0);
                } else if end_key {
                    self.pending_scroll_y = Some(self.max_scroll_y);
                }
            } else {
                // In Page mode (Manga / Western Comic):
                let next_trigger = if is_manga {
                    arrow_left || page_down || (mouse_wheel_y < -15.0)
                } else {
                    arrow_right || page_down || (mouse_wheel_y < -15.0)
                };

                let prev_trigger = if is_manga {
                    arrow_right || page_up || (mouse_wheel_y > 15.0)
                } else {
                    arrow_left || page_up || (mouse_wheel_y > 15.0)
                };

                if next_trigger {
                    self.next_comic_page();
                } else if prev_trigger {
                    self.prev_comic_page();
                } else if home_key {
                    self.current_page = 0;
                    self.sync_and_save_settings();
                } else if end_key && page_count > 0 {
                    self.current_page = page_count.saturating_sub(1);
                    self.sync_and_save_settings();
                }
            }
        } else if let Some(doc) = &self.document {
            let next_non_empty = ((self.current_chapter + 1)..total_chapters)
                .find(|&idx| doc.chapters().get(idx).map(|c| !c.is_empty()).unwrap_or(false));
            let prev_non_empty = (0..self.current_chapter)
                .rev()
                .find(|&idx| doc.chapters().get(idx).map(|c| !c.is_empty()).unwrap_or(false));

            if next_chapter {
                if let Some(next_idx) = next_non_empty {
                    self.current_chapter = next_idx;
                    self.pending_scroll_y = Some(0.0);
                    self.last_rendered_chapter = self.current_chapter;
                }
            } else if prev_chapter {
                if let Some(prev_idx) = prev_non_empty {
                    self.current_chapter = prev_idx;
                    self.pending_scroll_y = Some(0.0);
                    self.last_rendered_chapter = self.current_chapter;
                }
            } else if page_down {
                if self.current_scroll_y + 15.0 >= self.max_scroll_y {
                    if let Some(next_idx) = next_non_empty {
                        self.current_chapter = next_idx;
                        self.pending_scroll_y = Some(0.0);
                        self.last_rendered_chapter = self.current_chapter;
                    }
                } else {
                    let target = (self.current_scroll_y + page_step).min(self.max_scroll_y);
                    self.pending_scroll_y = Some(target);
                }
            } else if page_up {
                if self.current_scroll_y <= 5.0 {
                    if let Some(prev_idx) = prev_non_empty {
                        self.current_chapter = prev_idx;
                        self.pending_scroll_y = Some(0.0);
                        self.last_rendered_chapter = self.current_chapter;
                    }
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

            // Automatic scrolling logic
            if self.is_autoscrolling {
                let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.1);
                ctx.request_repaint();

                let at_bottom = self.current_scroll_y + 4.0 >= self.max_scroll_y;

                if at_bottom {
                    // If reached end of chapter, automatically move to next chapter if available and non-empty
                    if let Some(next_idx) = next_non_empty {
                        self.current_chapter = next_idx;
                        self.pending_scroll_y = Some(0.0);
                        self.last_rendered_chapter = self.current_chapter;
                        self.autoscroll_phase_elapsed = 0.0;
                        self.autoscroll_in_pause = true; // small pause at chapter start
                    } else {
                        // Book is finished
                        self.is_autoscrolling = false;
                    }
                } else if !self.settings.auto_scroll.pause_mode {
                    // Smooth continuous autoscroll
                    let speed = self.settings.auto_scroll.continuous_speed.clamp(5.0, 400.0);
                    let next_y = (self.current_scroll_y + speed * dt).min(self.max_scroll_y);
                    self.pending_scroll_y = Some(next_y);
                } else {
                // Step + Pause mode
                let pause_dur = self.settings.auto_scroll.pause_duration.max(0.2);
                let move_dur = self.settings.auto_scroll.step_duration.max(0.1);

                self.autoscroll_phase_elapsed += dt;

                if self.autoscroll_in_pause {
                    if self.autoscroll_phase_elapsed >= pause_dur {
                        // Switch from pause to moving
                        self.autoscroll_in_pause = false;
                        self.autoscroll_phase_elapsed = 0.0;
                        self.step_start_scroll_y = self.current_scroll_y;
                        self.target_step_scroll_y = (self.current_scroll_y + self.settings.auto_scroll.step_distance).min(self.max_scroll_y);
                    }
                } else {
                    // In moving phase: smooth progress from step_start_scroll_y to target_step_scroll_y
                    let progress = (self.autoscroll_phase_elapsed / move_dur).clamp(0.0, 1.0);
                    // Smooth hermite/ease-in-out interpolation
                    let smooth_t = progress * progress * (3.0 - 2.0 * progress);
                    let current_target = self.step_start_scroll_y + (self.target_step_scroll_y - self.step_start_scroll_y) * smooth_t;
                    self.pending_scroll_y = Some(current_target);

                    if self.autoscroll_phase_elapsed >= move_dur {
                        // Switch from moving to pause
                        self.autoscroll_in_pause = true;
                        self.autoscroll_phase_elapsed = 0.0;
                    }
                }
            }
        }
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
                if ui.button("📂 Открыть...").on_hover_text("Открыть файл книги или комикса (FB2, EPUB, CBZ, CBR, CB7, ZIP, RAR, 7Z)").clicked() {
                    self.open_file_dialog();
                }

                if ui.button("📁 Папка...").on_hover_text("Открыть каталог с изображениями манги или комикса").clicked() {
                    self.open_folder_dialog();
                }

                if ui.button("📚 Недавние...").on_hover_text("Список недавно открытых книг").clicked() {
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

                // Comic controls if a comic is loaded, else text controls
                if self.is_comic() {
                    let total_pages = self.comic_page_count();

                    ui.label("Режим:");
                    egui::ComboBox::from_id_salt("comic_mode_combo")
                        .selected_text(match self.settings.comic.reading_mode.as_str() {
                            "Manga" => "Манга (справа-налево)",
                            "Comic" => "Комикс (слева-направо)",
                            "Webtoon" => "Вебтун (вертикально)",
                            _ => "Комикс",
                        })
                        .show_ui(ui, |ui| {
                            let _ = ui.selectable_value(&mut self.settings.comic.reading_mode, "Manga".to_string(), "Манга (справа-налево)");
                            let _ = ui.selectable_value(&mut self.settings.comic.reading_mode, "Comic".to_string(), "Комикс (слева-направо)");
                            let _ = ui.selectable_value(&mut self.settings.comic.reading_mode, "Webtoon".to_string(), "Вебтун (вертикально)");
                        });

                    if self.settings.comic.reading_mode != "Webtoon" {
                        ui.separator();
                        ui.label("Разворот:");
                        let _ = ui.selectable_value(&mut self.settings.comic.page_layout, "Single".to_string(), "1 стр");
                        let _ = ui.selectable_value(&mut self.settings.comic.page_layout, "Double".to_string(), "2 стр");

                        ui.separator();
                        ui.label("Масштаб:");
                        egui::ComboBox::from_id_salt("comic_fit_combo")
                            .selected_text(match self.settings.comic.fit_mode.as_str() {
                                "FitWindow" => "По окну",
                                "FitWidth" => "По ширине",
                                "FitHeight" => "По высоте",
                                "Original" => "100%",
                                _ => "По окну",
                            })
                            .show_ui(ui, |ui| {
                                let _ = ui.selectable_value(&mut self.settings.comic.fit_mode, "FitWindow".to_string(), "По окну");
                                let _ = ui.selectable_value(&mut self.settings.comic.fit_mode, "FitWidth".to_string(), "По ширине");
                                let _ = ui.selectable_value(&mut self.settings.comic.fit_mode, "FitHeight".to_string(), "По высоте");
                                let _ = ui.selectable_value(&mut self.settings.comic.fit_mode, "Original".to_string(), "Оригинал (100%)");
                            });

                        ui.separator();
                        if ui.button("« Стр").on_hover_text("Предыдущая страница").clicked() {
                            self.prev_comic_page();
                        }

                        let mut page_slider_val = (self.current_page + 1) as i32;
                        let max_p = (total_pages as i32).max(1);
                        if ui.add(egui::Slider::new(&mut page_slider_val, 1..=max_p).text(format!("/ {max_p}"))).changed() {
                            self.current_page = (page_slider_val - 1).clamp(0, max_p - 1) as usize;
                            self.sync_and_save_settings();
                        }

                        if ui.button("Стр »").on_hover_text("Следующая страница").clicked() {
                            self.next_comic_page();
                        }
                    }

                    ui.separator();
                } else {
                    // Font size controls for text books
                    ui.label("Размер шрифта:");
                    if ui.button("➖").clicked() && self.font_size > 12.0 {
                        self.font_size -= 1.0;
                    }
                    ui.label(format!("{:.0}", self.font_size));
                    if ui.button("➕").clicked() && self.font_size < 36.0 {
                        self.font_size += 1.0;
                    }

                    ui.separator();
                }

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

                ui.separator();

                if self.document.is_some() {
                    let scroll_btn_text = if self.is_autoscrolling { "⏸ Стоп авто (A)" } else { "▶ Авто (A)" };
                    if ui.button(scroll_btn_text).on_hover_text("Автоматическая промотка книги вперед (горячая клавиша: A, остановка: A или Esc)").clicked() {
                        self.is_autoscrolling = !self.is_autoscrolling;
                        self.autoscroll_phase_elapsed = 0.0;
                        self.autoscroll_in_pause = false;
                        self.step_start_scroll_y = self.current_scroll_y;
                        self.target_step_scroll_y = (self.current_scroll_y + self.settings.auto_scroll.step_distance).min(self.max_scroll_y);
                    }
                    ui.separator();
                }

                if ui.button("⚙ Настройки...").on_hover_text("Параметры QuietReader и автопрокрутки").clicked() {
                    self.show_settings_dialog = !self.show_settings_dialog;
                }

                ui.separator();

                let fs_text = if self.is_fullscreen { "🗗 Оконный режим (F / Esc)" } else { "⛶ На весь экран (F)" };
                if ui.button(fs_text).on_hover_text("Полноэкранный режим чтения (горячая клавиша: F, выход: F или Esc)").clicked() {
                    self.is_fullscreen = !self.is_fullscreen;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.is_fullscreen));
                }
            });
        });

        // Bottom Status Bar
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
                    ui.separator();
                } else if let Some(ref msg) = self.status_message {
                    ui.label(msg);
                } else {
                    ui.label("Готов к чтению");
                }

                if self.is_comic() {
                    let total = self.comic_page_count();
                    let percent = if total > 0 {
                        ((self.current_page + 1) as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("Страница {} из {}  •  {:.1}%", self.current_page + 1, total, percent))
                                .strong(),
                        );
                    });
                } else if let Some(doc) = &self.document {
                    let total = doc.chapters().len();
                    let percent = if total > 0 {
                        // Chapter fraction + scroll fraction within chapter
                        let chapter_frac = self.current_chapter as f32;
                        let scroll_frac = if self.max_scroll_y > 1.0 {
                            (self.current_scroll_y / self.max_scroll_y).clamp(0.0, 100.0)
                        } else {
                            1.0
                        };
                        let total_frac = (chapter_frac + scroll_frac) / (total as f32);
                        (total_frac * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("Раздел {} из {}  •  {:.1}%", self.current_chapter + 1, total, percent))
                                .strong(),
                        );
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
                                if let Some(comic) = doc.as_comic() {
                                    let page_count = comic.page_count();
                                    let toc = doc.toc();

                                    // If there are multiple chapters/subfolders, show chapter list
                                    if toc.len() > 1 {
                                        ui.label(RichText::new("Главы / Разделы:").strong());
                                        ui.add_space(4.0);
                                        for item in toc {
                                            render_toc_item(
                                                ui,
                                                item,
                                                self.current_chapter,
                                                &mut self.current_chapter,
                                                self.theme.accent_color(),
                                            );
                                        }
                                        ui.add_space(8.0);
                                        ui.separator();
                                        ui.add_space(4.0);
                                    }

                                    // Page list
                                    ui.label(RichText::new(format!("Страницы ({page_count}):")).strong());
                                    ui.add_space(4.0);

                                    let is_double = self.settings.comic.page_layout == "Double" && self.settings.comic.reading_mode != "Webtoon";
                                    let step = if is_double { 2 } else { 1 };

                                    let mut new_comic_page = None;
                                    for p in (0..page_count).step_by(step) {
                                        let is_current = if is_double {
                                            self.current_page == p || (p + 1 < page_count && self.current_page == p + 1)
                                        } else {
                                            self.current_page == p
                                        };

                                        let label = if is_double && p + 1 < page_count {
                                            format!("Стр. {}-{} ({})", p + 1, p + 2, comic.page_name(p))
                                        } else {
                                            format!("Стр. {} ({})", p + 1, comic.page_name(p))
                                        };

                                        let mut rich = RichText::new(label);
                                        if is_current {
                                            rich = rich.strong().color(self.theme.accent_color());
                                        }

                                        let btn = egui::Button::selectable(is_current, rich).wrap();
                                        if ui.add(btn).clicked() {
                                            new_comic_page = Some(p);
                                        }
                                    }

                                    if let Some(p) = new_comic_page {
                                        self.current_page = p;
                                    }
                                } else {
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
                if self.is_comic() {
                    self.render_comic_view(ui, &ctx);
                    return;
                }

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
                                let full_chapter_text = {
                                    let mut s = String::new();
                                    if !ch.title.is_empty() {
                                        s.push_str(&ch.title);
                                        s.push_str("\n\n");
                                    }
                                    for (i, p) in ch.paragraphs.iter().enumerate() {
                                        if i > 0 {
                                            s.push_str("\n\n");
                                        }
                                        s.push_str(p);
                                    }
                                    s
                                };

                                ui.add_space(20.0);
                                if !ch.title.is_empty() {
                                    let title_heading = RichText::new(&ch.title)
                                        .size(self.font_size * 1.35)
                                        .strong()
                                        .color(self.theme.text_color());
                                    let title_text_copy = ch.title.clone();
                                    let full_text_copy = full_chapter_text.clone();
                                    let resp = ui.add(egui::Label::new(title_heading).selectable(true));
                                    resp.context_menu(|ui| {
                                        if ui.button("📋 Копировать").clicked() {
                                            ui.ctx().copy_text(title_text_copy);
                                            ui.close();
                                        }
                                        if ui.button("🔲 Выбрать всё (копировать раздел)").clicked() {
                                            ui.ctx().copy_text(full_text_copy);
                                            ui.close();
                                        }
                                    });
                                    ui.add_space(24.0);
                                }

                                // Render paragraphs
                                for paragraph in &ch.paragraphs {
                                    if paragraph.is_empty() {
                                        ui.add_space(self.font_size * 0.7);
                                    } else {
                                        let text_content = format!("    {paragraph}");
                                        let text = RichText::new(&text_content)
                                            .size(self.font_size)
                                            .color(self.theme.text_color());
                                        let resp = ui.add(egui::Label::new(text).wrap().selectable(true));
                                        let text_to_copy = paragraph.clone();
                                        let full_text_copy = full_chapter_text.clone();
                                        resp.context_menu(|ui| {
                                            if ui.button("📋 Копировать").clicked() {
                                                ui.ctx().copy_text(text_to_copy);
                                                ui.close();
                                            }
                                            if ui.button("🔲 Выбрать всё (копировать раздел)").clicked() {
                                                ui.ctx().copy_text(full_text_copy);
                                                ui.close();
                                            }
                                        });
                                        ui.add_space(self.font_size * 0.4);
                                    }
                                }

                                ui.add_space(40.0);

                                // Bottom Chapter Navigation
                                ui.horizontal(|ui| {
                                    let prev_chapter_idx = (0..self.current_chapter)
                                        .rev()
                                        .find(|&idx| doc.chapters().get(idx).map(|c| !c.is_empty()).unwrap_or(false));

                                    let next_chapter_idx = ((self.current_chapter + 1)..total_chapters)
                                        .find(|&idx| doc.chapters().get(idx).map(|c| !c.is_empty()).unwrap_or(false));

                                    if let Some(prev_idx) = prev_chapter_idx {
                                        if ui.button("« Предыдущий раздел").clicked() {
                                            self.current_chapter = prev_idx;
                                            self.pending_scroll_y = Some(0.0);
                                            self.last_rendered_chapter = self.current_chapter;
                                        }
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if let Some(next_idx) = next_chapter_idx {
                                            if ui.button("Следующий раздел »").clicked() {
                                                self.current_chapter = next_idx;
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

                    // Middle-click fast scrolling logic
                    ctx.input(|i| {
                        let pointer = &i.pointer;
                        if pointer.button_pressed(egui::PointerButton::Middle) {
                            if let Some(pos) = pointer.interact_pos() {
                                if self.middle_drag_anchor.is_some() {
                                    self.middle_drag_anchor = None; // toggle off
                                } else {
                                    self.middle_drag_anchor = Some(pos); // set anchor
                                    self.is_autoscrolling = false; // cancel regular autoscroll
                                }
                            }
                        } else if (pointer.button_pressed(egui::PointerButton::Primary) || pointer.button_pressed(egui::PointerButton::Secondary))
                            && self.middle_drag_anchor.is_some()
                        {
                            self.middle_drag_anchor = None; // cancel on left or right click
                        }
                    });

                    if let Some(anchor) = self.middle_drag_anchor {
                        ctx.request_repaint();
                        if let Some(current_pos) = ctx.pointer_latest_pos() {
                            let delta_y = current_pos.y - anchor.y;
                            // Deadzone of 12px around anchor
                            if delta_y.abs() > 12.0 {
                                let sign = delta_y.signum();
                                let intensity = ((delta_y.abs() - 12.0) / 18.0).powf(1.6);
                                let speed = sign * intensity * 15.0; // speed in px per frame
                                let dt = ctx.input(|i| i.stable_dt).clamp(0.001, 0.1);
                                let delta_scroll = speed * (dt * 60.0);

                                let new_scroll = (self.current_scroll_y + delta_scroll).clamp(0.0, self.max_scroll_y);
                                self.pending_scroll_y = Some(new_scroll);

                                if new_scroll >= self.max_scroll_y && delta_y > 0.0 && self.current_chapter + 1 < total_chapters {
                                    self.current_chapter += 1;
                                    self.pending_scroll_y = Some(0.0);
                                    self.last_rendered_chapter = self.current_chapter;
                                } else if new_scroll <= 0.0 && delta_y < 0.0 && self.current_chapter > 0 {
                                    self.current_chapter -= 1;
                                    self.pending_scroll_y = Some(999999.0); // jump to end of previous chapter
                                    self.last_rendered_chapter = self.current_chapter;
                                }
                            }

                            // Draw visual origin marker and direction line
                            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("middle_scroll_marker")));
                            painter.circle_filled(anchor, 8.0, Color32::from_black_alpha(160));
                            painter.circle_stroke(anchor, 8.0, egui::Stroke::new(2.0, self.theme.accent_color()));
                            if delta_y.abs() > 12.0 {
                                painter.line_segment([anchor, current_pos], egui::Stroke::new(1.5, self.theme.accent_color()));
                            }
                        }
                    }

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
                            RichText::new("Минималистичная, портативная читалка книг, комиксов и манги")
                                .size(18.0)
                                .color(self.theme.text_color()),
                        );
                        ui.add_space(30.0);

                        ui.horizontal(|ui| {
                            let open_btn = egui::Button::new(
                                RichText::new("📂 Выбрать книгу / комикс")
                                    .size(17.0)
                            ).min_size(Vec2::new(260.0, 46.0));

                            if ui.add(open_btn).on_hover_text("FB2, EPUB, CBZ, CBR, CB7, ZIP, RAR, 7Z").clicked() {
                                self.open_file_dialog();
                            }

                            let open_dir_btn = egui::Button::new(
                                RichText::new("📁 Открыть папку манги")
                                    .size(17.0)
                            ).min_size(Vec2::new(240.0, 46.0));

                            if ui.add(open_dir_btn).on_hover_text("Каталог с изображениями (JPG, PNG, WebP)").clicked() {
                                self.open_folder_dialog();
                            }
                        });

                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("или перетащите файл (FB2, EPUB, CBZ, CBR, CB7) или папку с мангой в это окно")
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
                    ui.label(
                        RichText::new("Сделать QuietReader основной читалкой для файлов .fb2?")
                            .size(15.0)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        "QuietReader зарегистрируется в Windows для открытия файлов .fb2 по двойному щелчку и добавится в меню «Открыть с помощью». Также откроются параметры Windows для подтверждения выбора по умолчанию.",
                    );
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.dont_show_association_again, "Не показывать снова");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("✔ Сделать по умолчанию").strong()).clicked() {
                            self.register_file_association();
                            crate::platform::set_association_prompt_dismissed(true);
                            if self.dont_show_association_again {
                                self.settings.dismiss_association_prompt = true;
                                let _ = self.settings.save();
                            }
                            self.show_association_dialog = false;
                        }
                        if ui.button("✖ Не сейчас").clicked() {
                            if self.dont_show_association_again {
                                crate::platform::set_association_prompt_dismissed(true);
                                self.settings.dismiss_association_prompt = true;
                                let _ = self.settings.save();
                            }
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

        // Settings Dialog
        if self.show_settings_dialog {
            egui::Window::new("⚙ Настройки QuietReader")
                .collapsible(false)
                .resizable(true)
                .default_size([540.0, 460.0])
                .min_size([460.0, 360.0])
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Параметры приложения");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Закрыть").clicked() {
                                self.show_settings_dialog = false;
                            }
                        });
                    });
                    ui.separator();

                    ScrollArea::vertical().show(ui, |ui| {
                        // SECTION 1: АВТОМАТИЧЕСКАЯ ПРОМОТКА
                        ui.group(|ui| {
                            ui.label(RichText::new("📜 Автоматическая промотка страниц").strong().size(15.0));
                            ui.add_space(4.0);
                            ui.label("Режим автоматического движения книги без необходимости постоянно использовать колесико или клавиатуру.");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Режим прокрутки:");
                                ui.radio_value(&mut self.settings.auto_scroll.pause_mode, true, "С паузами (шаг + пауза для чтения)");
                                ui.radio_value(&mut self.settings.auto_scroll.pause_mode, false, "Непрерывный (плавный скролл)");
                            });

                            ui.add_space(8.0);

                            if self.settings.auto_scroll.pause_mode {
                                ui.label(RichText::new("Параметры режима с паузами:").italics());
                                ui.add_space(4.0);

                                ui.horizontal(|ui| {
                                    ui.label("Интервал паузы между сдвигами:");
                                    ui.add(egui::Slider::new(&mut self.settings.auto_scroll.pause_duration, 0.5..=15.0).suffix(" сек"));
                                });
                                ui.label(
                                    RichText::new("Время остановки, чтобы глаза успели прочитать отрывок текста.")
                                        .size(12.0)
                                        .color(Color32::from_gray(140)),
                                );

                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label("Размер шага сдвига:");
                                    ui.add(egui::Slider::new(&mut self.settings.auto_scroll.step_distance, 30.0..=350.0).suffix(" px"));
                                });

                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label("Плавность движения (длительность шага):");
                                    ui.add(egui::Slider::new(&mut self.settings.auto_scroll.step_duration, 0.2..=3.0).suffix(" сек"));
                                });
                            } else {
                                ui.label(RichText::new("Параметры непрерывного режима:").italics());
                                ui.add_space(4.0);

                                ui.horizontal(|ui| {
                                    ui.label("Скорость прокрутки:");
                                    ui.add(egui::Slider::new(&mut self.settings.auto_scroll.continuous_speed, 10.0..=250.0).suffix(" px/сек"));
                                });
                            }

                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("Горячая клавиша запуска/паузы: 'A'. Любое ручное листание или нажатие Esc приостанавливает автопрокрутку.")
                                    .size(12.0)
                                    .color(self.theme.accent_color()),
                            );
                        });

                        ui.add_space(10.0);

                        // SECTION 2: ЭКРАН И ЧТЕНИЕ
                        ui.group(|ui| {
                            ui.label(RichText::new("📖 Чтение и экран").strong().size(15.0));
                            ui.add_space(6.0);

                            if ui.checkbox(&mut self.keep_screen_on, "🔆 Не выключать экран во время чтения")
                                .on_hover_text("Предотвращает отключение экрана и засыпание системы в Windows")
                                .changed()
                            {
                                crate::platform::set_keep_screen_on(self.keep_screen_on);
                                self.settings.keep_screen_on = self.keep_screen_on;
                            }

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Размер шрифта:");
                                if ui.button("➖").clicked() && self.font_size > 12.0 {
                                    self.font_size -= 1.0;
                                }
                                ui.label(format!("{:.0}", self.font_size));
                                if ui.button("➕").clicked() && self.font_size < 36.0 {
                                    self.font_size += 1.0;
                                }
                            });
                        });

                        ui.add_space(10.0);

                        // SECTION 3: КОМИКСЫ И МАНГА
                        ui.group(|ui| {
                            ui.label(RichText::new("📖 Комиксы и манга").strong().size(15.0));
                            ui.add_space(4.0);
                            ui.label("Параметры просмотра для графических форматов (CBZ, CBR, CB7, ZIP, RAR, 7Z, папки):");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Режим чтения по умолчанию:");
                                ui.radio_value(&mut self.settings.comic.reading_mode, "Manga".to_string(), "Манга (справа-налево)");
                                ui.radio_value(&mut self.settings.comic.reading_mode, "Comic".to_string(), "Комикс (слева-направо)");
                                ui.radio_value(&mut self.settings.comic.reading_mode, "Webtoon".to_string(), "Вебтун (лента)");
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Разворот страниц:");
                                ui.radio_value(&mut self.settings.comic.page_layout, "Single".to_string(), "Одиночная страница (1)");
                                ui.radio_value(&mut self.settings.comic.page_layout, "Double".to_string(), "Двойной разворот (2)");
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label("Масштабирование страниц:");
                                ui.radio_value(&mut self.settings.comic.fit_mode, "FitWindow".to_string(), "По окну");
                                ui.radio_value(&mut self.settings.comic.fit_mode, "FitWidth".to_string(), "По ширине");
                                ui.radio_value(&mut self.settings.comic.fit_mode, "FitHeight".to_string(), "По высоте");
                                ui.radio_value(&mut self.settings.comic.fit_mode, "Original".to_string(), "100%");
                            });
                        });

                        ui.add_space(10.0);

                        // SECTION 4: О ПРОГРАММЕ И ГОРЯЧИХ КЛАВИШАХ
                        ui.group(|ui| {
                            ui.label(RichText::new("⌨ Горячие клавиши").strong().size(15.0));
                            ui.add_space(4.0);
                            ui.label("• F — Вход и выход из полноэкранного режима");
                            ui.label("• Esc — Выход из полноэкранного режима или пауза автопрокрутки");
                            ui.label("• A — Запуск / остановка автоматической промотки");
                            ui.label("• Пробел / Shift+Пробел — Прокрутка страницы вниз / вверх");
                            ui.label("• PageDown / PageUp — Листание на страницу вниз / вверх");
                            ui.label("• Стрелки Вниз / Вверх — Плавный сдвиг на строку");
                            ui.label("• Стрелки Влево / Вправо — Листание страниц комикса/манги (с учетом направления)");
                            ui.label("• Колесико мыши — Листание страниц в комиксах или вертикальный скролл в вебтунах");
                            ui.label("• Home / End — Переход на первую / последнюю страницу");
                            ui.label("• Ctrl+Вправо / Ctrl+Влево — Переход к следующему / предыдущему разделу книги");
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("💾 Сохранить настройки").clicked() {
                                self.sync_and_save_settings();
                                self.show_settings_dialog = false;
                            }
                        });
                    });
                });
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

