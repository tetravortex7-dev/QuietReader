use super::theme::ReaderTheme;
use crate::document::{Document, TocItem};
use crate::settings::ComicSettings;
use eframe::egui::{self, Color32, Panel, RichText, ScrollArea};

pub struct TocView;

impl TocView {
    pub fn render(
        ui: &mut egui::Ui,
        document: &Option<std::sync::Arc<dyn Document>>,
        comic_settings: &ComicSettings,
        current_chapter: &mut usize,
        current_page: &mut usize,
        show_toc: &mut bool,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
    ) {
        if !*show_toc {
            return;
        }

        let doc = match document {
            Some(d) => d,
            None => return,
        };

        let t = lang.t();
        let available_w = ui.available_width();
        let max_toc_w = (available_w * 0.45).clamp(200.0, 480.0);
        let mut close_toc = false;

        Panel::left("toc_panel")
            .resizable(true)
            .show_separator_line(true)
            .min_size(140.0)
            .max_size(max_toc_w)
            .default_size(260.0)
            .show(ui, |ui| {
                ui.style_mut().interaction.resize_grab_radius_side = 8.0;
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                ui.horizontal(|ui| {
                    ui.heading(t.toc);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button("×")
                            .on_hover_text(t.hide_toc)
                            .clicked()
                        {
                            close_toc = true;
                        }
                    });
                });
                ui.add_space(4.0);

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

                            let is_double = comic_settings.page_layout == "Double"
                                && comic_settings.reading_mode != "Webtoon";
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
