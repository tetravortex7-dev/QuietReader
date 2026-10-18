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
    pub toggle_search: bool,
    pub toggle_tts: bool,
    pub toggle_fullscreen: bool,
    pub toggle_settings: bool,
    pub toggle_reading_ruler: bool,
    pub open_update: bool,
}

pub fn render_top_bar(
    ui: &mut egui::Ui,
    has_document: bool,
    is_fullscreen: bool,
    show_toc: bool,
    reading_ruler: bool,
    lang: crate::i18n::UiLanguage,
    update_available_version: Option<&str>,
) -> TopBarActions {
    let t = lang.t();
    let mut actions = TopBarActions {
        open_file: false,
        open_folder: false,
        toggle_recent: false,
        toggle_toc: false,
        toggle_search: false,
        toggle_tts: false,
        toggle_fullscreen: false,
        toggle_settings: false,
        toggle_reading_ruler: false,
        open_update: false,
    };

    Panel::top("top_bar").show(ui, |ui| {
        let is_rtl = lang.is_rtl();
        let main_layout = if is_rtl {
            egui::Layout::right_to_left(egui::Align::Center)
        } else {
            egui::Layout::left_to_right(egui::Align::Center)
        };
        let secondary_layout = if is_rtl {
            egui::Layout::left_to_right(egui::Align::Center)
        } else {
            egui::Layout::right_to_left(egui::Align::Center)
        };

        ui.with_layout(main_layout, |ui| {
            if icon_button(ui, Icon::OpenFile, t.open_file).on_hover_text(t.open_file_tooltip).clicked() {
                actions.open_file = true;
            }
            if icon_button(ui, Icon::Folder, t.open_folder).on_hover_text(t.open_folder_tooltip).clicked() {
                actions.open_folder = true;
            }
            if icon_button(ui, Icon::Recent, t.recent_books).on_hover_text(t.recent_books_tooltip).clicked() {
                actions.toggle_recent = true;
            }
            if has_document {
                ui.separator();
                let toc_text = if show_toc { t.hide_toc } else { t.toc };
                if icon_button(ui, Icon::Toc, toc_text).clicked() {
                    actions.toggle_toc = true;
                }
                if icon_button(ui, Icon::Search, t.search).on_hover_text(t.search_tooltip).clicked() {
                    actions.toggle_search = true;
                }
                if icon_button(ui, Icon::Speech, t.speech).on_hover_text(t.speech_tooltip).clicked() {
                    actions.toggle_tts = true;
                }
                let ruler_text = if reading_ruler { t.hide_reading_ruler } else { t.reading_ruler };
                if icon_button(ui, Icon::Ruler, ruler_text).on_hover_text(t.reading_ruler_tooltip).clicked() {
                    actions.toggle_reading_ruler = true;
                }
            }
            ui.with_layout(secondary_layout, |ui| {
                if icon_button(ui, Icon::Settings, t.settings).on_hover_text(t.settings_tooltip).clicked() {
                    actions.toggle_settings = true;
                }
                let (fs_icon, fs_text) = if is_fullscreen {
                    (Icon::Windowed, t.windowed)
                } else {
                    (Icon::Fullscreen, t.fullscreen)
                };
                if icon_button(ui, fs_icon, fs_text).on_hover_text(t.fullscreen_tooltip).clicked() {
                    actions.toggle_fullscreen = true;
                }
                if let Some(tag) = update_available_version {
                    let label = format!("⬆ v{tag}");
                    if ui
                        .add(egui::Button::new(RichText::new(label).color(egui::Color32::from_rgb(220, 140, 20)).strong()))
                        .on_hover_text(t.update_available_tip)
                        .clicked()
                    {
                        actions.open_update = true;
                    }
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
    lang: crate::i18n::UiLanguage,
) {
    let t = lang.t();
    Panel::bottom("bottom_bar").show(ui, |ui| {
        let is_rtl = lang.is_rtl();
        let main_layout = if is_rtl {
            egui::Layout::right_to_left(egui::Align::Center)
        } else {
            egui::Layout::left_to_right(egui::Align::Center)
        };
        let secondary_layout = if is_rtl {
            egui::Layout::left_to_right(egui::Align::Center)
        } else {
            egui::Layout::right_to_left(egui::Align::Center)
        };

        ui.with_layout(main_layout, |ui| {
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
                ui.label(t.ready_to_read);
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
                    (
                        format!("{} {cur}/{total}", t.page_prefix),
                        crate::i18n::format::format_percent(pct, lang),
                    )
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
                    (
                        format!("{} {cur}/{total}", t.page_prefix),
                        crate::i18n::format::format_percent(pct, lang),
                    )
                }
                DocumentViewKind::Text => {
                    if let Some(doc) = document {
                        let (ch_idx, total_chapters, cur_scroll, max_scroll) = text_chapter;
                        let (cur_page, total_pages, pct) =
                            calculate_text_pages(doc.chapters(), ch_idx, cur_scroll, max_scroll);
                        let page_info = format!("{} {cur_page}/{total_pages}", t.page_prefix);
                        let cur_str = if total_chapters > 1 {
                            let sec_str = t.section_x_of_y
                                .replacen("{}", &(ch_idx + 1).to_string(), 1)
                                .replacen("{}", &total_chapters.to_string(), 1);
                            format!("{page_info}  •  {sec_str}")
                        } else {
                            page_info
                        };
                        (cur_str, crate::i18n::format::format_percent(pct, lang))
                    } else {
                        (String::new(), String::new())
                    }
                }
            };
            if !cur_str.is_empty() {
                ui.with_layout(secondary_layout, |ui| {
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
