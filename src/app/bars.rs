use super::icons::{Icon, IconButton, IconLabel, icon_button};
use super::theme::ReaderTheme;
use crate::core::library::{DEFAULT_READING_WPM, estimate_minutes_left_in_chapter};
use crate::document::{Document, DocumentViewKind};
use eframe::egui::{self, Panel, RichText};
use std::sync::Arc;

pub struct TopBarActions {
    pub open_file: bool,
    pub open_folder: bool,
    pub toggle_recent: bool,
    pub toggle_library: bool,
    pub toggle_opds: bool,
    pub toggle_toc: bool,
    pub toggle_bookmark: bool,
    pub toggle_bookmarks_panel: bool,
    pub toggle_search: bool,
    pub toggle_tts: bool,
    pub toggle_fullscreen: bool,
    pub toggle_settings: bool,
    pub toggle_reading_ruler: bool,
    pub toggle_ambient: bool,
    pub open_update: bool,
    pub open_license: bool,
}

pub fn render_top_bar(
    ui: &mut egui::Ui,
    has_document: bool,
    library_visible: bool,
    is_fullscreen: bool,
    show_toc: bool,
    reading_ruler: bool,
    pro_active: bool,
    lang: crate::i18n::UiLanguage,
    update_available_version: Option<&str>,
) -> TopBarActions {
    let t = lang.t();
    let mut actions = TopBarActions {
        open_file: false,
        open_folder: false,
        toggle_recent: false,
        toggle_library: false,
        toggle_opds: false,
        toggle_toc: false,
        toggle_bookmark: false,
        toggle_bookmarks_panel: false,
        toggle_search: false,
        toggle_tts: false,
        toggle_fullscreen: false,
        toggle_settings: false,
        toggle_reading_ruler: false,
        toggle_ambient: false,
        open_update: false,
        open_license: false,
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
            if icon_button(ui, Icon::OpenFile, t.open_file)
                .on_hover_text(t.open_file_tooltip)
                .clicked()
            {
                actions.open_file = true;
            }
            if icon_button(ui, Icon::Folder, t.open_folder)
                .on_hover_text(t.open_folder_tooltip)
                .clicked()
            {
                actions.open_folder = true;
            }
            if icon_button(ui, Icon::Recent, t.recent_books)
                .on_hover_text(t.recent_books_tooltip)
                .clicked()
            {
                actions.toggle_recent = true;
            }
            if library_visible
                && icon_button(ui, Icon::Library, t.library_title)
                    .on_hover_text(t.library_open_tooltip)
                    .clicked()
            {
                actions.toggle_library = true;
            }
            if icon_button(ui, Icon::Globe, t.opds_catalogs)
                .on_hover_text(t.opds_catalogs_tooltip)
                .clicked()
            {
                actions.toggle_opds = true;
            }
            if has_document {
                ui.separator();
                let toc_text = if show_toc { t.hide_toc } else { t.toc };
                if icon_button(ui, Icon::Toc, toc_text)
                    .on_hover_text(t.toc_tooltip)
                    .clicked()
                {
                    actions.toggle_toc = true;
                }
                let bm_btn =
                    icon_button(ui, Icon::Bookmark, t.add_bookmark).on_hover_text(t.bookmark_added);
                if bm_btn.clicked() {
                    actions.toggle_bookmark = true;
                }
                if bm_btn.secondary_clicked() {
                    actions.toggle_bookmarks_panel = true;
                }
                if icon_button(ui, Icon::Search, t.search)
                    .on_hover_text(t.search_tooltip)
                    .clicked()
                {
                    actions.toggle_search = true;
                }
                if icon_button(ui, Icon::Speech, t.speech)
                    .on_hover_text(t.speech_tooltip)
                    .clicked()
                {
                    actions.toggle_tts = true;
                }
                let ruler_text = if reading_ruler {
                    t.hide_reading_ruler
                } else {
                    t.reading_ruler
                };
                if icon_button(ui, Icon::Ruler, ruler_text)
                    .on_hover_text(t.reading_ruler_tooltip)
                    .clicked()
                {
                    actions.toggle_reading_ruler = true;
                }
                if icon_button(ui, Icon::Headphones, t.ambient)
                    .on_hover_text(format!("{} (M)", t.ambient_tooltip))
                    .clicked()
                {
                    actions.toggle_ambient = true;
                }
            }
            ui.with_layout(secondary_layout, |ui| {
                let license_color = if pro_active {
                    egui::Color32::from_rgb(214, 164, 52)
                } else {
                    ui.visuals().text_color()
                };
                let license_tip = if pro_active {
                    t.license_pro_activated
                } else {
                    t.license_open_tooltip
                };
                if ui
                    .add(IconButton::new(Icon::Pro, t.license_open).color(license_color))
                    .on_hover_text(license_tip)
                    .clicked()
                {
                    actions.open_license = true;
                }
                if icon_button(ui, Icon::Settings, t.settings)
                    .on_hover_text(t.settings_tooltip)
                    .clicked()
                {
                    actions.toggle_settings = true;
                }
                let (fs_icon, fs_text) = if is_fullscreen {
                    (Icon::Windowed, t.windowed)
                } else {
                    (Icon::Fullscreen, t.fullscreen)
                };
                if icon_button(ui, fs_icon, fs_text)
                    .on_hover_text(t.fullscreen_tooltip)
                    .clicked()
                {
                    actions.toggle_fullscreen = true;
                }
                if let Some(tag) = update_available_version {
                    let label = format!("v{tag}");
                    if ui
                        .add(
                            IconButton::new(Icon::ChevronUp, &label)
                                .color(egui::Color32::from_rgb(220, 140, 20)),
                        )
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
    status_opacity: f32,
    view_kind: DocumentViewKind,
    pdf_page: (usize, usize),
    comic_page: (usize, usize),
    text_chapter: (usize, usize, f32, f32),
    lang: crate::i18n::UiLanguage,
) {
    let t = lang.t();
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

    let (cur_str, pct_str, progress_frac) = match view_kind {
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
            let frac = if total > 0 {
                (cur as f32 / total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (
                format!("{} {cur}/{total}", t.page_prefix),
                crate::i18n::format::format_percent(pct, lang),
                frac,
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
            let frac = if total > 0 {
                (cur as f32 / total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (
                format!("{} {cur}/{total}", t.page_prefix),
                crate::i18n::format::format_percent(pct, lang),
                frac,
            )
        }
        DocumentViewKind::Text => {
            if let Some(doc) = document {
                let (ch_idx, total_chapters, cur_scroll, max_scroll) = text_chapter;
                let (cur_page, total_pages, pct) =
                    calculate_text_pages(doc.chapters(), ch_idx, cur_scroll, max_scroll);
                let page_info = format!("{} {cur_page}/{total_pages}", t.page_prefix);
                let mut parts = vec![page_info];
                if total_chapters > 1 {
                    parts.push(
                        t.section_x_of_y
                            .replacen("{}", &(ch_idx + 1).to_string(), 1)
                            .replacen("{}", &total_chapters.to_string(), 1),
                    );
                }
                let chapter_chars = doc
                    .chapters()
                    .get(ch_idx)
                    .map(chapter_char_count)
                    .unwrap_or(0);
                if chapter_chars > 0 {
                    let chapter_progress = if max_scroll > 1.0 {
                        (cur_scroll / max_scroll).clamp(0.0, 1.0)
                    } else if ch_idx + 1 == doc.chapters().len() {
                        1.0
                    } else {
                        0.0
                    };
                    let minutes_left = estimate_minutes_left_in_chapter(
                        chapter_chars,
                        chapter_progress,
                        DEFAULT_READING_WPM,
                    );
                    parts.push(
                        t.chapter_time_left
                            .replace("{minutes}", &minutes_left.to_string()),
                    );
                }
                (
                    parts.join("  |  "),
                    crate::i18n::format::format_percent(pct, lang),
                    (pct / 100.0).clamp(0.0, 1.0),
                )
            } else {
                (String::new(), String::new(), 0.0)
            }
        }
    };

    Panel::bottom("bottom_bar").show(ui, |ui| {
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
                } else if let Some(err) = crate::settings::AppSettings::last_secret_error() {
                    ui.separator();
                    ui.add(
                        IconLabel::new(Icon::Warning, &t.secrets_storage_error.replace("{}", &err))
                            .color(egui::Color32::RED),
                    );
                } else if let Some(msg) = status_message {
                    if status_opacity > 0.01 {
                        ui.separator();
                        ui.label(
                            RichText::new(msg)
                                .color(theme.accent_color().gamma_multiply(status_opacity)),
                        );
                    }
                }
            } else if let Some(err) = crate::settings::AppSettings::last_save_error() {
                ui.add(IconLabel::new(Icon::Warning, &err).color(egui::Color32::RED));
            } else if let Some(err) = crate::settings::AppSettings::last_secret_error() {
                ui.add(
                    IconLabel::new(Icon::Warning, &t.secrets_storage_error.replace("{}", &err))
                        .color(egui::Color32::RED),
                );
            } else if let Some(msg) = status_message {
                if status_opacity > 0.01 {
                    ui.label(
                        RichText::new(msg).color(theme.text_color().gamma_multiply(status_opacity)),
                    );
                } else {
                    ui.label(t.ready_to_read);
                }
            } else {
                ui.label(t.ready_to_read);
            }

            if !cur_str.is_empty() {
                ui.with_layout(secondary_layout, |ui| {
                    ui.label(RichText::new(format!("{cur_str}  |  {pct_str}")).strong());
                });
            }
        });

        if document.is_some() && progress_frac > 0.0 {
            let bar_rect = ui.max_rect();
            let line_height = 2.0;
            let track_rect = egui::Rect::from_min_max(
                egui::pos2(bar_rect.min.x, bar_rect.min.y),
                egui::pos2(bar_rect.max.x, bar_rect.min.y + line_height),
            );
            ui.painter()
                .rect_filled(track_rect, 0.0, theme.accent_color().gamma_multiply(0.18));
            let fill_width = bar_rect.width() * progress_frac.clamp(0.0, 1.0);
            let fill_rect = if is_rtl {
                egui::Rect::from_min_max(
                    egui::pos2(bar_rect.max.x - fill_width, bar_rect.min.y),
                    egui::pos2(bar_rect.max.x, bar_rect.min.y + line_height),
                )
            } else {
                egui::Rect::from_min_max(
                    egui::pos2(bar_rect.min.x, bar_rect.min.y),
                    egui::pos2(bar_rect.min.x + fill_width, bar_rect.min.y + line_height),
                )
            };
            ui.painter()
                .rect_filled(fill_rect, 0.0, theme.accent_color());
        }
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
        let ch_len = chapter_char_count(ch);
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

fn chapter_char_count(chapter: &crate::document::BookChapter) -> usize {
    chapter.title.chars().count()
        + chapter
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.chars().count())
            .sum::<usize>()
}
