use super::bookmarks_view::{BookmarkViewState, BookmarksView};
use super::icons::{Icon, IconButton};
use super::theme::ReaderTheme;
use crate::document::{Document, TocItem};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Panel, RichText, ScrollArea, Vec2};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarTab {
    #[default]
    Toc,
    Bookmarks,
}

pub struct TocView;

impl TocView {
    pub fn render(
        ui: &mut egui::Ui,
        document: &Option<std::sync::Arc<dyn Document>>,
        current_chapter: &mut usize,
        current_page: &mut usize,
        show_toc: &mut bool,
        sidebar_tab: &mut SidebarTab,
        settings: &mut AppSettings,
        current_file_path: Option<&Path>,
        current_scroll_y: f32,
        pending_scroll_y: &mut Option<f32>,
        last_rendered_chapter: &mut usize,
        theme: &ReaderTheme,
        status_message: &mut Option<String>,
        return_location: &mut Option<(usize, f32, Option<usize>)>,
        bookmark_state: &mut BookmarkViewState,
        on_add_bookmark: &mut bool,
    ) {
        if !*show_toc {
            return;
        }

        let doc = match document {
            Some(d) => d,
            None => return,
        };

        let lang = settings.ui_language;
        let t = lang.t();
        let available_w = ui.available_width();
        let max_toc_w = (available_w * 0.45).clamp(240.0, 520.0);
        let mut close_toc = false;

        Panel::left("toc_panel")
            .resizable(true)
            .show_separator_line(true)
            .min_size(180.0)
            .max_size(max_toc_w)
            .default_size(300.0)
            .show(ui, |ui| {
                ui.style_mut().interaction.resize_grab_radius_side = 8.0;
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal(|ui| {
                    let is_toc = *sidebar_tab == SidebarTab::Toc;
                    let is_bm = *sidebar_tab == SidebarTab::Bookmarks;
                    let count = current_file_path.map_or(0, |path| {
                        settings
                            .bookmarks
                            .iter()
                            .filter(|b| b.book_path == path)
                            .count()
                    });
                    let bm_title = if count > 0 {
                        format!("{} ({count})", t.bookmarks)
                    } else {
                        t.bookmarks.to_string()
                    };

                    if ui
                        .selectable_label(is_toc, RichText::new(t.toc).strong())
                        .clicked()
                    {
                        *sidebar_tab = SidebarTab::Toc;
                    }
                    if ui
                        .selectable_label(is_bm, RichText::new(bm_title).strong())
                        .clicked()
                    {
                        *sidebar_tab = SidebarTab::Bookmarks;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close_btn =
                            IconButton::new(Icon::Cross, "").min_size(Vec2::splat(18.0));
                        if ui.add(close_btn).on_hover_text(t.hide_toc).clicked() {
                            close_toc = true;
                        }
                    });
                });
                ui.add_space(4.0);

                if *sidebar_tab == SidebarTab::Bookmarks {
                    let meta = doc.metadata();
                    BookmarksView::render(
                        ui,
                        settings,
                        current_file_path,
                        current_chapter,
                        current_scroll_y,
                        pending_scroll_y,
                        last_rendered_chapter,
                        current_page,
                        theme,
                        lang,
                        status_message,
                        &meta.title,
                        &meta.author,
                        return_location,
                        bookmark_state,
                        on_add_bookmark,
                    );
                    return;
                }

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width());
                        if let Some(page_content) = doc.page_content() {
                            let page_count = page_content.page_count();
                            let toc = doc.toc();

                            let has_custom_toc = doc.has_custom_toc();

                            if has_custom_toc {
                                for item in toc {
                                    render_toc_item(
                                        ui,
                                        item,
                                        *current_page,
                                        current_page,
                                        theme.accent_color(),
                                    );
                                }
                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(4.0);
                            }

                            let is_double = settings.comic.page_layout == "Double"
                                && settings.comic.reading_mode != "Webtoon";
                            let step = if is_double { 2 } else { 1 };
                            let total_rows = page_count.div_ceil(step);
                            let row_height = ui.text_style_height(&egui::TextStyle::Button) + 6.0;

                            let mut render_pages_virtual = |ui: &mut egui::Ui| {
                                let mut new_page = None;
                                ScrollArea::vertical()
                                    .id_salt("pages_virtual_scroll")
                                    .auto_shrink([false, false])
                                    .show_rows(ui, row_height, total_rows, |ui, row_range| {
                                        for row_idx in row_range {
                                            let p = row_idx * step;
                                            if p >= page_count {
                                                break;
                                            }
                                            let is_current = if is_double {
                                                *current_page == p
                                                    || (p + 1 < page_count
                                                        && *current_page == p + 1)
                                            } else {
                                                *current_page == p
                                            };

                                            let page_name = page_content.page_name(p);
                                            let label = if is_double && p + 1 < page_count {
                                                if page_name.is_empty() {
                                                    format!("{} {}-{}", t.page_prefix, p + 1, p + 2)
                                                } else {
                                                    format!(
                                                        "{} {}-{} ({page_name})",
                                                        t.page_prefix,
                                                        p + 1,
                                                        p + 2
                                                    )
                                                }
                                            } else if page_name.is_empty() {
                                                format!("{} {}", t.page_prefix, p + 1)
                                            } else {
                                                format!("{} {} ({page_name})", t.page_prefix, p + 1)
                                            };

                                            let mut rich = RichText::new(label);
                                            if is_current {
                                                rich = rich.strong().color(theme.accent_color());
                                            }

                                            let btn =
                                                egui::Button::selectable(is_current, rich).wrap();
                                            if ui.add(btn).clicked() {
                                                new_page = Some(p);
                                            }
                                        }
                                    });

                                if let Some(p) = new_page {
                                    *current_page = p;
                                }
                            };

                            if has_custom_toc {
                                egui::CollapsingHeader::new(format!("Все страницы ({page_count})"))
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        render_pages_virtual(ui);
                                    });
                            } else {
                                ui.label(
                                    RichText::new(format!("Страницы ({page_count}):")).strong(),
                                );
                                ui.add_space(4.0);
                                render_pages_virtual(ui);
                            }
                        } else {
                            let toc = doc.toc();
                            if toc.is_empty() {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(t.toc_no_chapters)
                                        .italics()
                                        .color(Color32::from_gray(140)),
                                );
                            } else {
                                for item in toc {
                                    render_toc_item(
                                        ui,
                                        item,
                                        *current_chapter,
                                        current_chapter,
                                        theme.accent_color(),
                                    );
                                }
                            }
                        }
                    });
            });

        if close_toc {
            *show_toc = false;
        }
    }
}

pub fn render_toc_item(
    ui: &mut egui::Ui,
    item: &TocItem,
    current_chapter: usize,
    selected_chapter: &mut usize,
    accent_color: Color32,
) {
    render_toc_item_bounded(ui, item, current_chapter, selected_chapter, accent_color, 0);
}

fn render_toc_item_bounded(
    ui: &mut egui::Ui,
    item: &TocItem,
    current_chapter: usize,
    selected_chapter: &mut usize,
    accent_color: Color32,
    depth: usize,
) {
    if depth >= 32 {
        return;
    }
    let is_selected = current_chapter == item.chapter_index;
    let title = if item.title.trim().is_empty() {
        "—"
    } else {
        item.title.as_str()
    };

    if item.children.is_empty() {
        let mut rich = RichText::new(title);
        if is_selected {
            rich = rich.strong().color(accent_color);
        }
        let btn = egui::Button::selectable(is_selected, rich).wrap();
        if ui.add(btn).clicked() {
            *selected_chapter = item.chapter_index;
        }
    } else {
        let header_id = ui.make_persistent_id(format!(
            "toc_{}_{}_{}",
            depth, item.chapter_index, item.title
        ));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), header_id, true)
            .show_header(ui, |ui| {
                let mut rich = RichText::new(title);
                if is_selected {
                    rich = rich.strong().color(accent_color);
                }
                let btn = egui::Button::selectable(is_selected, rich).wrap();
                if ui.add(btn).clicked() {
                    *selected_chapter = item.chapter_index;
                }
            })
            .body(|ui| {
                ui.indent(header_id.with("indent"), |ui| {
                    for child in &item.children {
                        render_toc_item_bounded(
                            ui,
                            child,
                            current_chapter,
                            selected_chapter,
                            accent_color,
                            depth + 1,
                        );
                    }
                });
            });
    }
}
