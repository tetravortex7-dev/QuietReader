use super::shelf_cover::{render_add_book_card, shelf_cover};
use super::wood_shelf::paint_wood_shelf;
use super::*;
use crate::app::icons::{Icon, IconButton, IconLabel, icon_button, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::{LibraryEntry, LibraryReadingStatus, ShelfViewMode, UiDesignMode};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use std::path::{Path, PathBuf};

const MINI_COVER_WIDTH: f32 = 56.0;
const MINI_COVER_HEIGHT: f32 = 84.0;
const MINI_ACTION_WIDTH: f32 = 120.0;

#[derive(Clone, Copy)]
struct CardMetrics {
    width: f32,
    cover_height: f32,
    min_height: f32,
    title_size: f32,
    secondary_size: f32,
    status_size: f32,
}

impl CardMetrics {
    fn for_view(view: ShelfViewMode) -> Self {
        match view {
            ShelfViewMode::Maxi => Self {
                width: 220.0,
                cover_height: 310.0,
                min_height: 440.0,
                title_size: 15.0,
                secondary_size: 11.5,
                status_size: 11.0,
            },
            ShelfViewMode::Mini => Self {
                width: 130.0,
                cover_height: 180.0,
                min_height: 280.0,
                title_size: 12.5,
                secondary_size: 10.5,
                status_size: 10.0,
            },
            ShelfViewMode::Standard => Self {
                width: 148.0,
                cover_height: 215.0,
                min_height: 325.0,
                title_size: 13.0,
                secondary_size: 10.5,
                status_size: 10.5,
            },
        }
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
        let in_progress_paths: std::collections::HashSet<PathBuf> = if library.design
            == UiDesignMode::Cozy
            && self.filter == LibraryFilter::All
            && query.is_empty()
        {
            library
                .sorted_entries()
                .into_iter()
                .filter(|e| e.status == LibraryReadingStatus::Reading)
                .take(3)
                .map(|e| e.path.clone())
                .collect()
        } else {
            std::collections::HashSet::new()
        };

        let mut entries: Vec<LibraryEntry> = library
            .sorted_entries()
            .into_iter()
            .filter(|entry| !in_progress_paths.contains(&entry.path))
            .filter(|entry| self.matches_filter(entry))
            .filter(|entry| self.matches_query(entry, &query))
            .cloned()
            .collect();

        if library.design == UiDesignMode::Cozy
            && self.filter == LibraryFilter::All
            && query.is_empty()
        {
            const SHOWCASE_ORDER: [&str; 11] = [
                "Невидимые города",
                "Дюна",
                "Левая рука Тьмы",
                "Хранители",
                "Вымышленные истории",
                "Поэтика пространства",
                "Норвежский лес",
                "Обделённые",
                "Дзен и искусство ухода за мотоциклом",
                "Пиранези",
                "Полиция памяти",
            ];
            let mut shelf_books: Vec<LibraryEntry> = Vec::new();
            for title in SHOWCASE_ORDER {
                if let Some(pos) = entries.iter().position(|e| e.title == title) {
                    shelf_books.push(entries.remove(pos));
                }
            }
            let other_books = entries;
            entries = shelf_books;
            entries.extend(other_books);
        }

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
        let radius = card_radius(library.design);
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
        if let Some(path) = remove_path {
            if library.remove(&path) {
                actions.save_library = true;
            }
        }
        if let Some((path, status)) = status_change {
            if library.set_status(&path, status) {
                actions.save_library = true;
            }
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
            let step = CardMetrics::for_view(view).width + 20.0;
            ((shelf_width + 4.0) / step).floor().max(1.0) as usize
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
        let is_cozy = !theme.is_dark();
        let metrics = CardMetrics::for_view(view);
        let avail_w = ui.available_width();
        let item_w = metrics.width + 12.0;
        let cols = ((avail_w + 10.0) / item_w).floor().max(1.0) as usize;

        let is_showcase_layout = is_cozy
            && self.filter == LibraryFilter::All
            && self.search_query.trim().is_empty()
            && cols >= 6
            && entries.len() >= 11;

        let mut chunks: Vec<Vec<&LibraryEntry>> = Vec::new();
        let mut has_add_card_in_row = None;

        if is_showcase_layout {
            chunks.push(entries[0..6].iter().collect());
            chunks.push(entries[6..11].iter().collect());
            has_add_card_in_row = Some(1);
            let mut remaining = &entries[11..];
            while !remaining.is_empty() {
                let take = remaining.len().min(cols);
                chunks.push(remaining[..take].iter().collect());
                remaining = &remaining[take..];
            }
        } else {
            for c in entries.chunks(cols) {
                chunks.push(c.iter().collect());
            }
        }

        for (row_idx, chunk) in chunks.iter().enumerate() {
            let is_last_row = row_idx + 1 == chunks.len();
            let row_min_x = ui.cursor().left();
            let row_w = ui.available_width();
            let row_h = metrics.min_height + 12.0;

            let row_resp = ui.allocate_ui_with_layout(
                Vec2::new(row_w, row_h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    for entry in chunk {
                        self.cover_loader.queue(&entry.path);
                        let inner = ui.allocate_ui_with_layout(
                            Vec2::new(metrics.width + 12.0, metrics.min_height + 12.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::Frame::new()
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
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        ui.set_width(metrics.width);
                                        ui.set_min_height(metrics.min_height);
                                        self.render_shelf_card(
                                            ui,
                                            entry,
                                            metrics,
                                            theme,
                                            t,
                                            radius,
                                            open_path,
                                            remove_path,
                                            status_change,
                                        );
                                    });
                            },
                        );

                        let card_rect = inner.response.rect;

                        if selected == Some(entry.path.as_path()) {
                            let accent = if is_cozy {
                                CozyPalette::PRIMARY
                            } else {
                                theme.accent_color()
                            };
                            ui.painter().rect_filled(
                                card_rect.expand(1.0),
                                radius + 1.0,
                                accent.gamma_multiply(0.08),
                            );
                            ui.painter().rect_stroke(
                                card_rect.expand(1.0),
                                radius + 1.0,
                                Stroke::new(2.5, accent),
                                StrokeKind::Inside,
                            );
                            if *scroll_to_selection {
                                ui.scroll_to_rect(card_rect, Some(egui::Align::Center));
                                *scroll_to_selection = false;
                            }
                        }
                        ui.add_space(8.0);
                    }

                    let should_render_add = has_add_card_in_row == Some(row_idx)
                        || (!is_showcase_layout && is_last_row && chunk.len() < cols);
                    if should_render_add {
                        let (add_rect, _add_resp) = ui.allocate_exact_size(
                            Vec2::new(metrics.width + 12.0, metrics.min_height + 12.0),
                            egui::Sense::hover(),
                        );
                        let mut child_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(add_rect)
                                .layout(egui::Layout::top_down(egui::Align::Center)),
                        );
                        render_add_book_card(
                            &mut child_ui,
                            metrics.width,
                            metrics.min_height,
                            t,
                            is_cozy,
                            theme,
                            actions,
                        );
                    }
                },
            );

            if is_cozy {
                let shelf_y = row_resp.response.rect.max.y;
                paint_wood_shelf(ui.painter(), row_min_x, row_min_x + row_w, shelf_y);
            }
            ui.add_space(26.0);
        }

        if !is_showcase_layout
            && (chunks.is_empty() || (chunks.last().map(|c| c.len() == cols).unwrap_or(false)))
        {
            let row_min_x = ui.cursor().left();
            let row_w = ui.available_width();
            let row_h = metrics.min_height + 12.0;

            let row_resp = ui.allocate_ui_with_layout(
                Vec2::new(row_w, row_h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    let (add_rect, _) = ui.allocate_exact_size(
                        Vec2::new(metrics.width + 12.0, metrics.min_height + 12.0),
                        egui::Sense::hover(),
                    );
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(add_rect)
                            .layout(egui::Layout::top_down(egui::Align::Center)),
                    );
                    render_add_book_card(
                        &mut child_ui,
                        metrics.width,
                        metrics.min_height,
                        t,
                        is_cozy,
                        theme,
                        actions,
                    );
                },
            );

            if is_cozy {
                let shelf_y = row_resp.response.rect.max.y;
                paint_wood_shelf(ui.painter(), row_min_x, row_min_x + row_w, shelf_y);
            }
            ui.add_space(26.0);
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

    #[allow(clippy::too_many_arguments)]
    fn render_shelf_card(
        &mut self,
        ui: &mut egui::Ui,
        entry: &LibraryEntry,
        metrics: CardMetrics,
        theme: &ReaderTheme,
        t: &Translations,
        _radius: f32,
        open_path: &mut Option<PathBuf>,
        _remove_path: &mut Option<PathBuf>,
        _status_change: &mut Option<(PathBuf, LibraryReadingStatus)>,
    ) {
        let is_cozy = !theme.is_dark();
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

        let cover_response = shelf_cover(
            ui,
            &entry.path,
            display_title,
            &entry.author,
            Vec2::new(metrics.width, metrics.cover_height),
            8.0,
            theme,
            &mut self.cover_loader,
            t.extra.default_book_title,
        );
        if cover_response.clicked() {
            *open_path = Some(entry.path.clone());
        }

        ui.add_space(8.0);
        let title_response = ui.add(
            egui::Label::new(
                RichText::new(display_title)
                    .strong()
                    .size(metrics.title_size)
                    .color(if is_cozy {
                        CozyPalette::ON_SURFACE
                    } else {
                        theme.text_color()
                    }),
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
        ui.add(
            egui::Label::new(RichText::new(secondary).size(metrics.secondary_size).color(
                if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                },
            ))
            .truncate(),
        );

        ui.add_space(6.0);
        let percent = entry.progress_percent().round() as u32;
        let is_finished = entry.status == LibraryReadingStatus::Finished || percent == 100;
        let fraction = percent as f32 / 100.0;
        let status_color = if is_finished {
            if is_cozy {
                CozyPalette::TERTIARY
            } else {
                Color32::from_rgb(72, 187, 120)
            }
        } else if is_cozy {
            CozyPalette::PRIMARY
        } else {
            theme.accent_color()
        };

        let (prog_r, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 14.0), egui::Sense::hover());
        let label = if is_finished {
            t.library_status_finished
        } else {
            t.library_progress_label
        };
        let font_id = egui::FontId::proportional(metrics.status_size);
        ui.painter().text(
            prog_r.left_center(),
            egui::Align2::LEFT_CENTER,
            label,
            font_id.clone(),
            if is_cozy {
                CozyPalette::ON_SURFACE_VARIANT
            } else {
                theme.secondary_text_color()
            },
        );
        ui.painter().text(
            prog_r.right_center(),
            egui::Align2::RIGHT_CENTER,
            format!("{percent}%"),
            font_id,
            status_color,
        );

        ui.add_space(3.0);
        let bar_h = 4.0;
        let (bar_rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), bar_h), egui::Sense::hover());
        ui.painter().rect_filled(
            bar_rect,
            2.0,
            if is_cozy {
                CozyPalette::SURFACE_CONTAINER
            } else {
                theme.panel_bg_color().gamma_multiply(1.3)
            },
        );
        if fraction > 0.0 {
            let fill_w = bar_rect.width() * fraction;
            ui.painter().rect_filled(
                Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_h)),
                2.0,
                status_color,
            );
        }

        ui.add_space(8.0);
        let (read_btn_rect, read_btn_resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), egui::Sense::click());
        let is_h = read_btn_resp.hovered();

        let is_primary_btn = !is_finished
            && (entry.title == "Дюна" || entry.status == LibraryReadingStatus::Reading);
        let btn_fill = if is_primary_btn {
            if is_h {
                CozyPalette::PRIMARY_CONTAINER
            } else {
                CozyPalette::PRIMARY
            }
        } else if is_h {
            CozyPalette::SURFACE_HIGH
        } else {
            CozyPalette::SURFACE_CONTAINER
        };

        let btn_text_color = if is_primary_btn {
            Color32::WHITE
        } else {
            CozyPalette::ON_SURFACE
        };

        ui.painter().rect_filled(read_btn_rect, 6.0, btn_fill);
        let play_pos = Pos2::new(read_btn_rect.center().x - 24.0, read_btn_rect.center().y);
        paint_icon(
            ui.painter(),
            Icon::Play,
            Rect::from_center_size(play_pos, Vec2::splat(11.0)),
            btn_text_color,
        );
        ui.painter().text(
            Pos2::new(read_btn_rect.center().x + 2.0, read_btn_rect.center().y),
            egui::Align2::CENTER_CENTER,
            t.library_read_action,
            egui::FontId::proportional(11.5),
            btn_text_color,
        );

        if read_btn_resp.clicked() {
            *open_path = Some(entry.path.clone());
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
