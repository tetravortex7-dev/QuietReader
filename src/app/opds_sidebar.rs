use super::icons::{Icon, IconButton, paint_icon};
use super::opds_view::OpdsBrowserState;
use super::theme::ReaderTheme;
use crate::opds::OpdsCatalog;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, Vec2};

impl OpdsBrowserState {
    pub(crate) fn render_catalogs_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();

        ui.vertical(|ui| {
            ui.set_width(240.0);
            ui.set_height(ui.available_height());
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

            let scroll_max_h = (ui.available_height()
                - if self.show_add_catalog { 260.0 } else { 70.0 })
            .max(100.0);

            ScrollArea::vertical()
                .id_salt("opds_catalog_list")
                .auto_shrink([false, false])
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
                            ui.set_width(224.0);
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
                            catalog_to_open = Some(cat.clone());
                        }

                        ui.add_space(4.0);
                    }

                    if let Some(cat) = catalog_to_open {
                        self.open_catalog(&cat);
                    }
                    if let Some(del_id) = catalog_to_delete {
                        settings.opds_catalogs.retain(|c| c.id != del_id);
                        if self.active_catalog_id.as_deref() == Some(&del_id) {
                            self.active_catalog_id = None;
                            self.current_feed = None;
                            self.breadcrumbs.clear();
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
                            .min_size(Vec2::new(220.0, 30.0))
                            .center(true),
                    )
                    .clicked()
                {
                    self.show_add_catalog = true;
                    self.add_error = None;
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
                ui.checkbox(&mut self.new_has_auth, t.opds_auth_required);
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
                ui.horizontal(|ui| {
                    if ui
                        .add(IconButton::new(Icon::Check, t.opds_save_btn))
                        .clicked()
                    {
                        let raw_name = self.new_name.trim();
                        let url = self.new_url.trim();
                        if url.is_empty() {
                            self.add_error = Some("Укажите адрес каталога (URL)".to_string());
                        } else if !url.starts_with("http://") && !url.starts_with("https://") {
                            self.add_error =
                                Some("Адрес должен начинаться с http:// или https://".to_string());
                        } else {
                            let name = if raw_name.is_empty() {
                                url.trim_start_matches("http://")
                                    .trim_start_matches("https://")
                                    .split('/')
                                    .next()
                                    .unwrap_or("OPDS Catalog")
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
                                username: if self.new_has_auth && !self.new_user.trim().is_empty() {
                                    Some(self.new_user.trim().to_string())
                                } else {
                                    None
                                },
                                password: if self.new_has_auth && !self.new_pass.trim().is_empty() {
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
                            self.open_catalog(&new_cat);
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
}
