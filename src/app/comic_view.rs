use super::document_loader::format_document_error;
use super::theme::ReaderTheme;
use crate::document::{DecodedImage, Document, DocumentError};
use crate::settings::ComicSettings;
use eframe::egui::{self, RichText, ScrollArea, Vec2};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

struct LoadRequest {
    doc_generation: u64,
    doc: Arc<dyn Document>,
    page_idx: usize,
    ctx: egui::Context,
}

struct LoadResponse {
    doc_generation: u64,
    page_idx: usize,
    result: Result<DecodedImage, DocumentError>,
}

pub struct ComicViewState {
    pub current_page: usize,
    pub textures: HashMap<usize, (egui::TextureHandle, u64)>,
    pub pending_pages: HashSet<usize>,
    pub failed_pages: HashMap<usize, String>,
    pub access_counter: u64,
    pub doc_generation: u64,
    pub pan_offset: Vec2,
    pub is_middle_dragging: bool,
    pub page_input: String,
    req_tx: Sender<LoadRequest>,
    resp_rx: Receiver<LoadResponse>,
}

fn spawn_comic_worker() -> (Sender<LoadRequest>, Receiver<LoadResponse>) {
    let (req_tx, req_rx) = mpsc::channel::<LoadRequest>();
    let (resp_tx, resp_rx) = mpsc::channel::<LoadResponse>();

    std::thread::Builder::new()
        .name("comic-page-worker".to_string())
        .spawn(move || {
            while let Ok(req) = req_rx.recv() {
                let doc = req.doc.clone();
                let page_idx = req.page_idx;
                let load_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Some(page_content) = doc.page_content() {
                        page_content.load_page_image(page_idx)
                    } else {
                        Err(DocumentError::Unsupported(
                            "Документ не поддерживает постраничный режим".to_string(),
                        ))
                    }
                }));

                let result = match load_res {
                    Ok(res) => res,
                    Err(_) => Err(DocumentError::Corrupted(
                        "Аварийный сбой при декодировании страницы комикса".to_string(),
                    )),
                };

                let _ = resp_tx.send(LoadResponse {
                    doc_generation: req.doc_generation,
                    page_idx,
                    result,
                });
                req.ctx.request_repaint();
            }
        })
        .ok();

    (req_tx, resp_rx)
}

impl Default for ComicViewState {
    fn default() -> Self {
        let (req_tx, resp_rx) = spawn_comic_worker();

        Self {
            current_page: 0,
            textures: HashMap::new(),
            pending_pages: HashSet::new(),
            failed_pages: HashMap::new(),
            access_counter: 0,
            doc_generation: 0,
            pan_offset: Vec2::ZERO,
            is_middle_dragging: false,
            page_input: String::new(),
            req_tx,
            resp_rx,
        }
    }
}

impl ComicViewState {
    pub fn clear_cache(&mut self) {
        self.textures.clear();
        self.pending_pages.clear();
        self.failed_pages.clear();
        self.pan_offset = Vec2::ZERO;
        self.doc_generation = self.doc_generation.wrapping_add(1);
    }

    pub fn page_count(&self, document: &Option<Arc<dyn Document>>) -> usize {
        document
            .as_ref()
            .and_then(|d| d.page_content())
            .map(|c| c.page_count())
            .unwrap_or(0)
    }

    pub fn is_effective_manga(
        &self,
        document: &Option<Arc<dyn Document>>,
        settings: &ComicSettings,
    ) -> bool {
        let is_manga_doc = document
            .as_ref()
            .and_then(|d| d.page_content())
            .map(|c| c.is_manga())
            .unwrap_or(false);

        match settings.reading_mode.as_str() {
            "Manga" => true,
            "Comic" => false,
            "Webtoon" => false,
            _ => is_manga_doc,
        }
    }

    pub fn next_page(&mut self, total_pages: usize, settings: &ComicSettings) {
        if total_pages == 0 {
            return;
        }
        let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
        let step = if is_double { 2 } else { 1 };
        self.current_page = (self.current_page + step).min(total_pages.saturating_sub(1));
        self.pan_offset = Vec2::ZERO;
    }

    pub fn prev_page(&mut self, settings: &ComicSettings) {
        let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
        let step = if is_double { 2 } else { 1 };
        self.current_page = self.current_page.saturating_sub(step);
        self.pan_offset = Vec2::ZERO;
    }

    fn poll_responses(&mut self, ctx: &egui::Context, status_message: &mut Option<String>) {
        loop {
            match self.resp_rx.try_recv() {
                Ok(resp) => {
                    if resp.doc_generation != self.doc_generation {
                        continue;
                    }
                    self.pending_pages.remove(&resp.page_idx);
                    match resp.result {
                        Ok(img) => {
                            self.access_counter += 1;
                            let counter = self.access_counter;
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                [img.width as usize, img.height as usize],
                                &img.rgba,
                            );
                            let texture = ctx.load_texture(
                                format!("comic_page_tex_{}_{}", self.doc_generation, resp.page_idx),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );

                            const MAX_CACHED_TEXTURES: usize = 16;
                            if self.textures.len() >= MAX_CACHED_TEXTURES
                                && let Some((&oldest_key, _)) =
                                    self.textures.iter().min_by_key(|(_, (_, acc))| *acc)
                            {
                                self.textures.remove(&oldest_key);
                            }

                            self.textures.insert(resp.page_idx, (texture, counter));
                        }
                        Err(e) => {
                            let msg = format_document_error(&e);
                            *status_message = Some(format!(
                                "Ошибка загрузки страницы {}: {msg}",
                                resp.page_idx + 1
                            ));
                            self.failed_pages.insert(resp.page_idx, msg);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    for page_idx in self.pending_pages.drain() {
                        let msg = "Поток декодирования аварийно завершился".to_string();
                        self.failed_pages.insert(page_idx, msg.clone());
                        *status_message =
                            Some(format!("Ошибка загрузки страницы {}: {msg}", page_idx + 1));
                    }
                    let (tx, rx) = spawn_comic_worker();
                    self.req_tx = tx;
                    self.resp_rx = rx;
                    break;
                }
            }
        }
    }

    pub fn request_page_texture(
        &mut self,
        ctx: &egui::Context,
        document: &Option<Arc<dyn Document>>,
        page_idx: usize,
    ) -> Option<egui::TextureHandle> {
        self.access_counter += 1;
        let counter = self.access_counter;

        if let Some(entry) = self.textures.get_mut(&page_idx) {
            entry.1 = counter;
            return Some(entry.0.clone());
        }

        if self.pending_pages.contains(&page_idx) || self.failed_pages.contains_key(&page_idx) {
            return None;
        }

        let doc = document.as_ref()?.clone();
        self.pending_pages.insert(page_idx);
        let _ = self.req_tx.send(LoadRequest {
            doc_generation: self.doc_generation,
            doc,
            page_idx,
            ctx: ctx.clone(),
        });
        None
    }

    pub fn render_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        page_count: usize,
        settings: &mut ComicSettings,
        theme: &ReaderTheme,
        is_manga: bool,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            let prev_enabled = self.current_page > 0;
            if ui
                .add_enabled(prev_enabled, egui::Button::new("◀"))
                .on_hover_text("Предыдущая страница")
                .clicked()
            {
                self.prev_page(settings);
            }

            if self.page_input.is_empty() {
                self.page_input = (self.current_page + 1).to_string();
            }
            let cur_display = self.current_page + 1;
            ui.label(RichText::new("Стр.").size(13.0));
            let text_edit = ui.add(
                egui::TextEdit::singleline(&mut self.page_input)
                    .desired_width(38.0)
                    .font(egui::TextStyle::Body),
            );
            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(target) =
                    super::paged_common::parse_page_input(&self.page_input, page_count)
                {
                    self.current_page = target;
                    self.pan_offset = Vec2::ZERO;
                }
                self.page_input = (self.current_page + 1).to_string();
            } else if !text_edit.has_focus() {
                self.page_input = cur_display.to_string();
            }

            ui.label(RichText::new(format!("из {page_count}")).size(13.0));

            let next_enabled = self.current_page + 1 < page_count;
            if ui
                .add_enabled(next_enabled, egui::Button::new("▶"))
                .on_hover_text("Следующая страница")
                .clicked()
            {
                self.next_page(page_count, settings);
            }

            ui.separator();

            let is_manga_active = settings.reading_mode == "Manga";
            if ui
                .selectable_label(is_manga_active, "🍙 Манга (RTL)")
                .on_hover_text("Режим чтения справа-налево (японская манга)")
                .clicked()
            {
                settings.reading_mode = "Manga".to_string();
            }

            let is_comic_active = settings.reading_mode == "Comic";
            if ui
                .selectable_label(is_comic_active, "📖 Комикс (LTR)")
                .on_hover_text("Режим чтения слева-направо (европейские и американские комиксы)")
                .clicked()
            {
                settings.reading_mode = "Comic".to_string();
            }

            ui.separator();

            let is_single = settings.page_layout == "Single" && settings.reading_mode != "Webtoon";
            if ui
                .selectable_label(is_single, "1 стр.")
                .on_hover_text("Одиночная страница")
                .clicked()
            {
                settings.page_layout = "Single".to_string();
                if settings.reading_mode == "Webtoon" {
                    settings.reading_mode = "Auto".to_string();
                }
                self.pan_offset = Vec2::ZERO;
            }

            let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
            if ui
                .selectable_label(is_double, "2 стр.")
                .on_hover_text("Двухстраничный разворот")
                .clicked()
            {
                settings.page_layout = "Double".to_string();
                if settings.reading_mode == "Webtoon" {
                    settings.reading_mode = "Auto".to_string();
                }
                self.pan_offset = Vec2::ZERO;
            }

            let is_webtoon = settings.reading_mode == "Webtoon";
            if ui
                .selectable_label(is_webtoon, "Свиток")
                .on_hover_text("Непрерывная вертикальная лента (Webtoon / манхва)")
                .clicked()
            {
                settings.reading_mode = "Webtoon".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui
                .button("−")
                .on_hover_text("Уменьшить масштаб (Ctrl -)")
                .clicked()
            {
                super::paged_common::zoom_out(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            let zoom_lbl = format!("{:.0}%", settings.zoom_percent);
            if ui
                .button(RichText::new(zoom_lbl).strong())
                .on_hover_text("Сбросить масштаб на 100% (Ctrl 0)")
                .clicked()
            {
                super::paged_common::zoom_reset(
                    &mut settings.zoom_percent,
                    &mut settings.fit_mode,
                    &mut self.pan_offset,
                );
            }

            if ui
                .button("+")
                .on_hover_text("Увеличить масштаб (Ctrl +)")
                .clicked()
            {
                super::paged_common::zoom_in(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            let fit_win_active = settings.fit_mode == "FitWindow";
            if ui
                .selectable_label(fit_win_active, "По окну")
                .on_hover_text("Вместить изображение целиком в окно")
                .clicked()
            {
                settings.fit_mode = "FitWindow".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            let fit_w_active = settings.fit_mode == "FitWidth";
            if ui
                .selectable_label(fit_w_active, "По ширине")
                .on_hover_text("Растянуть изображение по ширине окна")
                .clicked()
            {
                settings.fit_mode = "FitWidth".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            let fit_h_active = settings.fit_mode == "FitHeight";
            if ui
                .selectable_label(fit_h_active, "По высоте")
                .on_hover_text("Растянуть изображение по высоте окна")
                .clicked()
            {
                settings.fit_mode = "FitHeight".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let badge_txt = if is_manga {
                    "🍙 Манга"
                } else {
                    "🎨 Комикс"
                };
                ui.label(
                    RichText::new(badge_txt)
                        .size(11.0)
                        .strong()
                        .color(theme.accent_color()),
                );
            });
        });
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        document: &Option<Arc<dyn Document>>,
        settings: &mut ComicSettings,
        theme: &ReaderTheme,
        pending_scroll_y: &mut Option<f32>,
        current_scroll_y: &mut f32,
        viewport_height: &mut f32,
        max_scroll_y: &mut f32,
        status_message: &mut Option<String>,
    ) {
        self.poll_responses(ctx, status_message);

        let page_count = self.page_count(document);
        if page_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("В этом документе нет страниц").color(theme.text_color()));
            });
            return;
        }

        self.current_page = self.current_page.min(page_count.saturating_sub(1));
        let is_manga = self.is_effective_manga(document, settings);

        ui.add_space(2.0);
        self.render_toolbar(ui, page_count, settings, theme, is_manga);
        ui.add_space(4.0);
        ui.separator();

        let is_webtoon = settings.reading_mode == "Webtoon";

        if is_webtoon {
            let mut scroll_area = ScrollArea::vertical()
                .id_salt(("webtoon_scroll", self.doc_generation))
                .auto_shrink([false, false]);
            if let Some(offset) = pending_scroll_y.take() {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
            }

            let avail_w = ui.available_width().max(200.0);
            let zoom_factor = (settings.zoom_percent / 100.0).clamp(0.3, 4.0);
            let target_w = match settings.fit_mode.as_str() {
                "FitWidth" => (avail_w - 32.0).clamp(200.0, 4000.0),
                "Custom" => (avail_w * 0.85 * zoom_factor).clamp(200.0, 6000.0),
                _ => (avail_w * 0.85).clamp(320.0, 1200.0),
            };
            let est_row_h = target_w * 1.4;

            let scroll_output =
                scroll_area.show_rows(ui, est_row_h, page_count, |ui, row_range| {
                    ui.vertical_centered(|ui| {
                        if let Some(first_visible) = row_range.clone().next() {
                            self.current_page = first_visible.min(page_count.saturating_sub(1));
                        }
                        for p in row_range {
                            if let Some(tex) = self.request_page_texture(ctx, document, p) {
                                let tex_size = tex.size_vec2();
                                let h = target_w * (tex_size.y / tex_size.x);
                                let img_widget = egui::Image::from_texture(&tex)
                                    .fit_to_exact_size(Vec2::new(target_w, h));
                                ui.add(img_widget);
                                ui.add_space(8.0);
                            } else if let Some(err) = self.failed_pages.get(&p) {
                                ui.label(
                                    RichText::new(format!("Ошибка страницы {}: {err}", p + 1))
                                        .color(theme.accent_color()),
                                );
                            } else {
                                ui.add_space(24.0);
                                ui.label(
                                    RichText::new(format!("⏳ Загрузка страницы {}...", p + 1))
                                        .color(theme.text_color()),
                                );
                                ui.add_space(24.0);
                            }
                        }
                    });
                });

            *current_scroll_y = scroll_output.state.clamped_offset().y;
            *viewport_height = scroll_output.inner_rect.height();
            *max_scroll_y = (scroll_output.content_size.y - *viewport_height).max(0.0);

            super::paged_common::handle_middle_mouse_scroll(
                ctx,
                *current_scroll_y,
                *max_scroll_y,
                pending_scroll_y,
            );
        } else {
            let avail_size = ui.available_size();
            let is_double = settings.page_layout == "Double" && self.current_page + 1 < page_count;

            let pages_to_show: Vec<usize> = if is_double {
                if is_manga {
                    vec![self.current_page + 1, self.current_page]
                } else {
                    vec![self.current_page, self.current_page + 1]
                }
            } else {
                vec![self.current_page]
            };

            for &p in &pages_to_show {
                let _ = self.request_page_texture(ctx, document, p);
            }

            let tex_sizes: Vec<Vec2> = pages_to_show
                .iter()
                .map(|&p| {
                    self.textures
                        .get(&p)
                        .map(|t| t.0.size_vec2())
                        .unwrap_or(Vec2::new(600.0, 800.0))
                })
                .collect();

            let geom = super::paged_common::compute_paged_geometry(
                avail_size,
                &settings.fit_mode,
                settings.zoom_percent,
                0,
                &tex_sizes,
            );

            super::paged_common::handle_paged_pan(
                ctx,
                &mut self.pan_offset,
                geom.max_pan.x,
                geom.max_pan.y,
            );

            let mut prev_clicked = false;
            let mut next_clicked = false;

            let (container_rect, _) = ui.allocate_exact_size(avail_size, egui::Sense::hover());
            let panned_rect = container_rect.translate(self.pan_offset);

            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(panned_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child_ui.set_clip_rect(container_rect);

            child_ui.horizontal_centered(|ui| {
                ui.add_space(geom.pad_x);

                for (idx, &p) in pages_to_show.iter().enumerate() {
                    let display_size = geom.page_sizes[idx];
                    if let Some(entry) = self.textures.get(&p) {
                        let tex = entry.0.clone();
                        let img_widget = egui::Image::from_texture(&tex)
                            .fit_to_exact_size(display_size)
                            .sense(egui::Sense::click());
                        let img_resp = ui.add(img_widget);

                        if img_resp.clicked()
                            && let Some(pos) = img_resp.interact_pointer_pos()
                        {
                            let is_left = pos.x < img_resp.rect.center().x;
                            if (is_manga && is_left) || (!is_manga && !is_left) {
                                next_clicked = true;
                            } else {
                                prev_clicked = true;
                            }
                        }
                    } else if let Some(err) = self.failed_pages.get(&p) {
                        ui.vertical_centered(|ui| {
                            ui.add_space(avail_size.y * 0.4);
                            ui.label(
                                RichText::new(format!("Ошибка страницы {}: {err}", p + 1))
                                    .size(16.0)
                                    .color(theme.accent_color()),
                            );
                        });
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(avail_size.y * 0.45);
                            ui.label(
                                RichText::new(format!("Загрузка страницы {}...", p + 1))
                                    .size(18.0)
                                    .color(theme.text_color()),
                            );
                        });
                    }
                    ui.add_space(12.0);
                }
            });

            if self.current_page + 1 < page_count {
                let _ = self.request_page_texture(ctx, document, self.current_page + 1);
            }
            if self.current_page > 0 {
                let _ = self.request_page_texture(ctx, document, self.current_page - 1);
            }

            if next_clicked {
                self.next_page(page_count, settings);
            } else if prev_clicked {
                self.prev_page(settings);
            }
        }
    }
}
