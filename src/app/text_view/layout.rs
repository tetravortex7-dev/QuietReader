use super::footnotes::{self, ParagraphStyle};
use eframe::egui::{self, Color32, Stroke};
use std::collections::HashSet;

pub(super) fn append_with_highlight(
    job: &mut egui::text::LayoutJob,
    text: &str,
    base_format: &egui::TextFormat,
    query: &str,
    is_active_hit: bool,
    quotes: &[(&str, Color32)],
) {
    if quotes.is_empty() {
        append_with_query_highlight(job, text, base_format, query, is_active_hit);
        return;
    }

    let lower_text = crate::core::normalize_search_text(text);
    let mut intervals: Vec<(usize, usize, Color32)> = Vec::new();
    for (quote_text, marker_color) in quotes {
        let q_clean = quote_text.trim();
        if q_clean.is_empty() {
            continue;
        }
        let lower_q = crate::core::normalize_search_text(q_clean);
        let mut search_from = 0;
        while let Some(found_idx) = lower_text[search_from..].find(&lower_q) {
            let abs_start = search_from + found_idx;
            let abs_end = abs_start + lower_q.len();
            let safe_start = text.floor_char_boundary(abs_start);
            let safe_end = text.ceil_char_boundary(abs_end.min(text.len()));
            if safe_start < safe_end {
                intervals.push((safe_start, safe_end, *marker_color));
            }
            search_from = abs_end.max(search_from + 1);
            if search_from >= lower_text.len() {
                break;
            }
        }
    }

    if intervals.is_empty() {
        append_with_query_highlight(job, text, base_format, query, is_active_hit);
        return;
    }

    intervals.sort_by_key(|&(s, _, _)| s);
    let mut last_idx = 0;
    for (start, end, color) in intervals {
        if start < last_idx {
            continue;
        }
        if start > last_idx {
            append_with_query_highlight(
                job,
                &text[last_idx..start],
                base_format,
                query,
                is_active_hit,
            );
        }
        let mut quote_format = base_format.clone();
        quote_format.background = color;
        append_with_query_highlight(job, &text[start..end], &quote_format, query, is_active_hit);
        last_idx = end;
    }

    if last_idx < text.len() {
        append_with_query_highlight(job, &text[last_idx..], base_format, query, is_active_hit);
    }
}

fn append_with_query_highlight(
    job: &mut egui::text::LayoutJob,
    text: &str,
    base_format: &egui::TextFormat,
    query: &str,
    is_active_hit: bool,
) {
    let q = crate::core::normalize_search_text(query.trim());
    if q.is_empty() {
        job.append(text, 0.0, base_format.clone());
        return;
    }

    let lower_text = crate::core::normalize_search_text(text);
    let mut last_idx = 0;

    let highlight_format = if is_active_hit {
        egui::TextFormat {
            font_id: base_format.font_id.clone(),
            color: Color32::BLACK,
            background: Color32::from_rgb(255, 175, 0),
            italics: base_format.italics,
            ..Default::default()
        }
    } else {
        egui::TextFormat {
            font_id: base_format.font_id.clone(),
            color: base_format.color,
            background: Color32::from_rgba_unmultiplied(255, 215, 0, 110),
            italics: base_format.italics,
            ..Default::default()
        }
    };

    while let Some(found_idx) = lower_text[last_idx..].find(&q) {
        let match_start = last_idx + found_idx;
        let match_end = match_start + q.len();

        let safe_start = text.floor_char_boundary(match_start);
        let safe_end = text.ceil_char_boundary(match_end.min(text.len()));

        if safe_start > last_idx {
            job.append(&text[last_idx..safe_start], 0.0, base_format.clone());
        }

        job.append(&text[safe_start..safe_end], 0.0, highlight_format.clone());
        last_idx = safe_end;

        if last_idx >= text.len() {
            break;
        }
    }

    if last_idx < text.len() {
        job.append(&text[last_idx..], 0.0, base_format.clone());
    }
}

pub(super) fn create_paragraph_layout_job(
    paragraph: &str,
    style: &ParagraphStyle<'_>,
    markers: &mut Vec<footnotes::FootnoteMarker>,
) -> egui::text::LayoutJob {
    let spans = crate::document::parse_formatted_spans(paragraph);
    let mut job = egui::text::LayoutJob::default();
    if style.is_rtl {
        job.halign = egui::Align::RIGHT;
    }
    for (idx, span) in spans.iter().enumerate() {
        let text = if idx == 0 {
            if style.is_rtl {
                format!("{}    ", span.text)
            } else {
                format!("    {}", span.text)
            }
        } else {
            span.text.to_string()
        };
        if text.is_empty() {
            continue;
        }
        let family = match (span.bold, span.italic) {
            (true, true) => {
                egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD_ITALIC.into())
            }
            (true, false) => egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
            (false, true) => egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_ITALIC.into()),
            (false, false) => egui::FontFamily::Proportional,
        };
        let format = egui::TextFormat {
            font_id: egui::FontId::new(style.font_size, family),
            color: style.text_color,
            italics: span.italic,
            line_height: Some(style.line_height),
            ..Default::default()
        };
        footnotes::append_text_with_footnotes(
            &mut job,
            &text,
            &format,
            style,
            span.bold,
            span.italic,
            markers,
        );
    }
    if job.sections.is_empty() {
        let text = if style.is_rtl {
            format!("{paragraph}    ")
        } else {
            format!("    {paragraph}")
        };
        let format = egui::TextFormat {
            font_id: egui::FontId::proportional(style.font_size),
            color: style.text_color,
            line_height: Some(style.line_height),
            ..Default::default()
        };
        footnotes::append_text_with_footnotes(
            &mut job, &text, &format, style, false, false, markers,
        );
    }
    job
}

pub fn create_sample_preview_job(
    sample_text: &str,
    font_size: f32,
    text_color: Color32,
    line_height_scale: f32,
    bionic_reading: bool,
    is_rtl: bool,
) -> egui::text::LayoutJob {
    let footnote_numbers = std::collections::HashMap::new();
    let note_available = HashSet::new();
    let style = ParagraphStyle {
        font_size,
        text_color,
        accent_color: text_color,
        line_height: font_size * line_height_scale,
        query: "",
        is_active_hit: false,
        is_rtl,
        bionic_reading,
        quotes: &[],
        footnote_numbers: &footnote_numbers,
        note_available: &note_available,
    };
    let mut markers = Vec::new();
    create_paragraph_layout_job(sample_text, &style, &mut markers)
}

pub(super) fn selectable_label_preserving_on_secondary(
    ui: &mut egui::Ui,
    layout_job: egui::text::LayoutJob,
    allow_select: bool,
) -> egui::Response {
    let is_secondary_or_menu = ui.input(|i| {
        i.pointer.button_down(egui::PointerButton::Secondary)
            || i.pointer.button_pressed(egui::PointerButton::Secondary)
            || i.pointer.button_released(egui::PointerButton::Secondary)
    }) || egui::Popup::is_any_open(ui.ctx());

    if !allow_select {
        return ui.add(egui::Label::new(layout_job).wrap().selectable(false));
    }

    let has_sel = ui
        .ctx()
        .plugin::<egui::text_selection::LabelSelectionState>()
        .lock()
        .has_selection();
    let is_primary_released = ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
    let is_secondary_down = ui.input(|i| {
        i.pointer.button_pressed(egui::PointerButton::Secondary)
            || i.pointer.button_down(egui::PointerButton::Secondary)
    });

    if has_sel && (is_primary_released || is_secondary_down) {
        crate::app::selection::request_selection_capture(ui.ctx());
    }

    if !is_secondary_or_menu {
        return ui.add(egui::Label::new(layout_job).wrap().selectable(true));
    }

    let label = egui::Label::new(layout_job).wrap().selectable(false);
    let (galley_pos, galley, response) = label.layout_in_ui(ui);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Label, ui.is_enabled(), galley.text())
    });

    let click_response = ui.interact(response.rect, response.id, egui::Sense::click());

    if ui.is_rect_visible(response.rect) {
        let response_color = ui.style().visuals.text_color();
        let underline = if response.has_focus() || response.highlighted() {
            Stroke::new(1.0, response_color)
        } else {
            Stroke::NONE
        };

        let safe_response = ui.interact(response.rect, response.id, egui::Sense::hover());

        egui::text_selection::LabelSelectionState::label_text_selection(
            ui,
            &safe_response,
            galley_pos,
            galley,
            response_color,
            underline,
        );
    }

    click_response
}

pub(crate) fn page_index_after_wheel(index: usize, wheel_delta: f32, max_index: usize) -> usize {
    if wheel_delta < 0.0 {
        (index + 1).min(max_index)
    } else if wheel_delta > 0.0 {
        index.saturating_sub(1)
    } else {
        index
    }
}

pub(crate) fn page_offset(index: usize, page_height: f32, max_scroll: f32) -> f32 {
    (index as f32 * page_height).min(max_scroll.max(0.0))
}

pub(crate) fn split_paragraphs(paragraphs: &[String]) -> usize {
    if paragraphs.len() < 2 {
        return paragraphs.len();
    }
    let total: usize = paragraphs.iter().map(|text| text.chars().count()).sum();
    if total == 0 {
        return paragraphs.len() / 2;
    }
    let half = total / 2;
    let mut accumulated = 0usize;
    for (index, text) in paragraphs.iter().enumerate() {
        accumulated += text.chars().count();
        if accumulated >= half {
            return (index + 1).min(paragraphs.len());
        }
    }
    paragraphs.len()
}
