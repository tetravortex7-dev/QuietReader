use super::append_with_highlight;
use crate::app::icons::{Icon, paint_icon};
use crate::app::reader_chrome::ReaderSurfaceColors;
use crate::core::library::UiDesignMode;
use crate::document::{BookChapter, Document, footnote_tokens, strip_formatting_tags};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, RichText, Stroke, StrokeKind, Vec2};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(crate) struct FootnoteMarker {
    pub id: String,
    pub number: usize,
    pub start_char: usize,
    pub end_char: usize,
}

pub(super) struct ParagraphStyle<'a> {
    pub font_size: f32,
    pub text_color: Color32,
    pub accent_color: Color32,
    pub line_height: f32,
    pub query: &'a str,
    pub is_active_hit: bool,
    pub is_rtl: bool,
    pub bionic_reading: bool,
    pub is_opening: bool,
    pub is_blockquote: bool,
    pub quotes: &'a [(&'a str, Color32)],
    pub footnote_numbers: &'a HashMap<String, usize>,
    pub note_available: &'a HashSet<String>,
    pub tts_word_range: Option<(usize, usize)>,
}

pub(super) fn append_text_with_footnotes(
    job: &mut egui::text::LayoutJob,
    text: &str,
    base_format: &egui::TextFormat,
    style: &ParagraphStyle<'_>,
    bold: bool,
    italic: bool,
    markers: &mut Vec<FootnoteMarker>,
) {
    let tokens = footnote_tokens(text);
    if tokens.is_empty() {
        append_text(job, text, base_format, style, bold, italic);
        return;
    }
    let mut last = 0;
    for (id, start, end) in tokens {
        if start > last {
            append_text(job, &text[last..start], base_format, style, bold, italic);
        }
        let number = style.footnote_numbers.get(id).copied().unwrap_or(1);
        let marker_text = format!("[{number}]");
        if !style.note_available.contains(id) {
            job.append(&marker_text, 0.0, base_format.clone());
            last = end;
            continue;
        }
        let start_char = job.text.chars().count();
        let mut marker_format = base_format.clone();
        marker_format.color = style.accent_color;
        marker_format.background = style.accent_color.gamma_multiply(0.14);
        marker_format.underline = Stroke::new(1.0, style.accent_color.gamma_multiply(0.55));
        marker_format.line_height = Some(style.line_height);
        job.append(&marker_text, 0.0, marker_format);
        let end_char = job.text.chars().count();
        markers.push(FootnoteMarker {
            id: id.to_string(),
            number,
            start_char,
            end_char,
        });
        last = end;
    }
    if last < text.len() {
        append_text(job, &text[last..], base_format, style, bold, italic);
    }
}

fn append_text(
    job: &mut egui::text::LayoutJob,
    text: &str,
    base_format: &egui::TextFormat,
    style: &ParagraphStyle<'_>,
    bold: bool,
    italic: bool,
) {
    if !style.bionic_reading || bold {
        append_with_highlight(
            job,
            text,
            base_format,
            style.query,
            style.is_active_hit,
            style.quotes,
        );
        return;
    }
    for (chunk, is_bold) in crate::app::bionic::split_bionic_spans(text) {
        let family = match (is_bold, italic) {
            (true, true) => {
                egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD_ITALIC.into())
            }
            (true, false) => egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
            (false, true) => egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_ITALIC.into()),
            (false, false) => egui::FontFamily::Proportional,
        };
        let mut format = base_format.clone();
        format.font_id = egui::FontId::new(style.font_size, family);
        format.line_height = Some(style.line_height);
        append_with_highlight(
            job,
            chunk,
            &format,
            style.query,
            style.is_active_hit,
            style.quotes,
        );
    }
}

pub(super) fn collect_numbers(chapter: &BookChapter) -> HashMap<String, usize> {
    let mut numbers = HashMap::new();
    for text in
        std::iter::once(chapter.title.as_str()).chain(chapter.paragraphs.iter().map(String::as_str))
    {
        for (id, _, _) in footnote_tokens(text) {
            let next = numbers.len() + 1;
            numbers.entry(id.to_string()).or_insert(next);
        }
    }
    numbers
}

const FOOTNOTE_CARDS_STATE_KEY: &str = "quietreader_footnote_cards";

pub(super) struct FootnoteFrameState {
    prev_cards: Vec<(String, egui::Rect)>,
    next_cards: Vec<(String, egui::Rect)>,
    any_hit: bool,
}

impl FootnoteFrameState {
    pub(super) fn begin(ctx: &egui::Context) -> Self {
        let prev_cards = ctx
            .data(|data| {
                data.get_temp::<Vec<(String, egui::Rect)>>(egui::Id::unique(
                    FOOTNOTE_CARDS_STATE_KEY,
                ))
            })
            .unwrap_or_default();
        Self {
            prev_cards,
            next_cards: Vec::new(),
            any_hit: false,
        }
    }

    pub(super) fn finish(self, ctx: &egui::Context, clicked: bool, pinned: &mut Option<String>) {
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let over_any_card = pointer.is_some_and(|pos| {
            self.prev_cards
                .iter()
                .any(|(_, rect)| rect.expand(4.0).contains(pos))
        });
        if clicked && !self.any_hit && !over_any_card {
            *pinned = None;
        }
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::unique(FOOTNOTE_CARDS_STATE_KEY), self.next_cards);
        });
    }
}

pub(crate) fn update_pinned_on_markers(
    marker_ids: &[&str],
    hovered: &[bool],
    over_card: &[bool],
    clicked: bool,
    pinned: &mut Option<String>,
) -> bool {
    let mut hit_any = false;
    for ((id, &is_hovered), &is_over_card) in
        marker_ids.iter().zip(hovered.iter()).zip(over_card.iter())
    {
        if is_hovered || is_over_card {
            hit_any = true;
        }
        if clicked && is_hovered {
            if pinned.as_deref() == Some(*id) {
                *pinned = None;
            } else {
                *pinned = Some((*id).to_string());
            }
        }
    }
    hit_any
}

#[allow(clippy::too_many_arguments)]
pub(super) fn show_hover_card(
    ui: &mut egui::Ui,
    document: &dyn Document,
    response: &egui::Response,
    galley: &egui::Galley,
    markers: &[FootnoteMarker],
    colors: &ReaderSurfaceColors,
    design: UiDesignMode,
    t: &Translations,
    font_size: f32,
    pinned: &mut Option<String>,
    frame_state: &mut FootnoteFrameState,
) -> Option<String> {
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let clicked = ui.input(|i| i.pointer.primary_clicked());
    let origin = match galley.job.halign {
        egui::Align::LEFT => response.rect.left_top(),
        egui::Align::RIGHT => response.rect.right_top(),
        egui::Align::Center => egui::pos2(response.rect.center().x, response.rect.top()),
    };
    let suppressed_id_key = egui::Id::unique("quietreader_footnote_suppressed");
    let mut suppressed: Option<String> = ui.ctx().data(|d| d.get_temp(suppressed_id_key));

    let mut rects = Vec::with_capacity(markers.len());
    let mut hovered = Vec::with_capacity(markers.len());
    let mut over_card = Vec::with_capacity(markers.len());
    let mut marker_ids = Vec::with_capacity(markers.len());
    for marker in markers {
        let start = galley.pos_from_cursor(egui::text::CCursor::new(marker.start_char));
        let end = galley.pos_from_cursor(egui::text::CCursor::new(marker.end_char));
        let rect = egui::Rect::from_min_max(
            egui::pos2(origin.x + start.min.x, origin.y + start.min.y),
            egui::pos2(origin.x + end.min.x, origin.y + start.max.y),
        )
        .expand(2.0);
        let is_hovered = pointer.is_some_and(|position| rect.contains(position));
        let is_over_card = pointer.is_some_and(|position| {
            frame_state
                .prev_cards
                .iter()
                .any(|(id, card_rect)| id == &marker.id && card_rect.expand(4.0).contains(position))
        });
        if suppressed.as_deref() == Some(marker.id.as_str()) && !is_hovered {
            suppressed = None;
        }
        rects.push(rect);
        hovered.push(is_hovered);
        over_card.push(is_over_card);
        marker_ids.push(marker.id.as_str());
    }

    if clicked {
        for (idx, marker) in markers.iter().enumerate() {
            if hovered[idx] && suppressed.as_deref() == Some(marker.id.as_str()) {
                suppressed = None;
            }
        }
    }

    if update_pinned_on_markers(&marker_ids, &hovered, &over_card, clicked, pinned) {
        frame_state.any_hit = true;
    }

    let mut jump_to = None;
    for (idx, marker) in markers.iter().enumerate() {
        let is_pinned = pinned.as_deref() == Some(marker.id.as_str());
        let is_suppressed = suppressed.as_deref() == Some(marker.id.as_str());
        if (hovered[idx] && !is_suppressed) || over_card[idx] || is_pinned {
            let text = document.footnote(&marker.id);
            let can_jump = document.note_location(&marker.id).is_some();
            let outcome = show_card(
                ui,
                marker,
                rects[idx],
                response.rect,
                text,
                can_jump,
                colors,
                design,
                t,
                font_size,
                frame_state,
            );
            if outcome.dismiss {
                *pinned = None;
                suppressed = Some(marker.id.clone());
                break;
            }
            if let Some(note_id) = outcome.jump_to {
                *pinned = None;
                jump_to = Some(note_id);
                break;
            }
        }
    }
    ui.ctx().data_mut(|d| {
        if let Some(val) = suppressed {
            d.insert_temp(suppressed_id_key, val);
        } else {
            d.remove_temp::<String>(suppressed_id_key);
        }
    });
    jump_to
}

struct FootnoteCardOutcome {
    jump_to: Option<String>,
    dismiss: bool,
}

#[allow(clippy::too_many_arguments)]
fn show_card(
    ui: &mut egui::Ui,
    marker: &FootnoteMarker,
    marker_rect: egui::Rect,
    column_bounds: egui::Rect,
    text: Option<&str>,
    can_jump: bool,
    colors: &ReaderSurfaceColors,
    design: UiDesignMode,
    t: &Translations,
    font_size: f32,
    frame_state: &mut FootnoteFrameState,
) -> FootnoteCardOutcome {
    let mut outcome = FootnoteCardOutcome {
        jump_to: None,
        dismiss: false,
    };
    let screen = ui.ctx().content_rect();
    let card_width = 340.0_f32.min((screen.width() - 32.0).max(220.0));
    let (min_x, max_right) =
        if design == UiDesignMode::Cozy && column_bounds.width() >= card_width * 0.85 {
            (
                (column_bounds.min.x - 8.0).max(screen.left() + 16.0),
                (column_bounds.max.x + 8.0).min(screen.right() - 28.0),
            )
        } else {
            (screen.left() + 12.0, screen.right() - 12.0)
        };
    let mut position = egui::pos2(marker_rect.left() - 10.0, marker_rect.bottom() + 6.0);
    position.x = position
        .x
        .min((max_right - card_width).max(min_x))
        .max(min_x);
    if position.y + 220.0 > screen.bottom() {
        position.y = (marker_rect.top() - 200.0).max(screen.top() + 12.0);
    }
    let radius = design.corner_radius() as f32;
    let area_id = ui
        .scope_id()
        .with(("footnote_card", marker.id.as_str(), marker.start_char));
    let card = egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(position)
        .show(ui.ctx(), |ui| {
            let frame = egui::Frame::new()
                .fill(colors.card_bg)
                .stroke(Stroke::new(1.0, colors.card_border))
                .corner_radius(radius)
                .inner_margin(egui::Margin::symmetric(14, 12))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(if colors.is_dark { 90 } else { 26 }),
                });
            frame.show(ui, |ui| {
                ui.set_max_width(card_width);
                ui.horizontal(|ui| {
                    let (badge_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(badge_rect.center(), 10.0, colors.accent_soft_bg);
                    ui.painter().text(
                        badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        marker.number.to_string(),
                        egui::FontId::proportional(10.5),
                        colors.accent,
                    );
                    ui.label(
                        RichText::new(t.footnote_title.to_uppercase())
                            .size(10.5)
                            .strong()
                            .color(colors.page_muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (xr, xresp) =
                            ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::click());
                        if xresp.hovered() {
                            ui.painter().circle_filled(xr.center(), 9.0, colors.pill_bg);
                        }
                        paint_icon(
                            ui.painter(),
                            Icon::Cross,
                            egui::Rect::from_center_size(xr.center(), Vec2::splat(9.0)),
                            colors.page_muted,
                        );
                        if xresp.clicked() {
                            outcome.dismiss = true;
                        }
                    });
                });
                ui.add_space(6.0);
                if let Some(text) = text {
                    egui::ScrollArea::vertical()
                        .max_height(210.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(strip_formatting_tags(text))
                                    .size((font_size * 0.72).clamp(12.5, 16.0))
                                    .color(colors.page_ink),
                            );
                        });
                } else {
                    ui.label(
                        RichText::new(t.footnote_not_found)
                            .size((font_size * 0.72).clamp(12.5, 16.0))
                            .italics()
                            .color(colors.page_muted),
                    );
                }
                ui.add_space(8.0);
                let div_y = ui.cursor().min.y;
                let div_left = ui.cursor().min.x;
                ui.painter().line_segment(
                    [
                        egui::pos2(div_left, div_y),
                        egui::pos2(div_left + card_width - 28.0, div_y),
                    ],
                    Stroke::new(1.0, colors.bar_border.gamma_multiply(0.55)),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if can_jump {
                        let font = egui::FontId::proportional(11.5);
                        let label_galley = ui.painter().layout_no_wrap(
                            t.footnote_go_to.to_string(),
                            font,
                            colors.accent,
                        );
                        let w = label_galley.size().x + 18.0;
                        let (lr, lresp) =
                            ui.allocate_exact_size(Vec2::new(w, 22.0), egui::Sense::click());
                        if lresp.hovered() {
                            ui.painter().rect_filled(lr, 6.0, colors.accent_soft_bg);
                        }
                        ui.painter().galley(
                            egui::pos2(lr.min.x + 2.0, lr.center().y - label_galley.size().y * 0.5),
                            label_galley,
                            colors.accent,
                        );
                        paint_icon(
                            ui.painter(),
                            Icon::ArrowRight,
                            egui::Rect::from_center_size(
                                egui::pos2(lr.max.x - 7.0, lr.center().y),
                                Vec2::splat(10.0),
                            ),
                            colors.accent,
                        );
                        if lresp.clicked() {
                            outcome.jump_to = Some(marker.id.clone());
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let hide_font = egui::FontId::proportional(11.5);
                        let hide_g = ui.painter().layout_no_wrap(
                            t.extra.reader_footnote_hide.to_string(),
                            hide_font,
                            colors.page_muted,
                        );
                        let (hr, hresp) = ui.allocate_exact_size(
                            Vec2::new(hide_g.size().x + 14.0, 22.0),
                            egui::Sense::click(),
                        );
                        if hresp.hovered() {
                            ui.painter().rect(
                                hr,
                                6.0,
                                colors.pill_bg,
                                Stroke::new(1.0, colors.card_border),
                                StrokeKind::Inside,
                            );
                        }
                        ui.painter().galley(
                            egui::pos2(
                                hr.center().x - hide_g.size().x * 0.5,
                                hr.center().y - hide_g.size().y * 0.5,
                            ),
                            hide_g,
                            if hresp.hovered() {
                                colors.page_ink
                            } else {
                                colors.page_muted
                            },
                        );
                        if hresp.clicked() {
                            outcome.dismiss = true;
                        }
                    });
                });
            });
        });
    frame_state
        .next_cards
        .push((marker.id.clone(), card.response.rect));
    outcome
}
