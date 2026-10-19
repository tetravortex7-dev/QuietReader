use super::icons::{Icon, IconButton, paint_icon};
use super::theme::ReaderTheme;
use crate::core::{Bookmark, BookmarkColor, BookmarkKind};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, Vec2};
use std::path::Path;

#[derive(Default)]
pub struct BookmarkViewState {
    pub search_query: String,
    pub selected_color: Option<BookmarkColor>,
    pub editing_id: Option<String>,
    pub editing_text: String,
}

pub struct BookmarksView;

impl BookmarksView {
    pub fn render(
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        current_file_path: Option<&Path>,
        current_chapter: &mut usize,
        current_scroll_y: f32,
        pending_scroll_y: &mut Option<f32>,
        last_rendered_chapter: &mut usize,
        current_page: &mut usize,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
        status_message: &mut Option<String>,
        book_title: &str,
        book_author: &str,
        return_location: &mut Option<(usize, f32, Option<usize>)>,
        state: &mut BookmarkViewState,
        on_add_bookmark: &mut bool,
    ) {
        let t = lang.t();
        let Some(file_path) = current_file_path else {
            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(t.no_bookmarks_hint)
                        .italics()
                        .color(Color32::from_gray(140)),
                );
            });
            return;
        };

        if let Some((ret_ch, ret_sy, ret_pg)) = *return_location {
            let ret_btn = IconButton::new(Icon::ArrowLeft, t.return_to_reading)
                .min_size(Vec2::new(ui.available_width(), 26.0));
            if ui.add(ret_btn).clicked() {
                *current_chapter = ret_ch;
                *pending_scroll_y = Some(ret_sy);
                *last_rendered_chapter = ret_ch;
                if let Some(p) = ret_pg {
                    *current_page = p;
                }
                *return_location = None;
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
        }

        let add_btn = IconButton::new(Icon::Bookmark, t.add_bookmark)
            .min_size(Vec2::new(ui.available_width(), 26.0));
        if ui.add(add_btn).on_hover_text(t.bookmark_added).clicked() {
            *on_add_bookmark = true;
        }
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let search_edit = egui::TextEdit::singleline(&mut state.search_query)
                .hint_text(t.bookmark_search_placeholder)
                .desired_width(ui.available_width() - 28.0);
            ui.add(search_edit);
            if !state.search_query.is_empty() {
                let clear_btn = IconButton::new(Icon::Cross, "").min_size(Vec2::splat(18.0));
                if ui.add(clear_btn).clicked() {
                    state.search_query.clear();
                }
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let is_all = state.selected_color.is_none();
            if ui.selectable_label(is_all, t.filter_all).clicked() {
                state.selected_color = None;
            }

            for &color in &BookmarkColor::ALL {
                let is_sel = state.selected_color == Some(color);
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::click());
                let col = color.to_color32();
                if ui.is_rect_visible(rect) {
                    let center = rect.center();
                    ui.painter().circle_filled(center, 7.0, col);
                    if is_sel {
                        ui.painter().circle_stroke(
                            center,
                            8.5,
                            Stroke::new(1.8, theme.text_color()),
                        );
                    }
                }
                if resp.clicked() {
                    state.selected_color = if is_sel { None } else { Some(color) };
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        let query = state.search_query.trim().to_lowercase();
        let active_color = state.selected_color;

        let matching_indices: Vec<usize> = settings
            .bookmarks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.book_path == file_path)
            .filter(|(_, b)| active_color.map_or(true, |c| b.color == c))
            .filter(|(_, b)| {
                if query.is_empty() {
                    true
                } else {
                    b.label.to_lowercase().contains(&query)
                        || b.text.to_lowercase().contains(&query)
                        || b.note.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();

        let total_book_bookmarks = settings
            .bookmarks
            .iter()
            .filter(|b| b.book_path == file_path)
            .count();

        let available_h = ui.available_height() - 44.0;
        let mut to_delete_id: Option<String> = None;
        let mut save_note_action: Option<(String, String)> = None;

        ScrollArea::vertical()
            .id_salt("bookmarks_cards_scroll")
            .max_height(available_h.max(60.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width());

                if total_book_bookmarks == 0 {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(t.no_bookmarks_hint)
                            .italics()
                            .color(Color32::from_gray(140)),
                    );
                    return;
                }

                if matching_indices.is_empty() {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(t.no_matching_bookmarks)
                            .italics()
                            .color(Color32::from_gray(140)),
                    );
                    return;
                }

                for &idx in &matching_indices {
                    let b = &mut settings.bookmarks[idx];
                    let bm_id = b.id.clone();
                    let is_quote = b.kind == BookmarkKind::Quote;
                    let marker_color = b.color.to_color32();

                    egui::Frame::default()
                        .corner_radius(5.0)
                        .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.25)))
                        .fill(theme.panel_bg_color().gamma_multiply(0.6))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_max_width(ui.available_width());

                            ui.horizontal(|ui| {
                                let (dot_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                                ui.painter()
                                    .circle_filled(dot_rect.center(), 4.5, marker_color);

                                let header_label = if b.label.is_empty() {
                                    format!(
                                        "{} {:.0}%",
                                        t.bookmark_kind_bookmark, b.progress_percent
                                    )
                                } else if let Some(p) = b.page_number {
                                    format!("{}, {} {}", b.label, t.page_prefix, p + 1)
                                } else {
                                    format!("{} • {:.0}%", b.label, b.progress_percent)
                                };

                                let header_resp = ui.link(RichText::new(header_label).strong());
                                if header_resp.clicked() {
                                    let from =
                                        (*current_chapter, current_scroll_y, Some(*current_page));
                                    let diff = *current_chapter != b.chapter
                                        || (current_scroll_y - b.scroll_y).abs() > 40.0
                                        || b.page_number.map_or(false, |p| p != *current_page);
                                    if diff {
                                        *return_location = Some(from);
                                    }
                                    *current_chapter = b.chapter;
                                    *pending_scroll_y = Some(b.scroll_y);
                                    *last_rendered_chapter = b.chapter;
                                    if let Some(p) = b.page_number {
                                        *current_page = p;
                                    }
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let kind_tag = if is_quote {
                                            t.bookmark_kind_quote
                                        } else {
                                            t.bookmark_kind_bookmark
                                        };
                                        ui.label(
                                            RichText::new(kind_tag)
                                                .size(11.0)
                                                .color(theme.accent_color().gamma_multiply(0.8)),
                                        );
                                    },
                                );
                            });

                            ui.add_space(4.0);

                            if !b.text.is_empty() {
                                let body_text = if is_quote {
                                    RichText::new(&b.text).italics().size(13.0)
                                } else {
                                    RichText::new(&b.text)
                                        .size(12.5)
                                        .color(theme.secondary_text_color())
                                };
                                ui.label(body_text);
                                ui.add_space(4.0);
                            }

                            if state.editing_id.as_deref() == Some(&bm_id) {
                                ui.add_space(2.0);
                                ui.add(
                                    egui::TextEdit::multiline(&mut state.editing_text)
                                        .hint_text(t.note_placeholder)
                                        .desired_width(ui.available_width())
                                        .desired_rows(2),
                                );
                                ui.horizontal(|ui| {
                                    if ui.button(t.save_note).clicked() {
                                        save_note_action =
                                            Some((bm_id.clone(), state.editing_text.clone()));
                                        state.editing_id = None;
                                    }
                                    if ui.button(t.cancel_note).clicked() {
                                        state.editing_id = None;
                                    }
                                });
                            } else if !b.note.is_empty() {
                                egui::Frame::default()
                                    .corner_radius(3.0)
                                    .fill(theme.accent_color().gamma_multiply(0.08))
                                    .inner_margin(6.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let (ic_rect, _) = ui.allocate_exact_size(
                                                Vec2::splat(12.0),
                                                egui::Sense::hover(),
                                            );
                                            paint_icon(
                                                ui.painter(),
                                                Icon::Edit,
                                                ic_rect,
                                                theme.accent_color(),
                                            );
                                            ui.label(RichText::new(&b.note).size(12.0));
                                        });
                                    });
                                ui.add_space(2.0);
                            }

                            ui.horizontal(|ui| {
                                for &c in &BookmarkColor::ALL {
                                    let (c_rect, c_resp) = ui.allocate_exact_size(
                                        Vec2::splat(14.0),
                                        egui::Sense::click(),
                                    );
                                    let col = c.to_color32();
                                    if ui.is_rect_visible(c_rect) {
                                        let center = c_rect.center();
                                        ui.painter().circle_filled(center, 4.5, col);
                                        if b.color == c {
                                            ui.painter().circle_stroke(
                                                center,
                                                6.0,
                                                Stroke::new(1.4, theme.text_color()),
                                            );
                                        }
                                    }
                                    if c_resp.clicked() {
                                        b.color = c;
                                    }
                                }

                                ui.separator();

                                let note_btn_text = if b.note.is_empty() {
                                    t.add_note
                                } else {
                                    t.edit_note
                                };
                                if ui.small_button(note_btn_text).clicked() {
                                    state.editing_id = Some(bm_id.clone());
                                    state.editing_text = b.note.clone();
                                }

                                if ui.small_button(t.copy).clicked() {
                                    let copy_str = if b.note.is_empty() {
                                        b.text.clone()
                                    } else {
                                        format!("{}\n\n{}: {}", b.text, t.add_note, b.note)
                                    };
                                    ui.ctx().copy_text(copy_str);
                                    *status_message = Some(t.copy.to_string());
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let del_btn = IconButton::new(Icon::Trash, "")
                                            .min_size(Vec2::splat(18.0));
                                        if ui
                                            .add(del_btn)
                                            .on_hover_text(t.delete_bookmark)
                                            .clicked()
                                        {
                                            to_delete_id = Some(bm_id.clone());
                                        }
                                    },
                                );
                            });
                        });

                    ui.add_space(6.0);
                }
            });

        if let Some((id, note)) = save_note_action {
            if let Some(item) = settings.bookmarks.iter_mut().find(|b| b.id == id) {
                item.note = note;
            }
        }

        if let Some(del_id) = to_delete_id {
            settings.bookmarks.retain(|b| b.id != del_id);
            *status_message = Some(t.bookmark_removed.to_string());
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let copy_all_btn = IconButton::new(Icon::Copy, t.copy_all_bookmarks)
                .min_size(Vec2::new((ui.available_width() - 6.0) * 0.5, 26.0));
            if ui.add(copy_all_btn).clicked() {
                let cur_bms: Vec<Bookmark> = settings
                    .bookmarks
                    .iter()
                    .filter(|b| b.book_path == file_path)
                    .cloned()
                    .collect();
                let conspectus = generate_markdown_conspectus(book_title, book_author, &cur_bms, t);
                ui.ctx().copy_text(conspectus);
                *status_message = Some(t.bookmarks_copied.to_string());
            }

            let export_btn = IconButton::new(Icon::Download, t.export_markdown)
                .min_size(Vec2::new(ui.available_width(), 26.0));
            if ui.add(export_btn).clicked() {
                let cur_bms: Vec<Bookmark> = settings
                    .bookmarks
                    .iter()
                    .filter(|b| b.book_path == file_path)
                    .cloned()
                    .collect();
                let conspectus = generate_markdown_conspectus(book_title, book_author, &cur_bms, t);

                let safe_name = if book_title.is_empty() {
                    "book_notes.md".to_string()
                } else {
                    let sanitized: String = book_title
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                                c
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    format!("{}_notes.md", sanitized.trim())
                };

                if let Some(save_path) = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md"])
                    .set_file_name(&safe_name)
                    .save_file()
                {
                    if std::fs::write(&save_path, conspectus).is_ok() {
                        *status_message = Some(t.bookmarks_exported.to_string());
                    }
                }
            }
        });
    }
}

pub fn generate_markdown_conspectus(
    book_title: &str,
    book_author: &str,
    bookmarks: &[Bookmark],
    t: &crate::i18n::Translations,
) -> String {
    let mut sorted = bookmarks.to_vec();
    sorted.sort_by(|a, b| {
        a.chapter
            .cmp(&b.chapter)
            .then(
                a.scroll_y
                    .partial_cmp(&b.scroll_y)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.timestamp.cmp(&b.timestamp))
    });

    let mut md = String::new();
    let title = if book_title.is_empty() {
        "QuietReader Notes"
    } else {
        book_title
    };
    md.push_str(&format!("# {}\n\n", title));
    if !book_author.is_empty() {
        md.push_str(&format!("**Author**: {}\n\n", book_author));
    }
    md.push_str("---\n\n");

    let mut current_ch = None;
    for b in &sorted {
        let ch_id = (b.chapter, b.label.as_str());
        if current_ch != Some(ch_id) {
            current_ch = Some(ch_id);
            md.push_str(&format!("## {}\n\n", b.label));
        }

        if b.kind == BookmarkKind::Quote {
            for line in b.text.lines() {
                md.push_str(&format!("> {}\n", line));
            }
            md.push('\n');
        } else {
            md.push_str(&format!("*{}*: {}\n\n", t.bookmark_kind_bookmark, b.text));
        }

        if !b.note.is_empty() {
            md.push_str(&format!("**{}**: {}\n\n", t.add_note, b.note));
        }

        md.push_str("---\n\n");
    }

    md
}
