use super::{Icon, paint_icon};
use eframe::egui::{self, Color32, Pos2, Rect, Response, StrokeKind, Ui, Vec2, Widget};

pub struct IconButton<'a> {
    pub icon: Icon,
    pub text: &'a str,
    pub font_size: Option<f32>,
    pub min_size: Option<Vec2>,
    pub selected: bool,
    pub center_content: bool,
    pub color: Option<Color32>,
    pub fill: Option<Color32>,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: Icon, text: &'a str) -> Self {
        Self {
            icon,
            text,
            font_size: None,
            min_size: None,
            selected: false,
            center_content: false,
            color: None,
            fill: None,
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = Some(size);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn center(mut self, center: bool) -> Self {
        self.center_content = center;
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    #[allow(dead_code)]
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }
}

impl<'a> Widget for IconButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let padding = ui.spacing().button_padding;
        let icon_size = self.font_size.unwrap_or(15.0);
        let has_text = !self.text.is_empty();
        let gap = if has_text { 6.0 } else { 0.0 };

        let font_id = match self.font_size {
            Some(sz) => egui::FontId::proportional(sz),
            None => egui::TextStyle::Button.resolve(ui.style()),
        };
        let initial_text_color = self.color.unwrap_or_else(|| {
            ui.visuals()
                .override_text_color
                .unwrap_or_else(|| ui.visuals().text_color())
        });
        let galley =
            ui.painter()
                .layout_no_wrap(self.text.to_string(), font_id.clone(), initial_text_color);
        let text_size = if has_text { galley.size() } else { Vec2::ZERO };

        let mut desired_size = Vec2::new(
            padding.x * 2.0 + icon_size + gap + text_size.x,
            (text_size.y.max(icon_size) + padding.y * 2.0).max(ui.spacing().interact_size.y),
        );

        if let Some(min_sz) = self.min_size {
            desired_size.x = desired_size.x.max(min_sz.x);
            desired_size.y = desired_size.y.max(min_sz.y);
        }

        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = if self.selected {
                ui.style().interact_selectable(&response, true)
            } else {
                *ui.style().interact(&response)
            };

            let item_color = self.color.unwrap_or_else(|| visuals.text_color());
            let galley = if item_color != initial_text_color && has_text {
                ui.painter()
                    .layout_no_wrap(self.text.to_string(), font_id, item_color)
            } else {
                galley
            };

            let bg_fill = self.fill.unwrap_or(visuals.bg_fill);

            ui.painter().rect(
                rect,
                visuals.corner_radius,
                bg_fill,
                visuals.bg_stroke,
                StrokeKind::Inside,
            );

            let content_w = if has_text {
                icon_size + gap + text_size.x
            } else {
                icon_size
            };
            let start_x = if self.center_content || self.min_size.is_some() || !has_text {
                rect.min.x + (rect.width() - content_w).max(0.0) * 0.5
            } else {
                rect.min.x + padding.x
            };

            let icon_rect = Rect::from_center_size(
                Pos2::new(start_x + icon_size * 0.5, rect.center().y),
                Vec2::splat(icon_size),
            );
            paint_icon(ui.painter(), self.icon, icon_rect, item_color);

            if has_text {
                let text_pos = Pos2::new(
                    start_x + icon_size + gap,
                    rect.center().y - text_size.y * 0.5,
                );
                ui.painter().galley(text_pos, galley, item_color);
            }
        }

        response
    }
}

pub struct IconLabel<'a> {
    pub icon: Icon,
    pub text: &'a str,
    pub font_size: f32,
    pub color: Option<Color32>,
    pub strong: bool,
}

impl<'a> IconLabel<'a> {
    pub fn new(icon: Icon, text: &'a str) -> Self {
        Self {
            icon,
            text,
            font_size: 14.0,
            color: None,
            strong: false,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }
}

impl<'a> Widget for IconLabel<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let text_color = self.color.unwrap_or_else(|| ui.visuals().text_color());
        let icon_size = self.font_size;
        let has_text = !self.text.is_empty();
        let gap = if has_text { 6.0 } else { 0.0 };

        let font_id = if self.strong {
            egui::FontId::new(
                self.font_size,
                egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
            )
        } else {
            egui::FontId::proportional(self.font_size)
        };

        let galley = if has_text {
            Some(
                ui.painter()
                    .layout_no_wrap(self.text.to_string(), font_id, text_color),
            )
        } else {
            None
        };
        let text_size = galley.as_ref().map(|g| g.size()).unwrap_or(Vec2::ZERO);

        let desired_size = if has_text {
            Vec2::new(icon_size + gap + text_size.x, text_size.y.max(icon_size))
        } else {
            Vec2::splat(icon_size)
        };
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let icon_rect = Rect::from_center_size(
                Pos2::new(rect.min.x + icon_size * 0.5, rect.center().y),
                Vec2::splat(icon_size),
            );
            paint_icon(ui.painter(), self.icon, icon_rect, text_color);

            if let Some(g) = galley {
                let text_pos = Pos2::new(
                    rect.min.x + icon_size + gap,
                    rect.center().y - text_size.y * 0.5,
                );
                ui.painter().galley(text_pos, g, text_color);
            }
        }

        response
    }
}

pub fn icon_button<'a>(ui: &mut Ui, icon: Icon, text: &'a str) -> Response {
    ui.add(IconButton::new(icon, text))
}

pub fn selectable_icon_button<'a>(
    ui: &mut Ui,
    selected: bool,
    icon: Icon,
    text: &'a str,
) -> Response {
    ui.add(
        IconButton::new(icon, text)
            .selected(selected)
            .center(true)
            .min_size(Vec2::new(115.0, 32.0)),
    )
}

pub const STATUS_ARROW_SEP: char = '\x1f';

pub fn paint_dot_separator(ui: &mut Ui, font_size: f32, color: Color32) {
    let radius = (font_size * 0.14).clamp(1.2, 2.0);
    let width = (font_size * 0.6).max(6.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, font_size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().circle_filled(rect.center(), radius, color);
    }
}

pub fn paint_arrow_pair(ui: &mut Ui, left: &str, right: &str, color: Color32, is_rtl: bool) {
    let icon = if is_rtl {
        Icon::ArrowLeft
    } else {
        Icon::ArrowRight
    };
    ui.label(egui::RichText::new(left).color(color));
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(13.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_icon(ui.painter(), icon, rect, color);
    }
    ui.label(egui::RichText::new(right).color(color));
}

fn measure_dotted_tokens(
    painter: &egui::Painter,
    tokens: &[&str],
    font_id: &egui::FontId,
    text_color: Color32,
    max_width: Option<f32>,
) -> (Vec<std::sync::Arc<egui::Galley>>, f32, f32, f32, f32) {
    let galleys: Vec<_> = tokens
        .iter()
        .map(|tok| painter.layout_no_wrap((*tok).to_string(), font_id.clone(), text_color))
        .collect();
    let dot_radius = (font_id.size * 0.14).clamp(1.2, 2.0);
    let mut gap = (font_id.size * 0.45).max(4.0);
    let text_w: f32 = galleys.iter().map(|g| g.size().x).sum();
    let max_h: f32 = galleys
        .iter()
        .map(|g| g.size().y)
        .fold(font_id.size, f32::max);
    let seps = galleys.len().saturating_sub(1) as f32;
    if seps > 0.0
        && let Some(limit) = max_width
    {
        let dot_w = seps * dot_radius * 2.0;
        let max_gap = ((limit - text_w - dot_w) / (seps * 2.0)).max(1.5);
        gap = gap.min(max_gap);
    }
    let total_w = text_w + seps * (gap * 2.0 + dot_radius * 2.0);
    (galleys, gap, dot_radius, total_w, max_h)
}

pub fn paint_dotted_tokens(
    painter: &egui::Painter,
    rect: Rect,
    tokens: &[&str],
    font_id: egui::FontId,
    text_color: Color32,
    dot_color: Color32,
) {
    if tokens.is_empty() {
        return;
    }
    let (galleys, gap, dot_radius, total_w, _) =
        measure_dotted_tokens(painter, tokens, &font_id, text_color, Some(rect.width()));
    let mut x = (rect.center().x - total_w * 0.5).max(rect.min.x);
    let cy = rect.center().y;
    for (idx, galley) in galleys.into_iter().enumerate() {
        if idx > 0 {
            x += gap;
            painter.circle_filled(Pos2::new(x + dot_radius, cy), dot_radius, dot_color);
            x += dot_radius * 2.0 + gap;
        }
        let g_size = galley.size();
        painter.galley(Pos2::new(x, cy - g_size.y * 0.5), galley, text_color);
        x += g_size.x;
    }
}

pub fn dotted_strip(
    ui: &mut Ui,
    tokens: &[&str],
    font_size: f32,
    text_color: Color32,
    dot_color: Color32,
) {
    if tokens.is_empty() {
        return;
    }
    let font_id = egui::FontId::proportional(font_size);
    let (_, _, _, total_w, max_h) =
        measure_dotted_tokens(ui.painter(), tokens, &font_id, text_color, None);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, max_h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_dotted_tokens(ui.painter(), rect, tokens, font_id, text_color, dot_color);
    }
}

pub fn dotted_wrapped_row(
    ui: &mut Ui,
    segments: &[&str],
    font_size: f32,
    text_color: Color32,
    dot_color: Color32,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (idx, seg) in segments.iter().enumerate() {
            if idx > 0 {
                paint_dot_separator(ui, font_size, dot_color);
            }
            ui.label(egui::RichText::new(*seg).size(font_size).color(text_color));
        }
    });
}

pub fn dotted_responsive_row(
    ui: &mut Ui,
    segments: &[&str],
    font_size: f32,
    text_color: Color32,
    dot_color: Color32,
) {
    if segments.is_empty() {
        return;
    }
    let font_id = egui::FontId::proportional(font_size);
    let (_, _, _, total_w, _) =
        measure_dotted_tokens(ui.painter(), segments, &font_id, text_color, None);
    if total_w <= ui.available_width() {
        dotted_strip(ui, segments, font_size, text_color, dot_color);
    } else {
        dotted_wrapped_row(ui, segments, font_size, text_color, dot_color);
    }
}
