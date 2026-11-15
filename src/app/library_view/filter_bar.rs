use super::{LibraryFilter, LibraryViewState};
use crate::app::icons::{Icon, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::{
    LibraryMode, LibraryReadingStatus, LibrarySortMode, LibraryStore, ShelfViewMode,
};
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

    let filters = [
        (
            LibraryFilter::All,
            format!("{} ({})", t.library_filter_all, all_count),
        ),
        (
            LibraryFilter::Reading,
            format!("{} ({})", t.library_filter_reading, reading_count),
        ),
        (
            LibraryFilter::Finished,
            format!("{} ({})", t.library_filter_finished, finished_count),
        ),
    ];

    egui::Frame::new()
        .fill(bar_fill)
        .stroke(border_stroke)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            let total_w = ui.available_width();
            ui.set_width(total_w);

            let filter_widths: Vec<f32> = filters
                .iter()
                .map(|(_, label)| measure_text_w(ui, label, 11.5) + 18.0)
                .collect();
            let filters_total_w: f32 =
                filter_widths.iter().sum::<f32>() + (filters.len() as f32 - 1.0) * 4.0;

            let d_items = [
                (ShelfViewMode::Mini, t.library_view_mini),
                (ShelfViewMode::Standard, t.library_view_standard),
                (ShelfViewMode::Maxi, t.library_view_maxi),
            ];
            let d_widths: Vec<f32> = d_items
                .iter()
                .map(|(_, l)| (measure_text_w(ui, l, 11.0) + 16.0).max(40.0))
                .collect();
            let density_total_w: f32 = d_widths.iter().sum();

            let m_items = [
                (LibraryMode::Shelf, Icon::Grid, t.library_mode_shelf),
                (LibraryMode::Folder, Icon::Folder, t.library_mode_folder),
                (LibraryMode::ReaderOnly, Icon::Book, t.library_mode_reader),
            ];
            let m_widths: Vec<f32> = m_items
                .iter()
                .map(|(_, _, l)| (measure_text_w(ui, l, 11.0) + 30.0).max(60.0))
                .collect();
            let mode_total_w: f32 = m_widths.iter().sum();

            let sort_text = sort_mode_label(library.sort_mode, t);
            let sort_w = (measure_text_w(ui, sort_text, 11.0) + 42.0).max(96.0);

            let controls_w = density_total_w + mode_total_w + sort_w + 36.0;
            let min_search_w = 180.0;
            let single_row = total_w >= filters_total_w + min_search_w + controls_w + 24.0;

            if single_row {
                ui.horizontal(|ui| {
                    render_filter_pills(ui, &filters, &filter_widths, state, theme, is_cozy);
                    ui.add_space(8.0);
                    let search_w =
                        (ui.available_width() - controls_w - 10.0).clamp(min_search_w, 340.0);
                    render_search_field(
                        ui,
                        search_w,
                        &mut state.search_query,
                        border_stroke,
                        theme,
                        t,
                        is_cozy,
                    );
                    let rem = (ui.available_width() - controls_w).max(0.0);
                    if rem > 0.0 {
                        ui.add_space(rem);
                    }
                    render_right_controls(
                        ui,
                        &d_items,
                        &d_widths,
                        density_total_w,
                        &m_items,
                        &m_widths,
                        mode_total_w,
                        sort_w,
                        library,
                        theme,
                        t,
                        is_cozy,
                        &mut save_library,
                    );
                });
            } else {
                if total_w >= filters_total_w + 150.0 {
                    ui.horizontal(|ui| {
                        render_filter_pills(ui, &filters, &filter_widths, state, theme, is_cozy);
                        let avail = ui.available_width();
                        let search_w = avail.clamp(140.0, 360.0);
                        if avail > search_w {
                            ui.add_space(avail - search_w);
                        }
                        render_search_field(
                            ui,
                            search_w,
                            &mut state.search_query,
                            border_stroke,
                            theme,
                            t,
                            is_cozy,
                        );
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        render_filter_pills(ui, &filters, &filter_widths, state, theme, is_cozy);
                    });
                    ui.add_space(6.0);
                    render_search_field(
                        ui,
                        total_w.min(360.0),
                        &mut state.search_query,
                        border_stroke,
                        theme,
                        t,
                        is_cozy,
                    );
                }

                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    render_right_controls(
                        ui,
                        &d_items,
                        &d_widths,
                        density_total_w,
                        &m_items,
                        &m_widths,
                        mode_total_w,
                        sort_w,
                        library,
                        theme,
                        t,
                        is_cozy,
                        &mut save_library,
                    );
                });
            }
        });

    save_library
}

fn measure_text_w(ui: &mut egui::Ui, text: &str, font_size: f32) -> f32 {
    ui.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(font_size),
            Color32::WHITE,
        )
        .size()
        .x
    })
}

fn render_filter_pills(
    ui: &mut egui::Ui,
    filters: &[(LibraryFilter, String); 3],
    widths: &[f32],
    state: &mut LibraryViewState,
    theme: &ReaderTheme,
    is_cozy: bool,
) {
    for (idx, (f, full_label)) in filters.iter().enumerate() {
        let is_active = state.filter == *f;
        let btn_w = widths[idx];
        let (btn_rect, btn_resp) =
            ui.allocate_exact_size(Vec2::new(btn_w, 26.0), egui::Sense::click());
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
                full_label,
                egui::FontId::proportional(11.5),
                Color32::WHITE,
            );
        } else {
            let fill = if btn_resp.hovered() {
                if is_cozy {
                    CozyPalette::SURFACE_HIGH
                } else {
                    theme.panel_bg_color().gamma_multiply(1.25)
                }
            } else if is_cozy {
                CozyPalette::SURFACE_CONTAINER
            } else {
                theme.panel_bg_color().gamma_multiply(1.1)
            };
            ui.painter().rect_filled(btn_rect, 6.0, fill);
            ui.painter().text(
                btn_rect.center(),
                egui::Align2::CENTER_CENTER,
                full_label,
                egui::FontId::proportional(11.5),
                if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                },
            );
        }
        if btn_resp.clicked() {
            state.filter = *f;
        }
        if idx + 1 < filters.len() {
            ui.add_space(4.0);
        }
    }
}

fn render_search_field(
    ui: &mut egui::Ui,
    search_width: f32,
    search_query: &mut String,
    border_stroke: Stroke,
    theme: &ReaderTheme,
    t: &Translations,
    is_cozy: bool,
) {
    let (search_rect, _) =
        ui.allocate_exact_size(Vec2::new(search_width, 28.0), egui::Sense::hover());
    ui.painter().rect_filled(
        search_rect,
        6.0,
        if is_cozy {
            CozyPalette::CARD_BG
        } else {
            theme.panel_bg_color().gamma_multiply(1.18)
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

    let has_query = !search_query.is_empty();
    let right_pad = if has_query { 24.0 } else { 6.0 };

    let mut text_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                Pos2::new(search_rect.min.x + 26.0, search_rect.min.y + 2.0),
                Pos2::new(search_rect.max.x - right_pad, search_rect.max.y - 2.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    text_ui.add(
        egui::TextEdit::singleline(search_query)
            .hint_text(t.library_search_full_hint)
            .frame(egui::Frame::NONE)
            .desired_width((search_width - 26.0 - right_pad).max(40.0)),
    );

    if has_query {
        let clear_rect = Rect::from_center_size(
            Pos2::new(search_rect.max.x - 13.0, search_rect.center().y),
            Vec2::splat(18.0),
        );
        let clear_resp = ui.interact(
            clear_rect,
            ui.scope_id().with("library_search_clear"),
            egui::Sense::click(),
        );
        if clear_resp.hovered() {
            ui.painter().rect_filled(
                clear_rect,
                4.0,
                if is_cozy {
                    CozyPalette::SURFACE_HIGH
                } else {
                    theme.panel_bg_color().gamma_multiply(1.35)
                },
            );
        }
        paint_icon(
            ui.painter(),
            Icon::Cross,
            Rect::from_center_size(clear_rect.center(), Vec2::splat(10.0)),
            if is_cozy {
                CozyPalette::ON_SURFACE_VARIANT
            } else {
                theme.secondary_text_color()
            },
        );
        if clear_resp.on_hover_text(t.extra.clear_search).clicked() {
            search_query.clear();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_right_controls(
    ui: &mut egui::Ui,
    d_items: &[(ShelfViewMode, &'static str); 3],
    d_widths: &[f32],
    density_total_w: f32,
    m_items: &[(LibraryMode, Icon, &'static str); 3],
    m_widths: &[f32],
    mode_total_w: f32,
    sort_w: f32,
    library: &mut LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    is_cozy: bool,
    save_library: &mut bool,
) {
    let segment_bg = if is_cozy {
        CozyPalette::SURFACE_CONTAINER
    } else {
        theme.panel_bg_color().gamma_multiply(1.1)
    };
    let active_pill_bg = if is_cozy {
        CozyPalette::CARD_BG
    } else {
        theme.panel_bg_color().gamma_multiply(1.35)
    };
    let hover_pill_bg = if is_cozy {
        CozyPalette::SURFACE_HIGH
    } else {
        theme.panel_bg_color().gamma_multiply(1.22)
    };
    let active_fg = if is_cozy {
        CozyPalette::PRIMARY
    } else {
        theme.accent_color()
    };
    let inactive_fg = if is_cozy {
        CozyPalette::ON_SURFACE_VARIANT
    } else {
        theme.secondary_text_color()
    };

    let density_rect = ui
        .allocate_exact_size(Vec2::new(density_total_w, 26.0), egui::Sense::hover())
        .0;
    ui.painter().rect_filled(density_rect, 6.0, segment_bg);
    let mut d_x = density_rect.min.x;
    for (idx, (v, label)) in d_items.iter().enumerate() {
        let w = d_widths[idx];
        let is_active = library.shelf_view == *v;
        let btn_r =
            Rect::from_min_size(Pos2::new(d_x, density_rect.min.y + 1.0), Vec2::new(w, 24.0));
        let resp = ui.interact(
            btn_r,
            ui.scope_id().with(("dense", idx)),
            egui::Sense::click(),
        );
        if is_active {
            ui.painter().rect_filled(btn_r, 5.0, active_pill_bg);
        } else if resp.hovered() {
            ui.painter().rect_filled(btn_r, 5.0, hover_pill_bg);
        }
        ui.painter().text(
            btn_r.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(11.0),
            if is_active { active_fg } else { inactive_fg },
        );
        if resp.clicked() && library.shelf_view != *v {
            library.shelf_view = *v;
            *save_library = true;
        }
        d_x += w;
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    let mode_rect = ui
        .allocate_exact_size(Vec2::new(mode_total_w, 26.0), egui::Sense::hover())
        .0;
    ui.painter().rect_filled(mode_rect, 6.0, segment_bg);
    let mut m_x = mode_rect.min.x;
    for (idx, (mode, icon, label)) in m_items.iter().enumerate() {
        let w = m_widths[idx];
        let is_active = library.mode == *mode;
        let btn_r = Rect::from_min_size(Pos2::new(m_x, mode_rect.min.y + 1.0), Vec2::new(w, 24.0));
        let resp = ui.interact(
            btn_r,
            ui.scope_id().with(("mode", idx)),
            egui::Sense::click(),
        );
        if is_active {
            ui.painter().rect_filled(btn_r, 5.0, active_pill_bg);
        } else if resp.hovered() {
            ui.painter().rect_filled(btn_r, 5.0, hover_pill_bg);
        }
        let fg = if is_active { active_fg } else { inactive_fg };
        let icon_r = Rect::from_center_size(
            Pos2::new(btn_r.min.x + 12.0, btn_r.center().y),
            Vec2::splat(11.0),
        );
        paint_icon(ui.painter(), *icon, icon_r, fg);
        ui.painter().text(
            Pos2::new(btn_r.min.x + 22.0, btn_r.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.0),
            fg,
        );
        if resp.clicked() && library.mode != *mode {
            library.mode = *mode;
            *save_library = true;
        }
        m_x += w;
    }

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    let (sort_rect, sort_resp) =
        ui.allocate_exact_size(Vec2::new(sort_w, 26.0), egui::Sense::click());
    let sort_fill = if sort_resp.hovered() {
        hover_pill_bg
    } else {
        segment_bg
    };
    ui.painter().rect_filled(sort_rect, 6.0, sort_fill);
    paint_icon(
        ui.painter(),
        Icon::Sort,
        Rect::from_center_size(
            Pos2::new(sort_rect.min.x + 13.0, sort_rect.center().y),
            Vec2::splat(11.5),
        ),
        inactive_fg,
    );
    ui.painter().text(
        Pos2::new(sort_rect.min.x + 24.0, sort_rect.center().y),
        egui::Align2::LEFT_CENTER,
        sort_mode_label(library.sort_mode, t),
        egui::FontId::proportional(11.0),
        if is_cozy {
            CozyPalette::ON_SURFACE
        } else {
            theme.text_color()
        },
    );
    paint_icon(
        ui.painter(),
        Icon::ChevronDown,
        Rect::from_center_size(
            Pos2::new(sort_rect.max.x - 11.0, sort_rect.center().y),
            Vec2::splat(10.0),
        ),
        inactive_fg,
    );
    let sort_resp = sort_resp.on_hover_text(t.extra.library_sort_tooltip);

    let sort_options = [
        (LibrarySortMode::RecentlyOpened, t.library_sort_recent),
        (LibrarySortMode::Title, t.extra.library_sort_title),
        (LibrarySortMode::Author, t.extra.library_sort_author),
        (LibrarySortMode::Progress, t.extra.library_sort_progress),
        (LibrarySortMode::RecentlyAdded, t.extra.library_sort_added),
    ];

    egui::Popup::menu(&sort_resp).show(|ui| {
        ui.set_min_width(160.0);
        for (mode, label) in sort_options {
            let selected = library.sort_mode == mode;
            if ui.selectable_label(selected, label).clicked() {
                if library.sort_mode != mode {
                    library.sort_mode = mode;
                    *save_library = true;
                }
                ui.close();
            }
        }
    });
}

pub(crate) fn sort_mode_label(mode: LibrarySortMode, t: &Translations) -> &'static str {
    match mode {
        LibrarySortMode::RecentlyOpened => t.library_sort_recent,
        LibrarySortMode::Title => t.extra.library_sort_title,
        LibrarySortMode::Author => t.extra.library_sort_author,
        LibrarySortMode::Progress => t.extra.library_sort_progress,
        LibrarySortMode::RecentlyAdded => t.extra.library_sort_added,
    }
}
