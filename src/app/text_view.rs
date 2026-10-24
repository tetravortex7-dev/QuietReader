use super::icons::{Icon, IconButton, IconLabel};
use super::theme::{ReaderTheme, bookmark_highlight_color};
use crate::document::Document;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;

mod footnotes;
pub(crate) mod layout;

use footnotes::{ParagraphStyle, collect_numbers, show_hover_card};
pub use layout::create_sample_preview_job;
use layout::{
    append_with_highlight, create_paragraph_layout_job, selectable_label_preserving_on_secondary,
};

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
        settings: &mut AppSettings,
        text_images: &mut super::text_images::TextImageState,
        on_open_file: &mut bool,
        on_open_folder: &mut bool,
        on_load_recent: &mut Option<PathBuf>,
        search_highlight: Option<(&str, Option<usize>)>,
        tts_highlight: Option<usize>,
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

            let scroll_output =
                scroll_area.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_max_width(column_width);

                        if let Some(ch) = chapter {
                            ui.add_space(20.0);
                            let clean_title = crate::document::strip_formatting_tags(&ch.title);
                            let (query_str, active_para) = match search_highlight {
                                Some((q, p)) => (q, p),
                                None => ("", Some(usize::MAX)),
                            };

                            if !clean_title.is_empty() {
                                let is_active_title =
                                    active_para.is_none() && !query_str.is_empty();
                                let mut title_job = egui::text::LayoutJob::default();
                                if is_rtl {
                                    title_job.halign = egui::Align::RIGHT;
                                }
                                let base_title_fmt = egui::TextFormat {
                                    font_id: egui::FontId::new(
                                        font_size * 1.35,
                                        egui::FontFamily::Name(
                                            crate::app::fonts::FONT_FAMILY_BOLD.into(),
                                        ),
                                    ),
                                    color: theme.text_color(),
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
                                    let selected_text =
                                        crate::app::selection::get_selected_text(ui.ctx());
                                    let has_selection = selected_text
                                        .as_ref()
                                        .map_or(false, |s| !s.trim().is_empty())
                                        || crate::app::selection::has_active_selection(ui.ctx());
                                    if selected_text.is_none() && has_selection {
                                        crate::app::selection::request_selection_capture(ui.ctx());
                                    }
                                    let prov_name = settings.translation.provider.name();

                                    if ui
                                        .add(
                                            IconButton::new(Icon::Speech, t.read_aloud_from_here)
                                                .min_size(Vec2::new(ui.available_width(), 26.0)),
                                        )
                                        .clicked()
                                    {
                                        *on_start_tts = Some(0);
                                        ui.close();
                                    }
                                    if ui
                                        .add(
                                            IconButton::new(Icon::Globe, t.translate_paragraph)
                                                .min_size(Vec2::new(ui.available_width(), 26.0)),
                                        )
                                        .on_hover_text(format!(
                                            "{} ({})",
                                            t.translate_paragraph, prov_name
                                        ))
                                        .clicked()
                                    {
                                        *on_translate = Some(clean_title.clone());
                                        ui.close();
                                    }
                                    let sel_btn = ui.add_enabled(
                                        has_selection,
                                        IconButton::new(Icon::Globe, t.translate_selection)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    );
                                    if sel_btn.clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            *on_translate = Some(text);
                                        }
                                        ui.close();
                                    }
                                    let quote_btn = ui.add_enabled(
                                        has_selection,
                                        IconButton::new(Icon::Quote, t.save_quote_bookmark)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    );
                                    if quote_btn.clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            *on_save_quote = Some(text);
                                        }
                                        ui.close();
                                    }
                                    if has_selection {
                                        sel_btn.on_hover_text(format!(
                                            "{} ({})",
                                            t.translate_selection, prov_name
                                        ));
                                    } else {
                                        sel_btn.on_disabled_hover_text(t.translate_selection_tip);
                                    }
                                    if ui.button(t.copy).clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            ui.ctx().copy_text(text);
                                        } else {
                                            ui.ctx().copy_text(clean_title.clone());
                                        }
                                        ui.close();
                                    }
                                    if ui.button(t.copy_section).clicked() {
                                        if let Some(full) =
                                            crate::core::ReaderCore::extract_chapter_text(
                                                doc.as_ref(),
                                                *current_chapter,
                                            )
                                        {
                                            ui.ctx().copy_text(full);
                                        }
                                        ui.close();
                                    }
                                });
                                ui.add_space(24.0);
                            }

                            if *current_chapter == 0
                                && let Some(header) = doc.header_text()
                            {
                                ui.label(
                                    RichText::new(header)
                                        .size(font_size * 0.78)
                                        .italics()
                                        .color(theme.text_color().gamma_multiply(0.6)),
                                );
                                ui.add_space(10.0);
                            }

                            let clip_rect = ui.clip_rect();
                            let margin = 400.0;
                            let view_top = clip_rect.min.y - margin;
                            let view_bot = clip_rect.max.y + margin;

                            let line_height = font_size * line_height_scale;
                            let para_spacing = font_size * 0.4;
                            let chars_per_line = (column_width / (font_size * 0.52)).max(10.0);
                            let two_columns = settings.text_two_columns;
                            let paragraph_count = ch.paragraphs.len();
                            let split_at = if two_columns {
                                layout::split_paragraphs(&ch.paragraphs)
                            } else {
                                paragraph_count
                            };
                            let footnote_numbers = collect_numbers(ch);
                            let mut available_notes: HashSet<String> = HashSet::new();
                            for (id, _, _) in std::iter::once(ch.title.as_str())
                                .chain(ch.paragraphs.iter().map(String::as_str))
                                .flat_map(crate::document::footnote_tokens)
                            {
                                if doc.footnote(id).is_some() || doc.note_location(id).is_some() {
                                    available_notes.insert(id.to_string());
                                }
                            }

                            let mut render_paragraphs =
                                |ui: &mut egui::Ui,
                                 range: std::ops::Range<usize>,
                                 chars_per_line: f32| {
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
                                            let mut markers = Vec::new();
                                            let mut layout_job = create_paragraph_layout_job(
                                                paragraph,
                                                &ParagraphStyle {
                                                    font_size,
                                                    text_color: theme.text_color(),
                                                    accent_color: theme.accent_color(),
                                                    line_height,
                                                    query: query_str,
                                                    is_active_hit: is_active_para,
                                                    is_rtl,
                                                    bionic_reading: settings
                                                        .effective_bionic_reading(),
                                                    quotes: &chapter_quotes,
                                                    footnote_numbers: &footnote_numbers,
                                                    note_available: &available_notes,
                                                },
                                                &mut markers,
                                            );
                                            let footnote_galley = if markers.is_empty() {
                                                None
                                            } else {
                                                layout_job.wrap.max_width = ui.available_width();
                                                Some(ui.painter().layout_job(layout_job.clone()))
                                            };
                                            let resp = selectable_label_preserving_on_secondary(
                                                ui,
                                                layout_job,
                                                allow_select,
                                            );
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
                                                    theme,
                                                    t,
                                                    font_size,
                                                    &mut pinned_note,
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
                                                    theme.accent_color().gamma_multiply(0.12),
                                                    Stroke::new(
                                                        1.0,
                                                        theme.accent_color().gamma_multiply(0.45),
                                                    ),
                                                    StrokeKind::Inside,
                                                );
                                            }
                                            resp.context_menu(|ui| {
                                    let selected_text =
                                        crate::app::selection::get_selected_text(ui.ctx());
                                    let has_selection = selected_text
                                        .as_ref()
                                        .map_or(false, |s| !s.trim().is_empty())
                                        || crate::app::selection::has_active_selection(ui.ctx());
                                    if selected_text.is_none() && has_selection {
                                        crate::app::selection::request_selection_capture(ui.ctx());
                                    }
                                    let prov_name = settings.translation.provider.name();

                                    if ui
                                        .add(
                                            IconButton::new(Icon::Speech, t.read_aloud_from_here)
                                                .min_size(Vec2::new(ui.available_width(), 26.0)),
                                        )
                                        .clicked()
                                    {
                                        *on_start_tts = Some(p_idx);
                                        ui.close();
                                    }
                                    if ui
                                        .add(
                                            IconButton::new(Icon::Globe, t.translate_paragraph)
                                                .min_size(Vec2::new(ui.available_width(), 26.0)),
                                        )
                                        .on_hover_text(format!(
                                            "{} ({})",
                                            t.translate_paragraph, prov_name
                                        ))
                                        .clicked()
                                    {
                                        *on_translate = Some(clean_paragraph.clone());
                                        ui.close();
                                    }
                                    let sel_btn = ui.add_enabled(
                                        has_selection,
                                        IconButton::new(Icon::Globe, t.translate_selection)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    );
                                    if sel_btn.clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            *on_translate = Some(text);
                                        }
                                        ui.close();
                                    }
                                    let dict_btn = ui.add_enabled(
                                        has_selection,
                                        IconButton::new(Icon::Book, t.dictionary_lookup)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    );
                                    if dict_btn.clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            *on_dictionary = Some(text);
                                        }
                                        ui.close();
                                    }
                                    let wiki_btn = ui.add_enabled(
                                        has_selection,
                                        IconButton::new(Icon::Globe, t.wikipedia_lookup)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    );
                                    if wiki_btn.clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            *on_wikipedia = Some(text);
                                        }
                                        ui.close();
                                    }
                                    if !has_selection {
                                        dict_btn.on_disabled_hover_text(t.dictionary_lookup);
                                        wiki_btn.on_disabled_hover_text(t.wikipedia_lookup);
                                    }
                                    let quote_btn = ui.add_enabled(
                                        has_selection,
                                        IconButton::new(Icon::Quote, t.save_quote_bookmark)
                                            .min_size(Vec2::new(ui.available_width(), 26.0)),
                                    );
                                    if quote_btn.clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            *on_save_quote = Some(text);
                                        }
                                        ui.close();
                                    }
                                    if has_selection {
                                        sel_btn.on_hover_text(format!(
                                            "{} ({})",
                                            t.translate_selection, prov_name
                                        ));
                                    } else {
                                        sel_btn.on_disabled_hover_text(t.translate_selection_tip);
                                    }
                                    if ui.button(t.copy).clicked() {
                                        let sel = selected_text.clone().or_else(|| {
                                            crate::app::selection::get_selected_text(ui.ctx())
                                        });
                                        if let Some(text) = sel {
                                            ui.ctx().copy_text(text);
                                        } else {
                                            ui.ctx().copy_text(clean_paragraph.clone());
                                        }
                                        ui.close();
                                    }
                                    if ui.button(t.copy_section).clicked() {
                                        if let Some(full) =
                                            crate::core::ReaderCore::extract_chapter_text(
                                                doc.as_ref(),
                                                *current_chapter,
                                            )
                                        {
                                            ui.ctx().copy_text(full);
                                        }
                                        ui.close();
                                    }
                                });
                                            ui.add_space(para_spacing);
                                        }
                                    }
                                };

                            if two_columns {
                                ui.horizontal_top(|ui| {
                                    let gutter = 36.0;
                                    let total = ui.available_width().min(column_width);
                                    let column_width_each = ((total - gutter) * 0.5).max(120.0);
                                    let column_chars =
                                        ((column_width_each) / (font_size * 0.52)).max(10.0);
                                    let mut render_column =
                                        |ui: &mut egui::Ui, range: std::ops::Range<usize>| {
                                            ui.vertical(|ui| {
                                                ui.set_width(column_width_each);
                                                render_paragraphs(ui, range, column_chars);
                                            });
                                        };
                                    if is_rtl {
                                        render_column(ui, split_at..paragraph_count);
                                        ui.add_space(gutter);
                                        render_column(ui, 0..split_at);
                                    } else {
                                        render_column(ui, 0..split_at);
                                        ui.add_space(gutter);
                                        render_column(ui, split_at..paragraph_count);
                                    }
                                });
                            } else {
                                render_paragraphs(ui, 0..paragraph_count, chars_per_line);
                            }

                            if *current_chapter + 1 >= total_chapters
                                && let Some(footer) = doc.footer_text()
                            {
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(footer)
                                        .size(font_size * 0.78)
                                        .italics()
                                        .color(theme.text_color().gamma_multiply(0.6)),
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
                    });
                });

            *current_scroll_y = scroll_output.state.clamped_offset().y;
            *viewport_height = scroll_output.inner_rect.height();
            *max_scroll_y =
                (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);

            if settings.text_paged_scroll && *viewport_height > 1.0 && *max_scroll_y > 0.0 {
                let index_key = egui::Id::unique(("quietreader_page_index", *current_chapter));
                let mut page_index: usize = ui.data(|data| data.get_temp(index_key).unwrap_or(0));
                let max_index = ((*max_scroll_y / *viewport_height).ceil() as usize).max(1);
                if wheel_delta.abs() > f32::EPSILON {
                    page_index = layout::page_index_after_wheel(page_index, wheel_delta, max_index);
                    let target = layout::page_offset(page_index, *viewport_height, *max_scroll_y);
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

            if let Some(path) = current_file_path {
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
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.add(
                    IconLabel::new(Icon::Book, "QuietReader")
                        .size(36.0)
                        .color(theme.accent_color())
                        .strong(),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new(t.app_subtitle)
                        .size(18.0)
                        .color(theme.text_color()),
                );
                ui.add_space(30.0);

                ui.horizontal(|ui| {
                    let open_btn = IconButton::new(Icon::OpenFile, t.choose_book_btn)
                        .font_size(17.0)
                        .min_size(Vec2::new(220.0, 46.0));

                    if ui
                        .add(open_btn)
                        .on_hover_text(t.open_file_tooltip)
                        .clicked()
                    {
                        *on_open_file = true;
                    }

                    let open_dir_btn = IconButton::new(Icon::Folder, t.open_manga_folder_btn)
                        .font_size(17.0)
                        .min_size(Vec2::new(200.0, 46.0));

                    if ui
                        .add(open_dir_btn)
                        .on_hover_text(t.images_folder_hint)
                        .clicked()
                    {
                        *on_open_folder = true;
                    }

                    let opds_btn = IconButton::new(Icon::Globe, t.opds_catalogs)
                        .font_size(17.0)
                        .min_size(Vec2::new(200.0, 46.0));

                    if ui
                        .add(opds_btn)
                        .on_hover_text(t.opds_catalogs_tooltip)
                        .clicked()
                    {
                        *on_open_opds = true;
                    }
                });

                ui.add_space(16.0);
                ui.label(
                    RichText::new(t.drag_drop_hint)
                        .size(14.0)
                        .color(Color32::from_gray(140)),
                );

                if !settings.recent_books.is_empty() {
                    ui.add_space(28.0);
                    ui.separator();
                    ui.add_space(12.0);
                    ui.label(RichText::new(t.recent_books_header).strong().size(16.0));
                    ui.add_space(10.0);

                    for book in settings.recent_books.iter().take(4) {
                        let sec_str = t.section_num.replace("{}", &(book.chapter + 1).to_string());
                        let title = if book.title.is_empty() {
                            t.extra.default_untitled
                        } else {
                            &book.title
                        };
                        let label = if book.author.is_empty() {
                            format!("{title} ({sec_str})")
                        } else {
                            format!("{} — {title} ({sec_str})", book.author)
                        };
                        if ui
                            .add(IconButton::new(Icon::Book, &label).font_size(14.0))
                            .clicked()
                        {
                            *on_load_recent = Some(book.path.clone());
                        }
                        ui.add_space(4.0);
                    }
                }
            });
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
        },
        &mut markers,
    );
    job.wrap.max_width = max_width;
    job
}
