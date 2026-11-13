use super::icons::{Icon, IconButton, IconLabel, paint_icon};
use super::opds_view::OpdsBrowserState;
use super::theme::ReaderTheme;
use crate::core::license::ProFeature;
use crate::i18n::format_opds_auth_error;
use crate::opds::{DownloadStatus, OpdsCatalog, OpdsEntry, validate_catalog_url};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, StrokeKind, Vec2};

impl OpdsBrowserState {
    pub(crate) fn render_catalogs_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        target_width: f32,
        compact: bool,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();
        let sidebar_w = target_width.clamp(140.0, ui.available_width().max(140.0));
        let card_inner_w = (sidebar_w - 16.0).max(110.0);

        ui.vertical(|ui| {
            ui.set_width(sidebar_w);
            if !compact {
                ui.set_height(ui.available_height());
            }
            ui.horizontal(|ui| {
                let (icon_rect, _) =
                    ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                paint_icon(ui.painter(), Icon::Globe, icon_rect, accent);
                ui.heading(
                    RichText::new(t.opds_my_catalogs)
                        .size(15.0)
                        .color(text_color)
                        .strong(),
                );
            });
            ui.add_space(8.0);

            let scroll_max_h = if compact {
                if self.show_add_catalog { 76.0 } else { 90.0 }
            } else {
                (ui.available_height() - if self.show_add_catalog { 260.0 } else { 70.0 })
                    .max(100.0)
            };

            ScrollArea::vertical()
                .id_salt("opds_catalog_list")
                .auto_shrink([false, compact])
                .max_height(scroll_max_h)
                .show(ui, |ui| {
                    let mut catalog_to_open = None;
                    let mut catalog_to_delete = None;

                    for cat in &settings.opds_catalogs {
                        let is_active = self.active_catalog_id.as_deref() == Some(&cat.id);
                        let card_fill = if is_active {
                            accent.gamma_multiply(0.16)
                        } else {
                            theme.bg_color()
                        };
                        let card_stroke = if is_active {
                            Stroke::new(1.5, accent)
                        } else {
                            Stroke::new(1.0, accent.gamma_multiply(0.18))
                        };

                        let frame = egui::Frame::new()
                            .fill(card_fill)
                            .stroke(card_stroke)
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(8, 6));

                        let resp = frame.show(ui, |ui| {
                            ui.set_width(card_inner_w);
                            ui.horizontal(|ui| {
                                let (icon_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                                paint_icon(
                                    ui.painter(),
                                    if cat.is_default {
                                        Icon::Book
                                    } else {
                                        Icon::Globe
                                    },
                                    icon_rect,
                                    if is_active { accent } else { sec_text },
                                );

                                let text = RichText::new(&cat.name)
                                    .size(12.5)
                                    .color(if is_active { accent } else { text_color })
                                    .strong();
                                ui.add(egui::Label::new(text).truncate());

                                if cat.username.is_some() {
                                    let (key_rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(12.0),
                                        egui::Sense::hover(),
                                    );
                                    paint_icon(
                                        ui.painter(),
                                        Icon::Key,
                                        key_rect,
                                        sec_text.gamma_multiply(0.7),
                                    );
                                }

                                if !cat.is_default {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .add(
                                                    IconButton::new(Icon::Trash, "")
                                                        .min_size(Vec2::new(20.0, 20.0))
                                                        .color(Color32::from_rgb(220, 80, 80)),
                                                )
                                                .on_hover_text(t.opds_delete_catalog)
                                                .clicked()
                                            {
                                                catalog_to_delete = Some(cat.id.clone());
                                            }
                                        },
                                    );
                                }
                            });
                        });

                        let interact =
                            ui.interact(resp.response.rect, resp.response.id, egui::Sense::click());
                        if interact.clicked() {
                            if cat.is_default || settings.can_use(ProFeature::OpdsPro) {
                                catalog_to_open = Some(cat.clone());
                            } else {
                                *on_pro_required = Some(ProFeature::OpdsPro);
                            }
                        }

                        ui.add_space(4.0);
                    }

                    if let Some(cat) = catalog_to_open {
                        self.open_catalog(&cat, settings);
                    }
                    if let Some(del_id) = catalog_to_delete {
                        settings.opds_catalogs.retain(|c| c.id != del_id);
                        if self.active_catalog_id.as_deref() == Some(&del_id) {
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
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            if !self.show_add_catalog {
                if ui
                    .add(
                        IconButton::new(Icon::Plus, t.opds_add_catalog)
                            .min_size(Vec2::new((sidebar_w - 4.0).max(110.0), 30.0))
                            .center(true),
                    )
                    .clicked()
                {
                    if settings.can_use(ProFeature::OpdsPro) {
                        self.show_add_catalog = true;
                        self.add_error = None;
                    } else {
                        *on_pro_required = Some(ProFeature::OpdsPro);
                    }
                }
            } else {
                ui.label(RichText::new(t.opds_add_catalog).strong().color(text_color));
                ui.add_space(3.0);
                ui.label(
                    RichText::new(t.opds_catalog_name)
                        .size(11.0)
                        .color(sec_text),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_name)
                        .hint_text(t.opds_catalog_name)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(2.0);
                ui.label(RichText::new(t.opds_catalog_url).size(11.0).color(sec_text));
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_url)
                        .hint_text("http://.../opds")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(2.0);
                let auth_response = ui.checkbox(&mut self.new_has_auth, t.opds_auth_required);
                if auth_response.changed()
                    && self.new_has_auth
                    && !settings.can_use(ProFeature::OpdsPro)
                {
                    self.new_has_auth = false;
                    *on_pro_required = Some(ProFeature::OpdsPro);
                }
                if self.new_has_auth {
                    ui.add_space(2.0);
                    ui.label(RichText::new(t.opds_username).size(11.0).color(sec_text));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_user)
                            .hint_text(t.opds_username)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label(RichText::new(t.opds_password).size(11.0).color(sec_text));
                    let pass_field = egui::TextEdit::singleline(&mut self.new_pass)
                        .password(true)
                        .hint_text(t.opds_password)
                        .desired_width(f32::INFINITY);
                    ui.add(pass_field);
                }

                if let Some(err) = &self.add_error {
                    ui.label(RichText::new(err).color(Color32::RED).size(11.0));
                }

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add(IconButton::new(Icon::Check, t.opds_save_btn))
                        .clicked()
                    {
                        if !settings.can_use(ProFeature::OpdsPro) {
                            *on_pro_required = Some(ProFeature::OpdsPro);
                        } else {
                            let raw_name = self.new_name.trim();
                            let url = self.new_url.trim();
                            let uses_auth = self.new_has_auth && !self.new_user.trim().is_empty();
                            if url.is_empty() {
                                self.add_error = Some(t.opds_url_required.to_string());
                            } else if let Err(err) = validate_catalog_url(url, uses_auth) {
                                self.add_error =
                                    Some(format_opds_auth_error(err, settings.ui_language));
                            } else {
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
                                    username: if self.new_has_auth
                                        && !self.new_user.trim().is_empty()
                                    {
                                        Some(self.new_user.trim().to_string())
                                    } else {
                                        None
                                    },
                                    password: if self.new_has_auth
                                        && !self.new_pass.trim().is_empty()
                                    {
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
                        }
                    }
                    if ui
                        .add(IconButton::new(Icon::Cross, t.opds_cancel_btn))
                        .clicked()
                    {
                        self.show_add_catalog = false;
                        self.add_error = None;
                    }
                });
            }
        });
    }

    pub(super) fn render_categories_grid(
        &mut self,
        ui: &mut egui::Ui,
        categories: &[&OpdsEntry],
        theme: &ReaderTheme,
        nav_url_to_open: &mut Option<(String, String)>,
    ) {
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();

        let avail_w = (ui.available_width() - 8.0).max(120.0);
        let cols = ((avail_w / 240.0).floor() as usize).clamp(1, 4);
        let card_w = ((avail_w - (cols.saturating_sub(1) as f32) * 8.0) / (cols as f32)).max(110.0);

        for row in categories.chunks(cols) {
            ui.horizontal(|ui| {
                for entry in row {
                    let card_frame = egui::Frame::new()
                        .fill(theme.bg_color())
                        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.2)))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(10, 8));

                    let resp = card_frame.show(ui, |ui| {
                        ui.set_width((card_w - 20.0).max(90.0));
                        ui.horizontal(|ui| {
                            let (icon_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                            paint_icon(ui.painter(), Icon::Folder, icon_rect, accent);
                            ui.add_space(4.0);
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&entry.title)
                                            .size(12.5)
                                            .strong()
                                            .color(text_color),
                                    )
                                    .truncate(),
                                );
                                if let Some(summary) = &entry.summary {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(summary).size(10.5).color(sec_text),
                                        )
                                        .truncate(),
                                    );
                                }
                            });
                        });
                    });

                    let interact =
                        ui.interact(resp.response.rect, resp.response.id, egui::Sense::click());
                    if interact.hovered() {
                        ui.painter().rect_stroke(
                            resp.response.rect,
                            6.0,
                            Stroke::new(1.5, accent),
                            StrokeKind::Inside,
                        );
                    }
                    if interact.clicked() {
                        if let Some(link) = &entry.navigation_link {
                            *nav_url_to_open = Some((entry.title.clone(), link.clone()));
                        }
                    }

                    ui.add_space(6.0);
                }
            });
            ui.add_space(6.0);
        }
    }

    pub(super) fn render_downloads_banner(
        &mut self,
        ui: &mut egui::Ui,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
    ) {
        if self.downloads.is_empty() {
            return;
        }
        let t = lang.t();

        let active_downloads: Vec<_> = self
            .downloads
            .iter()
            .filter(|(_, d)| {
                matches!(
                    d.status,
                    DownloadStatus::Downloading { .. }
                        | DownloadStatus::Queued
                        | DownloadStatus::Failed(_)
                )
            })
            .map(|(id, d)| (id.clone(), d.clone()))
            .collect();

        if active_downloads.is_empty() {
            return;
        }

        let mut dismiss_id: Option<String> = None;
        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.4)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                for (dl_id, item) in active_downloads {
                    let progress_w = (ui.available_width() * 0.35).clamp(80.0, 200.0);
                    ui.horizontal_wrapped(|ui| match item.status {
                        DownloadStatus::Downloading { downloaded, total } => {
                            let pct = if let Some(tot) = total
                                && tot > 0
                            {
                                downloaded as f32 / tot as f32
                            } else {
                                0.0
                            };
                            let mb_dl = downloaded as f32 / (1024.0 * 1024.0);
                            let info = if let Some(tot) = total {
                                let mb_tot = tot as f32 / (1024.0 * 1024.0);
                                format!(
                                    "{} [{}]: {:.1} {} / {:.1} {} ({:.0}%)",
                                    item.title,
                                    item.format_label,
                                    mb_dl,
                                    t.extra.megabytes_short,
                                    mb_tot,
                                    t.extra.megabytes_short,
                                    pct * 100.0
                                )
                            } else {
                                format!(
                                    "{} [{}]: {:.1} {}",
                                    item.title, item.format_label, mb_dl, t.extra.megabytes_short
                                )
                            };
                            ui.add(
                                IconLabel::new(Icon::Download, &info)
                                    .color(theme.accent_color())
                                    .size(12.0)
                                    .strong(),
                            );
                            if total.is_some() {
                                ui.add(egui::ProgressBar::new(pct).desired_width(progress_w));
                            }
                        }
                        DownloadStatus::Queued => {
                            let text = crate::i18n::fill(
                                t.extra.queued_download,
                                &[("{title}", &item.title)],
                            );
                            ui.add(IconLabel::new(Icon::Recent, &text).size(12.0));
                        }
                        DownloadStatus::Failed(err) => {
                            let text = format!("{} [{}]: {}", item.title, item.format_label, err);
                            ui.add(
                                IconLabel::new(Icon::Warning, &text)
                                    .color(Color32::RED)
                                    .size(12.0),
                            );
                            if ui
                                .add(
                                    IconButton::new(Icon::Cross, "")
                                        .min_size(Vec2::splat(18.0))
                                        .color(theme.secondary_text_color()),
                                )
                                .clicked()
                            {
                                dismiss_id = Some(dl_id);
                            }
                        }
                        DownloadStatus::Completed { .. } => {}
                    });
                }
            });
        if let Some(id) = dismiss_id {
            self.downloads.remove(&id);
        }
        ui.add_space(4.0);
    }
}
