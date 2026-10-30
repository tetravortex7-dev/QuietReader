use super::{LibraryFilter, LibraryViewState};
use crate::app::icons::{Icon, IconButton, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::{LibraryMode, LibraryReadingStatus, LibraryStore, ShelfViewMode};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

pub fn render_filter_bar(
    ui: &mut egui::Ui,
    state: &mut LibraryViewState,
    library: &mut LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
) -> bool {
    let mut save_library = false;
    let is_cozy = !theme.is_dark();
    let bar_fill = if is_cozy {
        CozyPalette::SURFACE_LOW
    } else {
        theme.panel_bg_color()
    };
    let border_stroke = Stroke::new(
        1.0,
        if is_cozy {
            CozyPalette::OUTLINE.gamma_multiply(0.35)
        } else {
            theme.accent_color().gamma_multiply(0.2)
        },
    );

    let all_count = library.entries.len();
    let reading_count = library
        .entries
        .iter()
        .filter(|e| e.status == LibraryReadingStatus::Reading)
        .count();
    let finished_count = library
        .entries
        .iter()
        .filter(|e| e.status == LibraryReadingStatus::Finished)
        .count();

    egui::Frame::new()
        .fill(bar_fill)
        .stroke(border_stroke)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            let total_w = ui.available_width();
            ui.set_width(total_w);

            ui.horizontal(|ui| {
                let filters = [
                    (LibraryFilter::All, format!("{all_count}")),
                    (LibraryFilter::Reading, format!("{reading_count}")),
                    (LibraryFilter::Finished, format!("{finished_count}")),
                ];
                for (f, count_str) in filters {
                    let is_active = state.filter == f;
                    let label_base = match f {
                        LibraryFilter::All => t.library_filter_all,
                        LibraryFilter::Reading => t.library_filter_reading,
                        LibraryFilter::Finished => t.library_filter_finished,
                    };
                    let full_label = format!("{label_base} ({count_str})");

                    let (btn_rect, btn_resp) = ui.allocate_exact_size(
                        Vec2::new(full_label.len() as f32 * 6.8 + 18.0, 26.0),
                        egui::Sense::click(),
                    );
                    if is_active {
                        ui.painter().rect_filled(
                            btn_rect,
                            6.0,
                            if is_cozy {
                                CozyPalette::PRIMARY
                            } else {
                                theme.accent_color()
                            },
                        );
                        ui.painter().text(
                            btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &full_label,
                            egui::FontId::proportional(11.5),
                            Color32::WHITE,
                        );
                    } else {
                        if btn_resp.hovered() {
                            ui.painter().rect_filled(
                                btn_rect,
                                6.0,
                                if is_cozy {
                                    CozyPalette::SURFACE_HIGH
                                } else {
                                    theme.panel_bg_color().gamma_multiply(1.2)
                                },
                            );
                        } else {
                            ui.painter().rect_filled(
                                btn_rect,
                                6.0,
                                if is_cozy {
                                    CozyPalette::SURFACE_CONTAINER
                                } else {
                                    theme.panel_bg_color()
                                },
                            );
                        }
                        ui.painter().text(
                            btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &full_label,
                            egui::FontId::proportional(11.5),
                            if is_cozy {
                                CozyPalette::ON_SURFACE_VARIANT
                            } else {
                                theme.secondary_text_color()
                            },
                        );
                    }

                    if btn_resp.clicked() {
                        state.filter = f;
                    }
                    ui.add_space(4.0);
                }

                let search_width = ui.available_width().max(160.0).min(380.0);
                let space_to_right = (ui.available_width() - search_width).max(0.0);
                if space_to_right > 0.0 {
                    ui.add_space(space_to_right);
                }

                let (search_rect, _) =
                    ui.allocate_exact_size(Vec2::new(search_width, 28.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    search_rect,
                    6.0,
                    if is_cozy {
                        CozyPalette::CARD_BG
                    } else {
                        theme.panel_bg_color().gamma_multiply(1.15)
                    },
                );
                ui.painter()
                    .rect_stroke(search_rect, 6.0, border_stroke, StrokeKind::Inside);
                let icon_r = Rect::from_center_size(
                    Pos2::new(search_rect.min.x + 14.0, search_rect.center().y),
                    Vec2::splat(12.0),
                );
                paint_icon(
                    ui.painter(),
                    Icon::Search,
                    icon_r,
                    if is_cozy {
                        CozyPalette::ON_SURFACE_VARIANT.gamma_multiply(0.6)
                    } else {
                        theme.secondary_text_color()
                    },
                );

                let mut text_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            Pos2::new(search_rect.min.x + 26.0, search_rect.min.y + 2.0),
                            Pos2::new(search_rect.max.x - 6.0, search_rect.max.y - 2.0),
                        ))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                text_ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(t.library_search_full_hint)
                        .frame(egui::Frame::NONE)
                        .desired_width(search_width - 32.0),
                );
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let density_rect = ui
                    .allocate_exact_size(Vec2::new(126.0, 26.0), egui::Sense::hover())
                    .0;
                ui.painter().rect_filled(
                    density_rect,
                    6.0,
                    if is_cozy {
                        CozyPalette::SURFACE_CONTAINER
                    } else {
                        theme.panel_bg_color()
                    },
                );
                let d_items = [
                    (ShelfViewMode::Mini, t.library_view_mini, 36.0),
                    (ShelfViewMode::Standard, t.library_view_standard, 54.0),
                    (ShelfViewMode::Maxi, t.library_view_maxi, 36.0),
                ];
                let mut d_x = density_rect.min.x;
                for (v, label, w) in d_items {
                    let is_active = library.shelf_view == v;
                    let btn_r = Rect::from_min_size(
                        Pos2::new(d_x, density_rect.min.y + 1.0),
                        Vec2::new(w, 24.0),
                    );
                    let resp = ui.interact(
                        btn_r,
                        ui.scope_id().with(format!("dense_{:?}", v)),
                        egui::Sense::click(),
                    );
                    if is_active {
                        ui.painter().rect_filled(
                            btn_r,
                            5.0,
                            if is_cozy {
                                CozyPalette::CARD_BG
                            } else {
                                theme.panel_bg_color().gamma_multiply(1.3)
                            },
                        );
                        ui.painter().text(
                            btn_r.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            if is_cozy {
                                CozyPalette::PRIMARY
                            } else {
                                theme.accent_color()
                            },
                        );
                    } else {
                        if resp.hovered() {
                            ui.painter().rect_filled(
                                btn_r,
                                5.0,
                                if is_cozy {
                                    CozyPalette::SURFACE_HIGH
                                } else {
                                    theme.panel_bg_color().gamma_multiply(1.1)
                                },
                            );
                        }
                        ui.painter().text(
                            btn_r.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(11.0),
                            if is_cozy {
                                CozyPalette::ON_SURFACE_VARIANT
                            } else {
                                theme.secondary_text_color()
                            },
                        );
                    }
                    if resp.clicked() && library.shelf_view != v {
                        library.shelf_view = v;
                        save_library = true;
                    }
                    d_x += w;
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let mode_rect = ui
                    .allocate_exact_size(Vec2::new(212.0, 26.0), egui::Sense::hover())
                    .0;
                ui.painter().rect_filled(
                    mode_rect,
                    6.0,
                    if is_cozy {
                        CozyPalette::SURFACE_CONTAINER
                    } else {
                        theme.panel_bg_color()
                    },
                );
                let m_items = [
                    (LibraryMode::Shelf, Icon::Grid, t.library_mode_shelf, 62.0),
                    (
                        LibraryMode::Folder,
                        Icon::Folder,
                        t.library_mode_folder,
                        62.0,
                    ),
                    (
                        LibraryMode::ReaderOnly,
                        Icon::Book,
                        t.library_mode_reader,
                        88.0,
                    ),
                ];
                let mut m_x = mode_rect.min.x;
                for (mode, icon, label, w) in m_items {
                    let is_active = library.mode == mode;
                    let btn_r = Rect::from_min_size(
                        Pos2::new(m_x, mode_rect.min.y + 1.0),
                        Vec2::new(w, 24.0),
                    );
                    let resp = ui.interact(
                        btn_r,
                        ui.scope_id().with(format!("mode_{:?}", mode)),
                        egui::Sense::click(),
                    );
                    let fg = if is_active {
                        ui.painter().rect_filled(
                            btn_r,
                            5.0,
                            if is_cozy {
                                CozyPalette::CARD_BG
                            } else {
                                theme.panel_bg_color().gamma_multiply(1.3)
                            },
                        );
                        if is_cozy {
                            CozyPalette::PRIMARY
                        } else {
                            theme.accent_color()
                        }
                    } else {
                        if resp.hovered() {
                            ui.painter().rect_filled(
                                btn_r,
                                5.0,
                                if is_cozy {
                                    CozyPalette::SURFACE_HIGH
                                } else {
                                    theme.panel_bg_color().gamma_multiply(1.1)
                                },
                            );
                        }
                        if is_cozy {
                            CozyPalette::ON_SURFACE_VARIANT
                        } else {
                            theme.secondary_text_color()
                        }
                    };
                    let icon_r = Rect::from_center_size(
                        Pos2::new(btn_r.min.x + 12.0, btn_r.center().y),
                        Vec2::splat(11.0),
                    );
                    paint_icon(ui.painter(), icon, icon_r, fg);
                    ui.painter().text(
                        Pos2::new(btn_r.min.x + 22.0, btn_r.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(11.0),
                        fg,
                    );
                    if resp.clicked() && library.mode != mode {
                        library.mode = mode;
                        save_library = true;
                    }
                    m_x += w;
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let sort_btn = IconButton::new(Icon::Sort, t.library_sort_recent).font_size(11.0);
                if ui.add(sort_btn).clicked() {}
            });
        });

    save_library
}
