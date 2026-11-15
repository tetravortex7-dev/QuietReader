mod cards;
mod feed;
mod palette;
mod sidebar;

use crate::app::opds_covers::OpdsCoverCache;
use crate::app::theme::ReaderTheme;
use crate::core::library::UiDesignMode;
use crate::core::license::ProFeature;
use crate::i18n::{Translations, format_opds_auth_error, format_opds_error};
use crate::opds::{
    DownloadItem, DownloadStatus, OpdsAcquisitionLink, OpdsAuth, OpdsAuthError, OpdsBreadcrumb,
    OpdsCatalog, OpdsClient, OpdsEntry, OpdsError, OpdsEvent, OpdsFeed,
};
use crate::settings::AppSettings;
use eframe::egui::{self, Stroke, Vec2};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpdsSortMode {
    #[default]
    CatalogDefault,
    TitleAsc,
    AuthorAsc,
    NewestFirst,
}

impl OpdsSortMode {
    pub const ALL: [Self; 4] = [
        Self::CatalogDefault,
        Self::TitleAsc,
        Self::AuthorAsc,
        Self::NewestFirst,
    ];

    pub fn label(self, t: &Translations) -> &'static str {
        match self {
            Self::CatalogDefault => t.extra.opds_sort_default,
            Self::TitleAsc => t.extra.opds_sort_title,
            Self::AuthorAsc => t.extra.opds_sort_author,
            Self::NewestFirst => t.extra.opds_sort_date,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpdsCompletedToast {
    pub download_id: String,
    pub title: String,
    pub format_label: String,
    pub file_path: PathBuf,
}

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
    pub selected_format: Option<String>,
    pub sort_mode: OpdsSortMode,
    pub completed_toast: Option<OpdsCompletedToast>,
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
            selected_format: None,
            sort_mode: OpdsSortMode::CatalogDefault,
            completed_toast: None,
        }
    }

    pub fn ensure_initial_catalog_loaded(&mut self, settings: &AppSettings) {
        if self.active_catalog_id.is_some()
            || self.current_feed.is_some()
            || self.is_loading
            || self.error_message.is_some()
        {
            return;
        }
        if let Some(first) = settings.opds_catalogs.first().cloned() {
            if first.is_default || settings.can_use(ProFeature::OpdsPro) {
                self.open_catalog(&first, settings);
            }
        }
    }

    pub(crate) fn apply_feed_event(
        &mut self,
        request_id: u64,
        url: String,
        result: Result<OpdsFeed, OpdsError>,
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
                self.selected_format = None;
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
                } => match result {
                    Ok(path) => {
                        let size_bytes = std::fs::metadata(&path).ok().map(|m| m.len());
                        if let Some(item) = self.downloads.get_mut(&download_id) {
                            item.status = DownloadStatus::Completed {
                                file_path: path.clone(),
                            };
                            item.size_bytes = size_bytes;
                            item.cancel_flag = None;
                            self.completed_toast = Some(OpdsCompletedToast {
                                download_id: download_id.clone(),
                                title: item.title.clone(),
                                format_label: item.format_label.clone(),
                                file_path: path,
                            });
                        }
                    }
                    Err(OpdsError::Cancelled) => {
                        self.downloads.remove(&download_id);
                    }
                    Err(e) => {
                        if let Some(item) = self.downloads.get_mut(&download_id) {
                            item.status = DownloadStatus::Failed(format_opds_error(
                                &e,
                                settings.ui_language,
                            ));
                            item.cancel_flag = None;
                        }
                    }
                },
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
        self.selected_format = None;
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

    pub(super) fn fetch_url(&mut self, url: &str, settings: &AppSettings) {
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

    pub fn cancel_download(&mut self, download_id: &str) {
        if let Some(item) = self.downloads.remove(download_id)
            && let Some(flag) = item.cancel_flag
        {
            flag.store(true, Ordering::SeqCst);
        }
        if self
            .completed_toast
            .as_ref()
            .is_some_and(|t| t.download_id == download_id)
        {
            self.completed_toast = None;
        }
    }

    fn enforce_pro_guard(&mut self, settings: &AppSettings) {
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
            self.selected_format = None;
            self.show_add_catalog = false;
        }
    }

    pub fn render_embedded(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        theme: &ReaderTheme,
        design: UiDesignMode,
        on_pro_required: &mut Option<ProFeature>,
    ) -> Option<PathBuf> {
        self.cover_manager.poll(ctx);
        self.enforce_pro_guard(settings);
        let mut on_open_file = self.new_download_to_open.take();
        let palette = palette::OpdsPalette::for_theme(theme, design);
        self.render_storefront_split(
            ctx,
            ui,
            settings,
            &palette,
            &mut on_open_file,
            on_pro_required,
        );
        on_open_file
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
        self.enforce_pro_guard(settings);

        if let Some(opened) = self.new_download_to_open.take() {
            *on_open_file = Some(opened);
        }

        let t = settings.ui_language.t();
        let palette = palette::OpdsPalette::for_theme(theme, UiDesignMode::Studio);
        let frame = egui::Frame::window(&ctx.global_style())
            .fill(palette.canvas_bg)
            .stroke(Stroke::new(1.5, palette.primary.gamma_multiply(0.5)))
            .inner_margin(egui::Margin::ZERO);

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
                self.render_storefront_split(
                    ctx,
                    ui,
                    settings,
                    &palette,
                    on_open_file,
                    on_pro_required,
                );
            });

        *show_window = is_open;
    }

    fn render_storefront_split(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        palette: &palette::OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
        on_pro_required: &mut Option<ProFeature>,
    ) {
        let total_w = ui.available_width();
        if total_w >= 700.0 {
            let sidebar_w = (total_w * 0.22).clamp(236.0, 290.0);
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    self.render_catalogs_sidebar(
                        ui,
                        sidebar_w,
                        false,
                        settings,
                        palette,
                        on_open_file,
                        on_pro_required,
                    );
                    self.render_feed_content(ctx, ui, settings, palette, on_open_file);
                },
            );
        } else {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                self.render_catalogs_sidebar(
                    ui,
                    total_w,
                    true,
                    settings,
                    palette,
                    on_open_file,
                    on_pro_required,
                );
                self.render_feed_content(ctx, ui, settings, palette, on_open_file);
            });
        }
    }

    pub(super) fn start_download(
        &mut self,
        entry: &OpdsEntry,
        acq: &OpdsAcquisitionLink,
        settings: &AppSettings,
    ) {
        let download_id = format!("{}_{}", entry.id, acq.format_label);
        let downloads_dir = crate::platform::downloads::downloads_dir();
        let auth = match self.active_auth(settings) {
            Ok(auth) => auth,
            Err(err) => {
                self.downloads.insert(
                    download_id,
                    DownloadItem {
                        book_id: entry.id.clone(),
                        title: entry.title.clone(),
                        author: entry.authors.first().cloned().unwrap_or_default(),
                        format_label: acq.format_label.clone(),
                        status: DownloadStatus::Failed(format_opds_auth_error(
                            err,
                            settings.ui_language,
                        )),
                        size_bytes: acq.length,
                        cancel_flag: None,
                    },
                );
                return;
            }
        };
        let author = entry.authors.first().cloned().unwrap_or_default();
        let cancel_flag = self.client.download(
            &download_id,
            &acq.href,
            &entry.title,
            &author,
            &acq.extension,
            auth,
            downloads_dir,
        );
        self.downloads.insert(
            download_id,
            DownloadItem {
                book_id: entry.id.clone(),
                title: entry.title.clone(),
                author,
                format_label: acq.format_label.clone(),
                status: DownloadStatus::Queued,
                size_bytes: acq.length,
                cancel_flag: Some(cancel_flag),
            },
        );
    }
}

pub(crate) fn collect_format_filters(feed: &OpdsFeed) -> Vec<String> {
    let mut set = BTreeSet::new();
    for entry in &feed.entries {
        for link in &entry.acquisition_links {
            set.insert(link.format_label.clone());
        }
    }
    set.into_iter().collect()
}

pub(crate) fn filter_and_sort_books<'a>(
    mut books: Vec<&'a OpdsEntry>,
    selected_format: Option<&str>,
    sort_mode: OpdsSortMode,
) -> Vec<&'a OpdsEntry> {
    if let Some(fmt) = selected_format {
        books.retain(|entry| {
            entry
                .acquisition_links
                .iter()
                .any(|link| link.format_label.eq_ignore_ascii_case(fmt))
        });
    }
    match sort_mode {
        OpdsSortMode::CatalogDefault => {}
        OpdsSortMode::TitleAsc => {
            books.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
        OpdsSortMode::AuthorAsc => {
            books.sort_by(|a, b| {
                let auth_a = a.authors.first().map(|s| s.to_lowercase()).unwrap_or_default();
                let auth_b = b.authors.first().map(|s| s.to_lowercase()).unwrap_or_default();
                auth_a
                    .cmp(&auth_b)
                    .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            });
        }
        OpdsSortMode::NewestFirst => {
            books.sort_by(|a, b| b.updated.as_deref().cmp(&a.updated.as_deref()));
        }
    }
    books
}

pub(crate) fn extract_entry_year(updated: Option<&str>) -> Option<&str> {
    let raw = updated?.trim();
    if raw.len() >= 4 {
        let prefix = &raw[..4];
        if prefix.bytes().all(|b| b.is_ascii_digit()) {
            return Some(prefix);
        }
    }
    None
}

pub(crate) fn format_byte_size(bytes: u64, t: &Translations) -> String {
    if bytes < 1024 {
        format!("{bytes} {}", t.extra.bytes_short)
    } else if bytes < 1024 * 1024 {
        let kb = (bytes as f64 / 1024.0).round() as u64;
        format!("{kb} {}", t.extra.opds_kilobytes_short)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{mb:.1} {}", t.extra.megabytes_short)
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
