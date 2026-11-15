use super::context_menu::{ContextMenuParams, render_text_context_menu};
use super::footnotes::{FootnoteFrameState, ParagraphStyle, show_hover_card};
use super::layout::{create_paragraph_layout_job, selectable_label_preserving_on_secondary};
use crate::app::icons::Icon;
use crate::app::reader_chrome::ReaderSurfaceColors;
use crate::app::text_images::TextImageState;
use crate::app::theme::ReaderTheme;
use crate::core::library::UiDesignMode;
use crate::document::{BookChapter, Document};
use crate::i18n::Translations;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, Stroke, StrokeKind, Vec2};
use paginate::{PageEntry, SpreadPage, get_or_build_pages, slice_layout_job};
#[cfg(all(test, feature = "enable_tests"))]
pub(crate) use paginate::{
    rebase_markers as test_rebase_markers, slice_layout_job as test_slice_layout_job,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

mod paginate;

const SPREAD_WHEEL_STATE_KEY: &str = "quietreader_spread_wheel_ts";
const SPREAD_PAGE_STATE_KEY: &str = "quietreader_spread_page_state";
const SPREAD_SEARCH_TRACK_KEY: &str = "quietreader_spread_last_search";
const SPREAD_TTS_TRACK_KEY: &str = "quietreader_spread_last_tts";

#[derive(Clone, Copy)]
struct SpreadPageState {
    page_idx: usize,
    total_pages: usize,
    last_start: usize,
    page_h: f32,
    emitted_scroll_y: f32,
    two_columns: bool,
}

pub(in crate::app::text_view) struct SpreadRenderParams<'a> {
    pub doc: &'a Arc<dyn Document>,
    pub chapter: &'a BookChapter,
    pub total_chapters: usize,
    pub current_file_path: &'a Option<PathBuf>,
    pub current_chapter: &'a mut usize,
    pub last_rendered_chapter: &'a mut usize,
    pub pending_scroll_y: &'a mut Option<f32>,
    pub current_scroll_y: &'a mut f32,
    pub viewport_height: &'a mut f32,
    pub max_scroll_y: &'a mut f32,
    pub font_size: f32,
    pub column_width: f32,
    pub theme: &'a ReaderTheme,
    pub design: UiDesignMode,
    pub colors: &'a ReaderSurfaceColors,
    pub ink_color: Color32,
    pub accent_color: Color32,
    pub spread_content_rect: Option<egui::Rect>,
    pub effective_two_columns: bool,
    pub settings: &'a AppSettings,
    pub text_images: &'a mut TextImageState,
    pub search_highlight: Option<(&'a str, Option<usize>)>,
    pub tts_highlight: Option<usize>,
    pub tts_word_range: Option<(usize, usize)>,
    pub on_start_tts: &'a mut Option<usize>,
    pub on_translate: &'a mut Option<String>,
    pub on_dictionary: &'a mut Option<String>,
    pub on_wikipedia: &'a mut Option<String>,
    pub on_save_quote: &'a mut Option<String>,
    pub pinned_note: &'a mut Option<String>,
    pub note_return: &'a mut Option<(usize, f32)>,
    pub note_jump: &'a mut Option<(usize, usize)>,
    pub mode_switch_ratio: Option<f32>,
    pub wheel_delta: f32,
    pub allow_select: bool,
    pub first_text_p_idx: Option<usize>,
    pub footnote_numbers: &'a HashMap<String, usize>,
    pub available_notes: &'a HashSet<String>,
    pub chapter_quotes: &'a [(&'a str, Color32)],
    pub t: &'a Translations,
    pub is_rtl: bool,
}

pub(in crate::app::text_view) fn render_spread_view(
    ui: &mut egui::Ui,
    mut params: SpreadRenderParams<'_>,
) -> egui::Rect {
    let ctx = ui.ctx().clone();
    let container_rect = params
        .spread_content_rect
        .unwrap_or_else(|| ui.available_rect_before_wrap());

    let top_reserved = if params.note_return.is_some() {
        30.0
    } else {
        0.0
    };

    let studio_footer_h = if params.spread_content_rect.is_none() {
        34.0
    } else {
        0.0
    };

    let work_rect = egui::Rect::from_min_max(
        egui::Pos2::new(container_rect.min.x, container_rect.min.y + top_reserved),
        egui::Pos2::new(
            container_rect.max.x,
            (container_rect.max.y - studio_footer_h)
                .max(container_rect.min.y + top_reserved + 120.0),
        ),
    );

    let is_cozy_frame = params.spread_content_rect.is_some();
    let pad_v = if is_cozy_frame { 8.0 } else { 14.0 };
    let gutter = if params.effective_two_columns {
        if is_cozy_frame { 52.0 } else { 44.0 }
    } else {
        0.0
    };

    let side_pad = if is_cozy_frame { 28.0 } else { 48.0 };
    let avail_w = (work_rect.width() - side_pad).max(240.0);
    let (page_w, total_w) = if params.effective_two_columns {
        let max_total = if is_cozy_frame {
            avail_w
        } else {
            avail_w.min((params.column_width * 1.48).max(760.0))
        };
        let col_w = ((max_total - gutter) * 0.5).max(110.0);
        (col_w, col_w * 2.0 + gutter)
    } else {
        let single_w = (work_rect.width() - 36.0)
            .min(params.column_width)
            .max(200.0)
            .min((work_rect.width() - 16.0).max(160.0));
        (single_w, single_w)
    };

    let page_h = (work_rect.height() - pad_v * 2.0).max(120.0);
    let pages = get_or_build_pages(ui, &params, page_w, page_h);
    let total_ch_pages = pages.len().max(1);
    let visible_slots = if params.effective_two_columns { 2 } else { 1 };
    let last_start = compute_last_spread_start(total_ch_pages, params.effective_two_columns);

    let mut cur_ch_page = resolve_current_page_index(
        &ctx,
        &mut params,
        total_ch_pages,
        last_start,
        page_h,
        &pages,
    );

    if !egui::Popup::is_any_open(&ctx)
        && container_rect.height() > 10.0
        && params.wheel_delta.abs() > 0.5
        && ui.rect_contains_pointer(container_rect)
    {
        let now = ui.input(|i| i.time);
        let wheel_id = egui::Id::unique(SPREAD_WHEEL_STATE_KEY);
        let last_wheel: f64 = ctx.data(|d| d.get_temp(wheel_id).unwrap_or(0.0));
        if now - last_wheel >= 0.16 {
            let step = visible_slots;
            if params.wheel_delta < 0.0 {
                if cur_ch_page + step < total_ch_pages {
                    cur_ch_page =
                        align_page(cur_ch_page + step, params.effective_two_columns, last_start);
                    ctx.data_mut(|d| d.insert_temp(wheel_id, now));
                } else if let Some(next_idx) = crate::core::ReaderCore::find_next_non_empty_chapter(
                    params.doc.as_ref(),
                    *params.current_chapter,
                ) {
                    *params.current_chapter = next_idx;
                    *params.last_rendered_chapter = next_idx;
                    *params.pending_scroll_y = Some(0.0);
                    *params.current_scroll_y = 0.0;
                    cur_ch_page = 0;
                    ctx.data_mut(|d| d.insert_temp(wheel_id, now));
                }
            } else if cur_ch_page >= step {
                cur_ch_page = align_page(
                    cur_ch_page.saturating_sub(step),
                    params.effective_two_columns,
                    last_start,
                );
                ctx.data_mut(|d| d.insert_temp(wheel_id, now));
            } else if cur_ch_page > 0 {
                cur_ch_page = 0;
                ctx.data_mut(|d| d.insert_temp(wheel_id, now));
            } else if let Some(prev_idx) = crate::core::ReaderCore::find_prev_non_empty_chapter(
                params.doc.as_ref(),
                *params.current_chapter,
            ) {
                *params.current_chapter = prev_idx;
                *params.last_rendered_chapter = prev_idx;
                *params.pending_scroll_y = Some(999_999.0);
                ctx.data_mut(|d| d.insert_temp(wheel_id, now));
            }
        }
    }

    let left_idx = cur_ch_page.min(total_ch_pages - 1);
    let right_idx = if params.effective_two_columns && left_idx + 1 < total_ch_pages {
        Some(left_idx + 1)
    } else {
        None
    };

    crate::app::reader_chrome::set_spread_right_visible(&ctx, right_idx.is_some());

    let emitted_scroll = (left_idx as f32) * page_h;
    *params.viewport_height = page_h;
    *params.max_scroll_y = (last_start as f32) * page_h;
    *params.current_scroll_y = emitted_scroll;
    *params.pending_scroll_y = None;

    let state_id = egui::Id::unique((
        SPREAD_PAGE_STATE_KEY,
        params.current_file_path,
        *params.current_chapter,
    ));
    ctx.data_mut(|d| {
        d.insert_temp(
            state_id,
            SpreadPageState {
                page_idx: left_idx,
                total_pages: total_ch_pages,
                last_start,
                page_h,
                emitted_scroll_y: emitted_scroll,
                two_columns: params.effective_two_columns,
            },
        );
    });

    if top_reserved > 0.0 {
        let back_rect = egui::Rect::from_min_max(
            container_rect.min,
            egui::Pos2::new(container_rect.max.x, container_rect.min.y + top_reserved),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(back_rect), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                if ui.button(params.t.back_to_text).clicked()
                    && let Some((return_chapter, return_scroll)) = params.note_return.take()
                {
                    *params.current_chapter = return_chapter;
                    *params.last_rendered_chapter = return_chapter;
                    *params.pending_scroll_y = Some(return_scroll);
                    *params.current_scroll_y = return_scroll;
                    *params.note_jump = None;
                    *params.pinned_note = None;
                }
            });
        });
    }

    let x_start = work_rect.center().x - total_w * 0.5;
    let y_start = work_rect.min.y + pad_v;

    let left_rect =
        egui::Rect::from_min_size(egui::Pos2::new(x_start, y_start), Vec2::new(page_w, page_h));
    let right_rect = egui::Rect::from_min_size(
        egui::Pos2::new(x_start + page_w + gutter, y_start),
        Vec2::new(page_w, page_h),
    );

    if params.effective_two_columns && !is_cozy_frame {
        let div_x = x_start + page_w + gutter * 0.5;
        ui.painter().line_segment(
            [
                egui::Pos2::new(div_x, y_start + 4.0),
                egui::Pos2::new(div_x, y_start + page_h - 4.0),
            ],
            Stroke::new(1.0, params.theme.text_color().gamma_multiply(0.14)),
        );
    }

    let mut footnote_state = FootnoteFrameState::begin(&ctx);

    let (first_rect, second_rect) = if params.is_rtl && params.effective_two_columns {
        (right_rect, left_rect)
    } else {
        (left_rect, right_rect)
    };

    render_page_in_rect(
        ui,
        first_rect,
        &pages[left_idx],
        &mut params,
        &mut footnote_state,
    );
    if let Some(r_idx) = right_idx {
        render_page_in_rect(
            ui,
            second_rect,
            &pages[r_idx],
            &mut params,
            &mut footnote_state,
        );
    }

    footnote_state.finish(
        &ctx,
        ui.input(|i| i.pointer.primary_clicked()),
        params.pinned_note,
    );

    if !is_cozy_frame {
        let nav_rect = egui::Rect::from_min_max(
            egui::Pos2::new(x_start, work_rect.max.y),
            egui::Pos2::new(x_start + total_w, container_rect.max.y),
        );
        let next_step = render_studio_spread_nav(
            ui,
            nav_rect,
            cur_ch_page,
            total_ch_pages,
            visible_slots,
            params.is_rtl,
            params.theme,
            params.t,
        );
        if next_step != 0 {
            let mut switched_chapter = false;
            if next_step > 0 {
                if cur_ch_page + visible_slots < total_ch_pages {
                    let next_p = align_page(
                        cur_ch_page + visible_slots,
                        params.effective_two_columns,
                        last_start,
                    );
                    *params.current_scroll_y = (next_p as f32) * page_h;
                } else if let Some(next_idx) = crate::core::ReaderCore::find_next_non_empty_chapter(
                    params.doc.as_ref(),
                    *params.current_chapter,
                ) {
                    *params.current_chapter = next_idx;
                    *params.last_rendered_chapter = next_idx;
                    *params.pending_scroll_y = Some(0.0);
                    *params.current_scroll_y = 0.0;
                    switched_chapter = true;
                }
            } else if cur_ch_page > 0 {
                let prev_p = align_page(
                    cur_ch_page.saturating_sub(visible_slots),
                    params.effective_two_columns,
                    last_start,
                );
                *params.current_scroll_y = (prev_p as f32) * page_h;
            } else if let Some(prev_idx) = crate::core::ReaderCore::find_prev_non_empty_chapter(
                params.doc.as_ref(),
                *params.current_chapter,
            ) {
                *params.current_chapter = prev_idx;
                *params.last_rendered_chapter = prev_idx;
                *params.pending_scroll_y = Some(999_999.0);
                switched_chapter = true;
            }
            if !switched_chapter {
                let updated_idx =
                    ((*params.current_scroll_y / page_h).round() as usize).min(last_start);
                ctx.data_mut(|d| {
                    d.insert_temp(
                        state_id,
                        SpreadPageState {
                            page_idx: updated_idx,
                            total_pages: total_ch_pages,
                            last_start,
                            page_h,
                            emitted_scroll_y: *params.current_scroll_y,
                            two_columns: params.effective_two_columns,
                        },
                    );
                });
            }
        }
    }

    work_rect
}

pub(crate) fn align_page(page: usize, two_columns: bool, last_start: usize) -> usize {
    let clamped = page.min(last_start);
    if two_columns {
        (clamped / 2) * 2
    } else {
        clamped
    }
}

pub(crate) fn compute_last_spread_start(total_pages: usize, two_columns: bool) -> usize {
    if total_pages <= 1 {
        return 0;
    }
    if two_columns {
        if total_pages <= 2 {
            0
        } else {
            ((total_pages - 1) / 2) * 2
        }
    } else {
        total_pages - 1
    }
}

fn resolve_current_page_index(
    ctx: &egui::Context,
    params: &mut SpreadRenderParams<'_>,
    total_ch_pages: usize,
    last_start: usize,
    page_h: f32,
    pages: &[SpreadPage],
) -> usize {
    let state_id = egui::Id::unique((
        SPREAD_PAGE_STATE_KEY,
        params.current_file_path,
        *params.current_chapter,
    ));
    let prev_state: Option<SpreadPageState> = ctx.data(|d| d.get_temp(state_id));

    let search_id = egui::Id::unique((
        SPREAD_SEARCH_TRACK_KEY,
        params.current_file_path,
        *params.current_chapter,
    ));
    let cur_search = params.search_highlight.map(|(q, p)| (q.to_string(), p));
    let prev_search: Option<(String, Option<usize>)> = ctx.data(|d| d.get_temp(search_id));
    let search_changed = cur_search != prev_search
        && cur_search
            .as_ref()
            .is_some_and(|(q, _)| !q.trim().is_empty());
    if cur_search != prev_search {
        ctx.data_mut(|d| {
            if let Some(v) = &cur_search {
                d.insert_temp(search_id, v.clone());
            } else {
                d.remove_temp::<(String, Option<usize>)>(search_id);
            }
        });
    }

    let tts_id = egui::Id::unique((
        SPREAD_TTS_TRACK_KEY,
        params.current_file_path,
        *params.current_chapter,
    ));
    let prev_tts: Option<usize> = ctx.data(|d| d.get_temp(tts_id));
    let tts_changed = params.tts_highlight != prev_tts && params.tts_highlight.is_some();
    ctx.data_mut(|d| {
        if let Some(p) = params.tts_highlight {
            d.insert_temp(tts_id, p);
        } else {
            d.remove_temp::<usize>(tts_id);
        }
    });

    if let Some((jump_ch, jump_p)) = *params.note_jump
        && jump_ch == *params.current_chapter
    {
        if let Some(found) = find_page_for_paragraph(pages, jump_p) {
            *params.note_jump = None;
            return align_page(found, params.effective_two_columns, last_start);
        }
        *params.note_jump = None;
    }

    if search_changed && let Some((_, active_para)) = cur_search {
        if let Some(p_idx) = active_para {
            if let Some(found) = find_page_for_paragraph(pages, p_idx) {
                return align_page(found, params.effective_two_columns, last_start);
            }
        } else if let Some(found) = pages.iter().position(|p| p.includes_header) {
            return align_page(found, params.effective_two_columns, last_start);
        }
    }

    if tts_changed && let Some(p_idx) = params.tts_highlight {
        if let Some(found) = find_page_for_paragraph(pages, p_idx) {
            return align_page(found, params.effective_two_columns, last_start);
        }
    }

    if let Some(ratio) = params.mode_switch_ratio {
        let target = ((ratio.clamp(0.0, 1.0)) * (last_start as f32)).round() as usize;
        return align_page(target, params.effective_two_columns, last_start);
    }

    if let Some(pending) = *params.pending_scroll_y {
        if pending >= 900_000.0 {
            return last_start;
        }
        let target = (pending / page_h.max(1.0)).round().max(0.0) as usize;
        return align_page(target, params.effective_two_columns, last_start);
    }

    if let Some(prev) = prev_state {
        if (*params.current_scroll_y - prev.emitted_scroll_y).abs() > 1.5 {
            let target = (*params.current_scroll_y / page_h.max(1.0))
                .round()
                .max(0.0) as usize;
            return align_page(target, params.effective_two_columns, last_start);
        }
        if prev.total_pages != total_ch_pages
            || (prev.page_h - page_h).abs() > 1.0
            || prev.two_columns != params.effective_two_columns
        {
            let ratio = if prev.last_start > 0 {
                (prev.page_idx as f32 / prev.last_start as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let target = (ratio * (last_start as f32)).round() as usize;
            return align_page(target, params.effective_two_columns, last_start);
        }
        return align_page(prev.page_idx, params.effective_two_columns, last_start);
    }

    let fallback_idx = (*params.current_scroll_y / page_h.max(1.0))
        .round()
        .max(0.0) as usize;
    align_page(fallback_idx, params.effective_two_columns, last_start)
}

fn find_page_for_paragraph(pages: &[SpreadPage], p_idx: usize) -> Option<usize> {
    pages
        .iter()
        .position(|page| page.para_indices.contains(&p_idx))
}

fn render_page_in_rect(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    page: &SpreadPage,
    params: &mut SpreadRenderParams<'_>,
    footnote_state: &mut FootnoteFrameState,
) {
    let line_height = params.font_size * params.settings.effective_line_height_scale();
    let is_cozy = params.design == UiDesignMode::Cozy;
    let align = if params.is_rtl {
        egui::Align::Max
    } else {
        egui::Align::Min
    };

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        ui.allocate_ui_with_layout(rect.size(), egui::Layout::top_down(align), |ui| {
            ui.set_width(rect.width());
            for entry in &page.entries {
                match entry {
                    PageEntry::ChapterHeader {
                        overline,
                        title_job,
                        title_text,
                        show_rule,
                        subtitle,
                        trailing_space,
                    } => {
                        if let Some(over) = overline {
                            ui.label(
                                RichText::new(over)
                                    .size(10.5)
                                    .strong()
                                    .color(params.accent_color),
                            );
                            ui.add_space(4.0);
                        }
                        if !title_text.is_empty() {
                            let resp = selectable_label_preserving_on_secondary(
                                ui,
                                title_job.clone(),
                                params.allow_select,
                            );
                            resp.context_menu(|ui| {
                                render_text_context_menu(
                                    ui,
                                    ContextMenuParams {
                                        doc: params.doc.as_ref(),
                                        current_chapter: *params.current_chapter,
                                        can_copy: params.doc.can_copy(),
                                        single_text: title_text,
                                        tts_paragraph_idx: 0,
                                        translation_provider: params.settings.translation.provider,
                                        t: params.t,
                                        on_start_tts: params.on_start_tts,
                                        on_translate: params.on_translate,
                                        on_dictionary: None,
                                        on_wikipedia: None,
                                        on_save_quote: params.on_save_quote,
                                    },
                                );
                            });
                            ui.add_space(if is_cozy { 10.0 } else { 20.0 });
                            if *show_rule {
                                let (rule_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(42.0, 2.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rule_rect,
                                    1.0,
                                    params.accent_color.gamma_multiply(0.45),
                                );
                                ui.add_space(14.0);
                            }
                        }
                        if let Some(sub) = subtitle {
                            ui.label(
                                RichText::new(sub)
                                    .size(params.font_size * 0.78)
                                    .italics()
                                    .color(params.ink_color.gamma_multiply(0.6)),
                            );
                            ui.add_space(10.0);
                        }
                        if *trailing_space > 0.0 {
                            ui.add_space(*trailing_space);
                        }
                    }
                    PageEntry::Space(h) => {
                        ui.add_space(*h);
                    }
                    PageEntry::Image {
                        id,
                        size,
                        trailing_space,
                    } => {
                        if let Some(texture) = params.text_images.texture(ui.ctx(), params.doc, id)
                        {
                            ui.add(egui::Image::from_texture(&texture).fit_to_exact_size(*size));
                        } else {
                            let (r, _) = ui.allocate_exact_size(*size, egui::Sense::hover());
                            ui.painter().rect_stroke(
                                r,
                                6.0,
                                Stroke::new(1.0, params.theme.text_color().gamma_multiply(0.25)),
                                StrokeKind::Inside,
                            );
                        }
                        ui.add_space(*trailing_space);
                    }
                    PageEntry::TextSlice {
                        para_idx,
                        raw_paragraph,
                        clean_paragraph,
                        job,
                        byte_start,
                        byte_end,
                        char_offset,
                        markers,
                        is_opening,
                        is_blockquote,
                        pad_top,
                        pad_bot,
                        trailing_space,
                    } => {
                        let is_tts_speaking = params.tts_highlight == Some(*para_idx);
                        let mut active_job = if is_tts_speaking && params.tts_word_range.is_some() {
                            let mut dummy_markers = Vec::new();
                            let query_str = params.search_highlight.map_or("", |(q, _)| q);
                            let active_para = params.search_highlight.and_then(|(_, p)| p);
                            let mut rebuilt = create_paragraph_layout_job(
                                raw_paragraph,
                                &ParagraphStyle {
                                    font_size: params.font_size,
                                    text_color: params.ink_color,
                                    accent_color: params.accent_color,
                                    line_height,
                                    query: query_str,
                                    is_active_hit: active_para == Some(*para_idx),
                                    is_rtl: params.is_rtl,
                                    bionic_reading: params.settings.effective_bionic_reading(),
                                    quotes: params.chapter_quotes,
                                    footnote_numbers: params.footnote_numbers,
                                    note_available: params.available_notes,
                                    tts_word_range: params.tts_word_range,
                                    is_opening: *is_opening,
                                    is_blockquote: *is_blockquote,
                                },
                                &mut dummy_markers,
                            );
                            rebuilt.wrap = job.wrap.clone();
                            slice_layout_job(&rebuilt, *byte_start, *byte_end, *char_offset == 0)
                        } else {
                            job.clone()
                        };

                        let (resp, footnote_galley) = if *is_blockquote {
                            let top_m = if *pad_top { 8 } else { 0 };
                            let bot_m = if *pad_bot { 8 } else { 0 };
                            let frame_resp = egui::Frame::new()
                                .fill(params.colors.quote_fill)
                                .corner_radius(egui::CornerRadius {
                                    nw: if params.is_rtl { 6 } else { 0 },
                                    sw: if params.is_rtl { 6 } else { 0 },
                                    ne: if params.is_rtl { 0 } else { 6 },
                                    se: if params.is_rtl { 0 } else { 6 },
                                })
                                .inner_margin(egui::Margin {
                                    left: if params.is_rtl { 12 } else { 14 },
                                    right: if params.is_rtl { 14 } else { 12 },
                                    top: top_m,
                                    bottom: bot_m,
                                })
                                .show(ui, |ui| {
                                    let fg = if markers.is_empty() {
                                        None
                                    } else {
                                        active_job.wrap.max_width = ui.available_width();
                                        Some(ui.painter().layout_job(active_job.clone()))
                                    };
                                    let r = selectable_label_preserving_on_secondary(
                                        ui,
                                        active_job,
                                        params.allow_select,
                                    );
                                    (r, fg)
                                });
                            let fr = frame_resp.response.rect;
                            let bar_rect = if params.is_rtl {
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
                            ui.painter()
                                .rect_filled(bar_rect, 1.5, params.colors.quote_border);
                            frame_resp.inner
                        } else {
                            let fg = if markers.is_empty() {
                                None
                            } else {
                                active_job.wrap.max_width = ui.available_width();
                                Some(ui.painter().layout_job(active_job.clone()))
                            };
                            let r = selectable_label_preserving_on_secondary(
                                ui,
                                active_job,
                                params.allow_select,
                            );
                            (r, fg)
                        };

                        if let Some(galley) = footnote_galley
                            && let Some(note_id) = show_hover_card(
                                ui,
                                params.doc.as_ref(),
                                &resp,
                                &galley,
                                markers,
                                params.colors,
                                params.design,
                                params.t,
                                params.font_size,
                                params.pinned_note,
                                footnote_state,
                            )
                            && let Some(location) = params.doc.note_location(&note_id)
                        {
                            *params.note_return =
                                Some((*params.current_chapter, *params.current_scroll_y));
                            *params.note_jump =
                                Some((location.chapter_index, location.paragraph_index));
                            *params.current_chapter = location.chapter_index;
                            *params.last_rendered_chapter = location.chapter_index;
                            *params.current_scroll_y = 0.0;
                            *params.pinned_note = None;
                        }

                        if is_tts_speaking {
                            ui.painter().rect(
                                resp.rect.expand2(Vec2::new(6.0, 2.0)),
                                4.0,
                                params.accent_color.gamma_multiply(0.12),
                                Stroke::new(1.0, params.accent_color.gamma_multiply(0.45)),
                                StrokeKind::Inside,
                            );
                        }

                        resp.context_menu(|ui| {
                            render_text_context_menu(
                                ui,
                                ContextMenuParams {
                                    doc: params.doc.as_ref(),
                                    current_chapter: *params.current_chapter,
                                    can_copy: params.doc.can_copy(),
                                    single_text: clean_paragraph,
                                    tts_paragraph_idx: *para_idx,
                                    translation_provider: params.settings.translation.provider,
                                    t: params.t,
                                    on_start_tts: params.on_start_tts,
                                    on_translate: params.on_translate,
                                    on_dictionary: Some(params.on_dictionary),
                                    on_wikipedia: Some(params.on_wikipedia),
                                    on_save_quote: params.on_save_quote,
                                },
                            );
                        });

                        if *trailing_space > 0.0 {
                            ui.add_space(*trailing_space);
                        }
                    }
                    PageEntry::FooterNote { text } => {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(text)
                                .size(params.font_size * 0.78)
                                .italics()
                                .color(params.ink_color.gamma_multiply(0.6)),
                        );
                    }
                }
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn render_studio_spread_nav(
    ui: &mut egui::Ui,
    nav_rect: egui::Rect,
    cur_ch_page: usize,
    total_ch_pages: usize,
    visible_slots: usize,
    is_rtl: bool,
    theme: &ReaderTheme,
    t: &Translations,
) -> i32 {
    let mut step = 0i32;
    ui.scope_builder(egui::UiBuilder::new().max_rect(nav_rect), |ui| {
        ui.painter().line_segment(
            [
                egui::Pos2::new(nav_rect.min.x, nav_rect.min.y),
                egui::Pos2::new(nav_rect.max.x, nav_rect.min.y),
            ],
            Stroke::new(1.0, theme.text_color().gamma_multiply(0.12)),
        );
        let end_vis = if visible_slots > 1 && cur_ch_page + 1 < total_ch_pages {
            format!("{}-{}", cur_ch_page + 1, cur_ch_page + 2)
        } else {
            format!("{}", cur_ch_page + 1)
        };
        let label = format!("{} {end_vis} {} {total_ch_pages}", t.page_prefix, t.page_of);
        let label_color = theme.text_color().gamma_multiply(0.75);
        let label_galley = ui.painter().layout_no_wrap(
            label.clone(),
            egui::FontId::proportional(11.5),
            label_color,
        );
        let btn_w = ui.spacing().button_padding.x * 2.0 + 12.0;
        let group_w = btn_w * 2.0 + label_galley.size().x + ui.spacing().item_spacing.x * 2.0;
        let lead = ((nav_rect.width() - group_w) * 0.5).max(0.0);
        ui.horizontal_centered(|ui| {
            let prev_icon = if is_rtl {
                Icon::ArrowRight
            } else {
                Icon::ArrowLeft
            };
            let next_icon = if is_rtl {
                Icon::ArrowLeft
            } else {
                Icon::ArrowRight
            };

            if lead > 0.0 {
                ui.add_space(lead);
            }
            if ui
                .add(crate::app::icons::IconButton::new(prev_icon, "").font_size(12.0))
                .on_hover_text(t.prev_section)
                .clicked()
            {
                step = -1;
            }
            ui.label(RichText::new(label).size(11.5).color(label_color));
            if ui
                .add(crate::app::icons::IconButton::new(next_icon, "").font_size(12.0))
                .on_hover_text(t.next_section)
                .clicked()
            {
                step = 1;
            }
        });
    });
    step
}
