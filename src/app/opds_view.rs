use crate::app::icons::{Icon, IconButton, IconLabel};
use crate::app::theme::ReaderTheme;
use crate::opds::{
    DownloadItem, DownloadStatus, OpdsAcquisitionLink, OpdsCatalog, OpdsClient, OpdsEntry,
    OpdsEvent, OpdsFeed,
};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct OpdsBrowserState {
    pub client: OpdsClient,
    pub active_catalog_id: Option<String>,
    pub current_url: Option<String>,
    pub history: Vec<String>,
    pub current_feed: Option<OpdsFeed>,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub search_query: String,
    pub downloads: HashMap<String, DownloadItem>,
    pub new_download_to_open: Option<PathBuf>,
    pub show_add_catalog: bool,
    pub new_name: String,
    pub new_url: String,
    pub new_has_auth: bool,
    pub new_user: String,
    pub new_pass: String,
    pub add_error: Option<String>,
    pub expanded_summaries: HashSet<String>,
}

impl Default for OpdsBrowserState {
    fn default() -> Self {
        Self::new()
    }
}

impl OpdsBrowserState {
    pub fn new() -> Self {
        Self {
            client: OpdsClient::new(),
            active_catalog_id: None,
            current_url: None,
            history: Vec::new(),
            current_feed: None,
            is_loading: false,
            error_message: None,
            search_query: String::new(),
            downloads: HashMap::new(),
            new_download_to_open: None,
            show_add_catalog: false,
            new_name: String::new(),
            new_url: String::new(),
            new_has_auth: false,
            new_user: String::new(),
            new_pass: String::new(),
            add_error: None,
            expanded_summaries: HashSet::new(),
        }
    }

    pub fn poll(&mut self, _settings: &mut AppSettings) {
        let events = self.client.poll_events();
        for event in events {
            match event {
                OpdsEvent::FeedLoaded { url, result, .. } => {
                    self.is_loading = false;
                    match result {
                        Ok(feed) => {
                            self.current_url = Some(url);
                            self.current_feed = Some(feed);
                            self.error_message = None;
                        }
                        Err(e) => {
                            self.error_message = Some(e);
                        }
                    }
                }
                OpdsEvent::DownloadProgress {
                    download_id,
                    downloaded,
                    total,
                } => {
                    if let Some(item) = self.downloads.get_mut(&download_id) {
                        item.status = DownloadStatus::Downloading { downloaded, total };
                    }
                }
                OpdsEvent::DownloadFinished {
                    download_id,
                    result,
                } => {
                    if let Some(item) = self.downloads.get_mut(&download_id) {
                        match result {
                            Ok(path) => {
                                item.status = DownloadStatus::Completed {
                                    file_path: path.clone(),
                                };
                                self.new_download_to_open = Some(path);
                            }
                            Err(e) => {
                                item.status = DownloadStatus::Failed(e);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn open_catalog(&mut self, catalog: &OpdsCatalog) {
        self.active_catalog_id = Some(catalog.id.clone());
        self.history.clear();
        self.is_loading = true;
        self.error_message = None;
        let auth = match (&catalog.username, &catalog.password) {
            (Some(u), Some(p)) if !u.is_empty() => Some((u.clone(), p.clone())),
            _ => None,
        };
        self.client.fetch_feed(&catalog.url, auth);
    }

    pub fn navigate_to(&mut self, url: &str, settings: &AppSettings) {
        if let Some(cur) = self.current_url.clone() {
            self.history.push(cur);
        }
        self.is_loading = true;
        self.error_message = None;
        let auth = self.active_auth(settings);
        self.client.fetch_feed(url, auth);
    }

    pub fn go_back(&mut self, settings: &AppSettings) {
        if let Some(prev) = self.history.pop() {
            self.is_loading = true;
            self.error_message = None;
            let auth = self.active_auth(settings);
            self.client.fetch_feed(&prev, auth);
        }
    }

    fn active_auth(&self, settings: &AppSettings) -> Option<(String, String)> {
        let cat_id = self.active_catalog_id.as_deref()?;
        let cat = settings.opds_catalogs.iter().find(|c| c.id == cat_id)?;
        match (&cat.username, &cat.password) {
            (Some(u), Some(p)) if !u.is_empty() => Some((u.clone(), p.clone())),
            _ => None,
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        show_window: &mut bool,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_open_file: &mut Option<PathBuf>,
    ) {
        if !*show_window {
            return;
        }

        if let Some(opened) = self.new_download_to_open.take() {
            *on_open_file = Some(opened);
        }

        let t = settings.ui_language.t();
        let bg_color = theme.panel_bg_color();
        let frame = egui::Frame::window(&ctx.global_style())
            .fill(bg_color)
            .stroke(Stroke::new(1.5, theme.accent_color().gamma_multiply(0.5)))
            .inner_margin(egui::Margin::same(12));

        let mut is_open = *show_window;

        egui::Window::new(t.opds_catalogs)
            .open(&mut is_open)
            .collapsible(false)
            .default_size([980.0, 680.0])
            .min_size([700.0, 480.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.render_catalogs_sidebar(ui, settings, theme);
                    ui.separator();
                    self.render_feed_content(ui, settings, theme);
                });
            });

        *show_window = is_open;
    }

    fn render_catalogs_sidebar(
        &mut self,
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let accent = theme.accent_color();

        ui.vertical(|ui| {
            ui.set_width(240.0);
            ui.heading(
                RichText::new(t.opds_catalogs)
                    .size(16.0)
                    .color(text_color)
                    .strong(),
            );
            ui.add_space(8.0);

            ScrollArea::vertical()
                .id_salt("opds_catalog_list")
                .max_height(ui.available_height() - 140.0)
                .show(ui, |ui| {
                    let mut catalog_to_open = None;
                    let mut catalog_to_delete = None;

                    for cat in &settings.opds_catalogs {
                        let is_active = self.active_catalog_id.as_deref() == Some(&cat.id);
                        ui.horizontal(|ui| {
                            let text = RichText::new(&cat.name).size(13.0).strong();
                            let btn = if is_active {
                                egui::Button::new(text.color(accent))
                                    .fill(accent.gamma_multiply(0.15))
                            } else {
                                egui::Button::new(text.color(text_color))
                            };

                            if ui.add_sized([180.0, 26.0], btn).clicked() {
                                catalog_to_open = Some(cat.clone());
                            }

                            if !cat.is_default
                                && ui
                                    .add(
                                        IconButton::new(Icon::Trash, "")
                                            .min_size(Vec2::new(26.0, 26.0))
                                            .color(Color32::from_rgb(220, 80, 80)),
                                    )
                                    .on_hover_text(t.opds_delete_catalog)
                                    .clicked()
                            {
                                catalog_to_delete = Some(cat.id.clone());
                            }
                        });
                        ui.add_space(3.0);
                    }

                    if let Some(cat) = catalog_to_open {
                        self.open_catalog(&cat);
                    }
                    if let Some(del_id) = catalog_to_delete {
                        settings.opds_catalogs.retain(|c| c.id != del_id);
                        if self.active_catalog_id.as_deref() == Some(&del_id) {
                            self.active_catalog_id = None;
                            self.current_feed = None;
                        }
                    }
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            if !self.show_add_catalog {
                if ui
                    .add(
                        IconButton::new(Icon::Plus, t.opds_add_catalog)
                            .min_size(Vec2::new(210.0, 28.0))
                            .center(true),
                    )
                    .clicked()
                {
                    self.show_add_catalog = true;
                    self.add_error = None;
                }
            } else {
                ui.label(RichText::new(t.opds_add_catalog).strong().color(text_color));
                ui.add_space(4.0);
                ui.label(
                    RichText::new(t.opds_catalog_name)
                        .size(11.0)
                        .color(text_color),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_name)
                        .hint_text(t.opds_catalog_name)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(t.opds_catalog_url)
                        .size(11.0)
                        .color(text_color),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_url)
                        .hint_text("http://.../opds")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(2.0);
                ui.checkbox(&mut self.new_has_auth, t.opds_auth_required);
                if self.new_has_auth {
                    ui.add_space(2.0);
                    ui.label(RichText::new(t.opds_username).size(11.0).color(text_color));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_user)
                            .hint_text(t.opds_username)
                            .desired_width(f32::INFINITY),
                    );
                    ui.label(RichText::new(t.opds_password).size(11.0).color(text_color));
                    let pass_field = egui::TextEdit::singleline(&mut self.new_pass)
                        .password(true)
                        .hint_text(t.opds_password)
                        .desired_width(f32::INFINITY);
                    ui.add(pass_field);
                }

                if let Some(err) = &self.add_error {
                    ui.label(RichText::new(err).color(Color32::RED).size(11.0));
                }

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

    fn render_feed_content(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        theme: &ReaderTheme,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let accent = theme.accent_color();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let can_go_back = !self.history.is_empty();
                if ui
                    .add_enabled(can_go_back, IconButton::new(Icon::ArrowLeft, t.opds_back))
                    .clicked()
                {
                    self.go_back(settings);
                }

                if let Some(cat_id) = &self.active_catalog_id
                    && let Some(cat) = settings.opds_catalogs.iter().find(|c| &c.id == cat_id)
                {
                    if ui.add(IconButton::new(Icon::Home, t.opds_root)).clicked() {
                        self.open_catalog(cat);
                    }
                }

                if ui
                    .add(IconButton::new(Icon::Refresh, t.opds_refresh))
                    .clicked()
                {
                    if let Some(cur) = self.current_url.clone() {
                        self.is_loading = true;
                        self.error_message = None;
                        let auth = self.active_auth(settings);
                        self.client.fetch_feed(&cur, auth);
                    }
                }

                ui.separator();

                let feed_title = self
                    .current_feed
                    .as_ref()
                    .map(|f| f.title.as_str())
                    .unwrap_or("OPDS");
                ui.label(
                    RichText::new(feed_title)
                        .strong()
                        .size(14.0)
                        .color(text_color),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let has_search = self
                        .current_feed
                        .as_ref()
                        .map(|f| f.search_template.is_some() || f.search_descriptor_url.is_some())
                        .unwrap_or(false);
                    if has_search {
                        let search_res = ui.add(IconButton::new(Icon::Search, t.opds_search_btn));
                        let enter_pressed = ui
                            .add(
                                egui::TextEdit::singleline(&mut self.search_query)
                                    .hint_text(t.opds_search_placeholder)
                                    .desired_width(170.0),
                            )
                            .lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        if (search_res.clicked() || enter_pressed)
                            && !self.search_query.trim().is_empty()
                        {
                            if let Some(feed) = &self.current_feed {
                                if let Some(cur) = self.current_url.clone() {
                                    self.history.push(cur);
                                }
                                self.is_loading = true;
                                self.error_message = None;
                                let auth = self.active_auth(settings);
                                self.client.search(
                                    feed.search_template.clone(),
                                    feed.search_descriptor_url.clone(),
                                    self.search_query.trim(),
                                    auth,
                                );
                            }
                        }
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            self.render_downloads_banner(ui, theme);

            if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new(t.loading_doc).size(16.0).color(accent));
                });
                return;
            }

            if let Some(err) = self.error_message.clone() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(
                            IconLabel::new(Icon::Warning, &err)
                                .color(Color32::RED)
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(8.0);
                        if ui
                            .add(IconButton::new(Icon::Refresh, t.opds_refresh))
                            .clicked()
                        {
                            if let Some(cur) = self.current_url.clone() {
                                self.is_loading = true;
                                self.error_message = None;
                                let auth = self.active_auth(settings);
                                self.client.fetch_feed(&cur, auth);
                            }
                        }
                    });
                });
                return;
            }

            let Some(feed) = self.current_feed.clone() else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("Выберите каталог слева для просмотра")
                            .size(14.0)
                            .color(text_color.gamma_multiply(0.7)),
                    );
                });
                return;
            };

            if feed.entries.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(t.opds_empty_feed)
                            .size(14.0)
                            .color(text_color.gamma_multiply(0.7)),
                    );
                });
                return;
            }

            let mut nav_url_to_open = None;

            ScrollArea::vertical()
                .id_salt("opds_entries_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in &feed.entries {
                        if let Some(nav_url) = &entry.navigation_link {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        IconButton::new(Icon::Folder, &entry.title)
                                            .font_size(13.0)
                                            .color(accent),
                                    )
                                    .clicked()
                                {
                                    nav_url_to_open = Some(nav_url.clone());
                                }
                                if let Some(summary) = &entry.summary {
                                    ui.label(
                                        RichText::new(summary)
                                            .size(11.0)
                                            .color(text_color.gamma_multiply(0.7)),
                                    );
                                }
                            });
                            ui.add_space(4.0);
                        } else {
                            self.render_book_entry(ui, entry, settings, theme);
                            ui.add_space(8.0);
                        }
                    }

                    if feed.prev_url.is_some() || feed.next_url.is_some() {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if let Some(p) = &feed.prev_url {
                                if ui
                                    .add(IconButton::new(Icon::ArrowLeft, t.opds_prev_page))
                                    .clicked()
                                {
                                    nav_url_to_open = Some(p.clone());
                                }
                            }
                            if let Some(n) = &feed.next_url {
                                if ui
                                    .add(IconButton::new(Icon::ArrowRight, t.opds_next_page))
                                    .clicked()
                                {
                                    nav_url_to_open = Some(n.clone());
                                }
                            }
                        });
                        ui.add_space(10.0);
                    }
                });

            if let Some(u) = nav_url_to_open {
                self.navigate_to(&u, settings);
            }
        });
    }

    fn render_downloads_banner(&mut self, ui: &mut egui::Ui, theme: &ReaderTheme) {
        if self.downloads.is_empty() {
            return;
        }

        let active_downloads: Vec<_> = self
            .downloads
            .values()
            .filter(|d| {
                matches!(
                    d.status,
                    DownloadStatus::Downloading { .. } | DownloadStatus::Queued
                )
            })
            .cloned()
            .collect();

        if active_downloads.is_empty() {
            return;
        }

        egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.4)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                for item in active_downloads {
                    ui.horizontal(|ui| match item.status {
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
                                    "{} [{}]: {:.1} MB / {:.1} MB ({:.0}%)",
                                    item.title,
                                    item.format_label,
                                    mb_dl,
                                    mb_tot,
                                    pct * 100.0
                                )
                            } else {
                                format!("{} [{}]: {:.1} MB", item.title, item.format_label, mb_dl)
                            };
                            ui.add(
                                IconLabel::new(Icon::Download, &info)
                                    .color(theme.accent_color())
                                    .size(12.0)
                                    .strong(),
                            );
                            if total.is_some() {
                                ui.add(egui::ProgressBar::new(pct).desired_width(120.0));
                            }
                        }
                        DownloadStatus::Queued => {
                            let text = format!("В очереди: {}", item.title);
                            ui.add(IconLabel::new(Icon::Recent, &text).size(12.0));
                        }
                        _ => {}
                    });
                }
            });
        ui.add_space(4.0);
    }

    fn render_book_entry(
        &mut self,
        ui: &mut egui::Ui,
        entry: &OpdsEntry,
        settings: &AppSettings,
        theme: &ReaderTheme,
    ) {
        let text_color = theme.text_color();
        let accent = theme.accent_color();

        let card_frame = egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.2)))
            .corner_radius(6.0)
            .inner_margin(egui::Margin::same(8));

        card_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&entry.title)
                            .size(14.0)
                            .strong()
                            .color(text_color),
                    );

                    if !entry.authors.is_empty() {
                        let authors_str = entry.authors.join(", ");
                        ui.label(RichText::new(authors_str).size(12.0).color(accent));
                    }

                    if !entry.categories.is_empty() {
                        let tags = entry.categories.join(" • ");
                        ui.label(
                            RichText::new(tags)
                                .size(10.0)
                                .color(text_color.gamma_multiply(0.6)),
                        );
                    }

                    if let Some(summary) = &entry.summary {
                        let is_expanded = self.expanded_summaries.contains(&entry.id);
                        if summary.len() > 180 && !is_expanded {
                            let truncated: String = summary.chars().take(180).collect();
                            ui.label(
                                RichText::new(format!("{truncated}..."))
                                    .size(11.0)
                                    .color(text_color.gamma_multiply(0.8)),
                            );
                            if ui
                                .add(
                                    IconButton::new(Icon::ChevronDown, "Развернуть")
                                        .font_size(11.0),
                                )
                                .clicked()
                            {
                                self.expanded_summaries.insert(entry.id.clone());
                            }
                        } else {
                            ui.label(
                                RichText::new(summary)
                                    .size(11.0)
                                    .color(text_color.gamma_multiply(0.8)),
                            );
                            if summary.len() > 180
                                && ui
                                    .add(
                                        IconButton::new(Icon::ChevronUp, "Свернуть")
                                            .font_size(11.0),
                                    )
                                    .clicked()
                            {
                                self.expanded_summaries.remove(&entry.id);
                            }
                        }
                    }

                    if !entry.acquisition_links.is_empty() {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            for acq in &entry.acquisition_links {
                                let btn = IconButton::new(Icon::Download, &acq.format_label)
                                    .font_size(12.0)
                                    .color(accent);
                                if ui.add(btn).clicked() {
                                    self.start_download(entry, acq, settings);
                                }
                            }
                        });
                    }
                });
            });
        });
    }

    fn start_download(
        &mut self,
        entry: &OpdsEntry,
        acq: &OpdsAcquisitionLink,
        settings: &AppSettings,
    ) {
        let download_id = format!("{}_{}", entry.id, acq.format_label);
        self.downloads.insert(
            download_id.clone(),
            DownloadItem {
                book_id: entry.id.clone(),
                title: entry.title.clone(),
                author: entry.authors.first().cloned().unwrap_or_default(),
                format_label: acq.format_label.clone(),
                status: DownloadStatus::Queued,
            },
        );

        let downloads_dir = AppSettings::get_settings_path()
            .parent()
            .map(|p| p.join("downloads"))
            .unwrap_or_else(|| PathBuf::from("downloads"));

        let auth = self.active_auth(settings);
        let author = entry.authors.first().cloned().unwrap_or_default();
        self.client.download(
            &download_id,
            &acq.href,
            &entry.title,
            &author,
            &acq.extension,
            auth,
            downloads_dir,
        );
    }
}
