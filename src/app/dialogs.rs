use super::icons::{Icon, IconButton, icon_button};
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

pub struct Dialogs;

impl Dialogs {
    pub fn pick_document_files(lang: crate::i18n::UiLanguage) -> Option<Vec<PathBuf>> {
        let t = lang.t();
        rfd::FileDialog::new()
            .add_filter(
                t.format_all_supported,
                &[
                    "pdf", "djvu", "djv", "fb2", "epub", "mobi", "prc", "azw", "azw3", "docx",
                    "doc", "rtf", "txt", "md", "markdown", "log", "cbz", "cbr", "cb7", "zip",
                    "rar", "7z", "xml",
                ],
            )
            .add_filter(t.format_word, &["docx", "doc", "dot"])
            .add_filter(t.format_rtf, &["rtf"])
            .add_filter(t.format_pdf, &["pdf"])
            .add_filter(t.format_djvu, &["djvu", "djv"])
            .add_filter(t.format_comics, &["cbz", "cbr", "cb7", "zip", "rar", "7z"])
            .add_filter(t.format_epub, &["epub"])
            .add_filter(t.format_fb2, &["fb2", "xml"])
            .add_filter("Kindle / MOBI / AZW3", &["mobi", "prc", "azw", "azw3"])
            .add_filter(t.format_txt, &["txt", "md", "markdown", "log"])
            .pick_files()
    }

    pub fn pick_comic_folder(lang: crate::i18n::UiLanguage) -> Option<PathBuf> {
        let t = lang.t();
        rfd::FileDialog::new()
            .set_title(t.choose_manga_folder_title)
            .pick_folder()
    }

    pub fn render_association_dialog(
        ctx: &egui::Context,
        show_dialog: &mut bool,
        dont_show_again: &mut bool,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_register_association: &mut bool,
        on_save_settings: &mut bool,
    ) {
        if !*show_dialog {
            return;
        }

        let t = settings.ui_language.t();
        let window_frame = egui::Frame::window(&ctx.global_style())
            .fill(theme.panel_bg_color())
            .stroke(egui::Stroke::new(
                1.5,
                theme.accent_color().gamma_multiply(0.6),
            ))
            .inner_margin(egui::Margin::symmetric(14, 12));

        egui::Window::new(t.assoc_dialog_title)
            .collapsible(false)
            .resizable(false)
            .frame(window_frame)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_max_width(460.0);
                ui.label(RichText::new(t.assoc_dialog_heading).size(15.0).strong());
                ui.add_space(4.0);
                ui.label(t.assoc_dialog_text);
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.small_button(t.select_all).clicked() {
                        settings.associated_extensions = crate::platform::SUPPORTED_EXTENSIONS_INFO
                            .iter()
                            .map(|(ext, _)| ext.to_string())
                            .collect();
                        *on_save_settings = true;
                    }
                    if ui.small_button(t.deselect_all).clicked() {
                        settings.associated_extensions.clear();
                        *on_save_settings = true;
                    }
                    if ui.small_button(t.only_books).clicked() {
                        settings.associated_extensions =
                            vec!["fb2".to_string(), "epub".to_string()];
                        *on_save_settings = true;
                    }
                });
                ui.add_space(6.0);

                egui::Frame::new()
                    .fill(theme.bg_color())
                    .stroke(egui::Stroke::new(
                        1.0,
                        theme.accent_color().gamma_multiply(0.2),
                    ))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(210.0)
                            .show(ui, |ui| {
                                for &(ext, _) in crate::platform::SUPPORTED_EXTENSIONS_INFO {
                                    let label = crate::i18n::format::format_extension_description(
                                        ext,
                                        settings.ui_language,
                                    );
                                    let mut is_checked = settings
                                        .associated_extensions
                                        .iter()
                                        .any(|e| e.eq_ignore_ascii_case(ext));
                                    if ui.checkbox(&mut is_checked, label).changed() {
                                        if is_checked {
                                            if !settings
                                                .associated_extensions
                                                .iter()
                                                .any(|e| e.eq_ignore_ascii_case(ext))
                                            {
                                                settings
                                                    .associated_extensions
                                                    .push(ext.to_string());
                                            }
                                        } else {
                                            settings
                                                .associated_extensions
                                                .retain(|e| !e.eq_ignore_ascii_case(ext));
                                        }
                                        *on_save_settings = true;
                                    }
                                }
                            });
                    });

                ui.add_space(8.0);
                ui.checkbox(dont_show_again, t.dont_show_assoc);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let has_selected = !settings.associated_extensions.is_empty();
                    let btn =
                        ui.add_enabled(has_selected, IconButton::new(Icon::Check, t.apply_assoc));
                    if btn.clicked() {
                        *on_register_association = true;
                        crate::platform::set_association_prompt_dismissed(true);
                        if *dont_show_again {
                            settings.dismiss_association_prompt = true;
                        }
                        *on_save_settings = true;
                        *show_dialog = false;
                    }
                    if ui
                        .add(IconButton::new(Icon::Close, t.remind_later))
                        .clicked()
                    {
                        if *dont_show_again {
                            crate::platform::set_association_prompt_dismissed(true);
                            settings.dismiss_association_prompt = true;
                            *on_save_settings = true;
                        }
                        *show_dialog = false;
                    }
                });
                ui.add_space(4.0);
            });
    }

    pub fn render_recent_dialog(
        ctx: &egui::Context,
        show_dialog: &mut bool,
        current_file_path: &Option<PathBuf>,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_open_file: &mut Option<PathBuf>,
        on_save_settings: &mut bool,
    ) {
        if !*show_dialog {
            return;
        }

        let t = settings.ui_language.t();
        let mut book_to_open = None;
        let mut clear_recent = false;

        let window_frame = egui::Frame::window(&ctx.global_style())
            .fill(theme.panel_bg_color())
            .stroke(egui::Stroke::new(
                1.5,
                theme.accent_color().gamma_multiply(0.6),
            ))
            .inner_margin(egui::Margin::symmetric(14, 12));

        egui::Window::new(t.recent_books)
            .collapsible(false)
            .resizable(true)
            .frame(window_frame)
            .default_size([520.0, 360.0])
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(t.reading_history);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t.close_btn).clicked() {
                            *show_dialog = false;
                        }
                        if !settings.recent_books.is_empty()
                            && icon_button(ui, Icon::Trash, t.clear_history)
                                .on_hover_text(t.clear_history_tip)
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
                            RichText::new(t.history_empty)
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
                                        let sec_info = t
                                            .section_x_of_y
                                            .replacen("{}", &(book.chapter + 1).to_string(), 1)
                                            .replacen("{}", "", 1);
                                        ui.label(
                                            RichText::new(format!(
                                                "{} • {}",
                                                sec_info
                                                    .trim_end_matches(" / ")
                                                    .trim_end_matches(" of "),
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
                                                t.book_open_now
                                            } else {
                                                t.read_now_btn
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
            *on_save_settings = true;
        }

        if let Some(path) = book_to_open {
            *show_dialog = false;
            *on_open_file = Some(path);
        }
    }
}
