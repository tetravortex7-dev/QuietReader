use super::reader_chrome::{ReaderSurfaceColors, is_blockquote_paragraph};
use super::theme::{ReaderTheme, bookmark_highlight_color};
use crate::core::library::UiDesignMode;
use crate::document::Document;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;

mod context_menu;
pub(crate) mod footnotes;
pub(crate) mod layout;
pub(crate) mod spread;

use context_menu::{ContextMenuParams, render_empty_reader_welcome, render_text_context_menu};
use footnotes::{FootnoteFrameState, ParagraphStyle, collect_numbers, show_hover_card};
pub use layout::create_sample_preview_job;
use layout::{
    append_with_highlight, create_paragraph_layout_job, selectable_label_preserving_on_secondary,
};

pub(crate) fn should_record_text_progress(
    settings: &AppSettings,
    path: &std::path::Path,
    chapter: usize,
    scroll_y: f32,
) -> bool {
    settings
        .get_book_progress(path)
        .is_none_or(|(saved_chapter, saved_scroll_y)| {
            saved_chapter != chapter || (saved_scroll_y - scroll_y).abs() > 20.0
        })
}

pub struct TextView;

impl TextView {
    pub fn render(
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        document: &Option<std::sync::Arc<dyn Document>>,
        current_file_path: &Option<PathBuf>,
        current_chapter: &mut usize,
        last_rendered_chapter: &mut usize,
        pending_scroll_y: &mut Option<f32>,
        current_scroll_y: &mut f32,
        viewport_height: &mut f32,
        max_scroll_y: &mut f32,
        font_size: f32,
        theme: &ReaderTheme,
        design: UiDesignMode,
        spread_content_rect: Option<egui::Rect>,
        effective_two_columns: bool,
        settings: &mut AppSettings,
        text_images: &mut super::text_images::TextImageState,
        on_open_file: &mut bool,
        on_open_folder: &mut bool,
        on_load_recent: &mut Option<PathBuf>,
        search_highlight: Option<(&str, Option<usize>)>,
        tts_highlight: Option<usize>,
        tts_word_range: Option<(usize, usize)>,
        on_start_tts: &mut Option<usize>,
        on_open_opds: &mut bool,
        on_translate: &mut Option<String>,
        on_dictionary: &mut Option<String>,
        on_wikipedia: &mut Option<String>,
        on_save_quote: &mut Option<String>,
    ) {
        let t = settings.ui_language.t();
        let is_rtl = settings.ui_language.is_rtl();
        let wheel_delta = ui.input(|input| {
            input
                .raw
                .events
                .iter()
                .fold(0.0f32, |total, event| match event {
                    egui::Event::MouseWheel {
                        delta, modifiers, ..
                    } if !modifiers.ctrl && !modifiers.command => total + delta.y,
                    _ => total,
                })
        });
        let pinned_id = egui::Id::unique((
            "quietreader_footnote_pinned",
            current_file_path,
            *current_chapter,
        ));
        let mut pinned_note = ui.ctx().data(|data| data.get_temp::<String>(pinned_id));
        let note_return_id = egui::Id::unique(("quietreader_note_return", current_file_path));
        let note_jump_id = egui::Id::unique(("quietreader_note_jump", current_file_path));
        let mut note_return: Option<(usize, f32)> =
            ui.ctx().data(|data| data.get_temp(note_return_id));
        let mut note_jump: Option<(usize, usize)> =
            ui.ctx().data(|data| data.get_temp(note_jump_id));
        if let Some(doc) = document {
            let colors = ReaderSurfaceColors::for_theme(theme, design);
            let ink_color = if design == UiDesignMode::Cozy {
                colors.page_ink
            } else {
                theme.text_color()
            };
            let accent_color = if design == UiDesignMode::Cozy {
                colors.accent
            } else {
                theme.accent_color()
            };
            let can_copy = doc.can_copy();
            crate::app::selection::set_copy_restricted(ui.ctx(), !can_copy);
            text_images.ensure_document(doc);
            text_images.poll(ui.ctx());
            let total_chapters = doc.chapters().len();
            let chapter = doc.chapters().get(*current_chapter);
            let line_height_scale = settings.effective_line_height_scale();
            let column_width = settings.effective_max_column_width();

            let chapter_quotes: Vec<(&str, Color32)> = settings
                .bookmarks
                .iter()
                .filter(|b| {
                    b.kind == crate::core::BookmarkKind::Quote
                        && current_file_path
                            .as_ref()
                            .map_or(false, |p| b.book_path == *p)
                        && b.chapter == *current_chapter
                        && !b.text.trim().is_empty()
                })
                .map(|b| {
                    (
                        b.text.as_str(),
                        bookmark_highlight_color(b.color, theme.is_dark()),
                    )
                })
                .collect();

            let allow_select = !ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
            let first_text_p_idx = chapter.and_then(|ch| {
                if design == UiDesignMode::Cozy && !is_rtl {
                    ch.paragraphs.iter().position(|p| {
                        let stripped = crate::document::strip_formatting_tags(p);
                        let trimmed = stripped.trim();
                        trimmed.chars().count() >= 24
                            && trimmed.chars().next().is_some_and(char::is_alphabetic)
                            && crate::document::image_token_id(p).is_none()
                            && !is_blockquote_paragraph(p)
                    })
                } else {
                    None
                }
            });
            let footnote_numbers = chapter.map(collect_numbers).unwrap_or_default();
            let mut available_notes: HashSet<String> = HashSet::new();
            if let Some(ch) = chapter {
                for (id, _, _) in std::iter::once(ch.title.as_str())
                    .chain(ch.paragraphs.iter().map(String::as_str))
                    .flat_map(crate::document::footnote_tokens)
                {
                    if doc.footnote(id).is_some() || doc.note_location(id).is_some() {
                        available_notes.insert(id.to_string());
                    }
                }
            }

            let mode_state_id = egui::Id::unique((
                "quietreader_last_text_mode",
                current_file_path,
                *current_chapter,
            ));
            let prev_mode: Option<(bool, f32, f32)> = ui.ctx().data(|d| d.get_temp(mode_state_id));
            let mode_switch_ratio = prev_mode.and_then(|(was_two, prev_y, prev_max)| {
                if was_two != settings.text_two_columns {
                    Some(if prev_max > 1.0 {
                        (prev_y / prev_max).clamp(0.0, 1.0)
                    } else {
                        0.0
                    })
                } else {
                    None
                }
            });

            if settings.text_two_columns
                && let Some(ch) = chapter
            {
                let spread_rect_drawn = spread::render_spread_view(
                    ui,
                    spread::SpreadRenderParams {
                        doc,
                        chapter: ch,
                        total_chapters,
                        current_file_path,
                        current_chapter,
                        last_rendered_chapter,
                        pending_scroll_y,
                        current_scroll_y,
                        viewport_height,
                        max_scroll_y,
                        font_size,
                        column_width,
                        theme,
                        design,
                        colors: &colors,
                        ink_color,
                        accent_color,
                        spread_content_rect,
                        effective_two_columns,
                        settings,
                        text_images,
                        search_highlight,
                        tts_highlight,
                        tts_word_range,
                        on_start_tts,
                        on_translate,
                        on_dictionary,
                        on_wikipedia,
                        on_save_quote,
                        pinned_note: &mut pinned_note,
                        note_return: &mut note_return,
                        note_jump: &mut note_jump,
                        mode_switch_ratio,
                        wheel_delta,
                        allow_select,
                        first_text_p_idx,
                        footnote_numbers: &footnote_numbers,
                        available_notes: &available_notes,
                        chapter_quotes: &chapter_quotes,
                        t,
                        is_rtl,
                    },
                );
                if settings.effective_reading_ruler() {
                    crate::app::bionic::render_reading_ruler(
                        ui,
                        spread_rect_drawn,
                        font_size,
                        theme,
                        settings,
                    );
                }
            } else {
                crate::app::reader_chrome::set_spread_right_visible(ui.ctx(), false);
                let mut render_scroll = |ui: &mut egui::Ui| {
                    if note_return.is_some() {
                        ui.horizontal(|ui| {
                            if ui.button(t.back_to_text).clicked() {
                                if let Some((return_chapter, return_scroll)) = note_return.take() {
                                    *current_chapter = return_chapter;
                                    *last_rendered_chapter = return_chapter;
                                    *pending_scroll_y = Some(return_scroll);
                                    *current_scroll_y = return_scroll;
                                }
                                note_jump = None;
                                pinned_note = None;
                            }
                        });
                    }

                    let mut scroll_area = ScrollArea::vertical()
                        .id_salt(("text_scroll", current_file_path, *current_chapter))
                        .auto_shrink([false, false]);
                    if let Some(offset) = pending_scroll_y.take() {
                        scroll_area = scroll_area.vertical_scroll_offset(offset);
                    }

                    let inner_max_w = if spread_content_rect.is_some() {
                        (ui.available_width() - 40.0)
                            .min(column_width)
                            .max(180.0)
                            .min(ui.available_width())
                    } else {
                        column_width.min(ui.available_width().max(180.0))
                    };

                    scroll_area.show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.set_max_width(inner_max_w);

                            if let Some(ch) = chapter {
                                ui.add_space(if spread_content_rect.is_some() {
                                    12.0
                                } else {
                                    20.0
                                });
                                let clean_title = crate::document::strip_formatting_tags(&ch.title);
                                let (query_str, active_para) = match search_highlight {
                                    Some((q, p)) => (q, p),
                                    None => ("", Some(usize::MAX)),
                                };

                                let line_height = font_size * line_height_scale;
                                let para_spacing = font_size * 0.4;
                                let chars_per_line = (column_width / (font_size * 0.52)).max(10.0);
                                let paragraph_count = ch.paragraphs.len();

                                let clip_rect = ui.clip_rect();
                                let margin = 400.0;
                                let view_top = clip_rect.min.y - margin;
                                let view_bot = clip_rect.max.y + margin;

                                let mut footnote_state = FootnoteFrameState::begin(ui.ctx());
                                let mut render_paragraphs =
                                    |ui: &mut egui::Ui,
                                     range: std::ops::Range<usize>,
                                     chars_per_line: f32,
                                     with_header: bool| {
                                    if with_header {
                                        if !clean_title.is_empty() {
                                            if design == UiDesignMode::Cozy && total_chapters > 1 {
                                                let overline = t
                                                    .extra
                                                    .chapter_short
                                                    .replace(
                                                        "{n}",
                                                        &(*current_chapter + 1).to_string(),
                                                    )
                                                    .to_uppercase();
                                                ui.label(
                                                    RichText::new(overline)
                                                        .size(10.5)
                                                        .strong()
                                                        .color(accent_color),
                                                );
                                                ui.add_space(4.0);
                                            }
                                            let is_active_title =
                                                active_para.is_none() && !query_str.is_empty();
                                            let mut title_job = egui::text::LayoutJob::default();
                                            if is_rtl {
                                                title_job.halign = egui::Align::RIGHT;
                                            }
                                            let base_title_fmt = egui::TextFormat {
                                                font_id: egui::FontId::new(
                                                    font_size
                                                        * if design == UiDesignMode::Cozy {
                                                            1.45
                                                        } else {
                                                            1.35
                                                        },
                                                    egui::FontFamily::Name(
                                                        crate::app::fonts::FONT_FAMILY_BOLD.into(),
                                                    ),
                                                ),
                                                color: ink_color,
                                                ..Default::default()
                                            };
                                            append_with_highlight(
                                                &mut title_job,
                                                &clean_title,
                                                &base_title_fmt,
                                                query_str,
                                                is_active_title,
                                                &chapter_quotes,
                                            );
                                            let resp = selectable_label_preserving_on_secondary(
                                                ui,
                                                title_job,
                                                allow_select,
                                            );
                                            resp.context_menu(|ui| {
                                                render_text_context_menu(
                                                    ui,
                                                    ContextMenuParams {
                                                        doc: doc.as_ref(),
                                                        current_chapter: *current_chapter,
                                                        can_copy,
                                                        single_text: &clean_title,
                                                        tts_paragraph_idx: 0,
                                                        translation_provider: settings
                                                            .translation
                                                            .provider,
                                                        t,
                                                        on_start_tts,
                                                        on_translate,
                                                        on_dictionary: None,
                                                        on_wikipedia: None,
                                                        on_save_quote,
                                                    },
                                                );
                                            });
                                            ui.add_space(if design == UiDesignMode::Cozy {
                                                10.0
                                            } else {
                                                24.0
                                            });
                                            if design == UiDesignMode::Cozy {
                                                let (rule_rect, _) = ui.allocate_exact_size(
                                                    Vec2::new(42.0, 2.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().rect_filled(
                                                    rule_rect,
                                                    1.0,
                                                    accent_color.gamma_multiply(0.45),
                                                );
                                                ui.add_space(14.0);
                                            }
                                        }

                                        if *current_chapter == 0
                                            && let Some(header) = doc.header_text()
                                        {
                                            ui.label(
                                                RichText::new(header)
                                                    .size(font_size * 0.78)
                                                    .italics()
                                                    .color(ink_color.gamma_multiply(0.6)),
                                            );
                                            ui.add_space(10.0);
                                        }
                                    }

                                    for p_idx in range {
                                        let paragraph = &ch.paragraphs[p_idx];
                                        if paragraph.is_empty() {
                                            ui.add_space(font_size * 0.7);
                                            continue;
                                        }

                                        if let Some(image_id) =
                                            crate::document::image_token_id(paragraph)
                                        {
                                            let max_width =
                                                ui.available_width().min(column_width).max(1.0);
                                            let cached_size = text_images.image_size(image_id);
                                            let est_height = cached_size
                                                .map(|size| {
                                                    if size.x > 0.0 {
                                                        let scale = (max_width / size.x)
                                                            .min(560.0 / size.y.max(1.0))
                                                            .min(1.0);
                                                        size.y * scale + para_spacing
                                                    } else {
                                                        font_size * 18.0
                                                    }
                                                })
                                                .unwrap_or(font_size * 18.0);
                                            let top_y = ui.cursor().min.y;
                                            let bot_y = top_y + est_height;
                                            if bot_y < view_top || top_y > view_bot {
                                                ui.add_space(est_height);
                                            } else if let Some(texture) =
                                                text_images.texture(ui.ctx(), doc, image_id)
                                            {
                                                let size = texture.size_vec2();
                                                let scale = (max_width / size.x.max(1.0))
                                                    .min(560.0 / size.y.max(1.0))
                                                    .min(1.0);
                                                ui.add(
                                                    egui::Image::from_texture(&texture)
                                                        .fit_to_exact_size(size * scale),
                                                );
                                                ui.add_space(para_spacing);
                                            } else {
                                                let (rect, _) = ui.allocate_exact_size(
                                                    Vec2::new(max_width, font_size * 18.0),
                                                    egui::Sense::hover(),
                                                );
                                                ui.painter().rect_stroke(
                                                    rect,
                                                    6.0,
                                                    Stroke::new(
                                                        1.0,
                                                        theme.text_color().gamma_multiply(0.25),
                                                    ),
                                                    StrokeKind::Inside,
                                                );
                                                ui.add_space(para_spacing);
                                            }
                                            continue;
                                        }

                                        let clean_paragraph =
                                            crate::document::strip_formatting_tags(paragraph);
                                        let char_count = clean_paragraph.chars().count();
                                        let num_lines =
                                            (char_count as f32 / chars_per_line).ceil().max(1.0);
                                        let est_height = num_lines * line_height + para_spacing;

                                        let top_y = ui.cursor().min.y;
                                        let bot_y = top_y + est_height;
                                        let is_note_target = note_jump.is_some_and(
                                            |(chapter_index, paragraph_index)| {
                                                chapter_index == *current_chapter
                                                    && paragraph_index == p_idx
                                            },
                                        );

                                        if (bot_y < view_top || top_y > view_bot) && !is_note_target
                                        {
                                            ui.add_space(est_height);
                                        } else {
                                            let is_active_para = active_para == Some(p_idx);
                                            let is_tts_speaking = tts_highlight == Some(p_idx);
                                            let current_word_range = if is_tts_speaking {
                                                tts_word_range
                                            } else {
                                                None
                                            };
                                            let is_opening = first_text_p_idx == Some(p_idx);
                                            let is_blockquote = design == UiDesignMode::Cozy
                                                && is_blockquote_paragraph(paragraph);
                                            let mut markers = Vec::new();
                                            let mut layout_job = create_paragraph_layout_job(
                                                paragraph,
                                                &ParagraphStyle {
                                                    font_size,
                                                    text_color: ink_color,
                                                    accent_color,
                                                    line_height,
                                                    query: query_str,
                                                    is_active_hit: is_active_para,
                                                    is_rtl,
                                                    bionic_reading: settings
                                                        .effective_bionic_reading(),
                                                    quotes: &chapter_quotes,
                                                    footnote_numbers: &footnote_numbers,
                                                    note_available: &available_notes,
                                                    tts_word_range: current_word_range,
                                                    is_opening,
                                                    is_blockquote,
                                                },
                                                &mut markers,
                                            );
                                            let (resp, footnote_galley) = if is_blockquote {
                                                let frame_resp = egui::Frame::new()
                                                    .fill(colors.quote_fill)
                                                    .corner_radius(egui::CornerRadius {
                                                        nw: if is_rtl { 6 } else { 0 },
                                                        sw: if is_rtl { 6 } else { 0 },
                                                        ne: if is_rtl { 0 } else { 6 },
                                                        se: if is_rtl { 0 } else { 6 },
                                                    })
                                                    .inner_margin(egui::Margin {
                                                        left: if is_rtl { 12 } else { 14 },
                                                        right: if is_rtl { 14 } else { 12 },
                                                        top: 8,
                                                        bottom: 8,
                                                    })
                                                    .show(ui, |ui| {
                                                        let fg = if markers.is_empty() {
                                                            None
                                                        } else {
                                                            layout_job.wrap.max_width =
                                                                ui.available_width();
                                                            Some(
                                                                ui.painter()
                                                                    .layout_job(layout_job.clone()),
                                                            )
                                                        };
                                                        let r =
                                                            selectable_label_preserving_on_secondary(
                                                                ui,
                                                                layout_job,
                                                                allow_select,
                                                            );
                                                        (r, fg)
                                                    });
                                                let fr = frame_resp.response.rect;
                                                let bar_rect = if is_rtl {
                                                    egui::Rect::from_min_max(
                                                        egui::Pos2::new(fr.max.x - 3.0, fr.min.y),
                                                        fr.max,
                                                    )
                                                } else {
                                                    egui::Rect::from_min_max(
                                                        fr.min,
                                                        egui::Pos2::new(fr.min.x + 3.0, fr.max.y),
                                                    )
                                                };
                                                ui.painter().rect_filled(
                                                    bar_rect,
                                                    1.5,
                                                    colors.quote_border,
                                                );
                                                frame_resp.inner
                                            } else {
                                                let fg = if markers.is_empty() {
                                                    None
                                                } else {
                                                    layout_job.wrap.max_width =
                                                        ui.available_width();
                                                    Some(ui.painter().layout_job(layout_job.clone()))
                                                };
                                                let r = selectable_label_preserving_on_secondary(
                                                    ui,
                                                    layout_job,
                                                    allow_select,
                                                );
                                                (r, fg)
                                            };
                                            if is_note_target {
                                                ui.scroll_to_rect(
                                                    resp.rect,
                                                    Some(egui::Align::Center),
                                                );
                                                if resp.rect.intersects(ui.clip_rect()) {
                                                    note_jump = None;
                                                }
                                            }
                                            if let Some(galley) = footnote_galley
                                                && let Some(note_id) = show_hover_card(
                                                    ui,
                                                    doc.as_ref(),
                                                    &resp,
                                                    &galley,
                                                    &markers,
                                                    &colors,
                                                    design,
                                                    t,
                                                    font_size,
                                                    &mut pinned_note,
                                                    &mut footnote_state,
                                                )
                                                && let Some(location) = doc.note_location(&note_id)
                                            {
                                                note_return =
                                                    Some((*current_chapter, *current_scroll_y));
                                                note_jump = Some((
                                                    location.chapter_index,
                                                    location.paragraph_index,
                                                ));
                                                *current_chapter = location.chapter_index;
                                                *last_rendered_chapter = location.chapter_index;
                                                *current_scroll_y = 0.0;
                                                pinned_note = None;
                                            }
                                            if is_tts_speaking {
                                                ui.painter().rect(
                                                    resp.rect.expand2(Vec2::new(8.0, 3.0)),
                                                    4.0,
                                                    accent_color.gamma_multiply(0.12),
                                                    Stroke::new(
                                                        1.0,
                                                        accent_color.gamma_multiply(0.45),
                                                    ),
                                                    StrokeKind::Inside,
                                                );
                                            }
                                            resp.context_menu(|ui| {
                                                render_text_context_menu(
                                                    ui,
                                                    ContextMenuParams {
                                                        doc: doc.as_ref(),
                                                        current_chapter: *current_chapter,
                                                        can_copy,
                                                        single_text: &clean_paragraph,
                                                        tts_paragraph_idx: p_idx,
                                                        translation_provider: settings
                                                            .translation
                                                            .provider,
                                                        t,
                                                        on_start_tts,
                                                        on_translate,
                                                        on_dictionary: Some(on_dictionary),
                                                        on_wikipedia: Some(on_wikipedia),
                                                        on_save_quote,
                                                    },
                                                );
                                            });
                                            ui.add_space(para_spacing);
                                        }
                                    }
                                };

                            let single_col_w = inner_max_w.min(ui.available_width());
                            let col_align = if is_rtl {
                                egui::Align::Max
                            } else {
                                egui::Align::Min
                            };
                            ui.allocate_ui_with_layout(
                                Vec2::new(single_col_w, 0.0),
                                egui::Layout::top_down(col_align),
                                |ui| {
                                    ui.set_width(single_col_w);
                                    render_paragraphs(
                                        ui,
                                        0..paragraph_count,
                                        chars_per_line,
                                        true,
                                    );
                                },
                            );
                            footnote_state.finish(
                                ui.ctx(),
                                ui.input(|i| i.pointer.primary_clicked()),
                                &mut pinned_note,
                            );

                            if *current_chapter + 1 >= total_chapters
                                && let Some(footer) = doc.footer_text()
                            {
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(footer)
                                        .size(font_size * 0.78)
                                        .italics()
                                        .color(ink_color.gamma_multiply(0.6)),
                                );
                            }

                            ui.add_space(40.0);

                            let (first_layout, second_layout) = if is_rtl {
                                (
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                )
                            } else {
                                (
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    egui::Layout::right_to_left(egui::Align::Center),
                                )
                            };

                            ui.with_layout(first_layout, |ui| {
                                let prev_chapter_idx = (0..*current_chapter).rev().find(|&idx| {
                                    doc.chapters()
                                        .get(idx)
                                        .map(|c| !c.is_empty())
                                        .unwrap_or(false)
                                });

                                let next_chapter_idx = ((*current_chapter + 1)..total_chapters)
                                    .find(|&idx| {
                                        doc.chapters()
                                            .get(idx)
                                            .map(|c| !c.is_empty())
                                            .unwrap_or(false)
                                    });

                                if let Some(prev_idx) = prev_chapter_idx
                                    && ui.button(t.prev_section).clicked()
                                {
                                    *current_chapter = prev_idx;
                                    *pending_scroll_y = Some(0.0);
                                    *current_scroll_y = 0.0;
                                    *last_rendered_chapter = *current_chapter;
                                }

                                ui.with_layout(second_layout, |ui| {
                                    if let Some(next_idx) = next_chapter_idx
                                        && ui.button(t.next_section).clicked()
                                    {
                                        *current_chapter = next_idx;
                                        *pending_scroll_y = Some(0.0);
                                        *current_scroll_y = 0.0;
                                        *last_rendered_chapter = *current_chapter;
                                    }
                                });
                            });

                            ui.add_space(30.0);
                        }
                    })
                })
                };

                let scroll_output = if let Some(rect) = spread_content_rect {
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), render_scroll)
                        .inner
                } else {
                    render_scroll(ui)
                };

                *current_scroll_y = scroll_output.state.clamped_offset().y;
                *viewport_height = scroll_output.inner_rect.height();
                *max_scroll_y =
                    (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);

                if let Some(ratio) = mode_switch_ratio {
                    let target = (ratio * *max_scroll_y).clamp(0.0, *max_scroll_y);
                    *pending_scroll_y = Some(target);
                    *current_scroll_y = target;
                    ui.ctx().request_repaint();
                } else if settings.text_paged_scroll
                    && *viewport_height > 1.0
                    && *max_scroll_y > 0.0
                {
                    let index_key = egui::Id::unique(("quietreader_page_index", *current_chapter));
                    let mut page_index: usize =
                        ui.data(|data| data.get_temp(index_key).unwrap_or(0));
                    let max_index = ((*max_scroll_y / *viewport_height).ceil() as usize).max(1);
                    if wheel_delta.abs() > f32::EPSILON {
                        page_index =
                            layout::page_index_after_wheel(page_index, wheel_delta, max_index);
                        let target =
                            layout::page_offset(page_index, *viewport_height, *max_scroll_y);
                        *pending_scroll_y = Some(target);
                        *current_scroll_y = target;
                        ui.data_mut(|data| data.insert_temp(index_key, page_index));
                    } else {
                        let actual = (*current_scroll_y / *viewport_height).round() as usize;
                        if actual != page_index {
                            ui.data_mut(|data| data.insert_temp(index_key, actual));
                        }
                    }
                }

                if settings.effective_reading_ruler() {
                    crate::app::bionic::render_reading_ruler(
                        ui,
                        scroll_output.inner_rect,
                        font_size,
                        theme,
                        settings,
                    );
                }
            }

            ui.ctx().data_mut(|d| {
                d.insert_temp(
                    mode_state_id,
                    (settings.text_two_columns, *current_scroll_y, *max_scroll_y),
                );
            });

            if let Some(path) = current_file_path
                && should_record_text_progress(settings, path, *current_chapter, *current_scroll_y)
            {
                let meta = doc.metadata();
                settings.record_book_progress(
                    path,
                    &meta.title,
                    &meta.author,
                    *current_chapter,
                    *current_scroll_y,
                );
            }
            ui.ctx()
                .data_mut(|data| data.insert_temp(pinned_id, pinned_note));
        } else {
            render_empty_reader_welcome(
                ui,
                theme,
                settings,
                t,
                on_open_file,
                on_open_folder,
                on_open_opds,
                on_load_recent,
            );
        }

        ui.ctx().data_mut(|data| {
            if let Some(value) = note_return {
                data.insert_temp(note_return_id, value);
            } else {
                data.remove_temp::<(usize, f32)>(note_return_id);
            }
            if let Some(value) = note_jump {
                data.insert_temp(note_jump_id, value);
            } else {
                data.remove_temp::<(usize, usize)>(note_jump_id);
            }
        });
    }
}

pub(crate) fn create_focus_layout_job(
    paragraph: &str,
    font_size: f32,
    text_color: Color32,
    accent_color: Color32,
    line_height: f32,
    is_rtl: bool,
    bionic_reading: bool,
    max_width: f32,
) -> egui::text::LayoutJob {
    let footnote_numbers = std::collections::HashMap::new();
    let available_notes = HashSet::new();
    let mut markers = Vec::new();
    let mut job = create_paragraph_layout_job(
        paragraph,
        &ParagraphStyle {
            font_size,
            text_color,
            accent_color,
            line_height,
            query: "",
            is_active_hit: false,
            is_rtl,
            bionic_reading,
            quotes: &[],
            footnote_numbers: &footnote_numbers,
            note_available: &available_notes,
            tts_word_range: None,
            is_opening: false,
            is_blockquote: false,
        },
        &mut markers,
    );
    job.wrap.max_width = max_width;
    job
}
