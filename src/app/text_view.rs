use super::theme::ReaderTheme;
use crate::document::Document;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Vec2};
use std::path::PathBuf;

/// Отрисовка центральной текстовой области книги (главы, абзацы, контекстное меню копирования)
/// и стартового приветственного экрана.
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
        _status_message: &mut Option<String>,
        on_open_file: &mut bool,
        on_open_folder: &mut bool,
        on_load_recent: &mut Option<PathBuf>,
    ) {
        if let Some(doc) = document {
            let total_chapters = doc.chapters().len();
            let chapter = doc.chapters().get(*current_chapter);

            let mut scroll_area = ScrollArea::vertical().auto_shrink([false, false]);
            if let Some(offset) = pending_scroll_y.take() {
                scroll_area = scroll_area.vertical_scroll_offset(offset);
            }

            let scroll_output = scroll_area.show(ui, |ui| {
                // Центрированная колонка фиксированной читаемой ширины (макс 760px)
                ui.vertical_centered(|ui| {
                    ui.set_max_width(760.0);

                    if let Some(ch) = chapter {
                        ui.add_space(20.0);
                        if !ch.title.is_empty() {
                            let title_heading = RichText::new(&ch.title)
                                .size(font_size * 1.35)
                                .strong()
                                .color(theme.text_color());
                            let resp = ui.add(egui::Label::new(title_heading).selectable(true));
                            resp.context_menu(|ui| {
                                if ui.button("📋 Копировать").clicked() {
                                    ui.ctx().copy_text(ch.title.clone());
                                    ui.close();
                                }
                                if ui.button("🔲 Выбрать всё (копировать раздел)").clicked()
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

                        // Рендеринг абзацев текста
                        for paragraph in &ch.paragraphs {
                            if paragraph.is_empty() {
                                ui.add_space(font_size * 0.7);
                            } else {
                                let text_content = format!("    {paragraph}");
                                let text = RichText::new(&text_content)
                                    .size(font_size)
                                    .color(theme.text_color());
                                let resp = ui.add(egui::Label::new(text).wrap().selectable(true));
                                resp.context_menu(|ui| {
                                    if ui.button("📋 Копировать").clicked() {
                                        ui.ctx().copy_text(paragraph.clone());
                                        ui.close();
                                    }
                                    if ui.button("🔲 Выбрать всё (копировать раздел)").clicked()
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
                                ui.add_space(font_size * 0.4);
                            }
                        }

                        ui.add_space(40.0);

                        // Навигационные кнопки перехода между разделами внизу страницы
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
                                        *last_rendered_chapter = *current_chapter;
                                    }
                                },
                            );
                        });

                        ui.add_space(30.0);
                    }
                });
            });

            *current_scroll_y = scroll_output.state.offset.y;
            *viewport_height = scroll_output.inner_rect.height();
            *max_scroll_y =
                (scroll_output.content_size.y - scroll_output.inner_rect.height()).max(0.0);

            // Сохранение прогресса чтения в память
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
            // Приветственный экран при отсутствии открытой книги
            ui.vertical_centered(|ui| {
                ui.add_space(80.0);
                ui.heading(
                    RichText::new("📖 QuietReader")
                        .size(36.0)
                        .color(theme.accent_color()),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Минималистичная, портативная читалка книг, документов и комиксов")
                        .size(18.0)
                        .color(theme.text_color()),
                );
                ui.add_space(30.0);

                ui.horizontal(|ui| {
                    let open_btn = egui::Button::new(
                        RichText::new("📂 Выбрать книгу / PDF / комикс").size(17.0),
                    )
                    .min_size(Vec2::new(280.0, 46.0));

                    if ui
                        .add(open_btn)
                        .on_hover_text("PDF, FB2, EPUB, TXT, MD, LOG, CBZ, CBR, CB7, ZIP, RAR, 7Z")
                        .clicked()
                    {
                        *on_open_file = true;
                    }

                    let open_dir_btn = egui::Button::new(
                        RichText::new("📁 Открыть папку манги").size(17.0),
                    )
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
                            format!("📖 {} (Раздел {})", book.title, book.chapter + 1)
                        } else {
                            format!("📖 {} — {} (Раздел {})", book.author, book.title, book.chapter + 1)
                        };
                        if ui.button(RichText::new(label).size(14.0)).clicked() {
                            *on_load_recent = Some(book.path.clone());
                        }
                        ui.add_space(4.0);
                    }
                }
            });
        }
    }
}
