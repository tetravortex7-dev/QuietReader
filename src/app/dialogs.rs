use super::theme::ReaderTheme;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Fonts,
    Appearance,
    AutoScroll,
    ComicsAndManga,
    Hotkeys,
    System,
}

/// Управление модальными диалогами: недавние книги, ассоциации файлов FB2
pub struct Dialogs;

impl Dialogs {
    /// Окно ассоциации файлов FB2 в Windows
    pub fn render_association_dialog(
        ctx: &egui::Context,
        show_dialog: &mut bool,
        dont_show_again: &mut bool,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_register_association: &mut bool,
    ) {
        if !*show_dialog {
            return;
        }

        let window_frame = egui::Frame::window(&ctx.global_style())
            .fill(theme.panel_bg_color())
            .stroke(egui::Stroke::new(
                1.5,
                theme.accent_color().gamma_multiply(0.6),
            ))
            .inner_margin(egui::Margin::symmetric(14, 12));

        egui::Window::new("📌 Ассоциация файлов FB2")
            .collapsible(false)
            .resizable(false)
            .frame(window_frame)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_max_width(420.0);
                ui.label(
                    RichText::new("Сделать QuietReader основной читалкой для файлов .fb2?")
                        .size(15.0)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    "QuietReader зарегистрируется в Windows для открытия файлов .fb2 по двойному щелчку и добавится в меню «Открыть с помощью». Также откроются параметры Windows для подтверждения выбора по умолчанию.",
                );
                ui.add_space(8.0);
                ui.checkbox(dont_show_again, "Не показывать снова");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("✔ Сделать по умолчанию").strong()).clicked() {
                        *on_register_association = true;
                        crate::platform::set_association_prompt_dismissed(true);
                        if *dont_show_again {
                            settings.dismiss_association_prompt = true;
                            let _ = settings.save();
                        }
                        *show_dialog = false;
                    }
                    if ui.button("✖ Не сейчас").clicked() {
                        if *dont_show_again {
                            crate::platform::set_association_prompt_dismissed(true);
                            settings.dismiss_association_prompt = true;
                            let _ = settings.save();
                        }
                        *show_dialog = false;
                    }
                });
                ui.add_space(4.0);
            });
    }

    /// Окно истории недавних книг
    pub fn render_recent_dialog(
        ctx: &egui::Context,
        show_dialog: &mut bool,
        current_file_path: &Option<PathBuf>,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_open_file: &mut Option<PathBuf>,
    ) {
        if !*show_dialog {
            return;
        }

        let mut book_to_open = None;
        let mut clear_recent = false;

        let window_frame = egui::Frame::window(&ctx.global_style())
            .fill(theme.panel_bg_color())
            .stroke(egui::Stroke::new(
                1.5,
                theme.accent_color().gamma_multiply(0.6),
            ))
            .inner_margin(egui::Margin::symmetric(14, 12));

        egui::Window::new("📚 Недавние книги")
            .collapsible(false)
            .resizable(true)
            .frame(window_frame)
            .default_size([520.0, 360.0])
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("История чтения");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Закрыть").clicked() {
                            *show_dialog = false;
                        }
                        if !settings.recent_books.is_empty()
                            && ui
                                .button("🗑 Очистить")
                                .on_hover_text("Очистить список недавних книг")
                                .clicked()
                        {
                            clear_recent = true;
                        }
                    });
                });
                ui.separator();

                if settings.recent_books.is_empty() {
                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Список недавних книг пуст")
                                .italics()
                                .color(Color32::from_gray(140)),
                        );
                    });
                } else {
                    ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        for book in &settings.recent_books {
                            let is_current = current_file_path.as_ref() == Some(&book.path);
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        let mut title_text =
                                            RichText::new(&book.title).strong().size(15.0);
                                        if is_current {
                                            title_text = title_text.color(theme.accent_color());
                                        }
                                        ui.label(title_text);

                                        if !book.author.is_empty() {
                                            ui.label(
                                                RichText::new(&book.author)
                                                    .size(13.0)
                                                    .color(Color32::from_gray(160)),
                                            );
                                        }

                                        let file_name = book
                                            .path
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("");
                                        ui.label(
                                            RichText::new(format!(
                                                "Раздел {} • {}",
                                                book.chapter + 1,
                                                file_name
                                            ))
                                            .size(12.0)
                                            .color(Color32::from_gray(130)),
                                        );
                                    });

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let btn_text = if is_current {
                                                "Открыта"
                                            } else {
                                                "Читать »"
                                            };
                                            if ui.button(btn_text).clicked() {
                                                book_to_open = Some(book.path.clone());
                                            }
                                        },
                                    );
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                }
            });

        if clear_recent {
            settings.recent_books.clear();
            settings.last_opened_book = None;
            let _ = settings.save();
        }

        if let Some(path) = book_to_open {
            *show_dialog = false;
            *on_open_file = Some(path);
        }
    }
}
