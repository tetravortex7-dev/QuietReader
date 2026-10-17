use super::icons::{Icon, IconLabel, icon_button};
use super::theme::ReaderTheme;
use crate::document::{Document, DocumentViewKind};
use eframe::egui::{self, Panel, RichText};
use std::sync::Arc;

pub struct TopBarActions {
    pub open_file: bool,
    pub open_folder: bool,
    pub toggle_recent: bool,
    pub toggle_toc: bool,
    pub toggle_fullscreen: bool,
    pub toggle_settings: bool,
}

pub fn render_top_bar(
    ui: &mut egui::Ui,
    has_document: bool,
    is_fullscreen: bool,
    show_toc: bool,
) -> TopBarActions {
    let mut actions = TopBarActions {
        open_file: false,
        open_folder: false,
        toggle_recent: false,
        toggle_toc: false,
        toggle_fullscreen: false,
        toggle_settings: false,
    };

    Panel::top("top_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if icon_button(ui, Icon::OpenFile, "Открыть...").on_hover_text("Открыть один или несколько файлов в новых вкладках (PDF, FB2, EPUB, TXT, MD, LOG, CBZ, CBR, CB7, ZIP, RAR, 7Z)").clicked() {
                actions.open_file = true;
            }
            if icon_button(ui, Icon::Folder, "Папка...").on_hover_text("Открыть каталог с изображениями манги или комикса в новой вкладке").clicked() {
                actions.open_folder = true;
            }
            if icon_button(ui, Icon::Recent, "Недавние...").on_hover_text("Список недавно открытых книг").clicked() {
                actions.toggle_recent = true;
            }
            if has_document {
                ui.separator();
                let toc_text = if show_toc { "Скрыть оглавление" } else { "Оглавление" };
                if icon_button(ui, Icon::Toc, toc_text).clicked() {
                    actions.toggle_toc = true;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (fs_icon, fs_text) = if is_fullscreen {
                    (Icon::Windowed, "Оконный (F)")
                } else {
                    (Icon::Fullscreen, "На весь экран (F)")
                };
                if icon_button(ui, fs_icon, fs_text).on_hover_text("Полноэкранный режим чтения (горячая клавиша: F, выход: F или Esc)").clicked() {
                    actions.toggle_fullscreen = true;
                }
                if icon_button(ui, Icon::Settings, "Настройки...").on_hover_text("Шрифты, темы оформления, автопрокрутка, комиксы и горячие клавиши").clicked() {
                    actions.toggle_settings = true;
                }
            });
        });
    });

    actions
}

pub fn render_bottom_bar(
    ui: &mut egui::Ui,
    document: &Option<Arc<dyn Document>>,
    theme: &ReaderTheme,
    status_message: Option<&str>,
    view_kind: DocumentViewKind,
    pdf_page: (usize, usize),
    comic_page: (usize, usize),
    text_chapter: (usize, usize, f32, f32),
) {
    Panel::bottom("bottom_bar").show(ui, |ui| {
        ui.horizontal(|ui| {
            if let Some(doc) = document {
                let meta = doc.metadata();
                let book_info = if meta.author.is_empty() {
                    meta.title.clone()
                } else {
                    format!("{} — {}", meta.author, meta.title)
                };
                ui.add(IconLabel::new(Icon::Book, &book_info).strong());
                if let Some(err) = crate::settings::AppSettings::last_save_error() {
                    ui.separator();
                    ui.add(IconLabel::new(Icon::Warning, &err).color(egui::Color32::RED));
                } else if let Some(msg) = status_message {
                    ui.separator();
                    ui.label(RichText::new(msg).color(theme.accent_color()));
                }
            } else if let Some(err) = crate::settings::AppSettings::last_save_error() {
                ui.add(IconLabel::new(Icon::Warning, &err).color(egui::Color32::RED));
            } else if let Some(msg) = status_message {
                ui.label(msg);
            } else {
                ui.label("Готов к чтению");
            }

            let (cur_str, pct_str) = match view_kind {
                DocumentViewKind::Pdf => {
                    let (cur_idx, total) = pdf_page;
                    let cur = if total > 0 {
                        (cur_idx + 1).min(total)
                    } else {
                        0
                    };
                    let pct = if total > 0 {
                        (cur as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };
                    (format!("Страница {cur}/{total}"), format!("{pct:.1}%"))
                }
                DocumentViewKind::Comic => {
                    let (cur_idx, total) = comic_page;
                    let cur = if total > 0 {
                        (cur_idx + 1).min(total)
                    } else {
                        0
                    };
                    let pct = if total > 0 {
                        (cur as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };
                    (format!("Страница {cur}/{total}"), format!("{pct:.1}%"))
                }
                DocumentViewKind::Text => {
                    if let Some(doc) = document {
                        let (ch_idx, total_chapters, cur_scroll, max_scroll) = text_chapter;
                        let (cur_page, total_pages, pct) =
                            calculate_text_pages(doc.chapters(), ch_idx, cur_scroll, max_scroll);
                        let page_info = format!("Страница {cur_page}/{total_pages}");
                        let cur_str = if total_chapters > 1 {
                            format!("{page_info}  •  Раздел {} из {total_chapters}", ch_idx + 1)
                        } else {
                            page_info
                        };
                        (cur_str, format!("{pct:.1}%"))
                    } else {
                        (String::new(), String::new())
                    }
                }
            };
            if !cur_str.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{cur_str}  •  {pct_str}")).strong());
                });
            }
        });
    });
}

pub fn calculate_text_pages(
    chapters: &[crate::document::BookChapter],
    ch_idx: usize,
    cur_scroll: f32,
    max_scroll: f32,
) -> (usize, usize, f32) {
    const CHARS_PER_PAGE: usize = 1800;
    let scroll_frac = if max_scroll > 1.0 {
        (cur_scroll / max_scroll).clamp(0.0, 1.0)
    } else if ch_idx + 1 == chapters.len() {
        1.0
    } else {
        0.0
    };
    let mut total_chars = 0usize;
    let mut chars_read = 0usize;
    for (i, ch) in chapters.iter().enumerate() {
        let ch_len = ch.title.chars().count()
            + ch.paragraphs
                .iter()
                .map(|p| p.chars().count())
                .sum::<usize>();
        if i < ch_idx {
            chars_read += ch_len;
        } else if i == ch_idx {
            chars_read += (ch_len as f64 * scroll_frac as f64).round() as usize;
        }
        total_chars += ch_len;
    }
    let total_pages = (total_chars.div_ceil(CHARS_PER_PAGE)).max(1);
    let cur_page = if total_chars == 0 {
        1
    } else {
        ((chars_read / CHARS_PER_PAGE) + 1).min(total_pages)
    };
    let pct = if total_chars > 0 {
        (chars_read as f32 / total_chars as f32 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    (cur_page, total_pages, pct)
}
