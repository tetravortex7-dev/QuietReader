use super::*;
use crate::core::library::ShelfViewMode;

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
                width: 250.0,
                cover_height: 360.0,
                min_height: 500.0,
                title_size: 17.0,
                secondary_size: 12.0,
                status_size: 12.0,
            },
            _ => Self {
                width: 190.0,
                cover_height: 198.0,
                min_height: 330.0,
                title_size: 14.0,
                secondary_size: 11.0,
                status_size: 11.0,
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
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(IconButton::new(Icon::Plus, t.library_add_books))
                .clicked()
            {
                actions.open_file = true;
            }
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text(t.library_search_hint)
                    .desired_width(230.0),
            );
            ui.add_space(6.0);
            for filter in [
                LibraryFilter::All,
                LibraryFilter::Reading,
                LibraryFilter::Finished,
            ] {
                if ui
                    .selectable_label(self.filter == filter, filter_label(filter, t))
                    .clicked()
                {
                    self.filter = filter;
                }
            }
            ui.add_space(6.0);
            ui.separator();
            for view in ShelfViewMode::ALL {
                let response = ui
                    .selectable_label(library.shelf_view == view, shelf_view_label(view, t))
                    .on_hover_text(t.library_view_tooltip);
                if response.clicked() && library.shelf_view != view {
                    library.shelf_view = view;
                    actions.save_library = true;
                }
            }
        });
        ui.add_space(10.0);

        let query = self.search_query.trim().to_lowercase();
        let entries: Vec<LibraryEntry> = library
            .sorted_entries()
            .into_iter()
            .filter(|entry| self.matches_filter(entry))
            .filter(|entry| self.matches_query(entry, &query))
            .cloned()
            .collect();

        if entries.is_empty() {
            if !query.is_empty() || self.filter != LibraryFilter::All {
                ui.vertical_centered(|ui| {
                    ui.add_space(70.0);
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
        let shelf_width = ui
            .available_width()
            .min(ui.ctx().content_rect().width() - 48.0)
            .max(240.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(shelf_width);
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
                    );
                }
            });

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
    ) {
        let metrics = CardMetrics::for_view(view);
        ui.horizontal_wrapped(|ui| {
            for entry in entries {
                self.cover_loader.queue(&entry.path);
                ui.allocate_ui_with_layout(
                    Vec2::new(metrics.width + 24.0, metrics.min_height + 24.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::Frame::new()
                            .fill(card_fill(theme))
                            .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.28)))
                            .corner_radius(radius)
                            .inner_margin(egui::Margin::same(12))
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
                ui.add_space(6.0);
            }
        });
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
    ) {
        for entry in entries {
            self.cover_loader.queue(&entry.path);
            ui.vertical(|ui| {
                egui::Frame::new()
                    .fill(card_fill(theme))
                    .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.28)))
                    .corner_radius(radius)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        let row_width = ui
                            .available_width()
                            .min(ui.ctx().content_rect().width() - 48.0)
                            .max(240.0);
                        ui.set_width(row_width);
                        ui.horizontal(|ui| {
                            let cover_response = shelf_cover(
                                ui,
                                &entry.path,
                                Vec2::new(MINI_COVER_WIDTH, MINI_COVER_HEIGHT),
                                radius.min(8.0),
                                theme,
                                &mut self.cover_loader,
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
                                    } else if entry.current_unit > 0 {
                                        let progress_text = t
                                            .library_progress
                                            .replace("{current}", &entry.current_unit.to_string())
                                            .replace("{total}", "?");
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
        radius: f32,
        open_path: &mut Option<PathBuf>,
        remove_path: &mut Option<PathBuf>,
        status_change: &mut Option<(PathBuf, LibraryReadingStatus)>,
    ) {
        let cover_response = shelf_cover(
            ui,
            &entry.path,
            Vec2::new(metrics.width, metrics.cover_height),
            radius,
            theme,
            &mut self.cover_loader,
        );
        if cover_response.clicked() {
            *open_path = Some(entry.path.clone());
        }

        ui.add_space(7.0);
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
                RichText::new(display_title)
                    .strong()
                    .size(metrics.title_size),
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
                .size(metrics.secondary_size)
                .color(theme.secondary_text_color()),
        );

        ui.add_space(5.0);
        let (status, status_color) = status_badge(entry.status, theme, t);
        ui.label(
            RichText::new(status)
                .size(metrics.status_size)
                .color(status_color),
        );

        ui.add_space(5.0);
        if entry.total_units > 0 {
            let current = (entry.current_unit + 1).min(entry.total_units);
            let fraction = current as f32 / entry.total_units as f32;
            let progress_text = t
                .library_progress
                .replace("{current}", &current.to_string())
                .replace("{total}", &entry.total_units.to_string());
            ui.add(
                egui::ProgressBar::new(fraction)
                    .desired_width(ui.available_width())
                    .text(progress_text),
            );
        } else if entry.current_unit > 0 {
            ui.label(
                RichText::new(
                    t.library_progress
                        .replace("{current}", &entry.current_unit.to_string())
                        .replace("{total}", "?"),
                )
                .size(10.0)
                .color(theme.secondary_text_color()),
            );
        }

        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            if ui.small_button(t.library_open_book).clicked() {
                *open_path = Some(entry.path.clone());
            }
            let (status_label, next_status) = if entry.status == LibraryReadingStatus::Finished {
                (t.library_mark_reading, LibraryReadingStatus::Reading)
            } else {
                (t.library_mark_finished, LibraryReadingStatus::Finished)
            };
            if ui.small_button(status_label).clicked() {
                *status_change = Some((entry.path.clone(), next_status));
            }
            if icon_button(ui, Icon::Trash, "")
                .on_hover_text(t.library_remove)
                .clicked()
            {
                *remove_path = Some(entry.path.clone());
            }
        });
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

fn shelf_cover(
    ui: &mut egui::Ui,
    path: &Path,
    size: Vec2,
    radius: f32,
    theme: &ReaderTheme,
    cover_loader: &mut CoverLoader,
) -> egui::Response {
    let (cover_rect, cover_response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect_filled(
        cover_rect,
        radius,
        theme.accent_color().gamma_multiply(0.08),
    );
    if let Some(texture) = cover_loader.texture(path) {
        let texture_size = texture.size_vec2();
        if texture_size.x > 0.0 && texture_size.y > 0.0 {
            let available = cover_rect.shrink(2.0);
            let scale =
                (available.width() / texture_size.x).min(available.height() / texture_size.y);
            let image_rect = egui::Rect::from_center_size(available.center(), texture_size * scale);
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    } else {
        paint_cover_placeholder(ui.painter(), cover_rect, theme, radius);
    }
    ui.painter().rect_stroke(
        cover_rect,
        radius,
        Stroke::new(1.0, theme.accent_color().gamma_multiply(0.3)),
        StrokeKind::Inside,
    );
    cover_response
}

fn shelf_view_label(view: ShelfViewMode, t: &Translations) -> &'static str {
    match view {
        ShelfViewMode::Mini => t.library_view_mini,
        ShelfViewMode::Standard => t.library_view_standard,
        ShelfViewMode::Maxi => t.library_view_maxi,
    }
}

fn filter_label(filter: LibraryFilter, t: &Translations) -> &'static str {
    match filter {
        LibraryFilter::All => t.library_filter_all,
        LibraryFilter::Reading => t.library_filter_reading,
        LibraryFilter::Finished => t.library_filter_finished,
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
        UiDesignMode::Cozy => 16.0,
        UiDesignMode::Studio => 6.0,
    }
}

fn card_fill(theme: &ReaderTheme) -> Color32 {
    if theme.is_dark() {
        theme.panel_bg_color().gamma_multiply(1.18)
    } else {
        theme.panel_bg_color().gamma_multiply(0.97)
    }
}

fn paint_cover_placeholder(
    painter: &egui::Painter,
    rect: egui::Rect,
    theme: &ReaderTheme,
    radius: f32,
) {
    let width = (rect.height() * 0.66).min(rect.width());
    let placeholder = egui::Rect::from_center_size(rect.center(), Vec2::new(width, rect.height()));
    let accent = theme.accent_color();
    painter.rect_filled(placeholder, radius, accent.gamma_multiply(0.16));
    let spine = egui::Rect::from_min_max(
        placeholder.min,
        egui::Pos2::new(
            placeholder.min.x + placeholder.width() * 0.15,
            placeholder.max.y,
        ),
    );
    painter.rect_filled(spine, radius, accent.gamma_multiply(0.26));
    let line_color = accent.gamma_multiply(0.32);
    for index in 0..4 {
        let y = placeholder.min.y + placeholder.height() * (0.22 + index as f32 * 0.09);
        painter.line_segment(
            [
                egui::Pos2::new(placeholder.min.x + placeholder.width() * 0.28, y),
                egui::Pos2::new(placeholder.max.x - placeholder.width() * 0.12, y),
            ],
            Stroke::new(1.0, line_color),
        );
    }
    super::super::icons::paint_icon(
        painter,
        Icon::Book,
        egui::Rect::from_center_size(
            placeholder.center(),
            Vec2::splat(placeholder.width() * 0.32),
        ),
        accent,
    );
    painter.rect_stroke(
        placeholder,
        radius,
        Stroke::new(1.0, accent.gamma_multiply(0.34)),
        StrokeKind::Inside,
    );
}
