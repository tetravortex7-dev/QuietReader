use super::document_loader::format_document_error;
use super::theme::ReaderTheme;
use crate::document::{DecodedImage, Document, DocumentError, PageTextSpan};
use crate::settings::PdfSettings;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, RichText, ScrollArea, Vec2};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct LoadRequest {
    doc_generation: u64,
    doc: Arc<dyn Document>,
    page_idx: usize,
    scale: f32,
    scale_bucket: u32,
    ctx: egui::Context,
}

struct LoadResponse {
    doc_generation: u64,
    page_idx: usize,
    scale_bucket: u32,
    result: Result<DecodedImage, DocumentError>,
    spans: Option<Vec<PageTextSpan>>,
}

pub struct PageTextureEntry {
    pub texture: egui::TextureHandle,
    pub scale_bucket: u32,
    pub last_access: u64,
    pub byte_size: usize,
}

pub struct PdfViewState {
    pub current_page: usize,
    pub textures: HashMap<usize, PageTextureEntry>,
    pub pending_pages: HashMap<usize, u32>,
    pub failed_pages: HashMap<usize, String>,
    pub access_counter: u64,
    pub total_texture_bytes: usize,
    pub doc_generation: u64,
    active_generation: Arc<AtomicU64>,
    latest_requested_scales: Arc<Mutex<HashMap<usize, u32>>>,
    pub pan_offset: Vec2,
    pub last_avail_size: Vec2,
    pub is_middle_dragging: bool,
    pub page_input: String,
    pub pending_target_page: Option<usize>,
    pub page_text_spans: HashMap<usize, Vec<PageTextSpan>>,
    pub selection_start: Option<Pos2>,
    pub selection_current: Option<Pos2>,
    pub selection_dirty: bool,
    pub selected_text: String,
    pub copy_notification: Option<(String, Instant)>,
    req_tx: Sender<LoadRequest>,
    resp_rx: Receiver<LoadResponse>,
}

fn spawn_pdf_worker(
    active_generation: Arc<AtomicU64>,
    latest_scales: Arc<Mutex<HashMap<usize, u32>>>,
) -> (Sender<LoadRequest>, Receiver<LoadResponse>) {
    let (req_tx, req_rx) = mpsc::channel::<LoadRequest>();
    let (resp_tx, resp_rx) = mpsc::channel::<LoadResponse>();

    std::thread::Builder::new()
        .name("pdf-page-worker".to_string())
        .spawn(move || {
            while let Ok(req) = req_rx.recv() {
                if req.doc_generation != active_generation.load(Ordering::Acquire) {
                    continue;
                }
                if let Ok(latest) = latest_scales.lock() {
                    if let Some(&latest_bucket) = latest.get(&req.page_idx) {
                        if latest_bucket != req.scale_bucket {
                            continue;
                        }
                    }
                }

                let doc = req.doc;
                let page_idx = req.page_idx;
                let scale = req.scale;
                let load_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if let Some(page_content) = doc.page_content() {
                        let img = page_content.load_page_image_scaled(page_idx, scale)?;
                        let spans = page_content.extract_page_text_spans(page_idx);
                        Ok((img, spans))
                    } else {
                        Err(DocumentError::Unsupported(
                            "Документ не поддерживает постраничный режим".to_string(),
                        ))
                    }
                }));

                if req.doc_generation != active_generation.load(Ordering::Acquire) {
                    continue;
                }

                let (result, spans) = match load_res {
                    Ok(Ok((img, spans))) => (Ok(img), spans),
                    Ok(Err(e)) => (Err(e), None),
                    Err(_) => (
                        Err(DocumentError::Corrupted(
                            "Аварийный сбой при декодировании страницы".to_string(),
                        )),
                        None,
                    ),
                };

                let _ = resp_tx.send(LoadResponse {
                    doc_generation: req.doc_generation,
                    page_idx,
                    scale_bucket: req.scale_bucket,
                    result,
                    spans,
                });
                req.ctx.request_repaint();
            }
        })
        .ok();

    (req_tx, resp_rx)
}

impl Default for PdfViewState {
    fn default() -> Self {
        let active_generation = Arc::new(AtomicU64::new(0));
        let latest_requested_scales = Arc::new(Mutex::new(HashMap::new()));
        let (req_tx, resp_rx) =
            spawn_pdf_worker(active_generation.clone(), latest_requested_scales.clone());
        Self {
            current_page: 0,
            textures: HashMap::new(),
            pending_pages: HashMap::new(),
            failed_pages: HashMap::new(),
            access_counter: 0,
            total_texture_bytes: 0,
            doc_generation: 0,
            active_generation,
            latest_requested_scales,
            pan_offset: Vec2::ZERO,
            last_avail_size: Vec2::new(800.0, 600.0),
            is_middle_dragging: false,
            page_input: String::new(),
            pending_target_page: None,
            page_text_spans: HashMap::new(),
            selection_start: None,
            selection_current: None,
            selection_dirty: false,
            selected_text: String::new(),
            copy_notification: None,
            req_tx,
            resp_rx,
        }
    }
}

impl PdfViewState {
    pub fn clear_cache(&mut self) {
        self.textures.clear();
        self.pending_pages.clear();
        self.failed_pages.clear();
        self.total_texture_bytes = 0;
        self.pan_offset = Vec2::ZERO;
        self.pending_target_page = None;
        self.page_text_spans.clear();
        self.selection_start = None;
        self.selection_current = None;
        self.selection_dirty = false;
        self.selected_text.clear();
        self.doc_generation = self.doc_generation.wrapping_add(1);
        self.active_generation
            .store(self.doc_generation, Ordering::Release);
        if let Ok(mut map) = self.latest_requested_scales.lock() {
            map.clear();
        }
    }

    pub fn page_count(&self, document: &Option<Arc<dyn Document>>) -> usize {
        document
            .as_ref()
            .and_then(|d| d.page_content())
            .map(|c| c.page_count())
            .unwrap_or(0)
    }

    pub fn go_to_page(&mut self, page: usize) {
        self.current_page = page;
        self.pending_target_page = Some(page);
        self.pan_offset = Vec2::ZERO;
        self.page_input = (page + 1).to_string();
        self.selection_start = None;
        self.selection_current = None;
        self.selection_dirty = false;
        self.selected_text.clear();
    }

    pub fn next_page(&mut self, total_pages: usize, settings: &PdfSettings) {
        if total_pages == 0 {
            return;
        }
        let step = if settings.layout == "Double" { 2 } else { 1 };
        let next = (self.current_page + step).min(total_pages.saturating_sub(1));
        self.go_to_page(next);
    }

    pub fn prev_page(&mut self, settings: &PdfSettings) {
        let step = if settings.layout == "Double" { 2 } else { 1 };
        let prev = self.current_page.saturating_sub(step);
        self.go_to_page(prev);
    }

    pub fn max_pan(&self, settings: &PdfSettings, page_count: usize) -> Vec2 {
        let is_double = settings.layout == "Double" && self.current_page + 1 < page_count;
        let pages_to_show: Vec<usize> = if is_double {
            vec![self.current_page, self.current_page + 1]
        } else {
            vec![self.current_page]
        };
        let tex_sizes: Vec<Vec2> = pages_to_show
            .iter()
            .map(|&p| {
                self.textures
                    .get(&p)
                    .map(|t| t.texture.size_vec2())
                    .unwrap_or(Vec2::new(600.0, 800.0))
            })
            .collect();
        let avail = if self.last_avail_size.x > 10.0 && self.last_avail_size.y > 10.0 {
            self.last_avail_size
        } else {
            Vec2::new(800.0, 600.0)
        };
        super::paged_common::compute_paged_geometry(
            avail,
            &settings.fit_mode,
            settings.zoom_percent,
            settings.rotation,
            &tex_sizes,
        )
        .max_pan
    }

    fn poll_responses(&mut self, ctx: &egui::Context, status_message: &mut Option<String>) {
        loop {
            match self.resp_rx.try_recv() {
                Ok(resp) => {
                    if resp.doc_generation != self.doc_generation {
                        continue;
                    }
                    if let Some(&pending_b) = self.pending_pages.get(&resp.page_idx) {
                        if pending_b == resp.scale_bucket {
                            self.pending_pages.remove(&resp.page_idx);
                        }
                    }
                    if let Some(spans) = resp.spans {
                        self.page_text_spans.insert(resp.page_idx, spans);
                    }
                    match resp.result {
                        Ok(img) => {
                            self.access_counter += 1;
                            let counter = self.access_counter;
                            let w = img.width as usize;
                            let h = img.height as usize;
                            let byte_size = w * h * 4;
                            let color_image =
                                egui::ColorImage::from_rgba_unmultiplied([w, h], &img.rgba);
                            let texture = ctx.load_texture(
                                format!(
                                    "pdf_tex_{}_{}_{}",
                                    self.doc_generation, resp.page_idx, resp.scale_bucket
                                ),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );

                            if let Some(old) = self.textures.remove(&resp.page_idx) {
                                self.total_texture_bytes =
                                    self.total_texture_bytes.saturating_sub(old.byte_size);
                            }

                            self.total_texture_bytes += byte_size;
                            self.textures.insert(
                                resp.page_idx,
                                PageTextureEntry {
                                    texture,
                                    scale_bucket: resp.scale_bucket,
                                    last_access: counter,
                                    byte_size,
                                },
                            );

                            const MAX_CACHED_PAGES: usize = 16;
                            const MAX_TEXTURE_BYTES: usize = 180 * 1024 * 1024;
                            while self.textures.len() > MAX_CACHED_PAGES
                                || self.total_texture_bytes > MAX_TEXTURE_BYTES
                            {
                                let cur = self.current_page;
                                let oldest_key = self
                                    .textures
                                    .iter()
                                    .filter(|(p, _)| {
                                        let p = **p;
                                        p != cur && p != cur + 1 && (cur == 0 || p != cur - 1)
                                    })
                                    .min_by_key(|(_, entry)| entry.last_access)
                                    .map(|(p, _)| *p)
                                    .or_else(|| {
                                        self.textures
                                            .iter()
                                            .min_by_key(|(_, entry)| entry.last_access)
                                            .map(|(p, _)| *p)
                                    });

                                if let Some(key) = oldest_key {
                                    if let Some(entry) = self.textures.remove(&key) {
                                        self.total_texture_bytes = self
                                            .total_texture_bytes
                                            .saturating_sub(entry.byte_size);
                                    }
                                } else {
                                    break;
                                }
                            }
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
                    for (page_idx, _) in self.pending_pages.drain() {
                        let msg = "Поток декодирования аварийно завершился".to_string();
                        self.failed_pages.insert(page_idx, msg.clone());
                        *status_message = Some(format!(
                            "Ошибка загрузки страницы {}: {msg}",
                            page_idx + 1
                        ));
                    }
                    let (tx, rx) = spawn_pdf_worker(
                        self.active_generation.clone(),
                        self.latest_requested_scales.clone(),
                    );
                    self.req_tx = tx;
                    self.resp_rx = rx;
                    break;
                }
            }
        }
    }

    pub fn compute_target_scale(&self, settings: &PdfSettings) -> f32 {
        let zoom_ratio = settings.zoom_percent / 100.0;
        let base = match settings.fit_mode.as_str() {
            "Custom" => (2.2 * zoom_ratio).clamp(1.5, 4.0),
            "FitWidth" => 2.5,
            "Original" => 2.0,
            _ => 2.25,
        };
        (base * 4.0).round() / 4.0
    }

    pub fn request_page_texture(
        &mut self,
        ctx: &egui::Context,
        document: &Option<Arc<dyn Document>>,
        page_idx: usize,
        scale: f32,
    ) -> Option<egui::TextureHandle> {
        self.access_counter += 1;
        let counter = self.access_counter;
        let scale_bucket = (scale * 4.0).round() as u32;

        let mut existing_tex = None;
        let mut need_re_rasterize = true;

        if let Some(entry) = self.textures.get_mut(&page_idx) {
            entry.last_access = counter;
            existing_tex = Some(entry.texture.clone());
            if entry.scale_bucket == scale_bucket {
                need_re_rasterize = false;
            }
        }

        if !need_re_rasterize {
            return existing_tex;
        }

        if let Some(&pending_bucket) = self.pending_pages.get(&page_idx) {
            if pending_bucket == scale_bucket {
                return existing_tex;
            }
        }

        if self.failed_pages.contains_key(&page_idx) {
            return existing_tex;
        }

        let doc = document.as_ref()?.clone();
        self.pending_pages.insert(page_idx, scale_bucket);
        if let Ok(mut map) = self.latest_requested_scales.lock() {
            map.insert(page_idx, scale_bucket);
        }

        let _ = self.req_tx.send(LoadRequest {
            doc_generation: self.doc_generation,
            doc,
            page_idx,
            scale,
            scale_bucket,
            ctx: ctx.clone(),
        });

        existing_tex
    }

    pub fn render_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        page_count: usize,
        settings: &mut PdfSettings,
        theme: &ReaderTheme,
        status_message: &mut Option<String>,
        format_label: &str,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if ui.add_enabled(self.current_page > 0, egui::Button::new("◀")).on_hover_text("Предыдущая страница (←, ↑, PageUp)").clicked() {
                let step = if settings.layout == "Double" { 2 } else { 1 };
                self.go_to_page(self.current_page.saturating_sub(step));
            }

            if self.page_input.is_empty() {
                self.page_input = (self.current_page + 1).to_string();
            }
            let cur_display = self.current_page + 1;
            ui.label(RichText::new("Стр.").size(13.0));
            let text_edit = ui.add(egui::TextEdit::singleline(&mut self.page_input).desired_width(38.0).font(egui::TextStyle::Body));
            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some(target) = super::paged_common::parse_page_input(&self.page_input, page_count) {
                    self.go_to_page(target);
                }
                self.page_input = (self.current_page + 1).to_string();
            } else if !text_edit.has_focus() {
                self.page_input = cur_display.to_string();
            }

            ui.label(RichText::new(format!("из {page_count}")).size(13.0));

            if ui.add_enabled(self.current_page + 1 < page_count, egui::Button::new("▶")).on_hover_text("Следующая страница (→, ↓, PageDown, Пробел)").clicked() {
                let step = if settings.layout == "Double" { 2 } else { 1 };
                self.go_to_page((self.current_page + step).min(page_count.saturating_sub(1)));
            }

            ui.separator();

            if ui.button("−").on_hover_text("Уменьшить масштаб (Ctrl -)").clicked() {
                super::paged_common::zoom_out(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            let zoom_lbl = format!("{:.0}%", settings.zoom_percent);
            if ui.button(RichText::new(zoom_lbl).strong()).on_hover_text("Сбросить масштаб на 100% (Ctrl 0)").clicked() {
                super::paged_common::zoom_reset(&mut settings.zoom_percent, &mut settings.fit_mode, &mut self.pan_offset);
            }

            if ui.button("+").on_hover_text("Увеличить масштаб (Ctrl +)").clicked() {
                super::paged_common::zoom_in(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            if ui.selectable_label(settings.fit_mode == "FitWidth", "По ширине").on_hover_text("Масштабировать по ширине окна").clicked() {
                settings.fit_mode = "FitWidth".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui.selectable_label(settings.fit_mode == "FitWindow", "Целая страница").on_hover_text("Уместить страницу целиком в окно").clicked() {
                settings.fit_mode = "FitWindow".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui.button("⟳ Повернуть").on_hover_text("Повернуть страницы на 90° по часовой стрелке").clicked() {
                settings.rotation = (settings.rotation + 90) % 360;
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui.selectable_label(settings.layout == "Single", "1 стр.").on_hover_text("Одиночная страница").clicked() {
                settings.layout = "Single".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui.selectable_label(settings.layout == "Double", "2 стр.").on_hover_text("Две страницы (разворот)").clicked() {
                settings.layout = "Double".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            if ui.selectable_label(settings.layout == "Continuous", "Свиток").on_hover_text("Непрерывный вертикальный свиток страниц").clicked() {
                settings.layout = "Continuous".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if !self.selected_text.is_empty() {
                if ui.button(RichText::new("📋 Скопировать").strong().color(theme.accent_color())).on_hover_text("Скопировать выделенный текст (Ctrl+C)").clicked() {
                    ui.ctx().copy_text(self.selected_text.clone());
                    let preview = truncate_str(&self.selected_text, 35);
                    *status_message = Some(format!("Скопировано: \"{preview}\""));
                    self.copy_notification = Some((format!("Скопировано: \"{preview}\""), Instant::now()));
                }
                if ui.small_button("✕").on_hover_text("Снять выделение").clicked() {
                    self.selection_start = None;
                    self.selection_current = None;
                    self.selection_dirty = false;
                    self.selected_text.clear();
                }
            } else {
                ui.label(RichText::new("Выделение: зажмите ЛКМ на странице").size(11.5).italics().color(theme.text_color().gamma_multiply(0.65)));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("📄 {format_label}")).size(11.0).strong().color(theme.accent_color()));
            });
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_page_with_selection<'a>(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        page_idx: usize,
        tex: &egui::TextureHandle,
        size: Vec2,
        rotation: i32,
        selected_spans_collector: &mut Vec<&'a str>,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let painter = ui.painter();
        let rot_deg = rotation.rem_euclid(360);

        if rot_deg == 0 {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            let uvs = match rot_deg {
                90 => [Pos2::new(0.0, 1.0), Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0)],
                180 => [Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0), Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0)],
                270 => [Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0), Pos2::new(0.0, 0.0)],
                _ => [Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0)],
            };
            let corners = [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ];
            let mut mesh = egui::Mesh::with_texture(tex.id());
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            for i in 0..4 {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: corners[i],
                    uv: uvs[i],
                    color: Color32::WHITE,
                });
            }
            painter.add(mesh);
        }

        if let Some(hover_pos) = response.hover_pos()
            && let Some(spans) = self.page_text_spans.get(&page_idx)
        {
            for span in spans {
                let span_rect = transform_span_bbox(span.bbox, rotation, rect);
                if span_rect.contains(hover_pos) {
                    ctx.set_cursor_icon(egui::CursorIcon::Text);
                    break;
                }
            }
        }

        if response.drag_started_by(egui::PointerButton::Primary) {
            let pos = response.interact_pointer_pos();
            self.selection_start = pos;
            self.selection_current = pos;
            self.selection_dirty = true;
        } else if response.dragged_by(egui::PointerButton::Primary) {
            let pos = response.interact_pointer_pos();
            if self.selection_current != pos {
                self.selection_current = pos;
                self.selection_dirty = true;
            }
        } else if response.clicked_by(egui::PointerButton::Primary) {
            self.selection_start = None;
            self.selection_current = None;
            self.selection_dirty = false;
            self.selected_text.clear();
        }

        if let (Some(start), Some(curr)) = (self.selection_start, self.selection_current) {
            let sel_rect = Rect::from_two_pos(start, curr);
            if (sel_rect.width() > 3.0 || sel_rect.height() > 3.0)
                && rect.intersects(sel_rect)
                && let Some(spans) = self.page_text_spans.get(&page_idx)
            {
                for span in spans {
                    let span_rect = transform_span_bbox(span.bbox, rotation, rect);
                    if span_rect.intersects(sel_rect) {
                        painter.rect_filled(
                            span_rect,
                            CornerRadius::same(2),
                            Color32::from_rgba_unmultiplied(33, 150, 243, 90),
                        );
                        if self.selection_dirty {
                            unsafe {
                                let span_ptr: *const str = span.text.as_str();
                                selected_spans_collector.push(&*span_ptr);
                            }
                        }
                    }
                }
            }
        }

        response
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        document: &Option<Arc<dyn Document>>,
        settings: &mut PdfSettings,
        theme: &ReaderTheme,
        pending_scroll_y: &mut Option<f32>,
        current_scroll_y: &mut f32,
        viewport_height: &mut f32,
        max_scroll_y: &mut f32,
        status_message: &mut Option<String>,
    ) {
        self.poll_responses(ctx, status_message);

        let format_label = document.as_ref().map(|d| d.format_name()).unwrap_or("Документ");
        let page_count = self.page_count(document);
        if page_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("В этом документе нет страниц").color(theme.text_color()),
                );
            });
            return;
        }

        self.current_page = self.current_page.min(page_count.saturating_sub(1));

        ui.add_space(2.0);
        self.render_toolbar(ui, page_count, settings, theme, status_message, format_label);
        ui.add_space(4.0);
        ui.separator();

        let render_scale = self.compute_target_scale(settings);
        let is_continuous = settings.layout == "Continuous";

        let avail_w = ui.available_width().max(200.0);
        let zoom_factor = settings.zoom_percent / 100.0;
        let target_w = match settings.fit_mode.as_str() {
            "FitWidth" => (avail_w - 32.0).clamp(200.0, 4000.0),
            "Custom" => (avail_w * 0.85 * zoom_factor).clamp(200.0, 6000.0),
            _ => (avail_w * 0.85).clamp(320.0, 1600.0),
        };
        let est_row_h = target_w * 1.414;
        let row_h_total = est_row_h + 12.0;

        let just_jumped = self.pending_target_page.is_some();
        if let Some(target_p) = self.pending_target_page.take() {
            self.current_page = target_p.min(page_count.saturating_sub(1));
            self.pan_offset = Vec2::ZERO;
            if is_continuous {
                let target_y = self.current_page as f32 * row_h_total;
                *pending_scroll_y = Some(target_y);
            }
        }

        let mut collected_selected_spans = Vec::new();

        if is_continuous {
            let mut scroll_area = ScrollArea::vertical()
                .id_salt(("pdf_continuous_scroll", self.doc_generation))
                .auto_shrink([false, false]);

            if let Some(offset) = pending_scroll_y.take() {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
            }

            let scroll_output =
                scroll_area.show_rows(ui, est_row_h, page_count, |ui, row_range| {
                    ui.vertical_centered(|ui| {
                        if !just_jumped && let Some(first_visible) = row_range.clone().next() {
                            self.current_page = first_visible.min(page_count.saturating_sub(1));
                        }

                        for p in row_range {
                            if let Some(tex) =
                                self.request_page_texture(ctx, document, p, render_scale)
                            {
                                let tex_size = tex.size_vec2();
                                let mut w = target_w;
                                let mut h = target_w * (tex_size.y / tex_size.x);
                                if settings.rotation == 90 || settings.rotation == 270 {
                                    std::mem::swap(&mut w, &mut h);
                                }
                                self.render_page_with_selection(
                                    ui,
                                    ctx,
                                    p,
                                    &tex,
                                    Vec2::new(w, h),
                                    settings.rotation,
                                    &mut collected_selected_spans,
                                );
                                ui.add_space(12.0);
                            } else if let Some(err) = self.failed_pages.get(&p) {
                                ui.label(
                                    RichText::new(format!("Ошибка страницы {}: {err}", p + 1))
                                        .color(theme.accent_color()),
                                );
                            } else {
                                ui.add_space(28.0);
                                ui.label(
                                    RichText::new(format!("⏳ Загрузка страницы {}...", p + 1))
                                        .color(theme.text_color()),
                                );
                                ui.add_space(28.0);
                            }
                        }
                    });
                });

            *current_scroll_y = scroll_output.state.offset.y;
            *viewport_height = scroll_output.inner_rect.height();
            *max_scroll_y = (scroll_output.content_size.y - *viewport_height).max(0.0);

            super::paged_common::handle_middle_mouse_scroll(
                ctx,
                scroll_output.state.offset.y,
                *max_scroll_y,
                pending_scroll_y,
            );
        } else {
            let avail_size = ui.available_size();
            self.last_avail_size = avail_size;
            let is_double = settings.layout == "Double" && self.current_page + 1 < page_count;
            let pages_to_show: Vec<usize> = if is_double {
                vec![self.current_page, self.current_page + 1]
            } else {
                vec![self.current_page]
            };

            for &p in &pages_to_show {
                let _ = self.request_page_texture(ctx, document, p, render_scale);
            }

            let tex_sizes: Vec<Vec2> = pages_to_show
                .iter()
                .map(|&p| {
                    self.textures
                        .get(&p)
                        .map(|t| t.texture.size_vec2())
                        .unwrap_or(Vec2::new(600.0, 800.0))
                })
                .collect();

            let geom = super::paged_common::compute_paged_geometry(
                avail_size,
                &settings.fit_mode,
                settings.zoom_percent,
                settings.rotation,
                &tex_sizes,
            );

            super::paged_common::handle_paged_pan(
                ctx,
                &mut self.pan_offset,
                geom.max_pan.x,
                geom.max_pan.y,
            );

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
                        let tex = entry.texture.clone();
                        self.render_page_with_selection(
                            ui,
                            ctx,
                            p,
                            &tex,
                            display_size,
                            settings.rotation,
                            &mut collected_selected_spans,
                        );
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
                    ui.add_space(12.0);
                }
            });

            if self.current_page + 1 < page_count {
                let _ =
                    self.request_page_texture(ctx, document, self.current_page + 1, render_scale);
            }
            if self.current_page > 0 {
                let _ =
                    self.request_page_texture(ctx, document, self.current_page - 1, render_scale);
            }
        }

        if self.selection_dirty {
            if !collected_selected_spans.is_empty() {
                self.selected_text = collected_selected_spans.join("\n");
            } else {
                self.selected_text.clear();
            }
            self.selection_dirty = false;
        }

        if !self.selected_text.is_empty() {
            let ctrl_c = ctx.input(|i| {
                (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(egui::Key::C)
            });
            if ctrl_c {
                ctx.copy_text(self.selected_text.clone());
                let preview = truncate_str(&self.selected_text, 35);
                *status_message = Some(format!("Скопировано в буфер: \"{preview}\""));
                self.copy_notification =
                    Some((format!("Скопировано: \"{preview}\""), Instant::now()));
            }
        }

        if !self.selected_text.is_empty()
            && let Some(curr) = self.selection_current
        {
            let pill_pos = curr + Vec2::new(12.0, 12.0);
            egui::Area::new(egui::Id::new("pdf_selection_pill"))
                .fixed_pos(pill_pos)
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new("📋 Скопировать (Ctrl+C)").size(12.0))
                                .clicked()
                            {
                                ctx.copy_text(self.selected_text.clone());
                                let preview = truncate_str(&self.selected_text, 35);
                                *status_message = Some(format!("Скопировано: \"{preview}\""));
                                self.copy_notification =
                                    Some((format!("Скопировано: \"{preview}\""), Instant::now()));
                            }
                            if ui.small_button("✕").clicked() {
                                self.selection_start = None;
                                self.selection_current = None;
                                self.selection_dirty = false;
                                self.selected_text.clear();
                            }
                        });
                    });
                });
        }

        if let Some((msg, time)) = &self.copy_notification {
            if time.elapsed().as_secs_f32() < 2.0 {
                egui::Area::new(egui::Id::new("pdf_copy_toast"))
                    .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -45.0))
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("✓ {msg}"))
                                    .strong()
                                    .color(theme.accent_color()),
                            );
                        });
                    });
            } else {
                self.copy_notification = None;
            }
        }
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let prefix: String = trimmed.chars().take(max_chars).collect();
        format!("{prefix}...")
    }
}

pub fn transform_span_bbox(bbox: [f32; 4], rotation: i32, rect: Rect) -> Rect {
    let (rx, ry, rw, rh) = (rect.min.x, rect.min.y, rect.width(), rect.height());
    match rotation.rem_euclid(360) {
        90 => {
            let (x0, x1) = (rect.max.x - bbox[3] * rw, rect.max.x - bbox[1] * rw);
            let (y0, y1) = (ry + bbox[0] * rh, ry + bbox[2] * rh);
            Rect::from_min_max(Pos2::new(x0.min(x1), y0.min(y1)), Pos2::new(x0.max(x1), y0.max(y1)))
        }
        180 => {
            let (x0, x1) = (rect.max.x - bbox[2] * rw, rect.max.x - bbox[0] * rw);
            let (y0, y1) = (rect.max.y - bbox[3] * rh, rect.max.y - bbox[1] * rh);
            Rect::from_min_max(Pos2::new(x0.min(x1), y0.min(y1)), Pos2::new(x0.max(x1), y0.max(y1)))
        }
        270 => {
            let (x0, x1) = (rx + bbox[1] * rw, rx + bbox[3] * rw);
            let (y0, y1) = (rect.max.y - bbox[2] * rh, rect.max.y - bbox[0] * rh);
            Rect::from_min_max(Pos2::new(x0.min(x1), y0.min(y1)), Pos2::new(x0.max(x1), y0.max(y1)))
        }
        _ => {
            let (x0, x1) = (rx + bbox[0] * rw, rx + bbox[2] * rw);
            let (y0, y1) = (ry + bbox[1] * rh, ry + bbox[3] * rh);
            Rect::from_min_max(Pos2::new(x0.min(x1), y0.min(y1)), Pos2::new(x0.max(x1), y0.max(y1)))
        }
    }
}

