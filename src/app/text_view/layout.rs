use super::footnotes::{self, ParagraphStyle};
use eframe::egui::{self, Color32, Stroke};
use std::collections::HashSet;

pub(crate) fn append_with_highlight(
    job: &mut egui::text::LayoutJob,
    text: &str,
    base_format: &egui::TextFormat,
    query: &str,
    is_active_hit: bool,
    quotes: &[(&str, Color32)],
) {
    if quotes.is_empty() || quotes.iter().all(|(quote, _)| quote.trim().is_empty()) {
        append_with_query_highlight(job, text, base_format, query, is_active_hit);
        return;
    }

    let lower_text = crate::core::normalize_search_text(text);
    let mut index: Option<crate::core::NormalizedSearchIndex> = None;
    let mut intervals: Vec<(usize, usize, Color32)> = Vec::new();

    for (quote_text, marker_color) in quotes {
        let q_clean = quote_text.trim();
        if q_clean.is_empty() {
            continue;
        }
        let lower_q = crate::core::normalize_search_text(q_clean);
        if lower_q.is_empty() || !lower_text.contains(&lower_q) {
            continue;
        }
        let idx = index.get_or_insert_with(|| {
            crate::core::NormalizedSearchIndex::from_normalized(text, lower_text.clone())
        });
        for (start, end) in idx.find_normalized_ranges(&lower_q) {
            if start < end {
                intervals.push((start, end, *marker_color));
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
        if end <= last_idx {
            continue;
        }
        let start = start.max(last_idx);
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
    if !lower_text.contains(&q) {
        job.append(text, 0.0, base_format.clone());
        return;
    }

    let index = crate::core::NormalizedSearchIndex::from_normalized(text, lower_text);
    let ranges = index.find_normalized_ranges(&q);
    if ranges.is_empty() {
        job.append(text, 0.0, base_format.clone());
        return;
    }

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

    let mut last_idx = 0;
    for (start, end) in ranges {
        if end <= last_idx {
            continue;
        }
        let start = start.max(last_idx);
        if start > last_idx {
            job.append(&text[last_idx..start], 0.0, base_format.clone());
        }
        job.append(&text[start..end], 0.0, highlight_format.clone());
        last_idx = end;
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
    let use_drop_cap = style.is_opening
        && !style.is_blockquote
        && !style.is_rtl
        && !style.bionic_reading
        && style.query.trim().is_empty()
        && style.tts_word_range.is_none();
    let indent_str = if style.is_opening || style.is_blockquote {
        ""
    } else {
        "    "
    };
    let mut drop_cap_done = false;
    for (idx, span) in spans.iter().enumerate() {
        let mut span_str = span.text;
        if idx == 0 && use_drop_cap && !drop_cap_done {
            let trimmed = span_str.trim_start();
            if let Some(first_ch) = trimmed.chars().next()
                && first_ch.is_alphabetic()
            {
                let ch_len = first_ch.len_utf8();
                let (cap_slice, rest_slice) = trimmed.split_at(ch_len);
                let cap_format = egui::TextFormat {
                    font_id: egui::FontId::new(
                        style.font_size * 1.48,
                        egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
                    ),
                    color: style.accent_color,
                    line_height: Some(style.line_height),
                    ..Default::default()
                };
                job.append(cap_slice, 0.0, cap_format);
                span_str = rest_slice;
                drop_cap_done = true;
            }
        }
        let text = if idx == 0 && !drop_cap_done && !indent_str.is_empty() {
            if style.is_rtl {
                format!("{span_str}{indent_str}")
            } else {
                format!("{indent_str}{span_str}")
            }
        } else {
            span_str.to_string()
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
            font_id: egui::FontId::new(
                if style.is_blockquote {
                    style.font_size * 0.96
                } else {
                    style.font_size
                },
                family,
            ),
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
            format!("{paragraph}{indent_str}")
        } else {
            format!("{indent_str}{paragraph}")
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
    if let Some((w_start, w_end)) = style.tts_word_range {
        let indent = if style.is_rtl || indent_str.is_empty() {
            0
        } else {
            4
        };
        let target_start = (w_start + indent).min(job.text.len());
        let target_end = (w_end + indent).min(job.text.len());
        let safe_start = job.text.floor_char_boundary(target_start);
        let safe_end = job.text.ceil_char_boundary(target_end);
        if safe_start < safe_end {
            highlight_range_in_job(
                &mut job,
                safe_start..safe_end,
                style.accent_color.gamma_multiply(0.28),
            );
        }
    }
    job
}

fn highlight_range_in_job(
    job: &mut egui::text::LayoutJob,
    range: std::ops::Range<usize>,
    bg: Color32,
) {
    if range.start >= range.end || range.end > job.text.len() {
        return;
    }
    let mut new_sections = Vec::with_capacity(job.sections.len() + 2);
    for sec in std::mem::take(&mut job.sections) {
        let s_start = sec.byte_range.start.0;
        let s_end = sec.byte_range.end.0;
        if s_end <= range.start || s_start >= range.end {
            new_sections.push(sec);
        } else {
            let overlap_start = s_start.max(range.start);
            let overlap_end = s_end.min(range.end);

            if s_start < overlap_start {
                new_sections.push(egui::text::LayoutSection {
                    leading_space: sec.leading_space,
                    byte_range: egui::text::ByteIndex(s_start)
                        ..egui::text::ByteIndex(overlap_start),
                    format: sec.format.clone(),
                });
            }
            let mut hl_fmt = sec.format.clone();
            hl_fmt.background = bg;
            new_sections.push(egui::text::LayoutSection {
                leading_space: if s_start < overlap_start {
                    0.0
                } else {
                    sec.leading_space
                },
                byte_range: egui::text::ByteIndex(overlap_start)
                    ..egui::text::ByteIndex(overlap_end),
                format: hl_fmt,
            });
            if overlap_end < s_end {
                new_sections.push(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: egui::text::ByteIndex(overlap_end)..egui::text::ByteIndex(s_end),
                    format: sec.format,
                });
            }
        }
    }
    job.sections = new_sections;
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
        is_opening: false,
        is_blockquote: false,
        quotes: &[],
        footnote_numbers: &footnote_numbers,
        note_available: &note_available,
        tts_word_range: None,
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

#[cfg(all(test, feature = "enable_tests"))]
pub(crate) fn split_paragraphs(paragraphs: &[String]) -> usize {
    split_paragraphs_weighted(paragraphs, 0)
}

#[cfg(all(test, feature = "enable_tests"))]
pub(crate) fn split_paragraphs_weighted(paragraphs: &[String], left_weight: usize) -> usize {
    if paragraphs.len() < 2 {
        return paragraphs.len();
    }
    let body_total: usize = paragraphs.iter().map(|text| text.chars().count()).sum();
    if body_total == 0 {
        return paragraphs.len() / 2;
    }
    let total = body_total + left_weight;
    let half = total / 2;
    let mut accumulated = left_weight;
    for (index, text) in paragraphs.iter().enumerate() {
        let prev = accumulated;
        accumulated += text.chars().count();
        if accumulated >= half {
            if left_weight > 0 && index > 0 && half.saturating_sub(prev) < accumulated - half {
                return index;
            }
            return (index + 1).min(paragraphs.len());
        }
    }
    paragraphs.len()
}
