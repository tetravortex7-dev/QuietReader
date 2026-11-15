use super::shelf_card::{ShelfCardLayout, render_shelf_card};
use super::shelf_cover::{render_add_book_card, shelf_cover};
use super::wood_shelf::paint_wood_shelf;
use super::*;
use crate::app::icons::{Icon, IconButton, IconLabel, icon_button};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::{
    LibraryEntry, LibraryReadingStatus, LibrarySortMode, ShelfViewMode, UiDesignMode,
};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, RichText, Stroke, StrokeKind, Vec2};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MINI_COVER_WIDTH: f32 = 56.0;
const MINI_COVER_HEIGHT: f32 = 84.0;
const MINI_ACTION_WIDTH: f32 = 120.0;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ShelfGridMetrics {
    pub cols: usize,
    pub gap: f32,
    pub card_outer_w: f32,
    pub card_outer_h: f32,
    pub card_inner_w: f32,
    pub card_inner_h: f32,
    pub cover_h: f32,
    pub title_size: f32,
    pub secondary_size: f32,
    pub status_size: f32,
}

pub(crate) fn compute_shelf_grid(avail_w: f32, view: ShelfViewMode) -> ShelfGridMetrics {
    let avail_w = avail_w.max(240.0);
    let gap = if avail_w < 540.0 { 10.0 } else { 14.0 };
    let (min_outer, max_cols, aspect, title_size, secondary_size, status_size) = match view {
        ShelfViewMode::Maxi => (
            if avail_w < 540.0 { 168.0 } else { 240.0 },
            4.0,
            1.34,
            15.0,
            11.5,
            11.0,
        ),
        ShelfViewMode::Mini | ShelfViewMode::Standard => (
            if avail_w < 540.0 { 144.0 } else { 166.0 },
            6.0,
            1.28,
            13.0,
            10.5,
            10.5,
        ),
    };

    let cols = ((avail_w + gap) / (min_outer + gap))
        .floor()
        .clamp(1.0, max_cols) as usize;
    let card_outer_w = ((avail_w - (cols - 1) as f32 * gap) / cols as f32)
        .floor()
        .max(132.0);
    let card_inner_w = (card_outer_w - 20.0).max(112.0);
    let cover_h = match view {
        ShelfViewMode::Maxi => (card_inner_w * aspect).clamp(215.0, 340.0).round(),
        _ => (card_inner_w * aspect).clamp(164.0, 198.0).round(),
    };
    let card_inner_h = cover_h + 98.0;
    let card_outer_h = card_inner_h + 20.0;

    ShelfGridMetrics {
        cols,
        gap,
        card_outer_w,
        card_outer_h,
        card_inner_w,
        card_inner_h,
        cover_h,
        title_size,
        secondary_size,
        status_size,
    }
}

impl LibraryViewState {
    pub(super) fn render_shelf(
        &mut self,
        ui: &mut egui::Ui,
        library: &mut LibraryStore,
        theme: &ReaderTheme,
        t: &Translations,
        actions: &mut LibraryActions,
    ) {
        let query = self.search_query.trim().to_lowercase();
        let skip_hero_dups = library.entries.len() > 6
            && library.design == UiDesignMode::Cozy
            && self.filter == LibraryFilter::All
            && query.is_empty()
            && library.sort_mode == LibrarySortMode::RecentlyOpened;

        let in_progress_paths: HashSet<PathBuf> = if skip_hero_dups {
            library
                .sorted_entries()
                .into_iter()
                .filter(|e| e.status == LibraryReadingStatus::Reading)
                .take(3)
                .map(|e| e.path.clone())
                .collect()
        } else {
            HashSet::new()
        };

        let entries: Vec<LibraryEntry> = library
            .sorted_entries_by(library.sort_mode)
            .into_iter()
            .filter(|entry| !in_progress_paths.contains(&entry.path))
            .filter(|entry| self.matches_filter(entry))
            .filter(|entry| self.matches_query(entry, &query))
            .cloned()
            .collect();

        if entries.is_empty() {
            if !query.is_empty() || self.filter != LibraryFilter::All {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(
                        RichText::new(t.search_nothing_found)
                            .size(14.0)
                            .color(theme.secondary_text_color()),
                    );
                });
            } else {
                self.render_shelf_empty(ui, theme, t, actions);
            }
            return;
        }

        let mut open_path = None;
        let mut remove_path = None;
        let mut status_change = None;
        let view = library.shelf_view;
        let design = library.design;
        let radius = card_radius(design);
        let shelf_width = ui.available_width().max(240.0);

        let mut keyboard_open = None;
        self.handle_shelf_keyboard(ui, &entries, view, shelf_width, &mut keyboard_open);
        if let Some(path) = keyboard_open {
            actions.open_path = Some(path);
        }

        let selected = self.keyboard_selection.clone();
        let mut scroll_to_selection = self.scroll_to_selection;
        self.scroll_to_selection = false;

        if view == ShelfViewMode::Mini {
            self.render_shelf_mini(
                ui,
                &entries,
                radius,
                theme,
                t,
                &mut open_path,
                &mut remove_path,
                &mut status_change,
                selected.as_deref(),
                &mut scroll_to_selection,
            );
        } else {
            self.render_shelf_grid(
                ui,
                &entries,
                view,
                design,
                radius,
                theme,
                t,
                &mut open_path,
                &mut remove_path,
                &mut status_change,
                selected.as_deref(),
                &mut scroll_to_selection,
                actions,
            );
        }

        if let Some(path) = open_path {
            actions.open_path = Some(path);
        }
        if let Some(path) = remove_path
            && library.remove(&path)
        {
            actions.save_library = true;
        }
        if let Some((path, status)) = status_change
            && library.set_status(&path, status)
        {
            actions.save_library = true;
        }
    }

    fn handle_shelf_keyboard(
        &mut self,
        ui: &egui::Ui,
        entries: &[LibraryEntry],
        view: ShelfViewMode,
        shelf_width: f32,
        open_path: &mut Option<PathBuf>,
    ) {
        if ui.ctx().text_edit_focused() {
            return;
        }
        let (left, right, up, down, home, end, enter, escape) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
                input.key_pressed(egui::Key::ArrowUp),
                input.key_pressed(egui::Key::ArrowDown),
                input.key_pressed(egui::Key::Home),
                input.key_pressed(egui::Key::End),
                input.key_pressed(egui::Key::Enter),
                input.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            if !self.search_query.is_empty() {
                self.search_query.clear();
            } else {
                self.keyboard_selection = None;
            }
            return;
        }
        if self
            .keyboard_selection
            .as_ref()
            .is_some_and(|path| !entries.iter().any(|entry| entry.path == *path))
        {
            self.keyboard_selection = None;
        }
        let current = self
            .keyboard_selection
            .as_ref()
            .and_then(|path| entries.iter().position(|entry| entry.path == *path));
        if enter {
            if !ui.ctx().memory(|memory| memory.focused().is_some())
                && let Some(index) = current
            {
                *open_path = Some(entries[index].path.clone());
            }
            return;
        }
        if !(left || right || up || down || home || end) {
            return;
        }
        let movement = if home {
            ShelfMove::Home
        } else if end {
            ShelfMove::End
        } else if up {
            ShelfMove::Up
        } else if down {
            ShelfMove::Down
        } else if left {
            ShelfMove::Left
        } else {
            ShelfMove::Right
        };
        let columns = if view == ShelfViewMode::Mini {
            1
        } else {
            compute_shelf_grid(shelf_width, view).cols
        };
        if let Some(target) = shelf_move_target(current, entries.len(), columns, movement) {
            self.keyboard_selection = Some(entries[target].path.clone());
            self.scroll_to_selection = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_shelf_grid(
        &mut self,
        ui: &mut egui::Ui,
        entries: &[LibraryEntry],
        view: ShelfViewMode,
        design: UiDesignMode,
        radius: f32,
        theme: &ReaderTheme,
        t: &Translations,
        open_path: &mut Option<PathBuf>,
        remove_path: &mut Option<PathBuf>,
        status_change: &mut Option<(PathBuf, LibraryReadingStatus)>,
        selected: Option<&Path>,
        scroll_to_selection: &mut bool,
        actions: &mut LibraryActions,
    ) {
        let is_cozy_palette = !theme.is_dark();
        let draw_wood = design == UiDesignMode::Cozy;
        let avail_w = ui.available_width();
        let grid = compute_shelf_grid(avail_w, view);
        let card_layout = ShelfCardLayout {
            inner_w: grid.card_inner_w,
            cover_h: grid.cover_h,
            inner_h: grid.card_inner_h,
            title_size: grid.title_size,
            secondary_size: grid.secondary_size,
            status_size: grid.status_size,
        };

        let mut chunks: Vec<Vec<&LibraryEntry>> = Vec::new();
        for c in entries.chunks(grid.cols) {
            chunks.push(c.iter().collect());
        }

        let card_stroke = Stroke::new(
            1.0,
            if is_cozy_palette {
                CozyPalette::OUTLINE.gamma_multiply(0.35)
            } else {
                theme.accent_color().gamma_multiply(0.28)
            },
        );

        for (row_idx, chunk) in chunks.iter().enumerate() {
            let is_last_row = row_idx + 1 == chunks.len();
            let row_min_x = ui.cursor().left();
            let row_w = avail_w;

            let row_resp = ui.allocate_ui_with_layout(
                Vec2::new(row_w, grid.card_outer_h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    for (col_idx, entry) in chunk.iter().enumerate() {
                        if col_idx > 0 {
                            ui.add_space(grid.gap);
                        }
                        self.cover_loader.queue(&entry.path);
                        let (slot_rect, _) = ui.allocate_exact_size(
                            Vec2::new(grid.card_outer_w, grid.card_outer_h),
                            egui::Sense::hover(),
                        );
                        let mut card_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(slot_rect)
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        egui::Frame::new()
                            .fill(card_fill(theme))
                            .stroke(card_stroke)
                            .corner_radius(radius)
                            .inner_margin(egui::Margin::same(10))
                            .show(&mut card_ui, |ui| {
                                ui.set_width(grid.card_inner_w);
                                ui.set_min_height(grid.card_inner_h);
                                render_shelf_card(
                                    ui,
                                    slot_rect,
                                    entry,
                                    card_layout,
                                    theme,
                                    t,
                                    &mut self.cover_loader,
                                    open_path,
                                    remove_path,
                                    status_change,
                                );
                            });

                        if selected == Some(entry.path.as_path()) {
                            let accent = if is_cozy_palette {
                                CozyPalette::PRIMARY
                            } else {
                                theme.accent_color()
                            };
                            ui.painter().rect_filled(
                                slot_rect.expand(1.0),
                                radius + 1.0,
                                accent.gamma_multiply(0.08),
                            );
                            ui.painter().rect_stroke(
                                slot_rect.expand(1.0),
                                radius + 1.0,
                                Stroke::new(2.5, accent),
                                StrokeKind::Inside,
                            );
                            if *scroll_to_selection {
                                ui.scroll_to_rect(slot_rect, Some(egui::Align::Center));
                                *scroll_to_selection = false;
                            }
                        }
                    }

                    if is_last_row && chunk.len() < grid.cols {
                        ui.add_space(grid.gap);
                        let (add_rect, _) = ui.allocate_exact_size(
                            Vec2::new(grid.card_outer_w, grid.card_outer_h),
                            egui::Sense::hover(),
                        );
                        let mut child_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(add_rect)
                                .layout(egui::Layout::top_down(egui::Align::Center)),
                        );
                        render_add_book_card(
                            &mut child_ui,
                            grid.card_outer_w,
                            grid.card_outer_h,
                            t,
                            is_cozy_palette,
                            theme,
                            actions,
                        );
                    }
                },
            );

            if draw_wood {
                let shelf_y = row_resp.response.rect.max.y + 1.0;
                paint_wood_shelf(ui.painter(), row_min_x, row_min_x + row_w, shelf_y, theme);
                ui.add_space(24.0);
            } else {
                ui.add_space(16.0);
            }
        }

        if chunks.is_empty() || chunks.last().is_some_and(|c| c.len() == grid.cols) {
            let row_min_x = ui.cursor().left();
            let row_w = avail_w;

            let row_resp = ui.allocate_ui_with_layout(
                Vec2::new(row_w, grid.card_outer_h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    let (add_rect, _) = ui.allocate_exact_size(
                        Vec2::new(grid.card_outer_w, grid.card_outer_h),
                        egui::Sense::hover(),
                    );
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(add_rect)
                            .layout(egui::Layout::top_down(egui::Align::Center)),
                    );
                    render_add_book_card(
                        &mut child_ui,
                        grid.card_outer_w,
                        grid.card_outer_h,
                        t,
                        is_cozy_palette,
                        theme,
                        actions,
                    );
                },
            );

            if draw_wood {
                let shelf_y = row_resp.response.rect.max.y + 1.0;
                paint_wood_shelf(ui.painter(), row_min_x, row_min_x + row_w, shelf_y, theme);
                ui.add_space(24.0);
            } else {
                ui.add_space(16.0);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_shelf_mini(
        &mut self,
        ui: &mut egui::Ui,
        entries: &[LibraryEntry],
        radius: f32,
        theme: &ReaderTheme,
        t: &Translations,
        open_path: &mut Option<PathBuf>,
        remove_path: &mut Option<PathBuf>,
        status_change: &mut Option<(PathBuf, LibraryReadingStatus)>,
        selected: Option<&Path>,
        scroll_to_selection: &mut bool,
    ) {
        let is_cozy = !theme.is_dark();
        for entry in entries {
            self.cover_loader.queue(&entry.path);
            ui.vertical(|ui| {
                let frame = egui::Frame::new()
                    .fill(card_fill(theme))
                    .stroke(Stroke::new(
                        1.0,
                        if is_cozy {
                            CozyPalette::OUTLINE.gamma_multiply(0.35)
                        } else {
                            theme.accent_color().gamma_multiply(0.28)
                        },
                    ))
                    .corner_radius(radius)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        let row_width = ui.available_width().max(240.0);
                        ui.set_width(row_width);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                            let cover_response = shelf_cover(
                                ui,
                                &entry.path,
                                &entry.title,
                                &entry.author,
                                Vec2::new(MINI_COVER_WIDTH, MINI_COVER_HEIGHT),
                                radius.min(8.0),
                                theme,
                                &mut self.cover_loader,
                                t.extra.default_book_title,
                            );
                            if cover_response.clicked() {
                                *open_path = Some(entry.path.clone());
                            }
                            ui.add_space(8.0);
                            let text_width =
                                (row_width - MINI_COVER_WIDTH - 40.0 - MINI_ACTION_WIDTH)
                                    .max(120.0);
                            ui.vertical(|ui| {
                                ui.set_width(text_width);
                                let file_name = entry
                                    .path
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or_default()
                                    .to_string();
                                let display_title = if entry.title.trim().is_empty() {
                                    file_name.as_str()
                                } else {
                                    entry.title.as_str()
                                };
                                let title_response = ui.add(
                                    egui::Label::new(
                                        RichText::new(display_title).strong().size(14.0),
                                    )
                                    .sense(egui::Sense::click())
                                    .truncate(),
                                );
                                if title_response.clicked() {
                                    *open_path = Some(entry.path.clone());
                                }
                                let secondary = if entry.author.trim().is_empty() {
                                    file_name
                                } else {
                                    entry.author.clone()
                                };
                                ui.label(
                                    RichText::new(secondary)
                                        .size(11.0)
                                        .color(theme.secondary_text_color()),
                                );
                                ui.horizontal_wrapped(|ui| {
                                    let (status, status_color) =
                                        status_badge(entry.status, theme, t);
                                    ui.label(RichText::new(status).size(10.5).color(status_color));
                                    if entry.total_units > 0 {
                                        let current =
                                            (entry.current_unit + 1).min(entry.total_units);
                                        let progress_text = t
                                            .library_progress
                                            .replace("{current}", &current.to_string())
                                            .replace("{total}", &entry.total_units.to_string());
                                        ui.label(
                                            RichText::new(progress_text)
                                                .size(10.5)
                                                .color(theme.secondary_text_color()),
                                        );
                                    }
                                });
                            });
                            ui.allocate_ui_with_layout(
                                Vec2::new(MINI_ACTION_WIDTH, MINI_COVER_HEIGHT),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    if icon_button(ui, Icon::OpenFile, "")
                                        .on_hover_text(t.library_open_book)
                                        .clicked()
                                    {
                                        *open_path = Some(entry.path.clone());
                                    }
                                    let (status_icon, status_label, next_status) =
                                        if entry.status == LibraryReadingStatus::Finished {
                                            (
                                                Icon::Book,
                                                t.library_mark_reading,
                                                LibraryReadingStatus::Reading,
                                            )
                                        } else {
                                            (
                                                Icon::Check,
                                                t.library_mark_finished,
                                                LibraryReadingStatus::Finished,
                                            )
                                        };
                                    if icon_button(ui, status_icon, "")
                                        .on_hover_text(status_label)
                                        .clicked()
                                    {
                                        *status_change = Some((entry.path.clone(), next_status));
                                    }
                                    if icon_button(ui, Icon::Trash, "")
                                        .on_hover_text(t.library_remove)
                                        .clicked()
                                    {
                                        *remove_path = Some(entry.path.clone());
                                    }
                                },
                            );
                        });
                    });
                if selected == Some(entry.path.as_path()) {
                    let accent = if is_cozy {
                        CozyPalette::PRIMARY
                    } else {
                        theme.accent_color()
                    };
                    ui.painter().rect_filled(
                        frame.response.rect,
                        radius + 1.0,
                        accent.gamma_multiply(0.08),
                    );
                    ui.painter().rect_stroke(
                        frame.response.rect,
                        radius + 1.0,
                        Stroke::new(2.5, accent),
                        StrokeKind::Inside,
                    );
                    if *scroll_to_selection {
                        ui.scroll_to_rect(frame.response.rect, Some(egui::Align::Center));
                        *scroll_to_selection = false;
                    }
                }
            });
            ui.add_space(4.0);
        }
    }

    fn render_shelf_empty(
        &self,
        ui: &mut egui::Ui,
        theme: &ReaderTheme,
        t: &Translations,
        actions: &mut LibraryActions,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(70.0);
            ui.add(
                IconLabel::new(Icon::Library, t.library_empty_title)
                    .size(26.0)
                    .strong()
                    .color(theme.accent_color()),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(t.library_empty_desc)
                    .size(13.0)
                    .color(theme.secondary_text_color()),
            );
            ui.add_space(16.0);
            if ui
                .add(IconButton::new(Icon::OpenFile, t.library_add_books))
                .clicked()
            {
                actions.open_file = true;
            }
        });
    }
}

fn status_badge(
    status: LibraryReadingStatus,
    theme: &ReaderTheme,
    t: &Translations,
) -> (&'static str, Color32) {
    match status {
        LibraryReadingStatus::Unread => (t.library_status_unread, theme.secondary_text_color()),
        LibraryReadingStatus::Reading => (t.library_status_reading, theme.accent_color()),
        LibraryReadingStatus::Finished => {
            (t.library_status_finished, Color32::from_rgb(72, 187, 120))
        }
    }
}

fn card_radius(design: UiDesignMode) -> f32 {
    match design {
        UiDesignMode::Cozy => 12.0,
        UiDesignMode::Studio => 6.0,
    }
}

fn card_fill(theme: &ReaderTheme) -> Color32 {
    if !theme.is_dark() {
        CozyPalette::CARD_BG
    } else {
        theme.panel_bg_color().gamma_multiply(1.18)
    }
}
