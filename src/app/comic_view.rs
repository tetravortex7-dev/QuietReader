use super::document_loader::{
    ERR_COMIC_DECODE_CRASH, ERR_DECODE_STREAM_CRASH, ERR_PAGED_UNSUPPORTED, format_page_error,
};
use super::icons::{Icon, IconButton, IconLabel};
use super::theme::ReaderTheme;
use crate::core::license::ProFeature;
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
    smart_dark: bool,
    target_bg: egui::Color32,
    target_text: egui::Color32,
    tolerance: u8,
}

struct LoadResponse {
    doc_generation: u64,
    page_idx: usize,
    result: Result<DecodedImage, DocumentError>,
    bbox: egui::Rect,
}

pub struct ComicViewState {
    pub current_page: usize,
    pub textures: HashMap<usize, (egui::TextureHandle, u64, usize)>,
    pub total_texture_bytes: usize,
    pub page_bboxes: HashMap<usize, egui::Rect>,
    pub pending_pages: HashSet<usize>,
    pub failed_pages: HashMap<usize, String>,
    pub access_counter: u64,
    pub doc_generation: u64,
    pub pan_offset: Vec2,
    pub is_middle_dragging: bool,
    pub page_input: String,
    pub last_smart_dark: bool,
    pub last_theme_bg: egui::Color32,
    pub last_crop_tolerance: u8,
    req_tx: Sender<LoadRequest>,
    resp_rx: Receiver<LoadResponse>,
}

const MAX_CACHED_TEXTURES: usize = 16;
const MAX_TEXTURE_BYTES: usize = 384 * 1024 * 1024;

pub(crate) fn comic_pages_to_show(
    current_page: usize,
    page_count: usize,
    double_layout: bool,
    is_manga: bool,
) -> Vec<usize> {
    let is_double = double_layout && current_page > 0 && current_page + 1 < page_count;
    if is_double {
        if is_manga {
            vec![current_page + 1, current_page]
        } else {
            vec![current_page, current_page + 1]
        }
    } else {
        vec![current_page]
    }
}

pub(crate) fn texture_byte_size(width: u32, height: u32) -> usize {
    super::rgba_byte_size(width, height)
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
                            ERR_PAGED_UNSUPPORTED.to_string(),
                        ))
                    }
                }));

                let (result, bbox) = match load_res {
                    Ok(Ok(mut img)) => {
                        let bbox =
                            crate::app::scan_processor::detect_content_bbox(&img, req.tolerance);
                        if req.smart_dark {
                            crate::app::scan_processor::apply_smart_dark_mode(
                                &mut img,
                                req.target_bg,
                                req.target_text,
                                true,
                            );
                        }
                        (Ok(img), bbox)
                    }
                    Ok(Err(e)) => (
                        Err(e),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    ),
                    Err(_) => (
                        Err(DocumentError::Corrupted(ERR_COMIC_DECODE_CRASH.to_string())),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    ),
                };

                let _ = resp_tx.send(LoadResponse {
                    doc_generation: req.doc_generation,
                    page_idx,
                    result,
                    bbox,
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
            total_texture_bytes: 0,
            page_bboxes: HashMap::new(),
            pending_pages: HashSet::new(),
            failed_pages: HashMap::new(),
            access_counter: 0,
            doc_generation: 0,
            pan_offset: Vec2::ZERO,
            is_middle_dragging: false,
            page_input: String::new(),
            last_smart_dark: false,
            last_theme_bg: egui::Color32::TRANSPARENT,
            last_crop_tolerance: 24,
            req_tx,
            resp_rx,
        }
    }
}

impl ComicViewState {
    pub fn clear_cache(&mut self) {
        self.textures.clear();
        self.total_texture_bytes = 0;
        self.page_bboxes.clear();
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
        let next = if is_double {
            if self.current_page == 0 {
                1
            } else {
                self.current_page.saturating_add(2)
            }
        } else {
            self.current_page.saturating_add(1)
        };
        self.current_page = next.min(total_pages.saturating_sub(1));
        self.pan_offset = Vec2::ZERO;
    }

    pub fn prev_page(&mut self, settings: &ComicSettings) {
        let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
        self.current_page = if is_double {
            if self.current_page <= 1 {
                0
            } else {
                self.current_page.saturating_sub(2)
            }
        } else {
            self.current_page.saturating_sub(1)
        };
        self.pan_offset = Vec2::ZERO;
    }

    fn poll_responses(
        &mut self,
        ctx: &egui::Context,
        status_message: &mut Option<String>,
        lang: crate::i18n::UiLanguage,
    ) {
        let t = lang.t();
        loop {
            match self.resp_rx.try_recv() {
                Ok(resp) => {
                    if resp.doc_generation != self.doc_generation {
                        continue;
                    }
                    self.pending_pages.remove(&resp.page_idx);
                    self.page_bboxes.insert(resp.page_idx, resp.bbox);
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
                            let byte_size = texture_byte_size(img.width, img.height);
                            if let Some((_, _, old_size)) = self.textures.remove(&resp.page_idx) {
                                self.total_texture_bytes =
                                    self.total_texture_bytes.saturating_sub(old_size);
                            }
                            while self.textures.len() >= MAX_CACHED_TEXTURES
                                || self.total_texture_bytes.saturating_add(byte_size)
                                    > MAX_TEXTURE_BYTES
                            {
                                let Some((&oldest_key, _)) = self
                                    .textures
                                    .iter()
                                    .min_by_key(|(_, (_, access, _))| *access)
                                else {
                                    break;
                                };
                                if let Some((_, _, old_size)) = self.textures.remove(&oldest_key) {
                                    self.total_texture_bytes =
                                        self.total_texture_bytes.saturating_sub(old_size);
                                }
                            }
                            self.textures
                                .insert(resp.page_idx, (texture, counter, byte_size));
                            self.total_texture_bytes =
                                self.total_texture_bytes.saturating_add(byte_size);
                        }
                        Err(e) => {
                            let msg = format_page_error(&e, lang);
                            *status_message = Some(crate::i18n::fill(
                                t.extra.page_load_error_detail,
                                &[
                                    ("{page}", &(resp.page_idx + 1).to_string()),
                                    ("{error}", &msg),
                                ],
                            ));
                            self.failed_pages.insert(resp.page_idx, msg);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    for page_idx in self.pending_pages.drain() {
                        let msg = format_page_error(
                            &DocumentError::Corrupted(ERR_DECODE_STREAM_CRASH.to_string()),
                            lang,
                        );
                        self.failed_pages.insert(page_idx, msg.clone());
                        *status_message = Some(crate::i18n::fill(
                            t.extra.page_load_error_detail,
                            &[("{page}", &(page_idx + 1).to_string()), ("{error}", &msg)],
                        ));
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
        settings: &ComicSettings,
        theme: &ReaderTheme,
        smart_scan_allowed: bool,
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
        let target_bg = theme.bg_color();
        let target_text = theme.text_color();
        let smart_dark = settings.smart_dark && smart_scan_allowed && theme.is_dark();
        let _ = self.req_tx.send(LoadRequest {
            doc_generation: self.doc_generation,
            doc,
            page_idx,
            ctx: ctx.clone(),
            smart_dark,
            target_bg,
            target_text,
            tolerance: settings.auto_crop_tolerance,
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
        lang: crate::i18n::UiLanguage,
        smart_scan_allowed: bool,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        let t = lang.t();
        let effective_smart_dark = settings.smart_dark && smart_scan_allowed;
        let effective_auto_crop = settings.auto_crop && smart_scan_allowed;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            let prev_enabled = self.current_page > 0;
            if ui
                .add_enabled(
                    prev_enabled,
                    IconButton::new(Icon::ArrowLeft, "").min_size(Vec2::new(28.0, 24.0)),
                )
                .on_hover_text(t.prev_page)
                .clicked()
            {
                self.prev_page(settings);
            }

            if self.page_input.is_empty() {
                self.page_input = (self.current_page + 1).to_string();
            }
            let cur_display = self.current_page + 1;
            ui.label(RichText::new(t.page_prefix).size(13.0));
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

            ui.label(RichText::new(format!("{} {page_count}", t.page_of)).size(13.0));

            let next_enabled = self.current_page + 1 < page_count;
            if ui
                .add_enabled(
                    next_enabled,
                    IconButton::new(Icon::ArrowRight, "").min_size(Vec2::new(28.0, 24.0)),
                )
                .on_hover_text(t.next_page)
                .clicked()
            {
                self.next_page(page_count, settings);
            }

            ui.separator();

            let is_manga_active = settings.reading_mode == "Manga";
            if ui
                .add(
                    IconButton::new(Icon::Book, t.manga_mode_btn)
                        .selected(is_manga_active)
                        .min_size(Vec2::new(96.0, 24.0)),
                )
                .clicked()
            {
                settings.reading_mode = "Manga".to_string();
            }

            let is_comic_active = settings.reading_mode == "Comic";
            if ui
                .add(
                    IconButton::new(Icon::Book, t.comic_mode_btn)
                        .selected(is_comic_active)
                        .min_size(Vec2::new(96.0, 24.0)),
                )
                .clicked()
            {
                settings.reading_mode = "Comic".to_string();
            }

            ui.separator();

            let is_single = settings.page_layout == "Single" && settings.reading_mode != "Webtoon";
            if ui.selectable_label(is_single, t.single_page_btn).clicked() {
                settings.page_layout = "Single".to_string();
                if settings.reading_mode == "Webtoon" {
                    settings.reading_mode = "Auto".to_string();
                }
                self.pan_offset = Vec2::ZERO;
            }

            let is_double = settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
            if ui.selectable_label(is_double, t.double_page_btn).clicked() {
                settings.page_layout = "Double".to_string();
                if settings.reading_mode == "Webtoon" {
                    settings.reading_mode = "Auto".to_string();
                }
                self.pan_offset = Vec2::ZERO;
            }

            let is_webtoon = settings.reading_mode == "Webtoon";
            if ui.selectable_label(is_webtoon, t.scroll_btn).clicked() {
                settings.reading_mode = "Webtoon".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            if ui
                .add(IconButton::new(Icon::Minus, "").min_size(Vec2::new(26.0, 24.0)))
                .on_hover_text(t.zoom_out)
                .clicked()
            {
                super::paged_common::zoom_out(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            let zoom_lbl = format!("{:.0}%", settings.zoom_percent);
            if ui
                .button(RichText::new(zoom_lbl).strong())
                .on_hover_text(t.zoom_reset)
                .clicked()
            {
                super::paged_common::zoom_reset(
                    &mut settings.zoom_percent,
                    &mut settings.fit_mode,
                    &mut self.pan_offset,
                );
            }

            if ui
                .add(IconButton::new(Icon::Plus, "").min_size(Vec2::new(26.0, 24.0)))
                .on_hover_text(t.zoom_in)
                .clicked()
            {
                super::paged_common::zoom_in(&mut settings.zoom_percent, &mut settings.fit_mode);
            }

            let fit_win_active = settings.fit_mode == "FitWindow";
            if ui
                .selectable_label(fit_win_active, t.fit_window_btn)
                .clicked()
            {
                settings.fit_mode = "FitWindow".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            let fit_w_active = settings.fit_mode == "FitWidth";
            if ui.selectable_label(fit_w_active, t.fit_width_btn).clicked() {
                settings.fit_mode = "FitWidth".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            let fit_h_active = settings.fit_mode == "FitHeight";
            if ui
                .selectable_label(fit_h_active, t.fit_height_btn)
                .clicked()
            {
                settings.fit_mode = "FitHeight".to_string();
                self.pan_offset = Vec2::ZERO;
            }

            ui.separator();

            let dark_resp = ui.add(
                crate::app::icons::IconButton::new(
                    crate::app::icons::Icon::Moon,
                    t.smart_dark_mode,
                )
                .selected(effective_smart_dark),
            );
            if dark_resp.on_hover_text(t.smart_dark_mode_tooltip).clicked() {
                if smart_scan_allowed {
                    settings.smart_dark = !settings.smart_dark;
                    self.clear_cache();
                } else {
                    *on_pro_required = Some(ProFeature::SmartScanProcessing);
                }
            }

            let crop_resp = ui
                .add(
                    crate::app::icons::IconButton::new(
                        crate::app::icons::Icon::Crop,
                        t.auto_crop_margins,
                    )
                    .selected(effective_auto_crop),
                )
                .on_hover_text(t.auto_crop_margins_tooltip);
            if crop_resp.clicked() {
                if smart_scan_allowed {
                    settings.auto_crop = !settings.auto_crop;
                    self.pan_offset = Vec2::ZERO;
                } else {
                    *on_pro_required = Some(ProFeature::SmartScanProcessing);
                }
            }
            if effective_auto_crop {
                crop_resp.context_menu(|ui| {
                    ui.label(RichText::new(t.crop_sensitivity).strong());
                    let slider = egui::Slider::new(&mut settings.auto_crop_tolerance, 5..=80)
                        .text(t.crop_tolerance);
                    if ui.add(slider).changed() {
                        self.clear_cache();
                    }
                });
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let badge_txt = if is_manga {
                    t.manga_mode_btn
                } else {
                    t.comic_mode_btn
                };
                ui.add(
                    IconLabel::new(Icon::Book, badge_txt)
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
        lang: crate::i18n::UiLanguage,
        smart_scan_allowed: bool,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        self.poll_responses(ctx, status_message, lang);

        let effective_smart_dark = settings.smart_dark && smart_scan_allowed;
        let auto_crop = settings.auto_crop && smart_scan_allowed;
        if effective_smart_dark != self.last_smart_dark
            || theme.bg_color() != self.last_theme_bg
            || settings.auto_crop_tolerance != self.last_crop_tolerance
        {
            self.last_smart_dark = effective_smart_dark;
            self.last_theme_bg = theme.bg_color();
            self.last_crop_tolerance = settings.auto_crop_tolerance;
            self.clear_cache();
        }

        let page_count = self.page_count(document);
        if page_count == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(lang.t().no_pages_in_doc).color(theme.text_color()));
            });
            return;
        }

        self.current_page = self.current_page.min(page_count.saturating_sub(1));
        let is_manga = self.is_effective_manga(document, settings);
        let t = lang.t();

        ui.add_space(2.0);
        self.render_toolbar(
            ui,
            page_count,
            settings,
            theme,
            is_manga,
            lang,
            smart_scan_allowed,
            on_pro_required,
        );
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
                            if let Some(tex) = self.request_page_texture(
                                ctx,
                                document,
                                p,
                                settings,
                                theme,
                                smart_scan_allowed,
                            ) {
                                let uv_rect = if auto_crop {
                                    self.page_bboxes.get(&p).copied().unwrap_or(
                                        egui::Rect::from_min_max(
                                            egui::Pos2::ZERO,
                                            egui::Pos2::new(1.0, 1.0),
                                        ),
                                    )
                                } else {
                                    egui::Rect::from_min_max(
                                        egui::Pos2::ZERO,
                                        egui::Pos2::new(1.0, 1.0),
                                    )
                                };
                                let tex_size = tex.size_vec2();
                                let content_size = Vec2::new(
                                    tex_size.x * uv_rect.width(),
                                    tex_size.y * uv_rect.height(),
                                );
                                let h = target_w * (content_size.y / content_size.x);
                                let img_widget = egui::Image::from_texture(&tex)
                                    .uv(uv_rect)
                                    .fit_to_exact_size(Vec2::new(target_w, h));
                                ui.add(img_widget);
                                ui.add_space(8.0);
                            } else if let Some(err) = self.failed_pages.get(&p) {
                                let label = format!("{} {}: {err}", t.page_load_error, p + 1);
                                ui.add(
                                    IconLabel::new(Icon::Warning, &label)
                                        .size(13.0)
                                        .color(theme.accent_color()),
                                );
                            } else {
                                ui.add_space(24.0);
                                let label = format!("{} {}...", t.page_loading, p + 1);
                                ui.add(
                                    IconLabel::new(Icon::Hourglass, &label)
                                        .size(14.0)
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
            let double_layout =
                settings.page_layout == "Double" && settings.reading_mode != "Webtoon";
            if double_layout && self.current_page > 0 && self.current_page % 2 == 0 {
                self.current_page -= 1;
            }
            let pages_to_show =
                comic_pages_to_show(self.current_page, page_count, double_layout, is_manga);

            for &p in &pages_to_show {
                let _ = self.request_page_texture(
                    ctx,
                    document,
                    p,
                    settings,
                    theme,
                    smart_scan_allowed,
                );
            }

            let tex_sizes: Vec<Vec2> = pages_to_show
                .iter()
                .map(|&p| {
                    let raw = self
                        .textures
                        .get(&p)
                        .map(|t| t.0.size_vec2())
                        .unwrap_or(Vec2::new(600.0, 800.0));
                    if auto_crop && let Some(bbox) = self.page_bboxes.get(&p) {
                        Vec2::new(raw.x * bbox.width(), raw.y * bbox.height())
                    } else {
                        raw
                    }
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
                        let uv_rect = if auto_crop {
                            self.page_bboxes
                                .get(&p)
                                .copied()
                                .unwrap_or(egui::Rect::from_min_max(
                                    egui::Pos2::ZERO,
                                    egui::Pos2::new(1.0, 1.0),
                                ))
                        } else {
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0))
                        };
                        let img_widget = egui::Image::from_texture(&tex)
                            .uv(uv_rect)
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
                                RichText::new(crate::i18n::fill(
                                    t.extra.page_error_detail,
                                    &[("{page}", &(p + 1).to_string()), ("{error}", err)],
                                ))
                                .size(16.0)
                                .color(theme.accent_color()),
                            );
                        });
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(avail_size.y * 0.45);
                            ui.label(
                                RichText::new(crate::i18n::fill(
                                    t.extra.page_loading_number,
                                    &[("{page}", &(p + 1).to_string())],
                                ))
                                .size(18.0)
                                .color(theme.text_color()),
                            );
                        });
                    }
                    ui.add_space(12.0);
                }
            });

            for offset in [1usize, 2] {
                if let Some(page) = self.current_page.checked_add(offset)
                    && page < page_count
                {
                    let _ = self.request_page_texture(
                        ctx,
                        document,
                        page,
                        settings,
                        theme,
                        smart_scan_allowed,
                    );
                }
            }
            for offset in [1usize, 2] {
                if let Some(page) = self.current_page.checked_sub(offset) {
                    let _ = self.request_page_texture(
                        ctx,
                        document,
                        page,
                        settings,
                        theme,
                        smart_scan_allowed,
                    );
                }
            }

            if next_clicked {
                self.next_page(page_count, settings);
            } else if prev_clicked {
                self.prev_page(settings);
            }
        }
    }
}
