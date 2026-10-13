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
            if ui.button("📂 Открыть...").on_hover_text("Открыть один или несколько файлов в новых вкладках (PDF, FB2, EPUB, TXT, MD, LOG, CBZ, CBR, CB7, ZIP, RAR, 7Z)").clicked() {
                actions.open_file = true;
            }
            if ui.button("📁 Папка...").on_hover_text("Открыть каталог с изображениями манги или комикса в новой вкладке").clicked() {
                actions.open_folder = true;
            }
            if ui.button("📚 Недавние...").on_hover_text("Список недавно открытых книг").clicked() {
                actions.toggle_recent = true;
            }
            if has_document {
                ui.separator();
                let toc_text = if show_toc { "📖 Скрыть оглавление" } else { "📖 Оглавление" };
                if ui.button(toc_text).clicked() {
                    actions.toggle_toc = true;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fs_text = if is_fullscreen { "🗗 Оконный (F)" } else { "⛶ На весь экран (F)" };
                if ui.button(fs_text).on_hover_text("Полноэкранный режим чтения (горячая клавиша: F, выход: F или Esc)").clicked() {
                    actions.toggle_fullscreen = true;
                }
                if ui.button("⚙ Настройки...").on_hover_text("Шрифты, темы оформления, автопрокрутка, комиксы и горячие клавиши").clicked() {
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
                ui.label(RichText::new(format!("📖 {book_info}")).strong());
                if let Some(err) = crate::settings::AppSettings::last_save_error() {
                    ui.separator();
                    ui.label(RichText::new(format!("⚠ {err}")).color(egui::Color32::RED));
                } else if let Some(msg) = status_message {
                    ui.separator();
                    ui.label(RichText::new(msg).color(theme.accent_color()));
                }
            } else if let Some(err) = crate::settings::AppSettings::last_save_error() {
                ui.label(RichText::new(format!("⚠ {err}")).color(egui::Color32::RED));
            } else if let Some(msg) = status_message {
                ui.label(msg);
            } else {
                ui.label("Готов к чтению");
            }

            let (cur_str, pct_str) = match view_kind {
                DocumentViewKind::Pdf => {
                    let (cur_idx, total) = pdf_page;
                    let cur = cur_idx + 1;
                    let pct = if total > 0 { (cur as f32 / total as f32 * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                    let fmt = document.as_ref().map(|d| d.format_name()).unwrap_or("PDF");
                    (format!("Страница {fmt} {cur} из {total}"), format!("{pct:.1}%"))
                }
                DocumentViewKind::Comic => {
                    let (cur_idx, total) = comic_page;
                    let cur = cur_idx + 1;
                    let pct = if total > 0 { (cur as f32 / total as f32 * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                    (format!("Страница {cur} из {total}"), format!("{pct:.1}%"))
                }
                DocumentViewKind::Text => {
                    if document.is_some() {
                        let (ch_idx, total, cur_scroll, max_scroll) = text_chapter;
                        let scroll_frac = if max_scroll > 1.0 { (cur_scroll / max_scroll).clamp(0.0, 1.0) } else { 1.0 };
                        let pct = if total > 0 { ((ch_idx as f32 + scroll_frac) / total as f32 * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                        (format!("Раздел {} из {total}", ch_idx + 1), format!("{pct:.1}%"))
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
