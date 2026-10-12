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

/// Состояние отображения, фонового воркера и кэш текстур для комиксов, манги и PDF
pub struct ComicViewState {
    pub current_page: usize,
    pub textures: HashMap<usize, (egui::TextureHandle, u64)>,
    pub pending_pages: HashSet<usize>,
    pub failed_pages: HashMap<usize, String>,
    pub access_counter: u64,
    pub doc_generation: u64,
    req_tx: Sender<LoadRequest>,
    resp_rx: Receiver<LoadResponse>,
}

impl Default for ComicViewState {
    fn default() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<LoadRequest>();
        let (resp_tx, resp_rx) = mpsc::channel::<LoadResponse>();

        std::thread::Builder::new()
            .name("page-decoder-worker".to_string())
            .spawn(move || {
                while let Ok(req) = req_rx.recv() {
                    let result = if let Some(page_content) = req.doc.page_content() {
                        page_content.load_page_image(req.page_idx)
                    } else {
                        Err(DocumentError::Unsupported(
                            "Документ не поддерживает постраничный режим".to_string(),
                        ))
                    };
                    let _ = resp_tx.send(LoadResponse {
                        doc_generation: req.doc_generation,
                        page_idx: req.page_idx,
                        result,
                    });
                    req.ctx.request_repaint();
                }
            })
            .ok();

        Self {
            current_page: 0,
            textures: HashMap::new(),
            pending_pages: HashSet::new(),
            failed_pages: HashMap::new(),
            access_counter: 0,
            doc_generation: 0,
            req_tx,
            resp_rx,
        }
    }
}

impl ComicViewState {
    /// Сбрасывает кэш текстур при открытии нового документа
    pub fn clear_cache(&mut self) {
        self.textures.clear();
        self.pending_pages.clear();
        self.failed_pages.clear();
        self.doc_generation = self.doc_generation.wrapping_add(1);
    }

    /// Общее количество страниц в текущем документе
    pub fn page_count(&self, document: &Option<Arc<dyn Document>>) -> usize {
        document
            .as_ref()
            .and_then(|d| d.page_content())
            .map(|c| c.page_count())
            .unwrap_or(0)
    }

    /// Определяет, должен ли документ читаться в режиме манги (справа-налево)
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

    /// Переход к следующей странице
    pub fn next_page(&mut self, total_pages: usize, settings: &ComicSettings) {
        if total_pages == 0 {
            return;
        }
        let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
        let step = if is_double { 2 } else { 1 };
        self.current_page = (self.current_page + step).min(total_pages.saturating_sub(1));
    }

    /// Переход к предыдущей странице
    pub fn prev_page(&mut self, settings: &ComicSettings) {
        let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
        let step = if is_double { 2 } else { 1 };
        self.current_page = self.current_page.saturating_sub(step);
    }

    /// Принимает декодированные страницы из рабочего потока
    fn poll_responses(&mut self, ctx: &egui::Context, status_message: &mut Option<String>) {
        while let Ok(resp) = self.resp_rx.try_recv() {
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
                        format!("page_tex_{}_{}", self.doc_generation, resp.page_idx),
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
    }

    /// Запрашивает страницу из кэша или отправляет задачу фоновому потоку
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

    /// Отрисовка страницы или разворота комикса/манги/вебтуна/PDF
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        document: &Option<Arc<dyn Document>>,
        settings: &ComicSettings,
        theme: &ReaderTheme,
        pending_scroll_y: &mut Option<f32>,
        current_scroll_y: &mut f32,
        viewport_height: &mut f32,
        max_scroll_y: &mut f32,
        status_message: &mut Option<String>,
    ) {
        self.poll_responses(ctx, status_message);

        let page_count = if let Some(doc) = document {
            if let Some(comic) = doc.page_content() {
                comic.page_count()
            } else {
                return;
            }
        } else {
            return;
        };

        if page_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("В этом документе нет страниц").color(theme.text_color()));
            });
            return;
        }

        self.current_page = self.current_page.min(page_count.saturating_sub(1));
        let is_webtoon = settings.reading_mode == "Webtoon";
        let is_manga = self.is_effective_manga(document, settings);
        let is_double = settings.page_layout == "Double" && !is_webtoon;

        if is_webtoon {
            let mut scroll_area = ScrollArea::vertical()
                .id_salt(("webtoon_scroll", self.doc_generation))
                .auto_shrink([false, false]);
            if let Some(offset) = pending_scroll_y.take() {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
            }

            let avail_w = ui.available_width().max(200.0);
            let target_w = (avail_w * 0.85).clamp(320.0, 960.0);
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

            *current_scroll_y = scroll_output.state.offset.y;
            *viewport_height = scroll_output.inner_rect.height();
            *max_scroll_y = (scroll_output.content_size.y - *viewport_height).max(0.0);
        } else {
            let avail_size = ui.available_size();

            let pages_to_show: Vec<usize> = if is_double && self.current_page + 1 < page_count {
                if is_manga {
                    vec![self.current_page + 1, self.current_page]
                } else {
                    vec![self.current_page, self.current_page + 1]
                }
            } else {
                vec![self.current_page]
            };

            let mut prev_clicked = false;
            let mut next_clicked = false;

            ui.horizontal_centered(|ui| {
                let num_pages = pages_to_show.len() as f32;
                let target_w_per_page = (avail_size.x - 20.0) / num_pages;

                for &p in &pages_to_show {
                    if let Some(tex) = self.request_page_texture(ctx, document, p) {
                        let tex_size = tex.size_vec2();
                        let (display_w, display_h) = match settings.fit_mode.as_str() {
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
                            _ => {
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
                                RichText::new(format!("❌ Ошибка страницы {}: {err}", p + 1))
                                    .size(16.0)
                                    .color(theme.accent_color()),
                            );
                        });
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(avail_size.y * 0.45);
                            ui.label(
                                RichText::new(format!("⏳ Загрузка страницы {}...", p + 1))
                                    .size(18.0)
                                    .color(theme.text_color()),
                            );
                        });
                    }
                }
            });

            // Предзагрузка соседних страниц n±1 в фоновом потоке
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
