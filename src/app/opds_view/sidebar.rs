use super::OpdsBrowserState;
use super::palette::{OpdsPalette, pill_badge, pill_button};
use crate::app::icons::{Icon, IconButton, paint_dot_separator, paint_icon};
use crate::core::license::ProFeature;
use crate::i18n::{Translations, fill, format_opds_auth_error};
use crate::opds::{DownloadStatus, OpdsCatalog, validate_catalog_url};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::path::PathBuf;

impl OpdsBrowserState {
    pub(super) fn render_catalogs_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        sidebar_w: f32,
        compact_top: bool,
        settings: &mut AppSettings,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        let t = settings.ui_language.t();
        let bg_frame = egui::Frame::new()
            .fill(palette.sidebar_bg)
            .stroke(Stroke::new(1.0, palette.card_border))
            .inner_margin(egui::Margin::same(if compact_top { 10 } else { 14 }));

        bg_frame.show(ui, |ui| {
            ui.set_width((sidebar_w - if compact_top { 20.0 } else { 28.0 }).max(140.0));
            if !compact_top {
                ui.set_min_height((ui.available_height()).max(120.0));
            }

            if compact_top {
                self.render_compact_sidebar(
                    ui,
                    t,
                    settings,
                    palette,
                    on_open_file,
                    on_pro_required,
                );
                return;
            }

            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(icon_rect, 7.0, palette.primary_soft);
                paint_icon(
                    ui.painter(),
                    Icon::Globe,
                    Rect::from_center_size(icon_rect.center(), Vec2::splat(14.0)),
                    palette.primary,
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t.extra.opds_connected_catalogs)
                        .strong()
                        .size(13.5)
                        .color(palette.on_surface),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count_text = format!("{} OPDS", settings.opds_catalogs.len());
                    pill_badge(
                        ui,
                        &count_text,
                        palette.surface_container,
                        palette.on_surface_variant,
                        10.0,
                        None,
                    );
                });
            });

            ui.add_space(10.0);

            let reserve_bottom = if self.completed_toast.is_some() {
                195.0
            } else {
                92.0
            };
            let list_max_h = (ui.available_height() - reserve_bottom).max(120.0);

            ScrollArea::vertical()
                .id_salt("opds_catalog_sidebar_scroll")
                .max_height(list_max_h)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    let defaults: Vec<OpdsCatalog> = settings
                        .opds_catalogs
                        .iter()
                        .filter(|c| c.is_default)
                        .cloned()
                        .collect();
                    let customs: Vec<OpdsCatalog> = settings
                        .opds_catalogs
                        .iter()
                        .filter(|c| !c.is_default)
                        .cloned()
                        .collect();

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t.extra.opds_default_section.to_uppercase())
                                .size(9.5)
                                .strong()
                                .color(palette.on_surface_variant),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(t.extra.opds_public_badge)
                                    .size(9.5)
                                    .color(palette.on_surface_variant.gamma_multiply(0.85)),
                            );
                        });
                    });
                    ui.add_space(4.0);

                    let mut catalog_to_open = None;
                    for cat in &defaults {
                        if self.render_catalog_row(ui, cat, false, t, palette) {
                            catalog_to_open = Some(cat.clone());
                        }
                        ui.add_space(4.0);
                    }

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t.extra.opds_my_servers_pro.to_uppercase())
                                .size(9.5)
                                .strong()
                                .color(palette.on_surface_variant),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            pill_badge(
                                ui,
                                t.extra.opds_protected_badge,
                                palette.secondary_soft,
                                palette.secondary,
                                9.0,
                                None,
                            );
                        });
                    });
                    ui.add_space(4.0);

                    let mut catalog_to_delete = None;
                    if customs.is_empty() {
                        egui::Frame::new()
                            .fill(palette.surface_container.gamma_multiply(0.6))
                            .stroke(Stroke::new(1.0, palette.card_border))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    RichText::new(t.extra.opds_custom_servers_empty)
                                        .size(11.0)
                                        .color(palette.on_surface_variant),
                                );
                            });
                    } else {
                        for cat in &customs {
                            let (clicked, delete_clicked) =
                                self.render_custom_catalog_row(ui, cat, t, palette);
                            if clicked {
                                if settings.can_use(ProFeature::OpdsPro) {
                                    catalog_to_open = Some(cat.clone());
                                } else {
                                    *on_pro_required = Some(ProFeature::OpdsPro);
                                }
                            }
                            if delete_clicked {
                                catalog_to_delete = Some(cat.id.clone());
                            }
                            ui.add_space(4.0);
                        }
                    }

                    if let Some(cat) = catalog_to_open {
                        self.open_catalog(&cat, settings);
                    }
                    if let Some(del_id) = catalog_to_delete {
                        settings.opds_catalogs.retain(|c| c.id != del_id);
                        self.clear_active_if_deleted(&del_id);
                    }

                    ui.add_space(8.0);
                    self.render_add_catalog_section(ui, t, settings, palette, on_pro_required);
                });

            self.render_sidebar_bottom(ui, t, palette, on_open_file);
        });
    }

    fn render_compact_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        t: &Translations,
        settings: &mut AppSettings,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t.extra.opds_connected_catalogs)
                    .strong()
                    .size(12.5)
                    .color(palette.on_surface),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        IconButton::new(Icon::Plus, t.opds_add_catalog)
                            .font_size(11.0)
                            .min_size(Vec2::new(0.0, 24.0)),
                    )
                    .clicked()
                {
                    if settings.can_use(ProFeature::OpdsPro) {
                        self.show_add_catalog = !self.show_add_catalog;
                        self.add_error = None;
                    } else {
                        *on_pro_required = Some(ProFeature::OpdsPro);
                    }
                }
            });
        });
        ui.add_space(4.0);
        ScrollArea::horizontal()
            .id_salt("opds_compact_catalogs_scroll")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let catalogs = settings.opds_catalogs.clone();
                    let mut open_cat = None;
                    for cat in &catalogs {
                        let selected = self.active_catalog_id.as_deref() == Some(&cat.id);
                        let (bg, fg, stroke) = if selected {
                            (palette.primary, palette.on_primary, Stroke::NONE)
                        } else {
                            (
                                palette.card_bg,
                                palette.on_surface,
                                Stroke::new(1.0, palette.card_border),
                            )
                        };
                        if pill_button(ui, &cat.name, bg, fg, stroke, 11.5, 26.0).clicked() {
                            if !cat.is_default && !settings.can_use(ProFeature::OpdsPro) {
                                *on_pro_required = Some(ProFeature::OpdsPro);
                            } else {
                                open_cat = Some(cat.clone());
                            }
                        }
                    }
                    if let Some(cat) = open_cat {
                        self.open_catalog(&cat, settings);
                    }
                });
            });

        if self.show_add_catalog {
            ui.add_space(6.0);
            self.render_add_catalog_form(ui, t, settings, palette, on_pro_required);
        }

        if let Some(toast) = self.completed_toast.clone() {
            ui.add_space(6.0);
            self.render_completed_toast_card(ui, &toast, t, palette, on_open_file);
        }
    }

    fn render_catalog_row(
        &self,
        ui: &mut egui::Ui,
        cat: &OpdsCatalog,
        _is_custom: bool,
        t: &Translations,
        palette: &OpdsPalette,
    ) -> bool {
        let is_selected = self.active_catalog_id.as_deref() == Some(&cat.id);
        let avail_w = ui.available_width();
        let row_h = 52.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, row_h), egui::Sense::click());
        let bg = if is_selected {
            palette.surface_highest
        } else if resp.hovered() {
            palette.surface_high
        } else {
            palette.card_bg
        };
        let border = if is_selected {
            Stroke::new(1.2, palette.primary.gamma_multiply(0.55))
        } else {
            Stroke::new(1.0, palette.card_border)
        };
        ui.painter().rect_filled(rect, 10.0, bg);
        ui.painter()
            .rect_stroke(rect, 10.0, border, StrokeKind::Inside);

        let icon_box = Rect::from_min_size(Pos2::new(rect.min.x + 10.0, rect.center().y - 16.0), Vec2::splat(32.0));
        let (icon_bg, icon_fg) = if is_selected {
            (palette.primary, palette.on_primary)
        } else {
            (palette.surface_container, palette.on_surface_variant)
        };
        ui.painter().rect_filled(icon_box, 8.0, icon_bg);
        let icon = match cat.id.as_str() {
            "standard_ebooks" => Icon::Book,
            "gutenberg" => Icon::Library,
            _ => Icon::Globe,
        };
        paint_icon(
            ui.painter(),
            icon,
            Rect::from_center_size(icon_box.center(), Vec2::splat(16.0)),
            icon_fg,
        );

        let dot_x = rect.max.x - 14.0;
        if is_selected {
            ui.painter()
                .circle_filled(Pos2::new(dot_x, rect.center().y), 3.5, palette.primary);
        }

        let text_left = icon_box.max.x + 10.0;
        let text_w = (dot_x - text_left - 10.0).max(40.0);
        let subtitle = catalog_subtitle(cat, t);
        let title_galley = ui.painter().layout_no_wrap(
            cat.name.clone(),
            egui::FontId::proportional(12.5),
            palette.on_surface,
        );
        let sub_galley = ui.painter().layout_no_wrap(
            subtitle,
            egui::FontId::proportional(10.5),
            palette.on_surface_variant,
        );
        let text_clip = Rect::from_min_max(
            Pos2::new(text_left, rect.min.y + 4.0),
            Pos2::new(text_left + text_w, rect.max.y - 4.0),
        );
        let text_painter = ui.painter().with_clip_rect(text_clip);
        text_painter.galley(
            Pos2::new(text_left, rect.min.y + 9.0),
            title_galley,
            palette.on_surface,
        );
        text_painter.galley(
            Pos2::new(text_left, rect.min.y + 27.0),
            sub_galley,
            palette.on_surface_variant,
        );

        resp.on_hover_text(&cat.url).clicked()
    }

    fn render_custom_catalog_row(
        &self,
        ui: &mut egui::Ui,
        cat: &OpdsCatalog,
        t: &Translations,
        palette: &OpdsPalette,
    ) -> (bool, bool) {
        let is_selected = self.active_catalog_id.as_deref() == Some(&cat.id);
        let avail_w = ui.available_width();
        let row_h = 54.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, row_h), egui::Sense::click());
        let bg = if is_selected {
            palette.surface_highest
        } else if resp.hovered() {
            palette.surface_high
        } else {
            palette.card_bg
        };
        let border = if is_selected {
            Stroke::new(1.2, palette.primary.gamma_multiply(0.55))
        } else {
            Stroke::new(1.0, palette.card_border)
        };
        ui.painter().rect_filled(rect, 10.0, bg);
        ui.painter()
            .rect_stroke(rect, 10.0, border, StrokeKind::Inside);

        let icon_box = Rect::from_min_size(Pos2::new(rect.min.x + 10.0, rect.center().y - 16.0), Vec2::splat(32.0));
        let (icon_bg, icon_fg) = if is_selected {
            (palette.primary, palette.on_primary)
        } else {
            (palette.surface_container, palette.secondary)
        };
        ui.painter().rect_filled(icon_box, 8.0, icon_bg);
        paint_icon(
            ui.painter(),
            if cat.has_credentials() {
                Icon::Key
            } else {
                Icon::Shield
            },
            Rect::from_center_size(icon_box.center(), Vec2::splat(15.0)),
            icon_fg,
        );

        let del_rect = Rect::from_center_size(
            Pos2::new(rect.max.x - 16.0, rect.center().y),
            Vec2::splat(22.0),
        );
        let del_resp = ui
            .interact(
                del_rect,
                egui::Id::new(("opds_del", &cat.id)),
                egui::Sense::click(),
            )
            .on_hover_text(t.opds_delete_catalog);
        if del_resp.hovered() {
            ui.painter()
                .rect_filled(del_rect, 6.0, palette.error_bg);
        }
        paint_icon(
            ui.painter(),
            Icon::Trash,
            Rect::from_center_size(del_rect.center(), Vec2::splat(12.0)),
            if del_resp.hovered() {
                palette.error_fg
            } else {
                palette.on_surface_variant
            },
        );

        let text_left = icon_box.max.x + 10.0;
        let text_w = (del_rect.min.x - text_left - 6.0).max(40.0);
        let scheme = if cat.url.starts_with("https://") {
            "HTTPS"
        } else {
            "HTTP"
        };
        let meta = if cat.has_credentials() {
            format!("{scheme} / Basic Auth")
        } else {
            catalog_host(&cat.url)
        };
        let clip = Rect::from_min_max(
            Pos2::new(text_left, rect.min.y + 4.0),
            Pos2::new(text_left + text_w, rect.max.y - 4.0),
        );
        let tp = ui.painter().with_clip_rect(clip);
        let title_galley = ui.painter().layout_no_wrap(
            cat.name.clone(),
            egui::FontId::proportional(12.5),
            palette.on_surface,
        );
        let meta_galley = ui.painter().layout_no_wrap(
            meta,
            egui::FontId::proportional(10.5),
            palette.on_surface_variant,
        );
        tp.galley(
            Pos2::new(text_left, rect.min.y + 10.0),
            title_galley,
            palette.on_surface,
        );
        tp.galley(
            Pos2::new(text_left, rect.min.y + 28.0),
            meta_galley,
            palette.on_surface_variant,
        );

        let row_clicked = resp.on_hover_text(&cat.url).clicked() && !del_resp.clicked();
        (row_clicked, del_resp.clicked())
    }

    fn render_add_catalog_section(
        &mut self,
        ui: &mut egui::Ui,
        t: &Translations,
        settings: &mut AppSettings,
        palette: &OpdsPalette,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        if !self.show_add_catalog {
            let avail_w = ui.available_width();
            let (rect, resp) =
                ui.allocate_exact_size(Vec2::new(avail_w, 34.0), egui::Sense::click());
            let fill = if resp.hovered() {
                palette.primary_soft.gamma_multiply(0.5)
            } else {
                palette.card_bg
            };
            ui.painter().rect_filled(rect, 9.0, fill);
            ui.painter().rect_stroke(
                rect,
                9.0,
                Stroke::new(1.0, palette.primary.gamma_multiply(0.45)),
                StrokeKind::Inside,
            );
            let label = t.extra.opds_add_catalog_cta;
            let font = egui::FontId::proportional(12.0);
            let galley = ui
                .painter()
                .layout_no_wrap(label.to_string(), font, palette.primary);
            let total_w = 13.0 + 6.0 + galley.size().x;
            let start_x = (rect.center().x - total_w * 0.5).max(rect.min.x + 8.0);
            paint_icon(
                ui.painter(),
                Icon::Plus,
                Rect::from_center_size(Pos2::new(start_x + 6.5, rect.center().y), Vec2::splat(13.0)),
                palette.primary,
            );
            ui.painter().galley(
                Pos2::new(start_x + 19.0, rect.center().y - galley.size().y * 0.5),
                galley,
                palette.primary,
            );
            if resp.clicked() {
                if settings.can_use(ProFeature::OpdsPro) {
                    self.show_add_catalog = true;
                    self.add_error = None;
                } else {
                    *on_pro_required = Some(ProFeature::OpdsPro);
                }
            }
        } else {
            self.render_add_catalog_form(ui, t, settings, palette, on_pro_required);
        }
    }

    fn render_add_catalog_form(
        &mut self,
        ui: &mut egui::Ui,
        t: &Translations,
        settings: &mut AppSettings,
        palette: &OpdsPalette,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        egui::Frame::new()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0, palette.primary.gamma_multiply(0.4)))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(t.opds_add_catalog)
                        .strong()
                        .size(12.5)
                        .color(palette.on_surface),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t.opds_catalog_name)
                        .size(10.5)
                        .color(palette.on_surface_variant),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_name)
                        .hint_text(t.opds_catalog_name)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(3.0);
                ui.label(
                    RichText::new(t.opds_catalog_url)
                        .size(10.5)
                        .color(palette.on_surface_variant),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_url)
                        .hint_text("https://.../opds")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(3.0);
                let auth_resp = ui.checkbox(&mut self.new_has_auth, t.opds_auth_required);
                if auth_resp.changed()
                    && self.new_has_auth
                    && !settings.can_use(ProFeature::OpdsPro)
                {
                    self.new_has_auth = false;
                    *on_pro_required = Some(ProFeature::OpdsPro);
                }
                if self.new_has_auth {
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(t.opds_username)
                            .size(10.5)
                            .color(palette.on_surface_variant),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_user)
                            .hint_text(t.opds_username)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label(
                        RichText::new(t.opds_password)
                            .size(10.5)
                            .color(palette.on_surface_variant),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_pass)
                            .password(true)
                            .hint_text(t.opds_password)
                            .desired_width(f32::INFINITY),
                    );
                }
                if let Some(err) = &self.add_error {
                    ui.add_space(3.0);
                    ui.label(RichText::new(err).color(palette.error_fg).size(10.5));
                }
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if pill_button(
                        ui,
                        t.opds_save_btn,
                        palette.primary,
                        palette.on_primary,
                        Stroke::NONE,
                        11.5,
                        28.0,
                    )
                    .clicked()
                    {
                        self.try_save_new_catalog(t, settings, on_pro_required);
                    }
                    if pill_button(
                        ui,
                        t.opds_cancel_btn,
                        palette.surface_container,
                        palette.on_surface,
                        Stroke::new(1.0, palette.card_border),
                        11.5,
                        28.0,
                    )
                    .clicked()
                    {
                        self.show_add_catalog = false;
                        self.add_error = None;
                    }
                });
            });
    }

    fn try_save_new_catalog(
        &mut self,
        t: &Translations,
        settings: &mut AppSettings,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        if !settings.can_use(ProFeature::OpdsPro) {
            *on_pro_required = Some(ProFeature::OpdsPro);
            return;
        }
        let raw_name = self.new_name.trim();
        let url = self.new_url.trim();
        let uses_auth = self.new_has_auth && !self.new_user.trim().is_empty();
        if url.is_empty() {
            self.add_error = Some(t.opds_url_required.to_string());
            return;
        }
        if let Err(err) = validate_catalog_url(url, uses_auth) {
            self.add_error = Some(format_opds_auth_error(err, settings.ui_language));
            return;
        }
        let name = if raw_name.is_empty() {
            url.trim_start_matches("http://")
                .trim_start_matches("https://")
                .split('/')
                .next()
                .unwrap_or(t.extra.opds_default_catalog)
        } else {
            raw_name
        };
        let new_cat = OpdsCatalog {
            id: format!(
                "custom_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ),
            name: name.to_string(),
            url: url.to_string(),
            username: if uses_auth {
                Some(self.new_user.trim().to_string())
            } else {
                None
            },
            password: if uses_auth && !self.new_pass.trim().is_empty() {
                Some(self.new_pass.trim().to_string())
            } else {
                None
            },
            is_default: false,
        };
        settings.opds_catalogs.push(new_cat.clone());
        self.show_add_catalog = false;
        self.new_name.clear();
        self.new_url.clear();
        self.new_user.clear();
        self.new_pass.clear();
        self.add_error = None;
        self.open_catalog(&new_cat, settings);
    }

    fn render_sidebar_bottom(
        &mut self,
        ui: &mut egui::Ui,
        t: &Translations,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
    ) {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            egui::Frame::new()
                .fill(palette.surface_container.gamma_multiply(0.7))
                .stroke(Stroke::new(1.0, palette.card_border))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (sr, _) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                        paint_icon(ui.painter(), Icon::Shield, sr, palette.tertiary_sage);
                        ui.label(
                            RichText::new(t.extra.opds_security_note)
                                .size(10.0)
                                .color(palette.on_surface_variant),
                        );
                    });
                });

            if let Some(toast) = self.completed_toast.clone() {
                ui.add_space(6.0);
                self.render_completed_toast_card(ui, &toast, t, palette, on_open_file);
            } else if let Some((dl_id, title, pct)) = self.first_active_download() {
                ui.add_space(6.0);
                self.render_active_download_mini(ui, &dl_id, &title, pct, t, palette);
            }
        });
    }

    fn render_active_download_mini(
        &mut self,
        ui: &mut egui::Ui,
        dl_id: &str,
        title: &str,
        pct: Option<f32>,
        t: &Translations,
        palette: &OpdsPalette,
    ) {
        egui::Frame::new()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0, palette.card_border))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.opds_downloading)
                            .strong()
                            .size(11.0)
                            .color(palette.primary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (xr, xresp) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
                        paint_icon(
                            ui.painter(),
                            Icon::Cross,
                            xr.shrink(3.0),
                            palette.on_surface_variant,
                        );
                        if xresp.clicked() {
                            self.cancel_download(dl_id);
                        }
                    });
                });
                ui.label(
                    RichText::new(title)
                        .size(11.0)
                        .color(palette.on_surface),
                );
                if let Some(frac) = pct {
                    ui.add_space(3.0);
                    let (bar_rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 5.0),
                        egui::Sense::hover(),
                    );
                    ui.painter()
                        .rect_filled(bar_rect, 2.5, palette.surface_high);
                    let fill_rect = Rect::from_min_size(
                        bar_rect.min,
                        Vec2::new(bar_rect.width() * frac.clamp(0.0, 1.0), 5.0),
                    );
                    ui.painter().rect_filled(fill_rect, 2.5, palette.primary);
                }
            });
    }

    fn render_completed_toast_card(
        &mut self,
        ui: &mut egui::Ui,
        toast: &super::OpdsCompletedToast,
        t: &Translations,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
    ) {
        egui::Frame::new()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.2, palette.tertiary_sage_border))
            .corner_radius(12.0)
            .inner_margin(egui::Margin::same(11))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (cr, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(cr.center(), 10.0, palette.tertiary_sage_bg);
                    paint_icon(
                        ui.painter(),
                        Icon::Check,
                        Rect::from_center_size(cr.center(), Vec2::splat(11.0)),
                        palette.tertiary_sage,
                    );
                    ui.label(
                        RichText::new(t.extra.opds_toast_done_title)
                            .strong()
                            .size(12.0)
                            .color(palette.on_surface),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (xr, xresp) =
                            ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
                        paint_icon(
                            ui.painter(),
                            Icon::Cross,
                            xr.shrink(3.0),
                            palette.on_surface_variant,
                        );
                        if xresp.clicked() {
                            self.completed_toast = None;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(
                        RichText::new(&toast.title)
                            .strong()
                            .size(11.0)
                            .color(palette.on_surface),
                    );
                    paint_dot_separator(ui, 10.5, palette.on_surface_variant);
                    ui.label(
                        RichText::new(format!(
                            "{} ({})",
                            t.extra.opds_toast_ready_to_read, toast.format_label
                        ))
                        .size(10.5)
                        .color(palette.on_surface_variant),
                    );
                });
                ui.add_space(7.0);
                ui.horizontal(|ui| {
                    if pill_button(
                        ui,
                        t.extra.opds_toast_open_btn,
                        palette.primary,
                        palette.on_primary,
                        Stroke::NONE,
                        11.0,
                        26.0,
                    )
                    .clicked()
                    {
                        *on_open_file = Some(toast.file_path.clone());
                    }
                    if pill_button(
                        ui,
                        t.extra.opds_toast_folder_btn,
                        palette.surface_container,
                        palette.on_surface,
                        Stroke::new(1.0, palette.card_border),
                        11.0,
                        26.0,
                    )
                    .clicked()
                    {
                        crate::platform::downloads::reveal_downloaded_file(&toast.file_path);
                    }
                });
            });
    }

    fn first_active_download(&self) -> Option<(String, String, Option<f32>)> {
        for (id, item) in &self.downloads {
            if let DownloadStatus::Downloading { downloaded, total } = item.status {
                let frac = total
                    .filter(|&tot| tot > 0)
                    .map(|tot| downloaded as f32 / tot as f32);
                return Some((id.clone(), item.title.clone(), frac));
            }
        }
        None
    }

    fn clear_active_if_deleted(&mut self, del_id: &str) {
        if self.active_catalog_id.as_deref() == Some(del_id) {
            self.pending_request_id = None;
            self.active_catalog_id = None;
            self.cover_manager.clear();
            self.current_url = None;
            self.last_requested_url = None;
            self.current_feed = None;
            self.breadcrumbs.clear();
            self.history.clear();
            self.is_loading = false;
            self.error_message = None;
        }
    }
}

fn catalog_subtitle(cat: &OpdsCatalog, t: &Translations) -> String {
    match cat.id.as_str() {
        "standard_ebooks" => t.extra.opds_sub_standard_ebooks.to_string(),
        "gutenberg" => t.extra.opds_sub_gutenberg.to_string(),
        "feedbooks" => t.extra.opds_sub_feedbooks.to_string(),
        _ => catalog_host(&cat.url),
    }
}

fn catalog_host(url: &str) -> String {
    let trimmed = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = trimmed.split('/').next().unwrap_or(url);
    fill("{host}", &[("{host}", host)])
}
