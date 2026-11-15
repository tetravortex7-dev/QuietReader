use super::OpdsBrowserState;
use super::palette::{OpdsPalette, pill_badge, pill_button};
use crate::app::icons::{Icon, IconButton, paint_icon};
use crate::i18n::{Translations, fill, format_opds_auth_error};
use crate::opds::{OpdsBreadcrumb, OpdsEntry};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::path::PathBuf;

impl OpdsBrowserState {
    pub(super) fn render_feed_content(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
    ) {
        let t = settings.ui_language.t();
        let outer_pad = if ui.available_width() < 560.0 { 10 } else { 18 };
        egui::Frame::new()
            .fill(palette.canvas_bg)
            .inner_margin(egui::Margin::same(outer_pad))
            .show(ui, |ui| {
                let feed_w = ui.available_width().max(180.0);
                let feed_h = ui.available_height().max(80.0);
                ui.set_width(feed_w);
                ui.set_height(feed_h);

                self.render_feed_header_bar(ui, t, settings, palette);
                ui.add_space(10.0);
                self.render_search_and_filter_card(ui, t, settings, palette, feed_w);
                ui.add_space(12.0);

                let scroll_h = ui.available_height().max(40.0);
                ScrollArea::vertical()
                    .id_salt("opds_feed_entries_scroll")
                    .max_height(scroll_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_feed_body(ctx, ui, t, settings, palette, on_open_file);
                    });
            });
    }

    fn render_feed_header_bar(
        &mut self,
        ui: &mut egui::Ui,
        t: &Translations,
        settings: &AppSettings,
        palette: &OpdsPalette,
    ) {
        ui.horizontal_wrapped(|ui| {
            let can_back = !self.history.is_empty();
            if self.nav_icon_square(ui, Icon::ArrowLeft, can_back, t.opds_back, palette) {
                self.go_back(settings);
            }
            let can_home = self.breadcrumbs.len() > 1;
            if self.nav_icon_square(ui, Icon::Home, can_home, t.opds_root, palette) {
                self.navigate_to_crumb(0, settings);
            }

            if !self.breadcrumbs.is_empty() {
                ui.add_space(4.0);
                let mut clicked_crumb = None;
                let total_crumbs = self.breadcrumbs.len();
                for (idx, crumb) in self.breadcrumbs.iter().enumerate() {
                    if idx > 0 {
                        let (ar, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                        paint_icon(
                            ui.painter(),
                            Icon::ArrowRight,
                            Rect::from_center_size(ar.center(), Vec2::splat(10.0)),
                            palette.on_surface_variant.gamma_multiply(0.65),
                        );
                    }
                    let is_last = idx == total_crumbs - 1;
                    let text_col = if is_last {
                        palette.primary
                    } else {
                        palette.on_surface_variant
                    };
                    let mut text = RichText::new(&crumb.title).size(12.5).color(text_col);
                    if is_last {
                        text = text.strong();
                    }
                    if idx == 0 {
                        let (gr, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                        paint_icon(
                            ui.painter(),
                            Icon::Globe,
                            Rect::from_center_size(gr.center(), Vec2::splat(12.0)),
                            text_col,
                        );
                    }
                    if is_last || crumb.url.is_empty() {
                        ui.label(text);
                    } else if ui.add(egui::Button::new(text).frame(false)).clicked() {
                        clicked_crumb = Some(idx);
                    }
                }
                if let Some(idx) = clicked_crumb {
                    self.navigate_to_crumb(idx, settings);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_refresh = self.current_url.is_some();
                if self.nav_icon_square(ui, Icon::Refresh, can_refresh, t.opds_refresh, palette) {
                    if let Some(cur) = self.current_url.clone() {
                        self.fetch_url(&cur, settings);
                    }
                }

                if self.active_catalog_id.is_some() || self.current_feed.is_some() {
                    let (dot_col, status_txt, status_bg, status_fg) = if self.is_loading {
                        (
                            palette.secondary,
                            t.extra.opds_status_loading,
                            palette.secondary_soft,
                            palette.secondary,
                        )
                    } else if self.error_message.is_some() {
                        (
                            palette.error_fg,
                            t.extra.opds_status_error,
                            palette.error_bg,
                            palette.error_fg,
                        )
                    } else {
                        (
                            palette.tertiary_sage,
                            t.extra.opds_status_active,
                            palette.tertiary_sage_bg,
                            palette.tertiary_sage,
                        )
                    };

                    egui::Frame::new()
                        .fill(status_bg)
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if self.is_loading {
                                    ui.add(egui::Spinner::new().size(11.0).color(dot_col));
                                } else {
                                    let (dr, _) =
                                        ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                                    ui.painter().circle_filled(dr.center(), 3.2, dot_col);
                                }
                                ui.label(
                                    RichText::new(status_txt)
                                        .strong()
                                        .size(11.0)
                                        .color(status_fg),
                                );
                            });
                        });

                    pill_badge(
                        ui,
                        "OPDS",
                        palette.surface_container,
                        palette.on_surface_variant,
                        10.5,
                        Some(Stroke::new(1.0, palette.card_border)),
                    );
                }
            });
        });
    }

    fn nav_icon_square(
        &self,
        ui: &mut egui::Ui,
        icon: Icon,
        enabled: bool,
        tooltip: &str,
        palette: &OpdsPalette,
    ) -> bool {
        let sense = if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(28.0), sense);
        let bg = if enabled && resp.hovered() {
            palette.surface_high
        } else {
            palette.card_bg
        };
        ui.painter().rect_filled(rect, 8.0, bg);
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(1.0, palette.card_border),
            StrokeKind::Inside,
        );
        let fg = if enabled {
            palette.on_surface
        } else {
            palette.on_surface_variant.gamma_multiply(0.38)
        };
        paint_icon(
            ui.painter(),
            icon,
            Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
            fg,
        );
        resp.on_hover_text(tooltip).clicked() && enabled
    }

    fn render_search_and_filter_card(
        &mut self,
        ui: &mut egui::Ui,
        t: &Translations,
        settings: &AppSettings,
        palette: &OpdsPalette,
        feed_w: f32,
    ) {
        let formats = self
            .current_feed
            .as_ref()
            .map(super::collect_format_filters)
            .unwrap_or_default();

        egui::Frame::new()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0, palette.card_border))
            .corner_radius(12.0)
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width((feed_w - 24.0).max(140.0));

                let has_remote_search = self
                    .current_feed
                    .as_ref()
                    .is_some_and(|f| f.search_template.is_some() || f.search_descriptor_url.is_some());
                let compact_bar = feed_w < 460.0;
                let btn_reserve = if compact_bar { 42.0 } else { 96.0 };
                let clear_reserve = if self.search_query.is_empty() {
                    0.0
                } else {
                    28.0
                };
                let input_w = (ui.available_width() - btn_reserve - clear_reserve - 44.0).max(90.0);

                let mut trigger_search = false;
                ui.horizontal(|ui| {
                    let (sr, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                    paint_icon(
                        ui.painter(),
                        Icon::Search,
                        Rect::from_center_size(sr.center(), Vec2::splat(14.0)),
                        palette.on_surface_variant,
                    );

                    let hint = if feed_w >= 620.0 {
                        t.extra.opds_opensearch_placeholder
                    } else {
                        t.opds_search_placeholder
                    };
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(hint)
                            .desired_width(input_w),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        trigger_search = true;
                    }

                    if !self.search_query.is_empty()
                        && self.nav_icon_square(
                            ui,
                            Icon::Cross,
                            true,
                            t.extra.clear_search,
                            palette,
                        )
                    {
                        self.search_query.clear();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = if compact_bar { "" } else { t.opds_search_btn };
                        if ui
                            .add(
                                IconButton::new(Icon::Search, btn_text)
                                    .font_size(12.0)
                                    .fill(palette.primary)
                                    .color(palette.on_primary)
                                    .min_size(Vec2::new(0.0, 28.0)),
                            )
                            .clicked()
                        {
                            trigger_search = true;
                        }
                    });
                });

                if trigger_search && !self.search_query.trim().is_empty() && has_remote_search {
                    self.execute_remote_search(t, settings);
                }

                if !formats.is_empty() {
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                        ui.label(
                            RichText::new(t.extra.opds_format_label.to_uppercase())
                                .size(10.0)
                                .strong()
                                .color(palette.on_surface_variant),
                        );

                        let all_selected = self.selected_format.is_none();
                        let (all_bg, all_fg, all_stroke) = if all_selected {
                            (palette.primary, palette.on_primary, Stroke::NONE)
                        } else {
                            (
                                palette.surface_container,
                                palette.on_surface_variant,
                                Stroke::new(1.0, palette.card_border),
                            )
                        };
                        if pill_button(ui, t.filter_all, all_bg, all_fg, all_stroke, 11.0, 24.0)
                            .clicked()
                        {
                            self.selected_format = None;
                        }

                        for fmt in &formats {
                            let is_sel = self.selected_format.as_deref() == Some(fmt.as_str());
                            let (bg, fg, stroke) = if is_sel {
                                (palette.primary, palette.on_primary, Stroke::NONE)
                            } else {
                                (
                                    palette.surface_container,
                                    palette.on_surface_variant,
                                    Stroke::new(1.0, palette.card_border),
                                )
                            };
                            if pill_button(ui, fmt, bg, fg, stroke, 11.0, 24.0).clicked() {
                                if is_sel {
                                    self.selected_format = None;
                                } else {
                                    self.selected_format = Some(fmt.clone());
                                }
                            }
                        }
                    });
                }
            });
    }

    fn execute_remote_search(&mut self, t: &Translations, settings: &AppSettings) {
        let Some(feed) = &self.current_feed else {
            return;
        };
        let template = feed.search_template.clone();
        let descriptor = feed.search_descriptor_url.clone();
        match self.active_auth(settings) {
            Ok(auth) => {
                if let Some(cur) = self.current_url.clone() {
                    self.history.push((cur, self.breadcrumbs.clone()));
                }
                self.breadcrumbs.push(OpdsBreadcrumb {
                    title: format!("{}: {}", t.search, self.search_query.trim()),
                    url: String::new(),
                });
                self.is_loading = true;
                self.error_message = None;
                self.pending_request_id = Some(self.client.search(
                    template,
                    descriptor,
                    self.search_query.trim(),
                    auth,
                ));
            }
            Err(err) => {
                self.pending_request_id = None;
                self.is_loading = false;
                self.error_message = Some(format_opds_auth_error(err, settings.ui_language));
            }
        }
    }

    fn render_feed_body(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        t: &Translations,
        settings: &AppSettings,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
    ) {
        if let Some(err) = self.error_message.clone() {
            egui::Frame::new()
                .fill(palette.error_bg)
                .stroke(Stroke::new(1.0, palette.error_fg.gamma_multiply(0.4)))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        let (ir, _) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                        paint_icon(ui.painter(), Icon::Warning, ir, palette.error_fg);
                        ui.label(
                            RichText::new(&err)
                                .size(12.0)
                                .color(palette.error_fg),
                        );
                        if self.last_requested_url.is_some()
                            && pill_button(
                                ui,
                                t.opds_retry,
                                palette.card_bg,
                                palette.error_fg,
                                Stroke::new(1.0, palette.error_fg.gamma_multiply(0.4)),
                                11.5,
                                26.0,
                            )
                            .clicked()
                        {
                            if let Some(url) = self.last_requested_url.clone() {
                                self.fetch_url(&url, settings);
                            }
                        }
                    });
                });
            ui.add_space(10.0);
        }

        if self.is_loading && self.current_feed.is_none() {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                ui.add(egui::Spinner::new().size(28.0).color(palette.primary));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t.extra.opds_status_loading)
                        .size(13.0)
                        .color(palette.on_surface_variant),
                );
            });
            return;
        }

        let Some(feed) = self.current_feed.clone() else {
            ui.add_space(36.0);
            egui::Frame::new()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.0, palette.card_border))
                .corner_radius(14.0)
                .inner_margin(egui::Margin::same(24))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let (gr, _) =
                            ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
                        paint_icon(ui.painter(), Icon::Globe, gr, palette.primary);
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(t.opds_select_catalog)
                                .size(14.0)
                                .color(palette.on_surface_variant),
                        );
                    });
                });
            return;
        };

        let catalog_name = settings
            .opds_catalogs
            .iter()
            .find(|c| self.active_catalog_id.as_deref() == Some(c.id.as_str()))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| feed.title.clone());

        let (navs, raw_books) = partition_entries(&feed.entries, &self.search_query);
        let books = super::filter_and_sort_books(
            raw_books,
            self.selected_format.as_deref(),
            self.sort_mode,
        );

        if navs.is_empty() && feed.entries.is_empty() {
            egui::Frame::new()
                .fill(palette.card_bg)
                .stroke(Stroke::new(1.0, palette.card_border))
                .corner_radius(12.0)
                .inner_margin(egui::Margin::same(24))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(t.opds_empty_feed)
                                .size(13.5)
                                .color(palette.on_surface_variant),
                        );
                    });
                });
        }

        let mut nav_to_open: Option<(String, String)> = None;
        if !navs.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t.opds_categories)
                        .strong()
                        .size(15.0)
                        .color(palette.on_surface),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count_str =
                        fill(t.extra.opds_shown_count, &[("{n}", &navs.len().to_string())]);
                    ui.label(
                        RichText::new(count_str)
                            .size(11.5)
                            .color(palette.on_surface_variant),
                    );
                });
            });
            ui.add_space(8.0);
            render_categories_grid(ui, &navs, palette, &mut nav_to_open);
            ui.add_space(16.0);
        }

        if let Some((title, url)) = nav_to_open {
            self.navigate_to(&title, &url, settings);
        }

        let has_any_books_in_feed = feed.entries.iter().any(|e| !e.acquisition_links.is_empty());
        if !books.is_empty() || has_any_books_in_feed {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t.extra.opds_books_in_catalog)
                        .strong()
                        .size(15.0)
                        .color(palette.on_surface),
                );
                let found_str = fill(t.extra.opds_found_count, &[("{n}", &books.len().to_string())]);
                ui.label(
                    RichText::new(found_str)
                        .size(12.0)
                        .color(palette.on_surface_variant),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let current_label = self.sort_mode.label(t);
                    egui::ComboBox::from_id_salt("opds_sort_mode_combo")
                        .selected_text(current_label)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            for mode in super::OpdsSortMode::ALL {
                                ui.selectable_value(&mut self.sort_mode, mode, mode.label(t));
                            }
                        });
                    let (sr, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                    paint_icon(
                        ui.painter(),
                        Icon::Sort,
                        Rect::from_center_size(sr.center(), Vec2::splat(13.0)),
                        palette.on_surface_variant,
                    );
                });
            });
            ui.add_space(8.0);

            if books.is_empty() {
                egui::Frame::new()
                    .fill(palette.card_bg)
                    .stroke(Stroke::new(1.0, palette.card_border))
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::same(20))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(t.opds_no_results)
                                    .size(13.0)
                                    .color(palette.on_surface_variant),
                            );
                        });
                    });
            } else {
                let avail_w = ui.available_width();
                let cols = if avail_w >= 860.0 { 2 } else { 1 };
                let gap = 14.0;
                let card_w = ((avail_w - gap * (cols as f32 - 1.0)) / cols as f32)
                    .floor()
                    .max(240.0);
                for chunk in books.chunks(cols) {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for entry in chunk {
                            self.render_book_card(
                                ctx,
                                ui,
                                entry,
                                &catalog_name,
                                settings,
                                palette,
                                card_w,
                                on_open_file,
                            );
                        }
                    });
                    ui.add_space(12.0);
                }
            }
        }

        if feed.prev_url.is_some() || feed.next_url.is_some() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if let Some(prev) = &feed.prev_url
                    && ui
                        .add(
                            IconButton::new(Icon::ArrowLeft, t.opds_prev_page)
                                .min_size(Vec2::new(0.0, 28.0)),
                        )
                        .clicked()
                {
                    let prev_clone = prev.clone();
                    self.fetch_url(&prev_clone, settings);
                }
                if let Some(next) = &feed.next_url
                    && ui
                        .add(
                            IconButton::new(Icon::ArrowRight, t.opds_next_page)
                                .min_size(Vec2::new(0.0, 28.0)),
                        )
                        .clicked()
                {
                    let next_clone = next.clone();
                    self.fetch_url(&next_clone, settings);
                }
            });
            ui.add_space(8.0);
        }
    }
}

fn partition_entries<'a>(
    entries: &'a [OpdsEntry],
    search_query: &str,
) -> (Vec<&'a OpdsEntry>, Vec<&'a OpdsEntry>) {
    let query = search_query.trim().to_lowercase();
    let mut navs = Vec::new();
    let mut books = Vec::new();
    for entry in entries {
        let matches = query.is_empty()
            || entry.title.to_lowercase().contains(&query)
            || entry.authors.iter().any(|a| a.to_lowercase().contains(&query))
            || entry
                .summary
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&query));
        if !matches {
            continue;
        }
        if !entry.acquisition_links.is_empty() {
            books.push(entry);
        } else if entry.navigation_link.is_some() {
            navs.push(entry);
        }
    }
    (navs, books)
}

fn render_categories_grid(
    ui: &mut egui::Ui,
    categories: &[&OpdsEntry],
    palette: &OpdsPalette,
    nav_url_to_open: &mut Option<(String, String)>,
) {
    let avail_w = ui.available_width();
    let cols = if avail_w >= 860.0 {
        3
    } else if avail_w >= 520.0 {
        2
    } else {
        1
    };
    let gap = 12.0;
    let card_w = ((avail_w - gap * (cols as f32 - 1.0)) / cols as f32)
        .floor()
        .max(160.0);
    let card_h = 68.0;

    for (row_idx, chunk) in categories.chunks(cols).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (col_idx, entry) in chunk.iter().enumerate() {
                let idx = row_idx * cols + col_idx;
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(card_w, card_h), egui::Sense::click());
                let bg = if resp.hovered() {
                    palette.card_footer_bg
                } else {
                    palette.card_bg
                };
                let border = if resp.hovered() {
                    Stroke::new(1.2, palette.primary.gamma_multiply(0.55))
                } else {
                    Stroke::new(1.0, palette.card_border)
                };
                ui.painter().rect_filled(rect, 11.0, bg);
                ui.painter()
                    .rect_stroke(rect, 11.0, border, StrokeKind::Inside);

                let (tint_bg, tint_fg) = palette.category_tints[idx % palette.category_tints.len()];
                let icon_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + 12.0, rect.center().y - 19.0),
                    Vec2::splat(38.0),
                );
                ui.painter().rect_filled(icon_rect, 9.0, tint_bg);
                let icon = match idx % 3 {
                    0 => Icon::Folder,
                    1 => Icon::Flame,
                    _ => Icon::Library,
                };
                paint_icon(
                    ui.painter(),
                    icon,
                    Rect::from_center_size(icon_rect.center(), Vec2::splat(17.0)),
                    tint_fg,
                );

                let arrow_rect = Rect::from_center_size(
                    Pos2::new(rect.max.x - 16.0, rect.center().y),
                    Vec2::splat(14.0),
                );
                paint_icon(
                    ui.painter(),
                    Icon::ArrowRight,
                    arrow_rect,
                    if resp.hovered() {
                        palette.primary
                    } else {
                        palette.on_surface_variant.gamma_multiply(0.55)
                    },
                );

                let text_left = icon_rect.max.x + 11.0;
                let text_right = arrow_rect.min.x - 6.0;
                let text_clip = Rect::from_min_max(
                    Pos2::new(text_left, rect.min.y + 8.0),
                    Pos2::new(text_right.max(text_left + 30.0), rect.max.y - 8.0),
                );
                let tp = ui.painter().with_clip_rect(text_clip);
                let title_g = ui.painter().layout_no_wrap(
                    entry.title.clone(),
                    egui::FontId::proportional(13.0),
                    palette.on_surface,
                );
                let has_sum = entry.summary.as_deref().is_some_and(|s| !s.trim().is_empty());
                if has_sum {
                    tp.galley(
                        Pos2::new(text_left, rect.min.y + 13.0),
                        title_g,
                        palette.on_surface,
                    );
                    if let Some(sum) = &entry.summary {
                        let sum_g = ui.painter().layout_no_wrap(
                            sum.trim().to_string(),
                            egui::FontId::proportional(11.0),
                            palette.on_surface_variant,
                        );
                        tp.galley(
                            Pos2::new(text_left, rect.min.y + 34.0),
                            sum_g,
                            palette.on_surface_variant,
                        );
                    }
                } else {
                    tp.galley(
                        Pos2::new(text_left, rect.center().y - title_g.size().y * 0.5),
                        title_g,
                        palette.on_surface,
                    );
                }

                if resp.clicked()
                    && let Some(nav_url) = &entry.navigation_link
                {
                    *nav_url_to_open = Some((entry.title.clone(), nav_url.clone()));
                }
            }
        });
        ui.add_space(10.0);
    }
}
