use crate::app::icons::{Icon, IconButton, IconLabel, paint_icon};
use crate::app::opds_covers::OpdsCoverCache;
use crate::app::theme::ReaderTheme;
use crate::core::license::ProFeature;
use crate::i18n::{format_opds_auth_error, format_opds_error};
use crate::opds::{
    DownloadItem, DownloadStatus, OpdsAcquisitionLink, OpdsAuth, OpdsAuthError, OpdsBreadcrumb,
    OpdsCatalog, OpdsClient, OpdsEntry, OpdsEvent, OpdsFeed,
};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Rect, RichText, ScrollArea, Stroke, StrokeKind, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct OpdsBrowserState {
    pub client: OpdsClient,
    pub cover_manager: OpdsCoverCache,
    pub active_catalog_id: Option<String>,
    pub current_url: Option<String>,
    pub breadcrumbs: Vec<OpdsBreadcrumb>,
    pub history: Vec<(String, Vec<OpdsBreadcrumb>)>,
    pub current_feed: Option<OpdsFeed>,
    pub pending_request_id: Option<u64>,
    pub last_requested_url: Option<String>,
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
            cover_manager: OpdsCoverCache::new(),
            active_catalog_id: None,
            current_url: None,
            breadcrumbs: Vec::new(),
            history: Vec::new(),
            current_feed: None,
            pending_request_id: None,
            last_requested_url: None,
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

    pub(crate) fn apply_feed_event(
        &mut self,
        request_id: u64,
        url: String,
        result: Result<OpdsFeed, crate::opds::OpdsError>,
        ui_lang: crate::i18n::UiLanguage,
    ) -> bool {
        if self.pending_request_id != Some(request_id) {
            return false;
        }
        self.pending_request_id = None;
        self.is_loading = false;
        match result {
            Ok(feed) => {
                self.current_url = Some(url.clone());
                self.last_requested_url = Some(url);
                self.current_feed = Some(feed);
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format_opds_error(&e, ui_lang));
            }
        }
        true
    }

    pub fn cancel_loading(&mut self) {
        self.pending_request_id = None;
        self.is_loading = false;
        if let Some((_, prev_bc)) = self.history.last() {
            if self.current_feed.is_some()
                && self.current_url.as_deref() != self.breadcrumbs.last().map(|b| b.url.as_str())
            {
                self.breadcrumbs = prev_bc.clone();
                self.history.pop();
            }
        }
    }

    pub fn poll(&mut self, settings: &mut AppSettings) {
        let events = self.client.poll_events();
        for event in events {
            match event {
                OpdsEvent::FeedLoaded {
                    request_id,
                    url,
                    result,
                } => {
                    self.apply_feed_event(request_id, url, result, settings.ui_language);
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
                                item.status = DownloadStatus::Failed(format_opds_error(
                                    &e,
                                    settings.ui_language,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn open_catalog(&mut self, catalog: &OpdsCatalog, settings: &AppSettings) {
        self.active_catalog_id = Some(catalog.id.clone());
        self.history.clear();
        self.breadcrumbs = vec![OpdsBreadcrumb {
            title: catalog.name.clone(),
            url: catalog.url.clone(),
        }];
        self.search_query.clear();
        self.error_message = None;
        self.last_requested_url = Some(catalog.url.clone());
        match OpdsAuth::from_catalog(catalog) {
            Ok(auth) => {
                self.is_loading = true;
                self.pending_request_id = Some(self.client.fetch_feed(&catalog.url, auth));
            }
            Err(err) => {
                self.pending_request_id = None;
                self.is_loading = false;
                self.error_message = Some(format_opds_auth_error(err, settings.ui_language));
            }
        }
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
        self.last_requested_url = Some(url.to_string());
        let auth = match self.active_auth(settings) {
            Ok(auth) => auth,
            Err(err) => {
                self.pending_request_id = None;
                self.is_loading = false;
                self.error_message = Some(format_opds_auth_error(err, settings.ui_language));
                return;
            }
        };
        self.is_loading = true;
        self.error_message = None;
        self.pending_request_id = Some(self.client.fetch_feed(url, auth));
    }

    pub(crate) fn active_auth(
        &self,
        settings: &AppSettings,
    ) -> Result<Option<OpdsAuth>, OpdsAuthError> {
        let Some(cat_id) = self.active_catalog_id.as_deref() else {
            return Ok(None);
        };
        let Some(cat) = settings.opds_catalogs.iter().find(|c| c.id == cat_id) else {
            return Ok(None);
        };
        OpdsAuth::from_catalog(cat)
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        show_window: &mut bool,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        on_open_file: &mut Option<PathBuf>,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        if !*show_window {
            return;
        }

        self.cover_manager.poll(ctx);

        if !settings.can_use(ProFeature::OpdsPro)
            && let Some(active_id) = self.active_catalog_id.clone()
            && let Some(catalog) = settings
                .opds_catalogs
                .iter()
                .find(|catalog| catalog.id == active_id)
            && !catalog.is_default
        {
            self.active_catalog_id = None;
            self.current_url = None;
            self.last_requested_url = None;
            self.current_feed = None;
            self.pending_request_id = None;
            self.cover_manager.clear();
            self.breadcrumbs.clear();
            self.history.clear();
            self.is_loading = false;
            self.error_message = None;
            self.search_query.clear();
            self.show_add_catalog = false;
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

        let screen_rect = ctx.content_rect();
        let avail_w = screen_rect.width().max(280.0);
        let avail_h = screen_rect.height().max(240.0);
        let max_w = (avail_w - 8.0).clamp(240.0, 1560.0);
        let max_h = (avail_h - 8.0).max(180.0);
        let min_w = (max_w - 16.0).clamp(240.0, 420.0);
        let min_h = (max_h - 16.0).clamp(180.0, 280.0);
        let default_w = (avail_w * 0.90).clamp(min_w, max_w);
        let default_h = (avail_h * 0.85).clamp(min_h, max_h);
        let default_pos = screen_rect.center() - Vec2::new(default_w * 0.5, default_h * 0.5);

        egui::Window::new(t.opds_catalogs)
            .id(egui::Id::unique("opds_storefront_window"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(true)
            .default_size([default_w, default_h])
            .default_pos([
                default_pos.x.max(screen_rect.left() + 4.0),
                default_pos.y.max(screen_rect.top() + 4.0),
            ])
            .min_size([min_w, min_h])
            .max_size([max_w, max_h])
            .constrain_to(screen_rect.shrink(4.0))
            .frame(frame)
            .show(ctx, |ui| {
                let total_w = ui.available_width();
                if total_w >= 640.0 {
                    let sidebar_w = (total_w * 0.23).clamp(180.0, 270.0);
                    ui.allocate_ui_with_layout(
                        ui.available_size(),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            self.render_catalogs_sidebar(
                                ui,
                                sidebar_w,
                                false,
                                settings,
                                theme,
                                on_pro_required,
                            );
                            ui.separator();
                            self.render_feed_content(ctx, ui, settings, theme);
                        },
                    );
                } else {
                    ui.vertical(|ui| {
                        self.render_catalogs_sidebar(
                            ui,
                            total_w,
                            true,
                            settings,
                            theme,
                            on_pro_required,
                        );
                        ui.separator();
                        self.render_feed_content(ctx, ui, settings, theme);
                    });
                }
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
            let feed_w = ui.available_width().max(180.0);
            let feed_h = ui.available_height().max(80.0);
            ui.set_width(feed_w);
            ui.set_height(feed_h);

            let search_w = if feed_w < 460.0 {
                (feed_w - 84.0).clamp(110.0, 220.0)
            } else {
                (feed_w * 0.28).clamp(140.0, 280.0)
            };

            ui.horizontal_wrapped(|ui| {
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

                let has_remote_search = self
                    .current_feed
                    .as_ref()
                    .map(|f| f.search_template.is_some() || f.search_descriptor_url.is_some())
                    .unwrap_or(false);

                let enter_pressed = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.search_query)
                            .hint_text(t.opds_search_placeholder)
                            .desired_width(search_w),
                    )
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));

                if !self.search_query.is_empty()
                    && ui
                        .add(IconButton::new(Icon::Cross, ""))
                        .on_hover_text(t.extra.clear_search)
                        .clicked()
                {
                    self.search_query.clear();
                }

                let search_btn = ui.add(IconButton::new(Icon::Search, t.opds_search_btn));

                if (search_btn.clicked() || enter_pressed)
                    && !self.search_query.trim().is_empty()
                    && has_remote_search
                {
                    if let Some(feed) = &self.current_feed {
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
                                    feed.search_template.clone(),
                                    feed.search_descriptor_url.clone(),
                                    self.search_query.trim(),
                                    auth,
                                ));
                            }
                            Err(err) => {
                                self.pending_request_id = None;
                                self.is_loading = false;
                                self.error_message =
                                    Some(format_opds_auth_error(err, settings.ui_language));
                            }
                        }
                    }
                }
            });

            if !self.breadcrumbs.is_empty() {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    let mut clicked_crumb = None;
                    for (idx, crumb) in self.breadcrumbs.iter().enumerate() {
                        if idx > 0 {
                            ui.add(
                                IconLabel::new(Icon::ArrowRight, "")
                                    .size(14.0)
                                    .color(sec_text),
                            );
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
                });
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            self.render_downloads_banner(ui, theme, settings.ui_language);

            if self.is_loading {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
                            paint_icon(ui.painter(), Icon::Refresh, icon_rect, accent);
                            ui.label(RichText::new(t.loading_doc).size(15.0).color(accent));
                        });
                        ui.add_space(8.0);
                        if ui
                            .add(IconButton::new(Icon::Cross, t.opds_cancel_btn))
                            .clicked()
                        {
                            self.cancel_loading();
                        }
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
                            if let Some(cur) = self
                                .last_requested_url
                                .clone()
                                .or_else(|| self.current_url.clone())
                            {
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
                        let content_w = (ui.available_width() - 4.0).max(160.0);
                        let max_grid_w = content_w.min(1360.0);
                        if content_w >= 920.0 && books.len() > 1 {
                            let col_w = ((max_grid_w - 8.0) * 0.5).max(200.0);
                            for row in books.chunks(2) {
                                ui.horizontal_top(|ui| {
                                    for book in row {
                                        self.render_book_card(
                                            ctx,
                                            ui,
                                            book,
                                            col_w,
                                            settings,
                                            theme,
                                            &mut nav_url_to_open,
                                        );
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        } else {
                            let col_w = max_grid_w;
                            for book in books {
                                self.render_book_card(
                                    ctx,
                                    ui,
                                    book,
                                    col_w,
                                    settings,
                                    theme,
                                    &mut nav_url_to_open,
                                );
                                ui.add_space(8.0);
                            }
                        }
                    }

                    if feed.prev_url.is_some() || feed.next_url.is_some() {
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
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

    fn render_book_card(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        entry: &OpdsEntry,
        card_width: f32,
        settings: &AppSettings,
        theme: &ReaderTheme,
        nav_url_to_open: &mut Option<(String, String)>,
    ) {
        let t = settings.ui_language.t();
        let text_color = theme.text_color();
        let sec_text = theme.secondary_text_color();
        let accent = theme.accent_color();
        let inner_w = (card_width - 20.0).max(160.0);

        let card_frame = egui::Frame::new()
            .fill(theme.bg_color())
            .stroke(Stroke::new(1.0, accent.gamma_multiply(0.2)))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(10));

        card_frame.show(ui, |ui| {
            ui.set_width(inner_w);
            ui.horizontal_top(|ui| {
                let cover_size = if card_width < 340.0 {
                    Vec2::new(52.0, 76.0)
                } else {
                    Vec2::new(72.0, 104.0)
                };
                let auth = self.active_auth(settings).ok().flatten();
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
                    ui.set_max_width(ui.available_width().max(80.0));
                    ui.add(
                        egui::Label::new(
                            RichText::new(&entry.title)
                                .size(14.0)
                                .strong()
                                .color(text_color),
                        )
                        .wrap(),
                    );

                    if !entry.authors.is_empty() {
                        let authors_str = entry.authors.join(", ");
                        ui.add(
                            egui::Label::new(RichText::new(authors_str).size(12.0).color(accent))
                                .wrap(),
                        );
                    }

                    if !entry.categories.is_empty() {
                        ui.horizontal_wrapped(|ui| {
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
                        let truncated_opt = truncate_summary_chars(summary, 180);
                        if let Some(truncated) = truncated_opt
                            && !is_expanded
                        {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(truncated).size(11.0).color(sec_text),
                                )
                                .wrap(),
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
                            let can_collapse = summary_exceeds_chars(summary, 180);
                            ui.add(
                                egui::Label::new(RichText::new(summary).size(11.0).color(sec_text))
                                    .wrap(),
                            );
                            if can_collapse
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

                    if !entry.acquisition_links.is_empty() || entry.navigation_link.is_some() {
                        ui.add_space(4.0);
                        let mut failed_errors: Vec<(String, String)> = Vec::new();
                        ui.horizontal_wrapped(|ui| {
                            for acq in &entry.acquisition_links {
                                let download_id = format!("{}_{}", entry.id, acq.format_label);
                                let item_opt = self.downloads.get(&download_id);

                                match item_opt.map(|i| &i.status) {
                                    Some(DownloadStatus::Queued) => {
                                        let queued_text = crate::i18n::fill(
                                            t.extra.queued_download,
                                            &[("{title}", &acq.format_label)],
                                        );
                                        ui.add(
                                            IconLabel::new(Icon::Recent, &queued_text)
                                                .size(11.5)
                                                .color(sec_text),
                                        );
                                    }
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
                                    Some(DownloadStatus::Failed(err)) => {
                                        failed_errors.push((acq.format_label.clone(), err.clone()));
                                        let retry_text =
                                            format!("{} ({})", t.opds_retry, acq.format_label);
                                        let btn = IconButton::new(Icon::Refresh, &retry_text)
                                            .font_size(11.5)
                                            .color(Color32::RED);
                                        if ui.add(btn).clicked() {
                                            self.start_download(entry, acq, settings);
                                        }
                                    }
                                    None => {
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
                            if let Some(nav_link) = &entry.navigation_link {
                                let nav_btn =
                                    IconButton::new(Icon::Folder, t.extra.opds_open_section)
                                        .font_size(11.5)
                                        .color(accent);
                                if ui.add(nav_btn).clicked() {
                                    *nav_url_to_open =
                                        Some((entry.title.clone(), nav_link.clone()));
                                }
                            }
                        });
                        for (fmt_label, err) in failed_errors {
                            ui.add(
                                IconLabel::new(Icon::Warning, &format!("[{fmt_label}] {err}"))
                                    .color(Color32::RED)
                                    .size(11.0),
                            );
                        }
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

        let downloads_dir = crate::platform::downloads::downloads_dir();

        let auth = match self.active_auth(settings) {
            Ok(auth) => auth,
            Err(err) => {
                if let Some(item) = self.downloads.get_mut(&download_id) {
                    item.status =
                        DownloadStatus::Failed(format_opds_auth_error(err, settings.ui_language));
                }
                return;
            }
        };
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

pub(crate) fn summary_exceeds_chars(summary: &str, max_chars: usize) -> bool {
    summary.chars().nth(max_chars).is_some()
}

pub(crate) fn truncate_summary_chars(summary: &str, max_chars: usize) -> Option<String> {
    let mut chars = summary.chars();
    let prefix: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        Some(format!("{prefix}..."))
    } else {
        None
    }
}
