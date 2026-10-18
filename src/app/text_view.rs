use super::icons::{Icon, IconButton, IconLabel};
use super::theme::ReaderTheme;
use crate::document::Document;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::path::PathBuf;

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
        on_open_file: &mut bool,
        on_open_folder: &mut bool,
        on_load_recent: &mut Option<PathBuf>,
        search_highlight: Option<(&str, Option<usize>)>,
        tts_highlight: Option<usize>,
        on_start_tts: &mut Option<usize>,
    ) {
        if let Some(doc) = document {
            let total_chapters = doc.chapters().len();
            let chapter = doc.chapters().get(*current_chapter);

            let allow_select = !ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle));

            let mut scroll_area = ScrollArea::vertical()
                .id_salt(("text_scroll", current_file_path, *current_chapter))
                .auto_shrink([false, false]);
            if let Some(offset) = pending_scroll_y.take() {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
            }

            let scroll_output = scroll_area.show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(760.0);

                    if let Some(ch) = chapter {
                        ui.add_space(20.0);
                        let clean_title = crate::document::strip_formatting_tags(&ch.title);
                        let (query_str, active_para) = match search_highlight {
                            Some((q, p)) => (q, p),
                            None => ("", Some(usize::MAX)),
                        };

                        if !clean_title.is_empty() {
                            let is_active_title = active_para.is_none() && !query_str.is_empty();
                            let mut title_job = egui::text::LayoutJob::default();
                            let base_title_fmt = egui::TextFormat {
                                font_id: egui::FontId::new(
                                    font_size * 1.35,
                                    egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
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
                            );
                            let resp =
                                ui.add(egui::Label::new(title_job).selectable(allow_select));
                            resp.context_menu(|ui| {
                                if ui.button("Скопировать").clicked() {
                                    ui.ctx().copy_text(clean_title.clone());
                                    ui.close();
                                }
                                if ui.button("Выбрать всё (копировать раздел)").clicked()
                                {
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

                        let clip_rect = ui.clip_rect();
                        let margin = 400.0;
                        let view_top = clip_rect.min.y - margin;
                        let view_bot = clip_rect.max.y + margin;

                        let line_height = font_size * 1.45;
                        let para_spacing = font_size * 0.4;
                        let chars_per_line = (760.0 / (font_size * 0.52)).max(10.0);

                        for (p_idx, paragraph) in ch.paragraphs.iter().enumerate() {
                            if paragraph.is_empty() {
                                ui.add_space(font_size * 0.7);
                                continue;
                            }

                            let clean_paragraph = crate::document::strip_formatting_tags(paragraph);
                            let char_count = clean_paragraph.chars().count();
                            let num_lines = (char_count as f32 / chars_per_line).ceil().max(1.0);
                            let est_height = num_lines * line_height + para_spacing;

                            let top_y = ui.cursor().min.y;
                            let bot_y = top_y + est_height;

                            if bot_y < view_top || top_y > view_bot {
                                ui.add_space(est_height);
                            } else {
                                let is_active_para = active_para == Some(p_idx);
                                let is_tts_speaking = tts_highlight == Some(p_idx);
                                let layout_job = create_paragraph_layout_job(
                                    paragraph,
                                    font_size,
                                    theme.text_color(),
                                    query_str,
                                    is_active_para,
                                );
                                let resp = ui.add(
                                    egui::Label::new(layout_job).wrap().selectable(allow_select),
                                );
                                if is_tts_speaking {
                                    ui.painter().rect(
                                        resp.rect.expand2(Vec2::new(8.0, 3.0)),
                                        4.0,
                                        theme.accent_color().gamma_multiply(0.12),
                                        Stroke::new(1.0, theme.accent_color().gamma_multiply(0.45)),
                                        StrokeKind::Inside,
                                    );
                                }
                                resp.context_menu(|ui| {
                                    if ui.add(IconButton::new(Icon::Speech, "Читать вслух с этого места")).clicked() {
                                        *on_start_tts = Some(p_idx);
                                        ui.close();
                                    }
                                    if ui.button("Скопировать").clicked() {
                                        ui.ctx().copy_text(clean_paragraph.clone());
                                        ui.close();
                                    }
                                    if ui.button("Выбрать всё (копировать раздел)").clicked()
                                    {
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

                        ui.add_space(40.0);

                        ui.horizontal(|ui| {
                            let prev_chapter_idx = (0..*current_chapter).rev().find(|&idx| {
                                doc.chapters()
                                    .get(idx)
                                    .map(|c| !c.is_empty())
                                    .unwrap_or(false)
                            });

                            let next_chapter_idx =
                                ((*current_chapter + 1)..total_chapters).find(|&idx| {
                                    doc.chapters()
                                        .get(idx)
                                        .map(|c| !c.is_empty())
                                        .unwrap_or(false)
                                });

                            if let Some(prev_idx) = prev_chapter_idx
                                && ui.button("« Предыдущий раздел").clicked()
                            {
                                *current_chapter = prev_idx;
                                *pending_scroll_y = Some(0.0);
                                *current_scroll_y = 0.0;
                                *last_rendered_chapter = *current_chapter;
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(next_idx) = next_chapter_idx
                                        && ui.button("Следующий раздел »").clicked()
                                    {
                                        *current_chapter = next_idx;
                                        *pending_scroll_y = Some(0.0);
                                        *current_scroll_y = 0.0;
                                        *last_rendered_chapter = *current_chapter;
                                    }
                                },
                            );
                        });

                        ui.add_space(30.0);
                    }
                });
            });

            *current_scroll_y = scroll_output.state.clamped_offset().y;
            *viewport_height = scroll_output.inner_rect.height();
            *max_scroll_y =
                (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);

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
                    RichText::new("Минималистичная, портативная читалка книг, документов и комиксов")
                        .size(18.0)
                        .color(theme.text_color()),
                );
                ui.add_space(30.0);

                ui.horizontal(|ui| {
                    let open_btn = IconButton::new(
                        Icon::OpenFile,
                        "Выбрать книгу / PDF / комикс",
                    )
                    .font_size(17.0)
                    .min_size(Vec2::new(280.0, 46.0));

                    if ui
                        .add(open_btn)
                        .on_hover_text("PDF, FB2, EPUB, TXT, MD, LOG, CBZ, CBR, CB7, ZIP, RAR, 7Z")
                        .clicked()
                    {
                        *on_open_file = true;
                    }

                    let open_dir_btn = IconButton::new(
                        Icon::Folder,
                        "Открыть папку манги",
                    )
                    .font_size(17.0)
                    .min_size(Vec2::new(240.0, 46.0));

                    if ui.add(open_dir_btn).on_hover_text("Каталог с изображениями (JPG, PNG, WebP)").clicked() {
                        *on_open_folder = true;
                    }
                });

                ui.add_space(16.0);
                ui.label(
                    RichText::new("или перетащите файл (PDF, FB2, EPUB, TXT, MD, LOG, CBZ, CBR, CB7) или папку с мангой в это окно")
                        .size(14.0)
                        .color(Color32::from_gray(140)),
                );

                if !settings.recent_books.is_empty() {
                    ui.add_space(28.0);
                    ui.separator();
                    ui.add_space(12.0);
                    ui.label(RichText::new("Недавние книги:").strong().size(16.0));
                    ui.add_space(10.0);

                    for book in settings.recent_books.iter().take(4) {
                        let label = if book.author.is_empty() {
                            format!("{} (Раздел {})", book.title, book.chapter + 1)
                        } else {
                            format!("{} — {} (Раздел {})", book.author, book.title, book.chapter + 1)
                        };
                        if ui.add(IconButton::new(Icon::Book, &label).font_size(14.0)).clicked() {
                            *on_load_recent = Some(book.path.clone());
                        }
                        ui.add_space(4.0);
                    }
                }
            });
        }
    }
}

fn append_with_highlight(
    job: &mut egui::text::LayoutJob,
    text: &str,
    base_format: &egui::TextFormat,
    query: &str,
    is_active_hit: bool,
) {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        job.append(text, 0.0, base_format.clone());
        return;
    }

    let lower_text = text.to_lowercase();
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

fn create_paragraph_layout_job(
    paragraph: &str,
    font_size: f32,
    text_color: Color32,
    query: &str,
    is_active_hit: bool,
) -> egui::text::LayoutJob {
    let spans = crate::document::parse_formatted_spans(paragraph);
    let mut job = egui::text::LayoutJob::default();
    for (idx, span) in spans.iter().enumerate() {
        let text = if idx == 0 {
            format!("    {}", span.text)
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
            font_id: egui::FontId::new(font_size, family),
            color: text_color,
            italics: span.italic,
            ..Default::default()
        };
        append_with_highlight(&mut job, &text, &format, query, is_active_hit);
    }
    if job.sections.is_empty() {
        let format = egui::TextFormat {
            font_id: egui::FontId::proportional(font_size),
            color: text_color,
            ..Default::default()
        };
        append_with_highlight(
            &mut job,
            &format!("    {paragraph}"),
            &format,
            query,
            is_active_hit,
        );
    }
    job
}
