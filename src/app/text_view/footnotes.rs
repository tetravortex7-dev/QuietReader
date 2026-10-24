use super::append_with_highlight;
use crate::app::theme::ReaderTheme;
use crate::document::{BookChapter, Document, footnote_tokens, strip_formatting_tags};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, RichText, Stroke};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub(super) struct FootnoteMarker {
    pub id: String,
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
    pub quotes: &'a [(&'a str, Color32)],
    pub footnote_numbers: &'a HashMap<String, usize>,
    pub note_available: &'a HashSet<String>,
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
        marker_format.background = style.accent_color.gamma_multiply(0.08);
        marker_format.underline = Stroke::new(1.0, style.accent_color.gamma_multiply(0.65));
        marker_format.line_height = Some(style.line_height);
        job.append(&marker_text, 0.0, marker_format);
        let end_char = job.text.chars().count();
        markers.push(FootnoteMarker {
            id: id.to_string(),
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

pub(super) fn show_hover_card(
    ui: &mut egui::Ui,
    document: &dyn Document,
    response: &egui::Response,
    galley: &egui::Galley,
    markers: &[FootnoteMarker],
    theme: &ReaderTheme,
    t: &Translations,
    font_size: f32,
    pinned: &mut Option<String>,
) -> Option<String> {
    let pointer = ui.input(|i| i.pointer.hover_pos());
    let clicked = ui.input(|i| i.pointer.primary_clicked());
    let card_state_id = egui::Id::new("quietreader_footnote_card_last");
    let last_card: Option<(String, egui::Rect)> =
        ui.ctx().data(|data| data.get_temp(card_state_id));
    let origin = match galley.job.halign {
        egui::Align::LEFT => response.rect.left_top(),
        egui::Align::RIGHT => response.rect.right_top(),
        egui::Align::Center => egui::pos2(response.rect.center().x, response.rect.top()),
    };
    let mut hovered_any = false;
    for marker in markers {
        let start = galley.pos_from_cursor(egui::text::CCursor::new(marker.start_char));
        let end = galley.pos_from_cursor(egui::text::CCursor::new(marker.end_char));
        let rect = egui::Rect::from_min_max(
            egui::pos2(origin.x + start.min.x, origin.y + start.min.y),
            egui::pos2(origin.x + end.min.x, origin.y + start.max.y),
        )
        .expand(2.0);
        let hovered = pointer.is_some_and(|position| rect.contains(position));
        if hovered {
            hovered_any = true;
        }
        if clicked && hovered {
            if pinned.as_deref() == Some(marker.id.as_str()) {
                *pinned = None;
            } else {
                *pinned = Some(marker.id.clone());
            }
        }
        let over_card = pointer.is_some_and(|position| {
            last_card.as_ref().is_some_and(|(id, card_rect)| {
                id == &marker.id && card_rect.expand(4.0).contains(position)
            })
        });

        if hovered || over_card || pinned.as_deref() == Some(marker.id.as_str()) {
            let text = document.footnote(&marker.id);
            let can_jump = document.note_location(&marker.id).is_some();
            if let Some(note_id) = show_card(ui, marker, rect, text, can_jump, theme, t, font_size)
            {
                *pinned = None;
                return Some(note_id);
            }
        }
        if clicked && !hovered_any && !over_card {
            *pinned = None;
        }
    }
    None
}

fn show_card(
    ui: &mut egui::Ui,
    marker: &FootnoteMarker,
    marker_rect: egui::Rect,
    text: Option<&str>,
    can_jump: bool,
    theme: &ReaderTheme,
    t: &Translations,
    font_size: f32,
) -> Option<String> {
    let mut jump_to = None;
    let screen = ui.ctx().content_rect();
    let card_width = 360.0_f32.min((screen.width() - 24.0).max(220.0));
    let mut position = egui::pos2(marker_rect.left(), marker_rect.bottom() + 6.0);
    position.x = position
        .x
        .min(screen.right() - card_width - 12.0)
        .max(screen.left() + 12.0);
    if position.y + 220.0 > screen.bottom() {
        position.y = (marker_rect.top() - 220.0).max(screen.top() + 12.0);
    }
    let area_id = ui
        .scope_id()
        .with(("footnote_card", marker.id.as_str(), marker.start_char));
    let card = egui::Area::new(area_id)
        .order(egui::Order::Tooltip)
        .fixed_pos(position)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(theme.panel_bg_color())
                .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.55)))
                .corner_radius(8.0)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_max_width(card_width);
                    ui.label(
                        RichText::new(t.footnote_title)
                            .size(11.0)
                            .strong()
                            .color(theme.accent_color()),
                    );
                    ui.add_space(3.0);
                    if let Some(text) = text {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(strip_formatting_tags(text))
                                        .size((font_size * 0.72).clamp(12.0, 16.0))
                                        .color(theme.text_color()),
                                );
                            });
                    } else {
                        ui.label(
                            RichText::new(t.footnote_not_found)
                                .size((font_size * 0.72).clamp(12.0, 16.0))
                                .italics()
                                .color(theme.text_color().gamma_multiply(0.7)),
                        );
                    }
                    if can_jump {
                        ui.add_space(6.0);
                        if ui.button(t.footnote_go_to).clicked() {
                            jump_to = Some(marker.id.clone());
                        }
                    }
                });
        });
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new("quietreader_footnote_card_last"),
            (marker.id.clone(), card.response.rect),
        );
    });
    jump_to
}
