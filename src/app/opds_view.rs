use crate::app::icons::{Icon, IconButton, IconLabel, paint_icon};
use crate::app::theme::ReaderTheme;
use crate::opds::{
    DownloadItem, DownloadStatus, OpdsAcquisitionLink, OpdsBreadcrumb, OpdsCatalog, OpdsClient,
    OpdsCoverManager, OpdsEntry, OpdsEvent, OpdsFeed,
};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Rect, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct OpdsBrowserState {
    pub client: OpdsClient,
    pub cover_manager: OpdsCoverManager,
    pub active_catalog_id: Option<String>,
    pub current_url: Option<String>,
    pub breadcrumbs: Vec<OpdsBreadcrumb>,
    pub history: Vec<(String, Vec<OpdsBreadcrumb>)>,
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
            cover_manager: OpdsCoverManager::new(),
            active_catalog_id: None,
            current_url: None,
            breadcrumbs: Vec::new(),
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
        self.breadcrumbs = vec![OpdsBreadcrumb {
            title: catalog.name.clone(),
            url: catalog.url.clone(),
        }];
        self.search_query.clear();
        self.is_loading = true;
        self.error_message = None;
        let auth = match (&catalog.username, &catalog.password) {
            (Some(u), Some(p)) if !u.is_empty() => Some((u.clone(), p.clone())),
            _ => None,
        };
        self.client.fetch_feed(&catalog.url, auth);
    }

    pub fn navigate_to(&mut self, title: &str, url: &str, settings: &AppSettings) {
        if let Some(cur) = self.current_url.clone() {
            self.history.push((cur, self.breadcrumbs.clone()));
        }
        self.breadcrumbs.push(OpdsBreadcrumb {
            title: title.to_string(),
            url: url.to_string(),
        });
        self.fetch_url(url, settings);
    }

    pub fn navigate_to_crumb(&mut self, crumb_index: usize, settings: &AppSettings) {
        if crumb_index >= self.breadcrumbs.len() {
            return;
        }
        if let Some(cur) = self.current_url.clone() {
            self.history.push((cur, self.breadcrumbs.clone()));
        }
        let target = self.breadcrumbs[crumb_index].clone();
        self.breadcrumbs.truncate(crumb_index + 1);
        self.fetch_url(&target.url, settings);
    }

    pub fn go_back(&mut self, settings: &AppSettings) {
        if let Some((prev_url, prev_bc)) = self.history.pop() {
            self.breadcrumbs = prev_bc;
            self.fetch_url(&prev_url, settings);
        }
    }

    fn fetch_url(&mut self, url: &str, settings: &AppSettings) {
        self.is_loading = true;
        self.error_message = None;
        let auth = self.active_auth(settings);
        self.client.fetch_feed(url, auth);
    }

    pub(crate) fn active_auth(&self, settings: &AppSettings) -> Option<(String, String)> {
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

        self.cover_manager.poll(ctx);

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

        let screen_rect = ctx
            .input(|i| i.viewport().inner_rect)
            .unwrap_or(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1280.0, 800.0),
            ));
        let default_w = (screen_rect.width() * 0.88).clamp(980.0, 1440.0);
        let default_h = (screen_rect.height() * 0.86).clamp(640.0, 960.0);
        let default_pos = screen_rect.center() - Vec2::new(default_w * 0.5, default_h * 0.5);

        egui::Window::new(t.opds_catalogs)
            .id(egui::Id::unique("opds_storefront_window"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_size([default_w, default_h])
            .default_pos([default_pos.x.max(10.0), default_pos.y.max(10.0)])
            .min_size([720.0, 480.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        self.render_catalogs_sidebar(ui, settings, theme);
                        ui.separator();
                        self.render_feed_content(ctx, ui, settings, theme);
                    },
                );
            });

        *show_window = is_open;
    }

    fn render_feed_content(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        theme: &ReaderTheme,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();

        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());
            ui.horizontal(|ui| {
                let can_go_back = !self.history.is_empty();
                if ui
                    .add_enabled(can_go_back, IconButton::new(Icon::ArrowLeft, t.opds_back))
                    .clicked()
                {
                    self.go_back(settings);
                }

                if ui
                    .add(IconButton::new(Icon::Refresh, t.opds_refresh))
                    .clicked()
                {
                    if let Some(cur) = self.current_url.clone() {
                        self.fetch_url(&cur, settings);
                    }
                }

                ui.separator();

                let mut clicked_crumb = None;
                for (idx, crumb) in self.breadcrumbs.iter().enumerate() {
                    if idx > 0 {
                        ui.label(RichText::new("›").size(14.0).color(sec_text));
                    }
                    let is_last = idx == self.breadcrumbs.len() - 1;
                    let crumb_btn = egui::Button::new(
                        RichText::new(&crumb.title)
                            .size(13.0)
                            .strong()
                            .color(if is_last { text_color } else { accent }),
                    )
                    .frame(false);
                    if ui.add(crumb_btn).clicked() && !is_last {
                        clicked_crumb = Some(idx);
                    }
                }
                if let Some(idx) = clicked_crumb {
                    self.navigate_to_crumb(idx, settings);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let has_remote_search = self
                        .current_feed
                        .as_ref()
                        .map(|f| f.search_template.is_some() || f.search_descriptor_url.is_some())
                        .unwrap_or(false);

                    let search_btn = ui.add(IconButton::new(Icon::Search, t.opds_search_btn));
                    if !self.search_query.is_empty()
                        && ui
                            .add(IconButton::new(Icon::Cross, ""))
                            .on_hover_text("Очистить поиск")
                            .clicked()
                    {
                        self.search_query.clear();
                    }

                    let enter_pressed = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text(t.opds_search_placeholder)
                                .desired_width(180.0),
                        )
                        .lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if (search_btn.clicked() || enter_pressed)
                        && !self.search_query.trim().is_empty()
                        && has_remote_search
                    {
                        if let Some(feed) = &self.current_feed {
                            if let Some(cur) = self.current_url.clone() {
                                self.history.push((cur, self.breadcrumbs.clone()));
                            }
                            self.breadcrumbs.push(OpdsBreadcrumb {
                                title: format!("🔍 {}", self.search_query.trim()),
                                url: String::new(),
                            });
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
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            self.render_downloads_banner(ui, theme);

            if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                        paint_icon(ui.painter(), Icon::Refresh, icon_rect, accent);
                        ui.label(RichText::new(t.loading_doc).size(15.0).color(accent));
                    });
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
                            .add(IconButton::new(Icon::Refresh, t.opds_retry))
                            .clicked()
                        {
                            if let Some(cur) = self.current_url.clone() {
                                self.fetch_url(&cur, settings);
                            }
                        }
                    });
                });
                return;
            }

            let Some(feed) = self.current_feed.clone() else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(t.opds_select_catalog)
                            .size(14.0)
                            .color(sec_text),
                    );
                });
                return;
            };

            let query = self.search_query.trim().to_lowercase();
            let filtered_entries: Vec<_> = feed
                .entries
                .iter()
                .filter(|e| {
                    if query.is_empty() {
                        true
                    } else {
                        e.title.to_lowercase().contains(&query)
                            || e.authors.iter().any(|a| a.to_lowercase().contains(&query))
                            || e.categories
                                .iter()
                                .any(|c| c.to_lowercase().contains(&query))
                            || e.summary
                                .as_ref()
                                .map(|s| s.to_lowercase().contains(&query))
                                .unwrap_or(false)
                    }
                })
                .collect();

            if filtered_entries.is_empty() {
                ui.centered_and_justified(|ui| {
                    let msg = if query.is_empty() {
                        t.opds_empty_feed
                    } else {
                        t.opds_no_results
                    };
                    ui.label(RichText::new(msg).size(14.0).color(sec_text));
                });
                return;
            }

            let mut nav_url_to_open = None;

            let categories: Vec<_> = filtered_entries
                .iter()
                .filter(|e| e.navigation_link.is_some() && e.acquisition_links.is_empty())
                .cloned()
                .collect();

            let books: Vec<_> = filtered_entries
                .iter()
                .filter(|e| !e.acquisition_links.is_empty() || e.navigation_link.is_none())
                .cloned()
                .collect();

            ScrollArea::vertical()
                .id_salt("opds_entries_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if !categories.is_empty() {
                        ui.label(
                            RichText::new(t.opds_categories)
                                .size(13.0)
                                .color(accent)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        self.render_categories_grid(ui, &categories, theme, &mut nav_url_to_open);
                        ui.add_space(10.0);
                        if !books.is_empty() {
                            ui.separator();
                            ui.add_space(8.0);
                        }
                    }

                    if !books.is_empty() {
                        if !categories.is_empty() {
                            ui.label(
                                RichText::new(t.opds_books)
                                    .size(13.0)
                                    .color(accent)
                                    .strong(),
                            );
                            ui.add_space(6.0);
                        }
                        for book in books {
                            self.render_book_card(ctx, ui, book, settings, theme);
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
                                    nav_url_to_open =
                                        Some((t.opds_prev_page.to_string(), p.clone()));
                                }
                            }
                            if let Some(n) = &feed.next_url {
                                if ui
                                    .add(IconButton::new(Icon::ArrowRight, t.opds_next_page))
                                    .clicked()
                                {
                                    nav_url_to_open =
                                        Some((t.opds_next_page.to_string(), n.clone()));
                                }
                            }
                        });
                        ui.add_space(10.0);
                    }
                });

            if let Some((nav_title, nav_url)) = nav_url_to_open {
                self.navigate_to(&nav_title, &nav_url, settings);
            }
        });
    }

    fn render_categories_grid(
        &mut self,
        ui: &mut egui::Ui,
        categories: &[&OpdsEntry],
        theme: &ReaderTheme,
        nav_url_to_open: &mut Option<(String, String)>,
    ) {
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();

        let avail_w = ui.available_width() - 8.0;
        let cols = if avail_w > 720.0 { 3 } else { 2 };
        let card_w = (avail_w - (cols as f32 - 1.0) * 8.0) / (cols as f32);

        for row in categories.chunks(cols) {
            ui.horizontal(|ui| {
                for entry in row {
                    let card_frame = egui::Frame::new()
                        .fill(theme.bg_color())
                        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.2)))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(10, 8));

                    let resp = card_frame.show(ui, |ui| {
                        ui.set_width(card_w);
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

    fn render_book_card(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        entry: &OpdsEntry,
        settings: &AppSettings,
        theme: &ReaderTheme,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();

        let card_frame = egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(Stroke::new(1.0, accent.gamma_multiply(0.2)))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(10));

        card_frame.show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let cover_size = Vec2::new(72.0, 104.0);
                let auth = self.active_auth(settings);
                let opt_tex = entry
                    .thumbnail_url
                    .as_deref()
                    .and_then(|url| self.cover_manager.get_texture(ctx, url, auth));

                if let Some(tex) = opt_tex {
                    let img = egui::Image::from_texture(&tex)
                        .fit_to_exact_size(cover_size)
                        .corner_radius(4.0);
                    ui.add(img);
                } else {
                    let (cover_rect, _) = ui.allocate_exact_size(cover_size, egui::Sense::hover());
                    ui.painter().rect(
                        cover_rect,
                        4.0,
                        accent.gamma_multiply(0.12),
                        Stroke::new(1.0, accent.gamma_multiply(0.25)),
                        StrokeKind::Inside,
                    );
                    let icon_rect = Rect::from_center_size(cover_rect.center(), Vec2::splat(26.0));
                    paint_icon(ui.painter(), Icon::Book, icon_rect, accent);
                }

                ui.add_space(8.0);

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
                        ui.horizontal(|ui| {
                            for cat in entry.categories.iter().take(4) {
                                let badge_frame = egui::Frame::new()
                                    .fill(accent.gamma_multiply(0.08))
                                    .stroke(Stroke::new(0.8, accent.gamma_multiply(0.2)))
                                    .corner_radius(3.0)
                                    .inner_margin(egui::Margin::symmetric(4, 1));
                                badge_frame.show(ui, |ui| {
                                    ui.label(RichText::new(cat).size(9.5).color(sec_text));
                                });
                            }
                        });
                    }

                    if let Some(summary) = &entry.summary {
                        let is_expanded = self.expanded_summaries.contains(&entry.id);
                        if summary.len() > 180 && !is_expanded {
                            let truncated: String = summary.chars().take(180).collect();
                            ui.label(
                                RichText::new(format!("{truncated}..."))
                                    .size(11.0)
                                    .color(sec_text),
                            );
                            if ui
                                .add(
                                    IconButton::new(Icon::ChevronDown, t.opds_expand)
                                        .font_size(11.0)
                                        .color(accent),
                                )
                                .clicked()
                            {
                                self.expanded_summaries.insert(entry.id.clone());
                            }
                        } else {
                            ui.label(RichText::new(summary).size(11.0).color(sec_text));
                            if summary.len() > 180
                                && ui
                                    .add(
                                        IconButton::new(Icon::ChevronUp, t.opds_collapse)
                                            .font_size(11.0)
                                            .color(accent),
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
                                let download_id = format!("{}_{}", entry.id, acq.format_label);
                                let item_opt = self.downloads.get(&download_id);

                                match item_opt.map(|i| &i.status) {
                                    Some(DownloadStatus::Downloading { downloaded, total }) => {
                                        let progress_label = if let Some(tot) = total
                                            && *tot > 0
                                        {
                                            format!(
                                                "{:.0}%",
                                                (*downloaded as f32 / *tot as f32) * 100.0
                                            )
                                        } else {
                                            t.opds_downloading.to_string()
                                        };
                                        let prog_text =
                                            format!("{} {}", t.opds_downloading, progress_label);
                                        let btn = IconButton::new(Icon::Download, &prog_text)
                                            .font_size(11.5)
                                            .color(accent);
                                        ui.add(btn);
                                    }
                                    Some(DownloadStatus::Completed { file_path }) => {
                                        let done_text = format!(
                                            "{} ({})",
                                            t.opds_download_done, acq.format_label
                                        );
                                        let btn = IconButton::new(Icon::Check, &done_text)
                                            .font_size(11.5)
                                            .color(Color32::from_rgb(60, 180, 75));
                                        if ui.add(btn).clicked() {
                                            self.new_download_to_open = Some(file_path.clone());
                                        }
                                    }
                                    _ => {
                                        let dl_text = format!(
                                            "{} {}",
                                            t.opds_download_format, acq.format_label
                                        );
                                        let btn = IconButton::new(Icon::Download, &dl_text)
                                            .font_size(11.5)
                                            .color(accent);
                                        if ui.add(btn).clicked() {
                                            self.start_download(entry, acq, settings);
                                        }
                                    }
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
