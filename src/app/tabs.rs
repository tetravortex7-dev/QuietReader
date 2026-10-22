use super::icons::{Icon, IconButton, IconLabel};
use super::theme::ReaderTheme;
use crate::core::ReaderCore;
use crate::document::{Document, DocumentViewKind};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct BookTab {
    pub file_path: PathBuf,
    pub title: String,
    pub format_badge: String,
    pub document: Option<Arc<dyn Document>>,
    pub is_loading: bool,
    pub load_error: Option<String>,
    pub request_id: u64,
    pub current_chapter: usize,
    pub last_rendered_chapter: usize,
    pub current_scroll_y: f32,
    pub viewport_height: f32,
    pub max_scroll_y: f32,
    pub comic_page: usize,
}

impl BookTab {
    pub fn new_loading(file_path: PathBuf, request_id: u64, lang: crate::i18n::UiLanguage) -> Self {
        let format_badge = Self::detect_format_badge(&file_path);
        let title = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| lang.t().extra.default_document_title.to_string());

        Self {
            file_path,
            title,
            format_badge,
            document: None,
            is_loading: true,
            load_error: None,
            request_id,
            current_chapter: 0,
            last_rendered_chapter: 0,
            current_scroll_y: 0.0,
            viewport_height: 600.0,
            max_scroll_y: 0.0,
            comic_page: 0,
        }
    }

    pub fn new(
        file_path: PathBuf,
        document: Arc<dyn Document>,
        current_chapter: usize,
        current_scroll_y: f32,
        comic_page: usize,
    ) -> Self {
        let meta_title = document.metadata().title.trim().to_string();
        let title = if !meta_title.is_empty() {
            meta_title
        } else {
            file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    crate::i18n::UiLanguage::Auto
                        .t()
                        .extra
                        .default_document_title
                        .to_string()
                })
        };
        let format_badge = document.format_name().to_string();

        Self {
            file_path,
            title,
            format_badge,
            document: Some(document),
            is_loading: false,
            load_error: None,
            request_id: 0,
            current_chapter,
            last_rendered_chapter: current_chapter,
            current_scroll_y,
            viewport_height: 600.0,
            max_scroll_y: 0.0,
            comic_page,
        }
    }

    pub fn update_loaded(
        &mut self,
        document: Arc<dyn Document>,
        chapter: usize,
        scroll_y: f32,
        comic_page: usize,
    ) {
        let meta_title = document.metadata().title.trim().to_string();
        if !meta_title.is_empty() {
            self.title = meta_title;
        }
        self.format_badge = document.format_name().to_string();
        self.document = Some(document);
        self.is_loading = false;
        self.load_error = None;
        self.current_chapter = chapter;
        self.last_rendered_chapter = chapter;
        self.current_scroll_y = scroll_y;
        self.comic_page = comic_page;
    }

    pub fn detect_format_badge(path: &Path) -> String {
        if path.is_dir() {
            return "DIR".to_string();
        }
        let name_lower = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name_lower.ends_with(".fb2.zip") {
            return "FB2".to_string();
        }
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref()
        {
            Some("pdf") => "PDF".to_string(),
            Some("djvu") | Some("djv") => "DJVU".to_string(),
            Some("fb2") | Some("xml") => "FB2".to_string(),
            Some("epub") => "EPUB".to_string(),
            Some("mobi") => "MOBI".to_string(),
            Some("prc") => "PRC".to_string(),
            Some("azw3") => "AZW3".to_string(),
            Some("azw") => "AZW".to_string(),
            Some("txt") => "TXT".to_string(),
            Some("md") | Some("markdown") => "MD".to_string(),
            Some("log") => "LOG".to_string(),
            Some("cbz") | Some("zip") => "CBZ".to_string(),
            Some("cbr") | Some("rar") => "CBR".to_string(),
            Some("cb7") | Some("7z") => "CB7".to_string(),
            Some(ext) => ext.to_uppercase(),
            None => "DOC".to_string(),
        }
    }

    pub fn short_title(&self, max_chars: usize) -> String {
        let count = self.title.chars().count();
        if count <= max_chars {
            self.title.clone()
        } else {
            let truncated: String = self
                .title
                .chars()
                .take(max_chars.saturating_sub(1))
                .collect();
            format!("{truncated}…")
        }
    }
}

pub enum TabBarAction {
    None,
    Activate(usize),
    Close(usize),
    OpenNewFile,
}

pub struct TabsBar;

impl TabsBar {
    pub fn render(
        ui: &mut egui::Ui,
        tabs: &[BookTab],
        active_tab: usize,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
    ) -> TabBarAction {
        let t = lang.t();
        let mut action = TabBarAction::None;

        ScrollArea::horizontal()
            .id_salt("open_books_tab_bar")
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    for (idx, tab) in tabs.iter().enumerate() {
                        let is_active = idx == active_tab;
                        let bg_fill = if is_active {
                            theme.accent_color().gamma_multiply(0.22)
                        } else if theme.is_dark() {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                        } else {
                            Color32::from_rgba_unmultiplied(0, 0, 0, 10)
                        };

                        let border_stroke = if is_active {
                            Stroke::new(1.5, theme.accent_color())
                        } else {
                            Stroke::NONE
                        };

                        let frame = egui::Frame::default()
                            .fill(bg_fill)
                            .stroke(border_stroke)
                            .corner_radius(6.0)
                            .inner_margin(egui::Margin::symmetric(8, 3));

                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                let badge_text = RichText::new(format!("[{}]", tab.format_badge))
                                    .size(11.0)
                                    .strong()
                                    .color(theme.accent_color());
                                ui.label(badge_text);

                                if tab.is_loading {
                                    ui.add(
                                        IconLabel::new(Icon::Hourglass, "")
                                            .size(11.0)
                                            .color(theme.accent_color()),
                                    );
                                } else if tab.load_error.is_some() {
                                    ui.add(
                                        IconLabel::new(Icon::Warning, "")
                                            .size(11.0)
                                            .color(theme.accent_color()),
                                    );
                                }
                                let label_str = if tab.is_loading || tab.load_error.is_some() {
                                    tab.short_title(22)
                                } else {
                                    tab.short_title(26)
                                };
                                let title_rich = if is_active {
                                    RichText::new(label_str)
                                        .size(13.0)
                                        .strong()
                                        .color(theme.text_color())
                                } else {
                                    RichText::new(label_str)
                                        .size(13.0)
                                        .color(theme.text_color())
                                };

                                let tab_btn = ui.add(
                                    egui::Button::new(title_rich)
                                        .frame(false)
                                        .min_size(egui::Vec2::new(40.0, 20.0)),
                                );

                                let tooltip = format!(
                                    "{}\n{}: {}\n{}: {}\n({})",
                                    tab.title,
                                    t.tab_format,
                                    tab.format_badge,
                                    t.tab_path,
                                    tab.file_path.display(),
                                    t.tab_close_hint
                                );
                                let tab_btn = tab_btn.on_hover_text(tooltip);

                                if tab_btn.clicked() && !is_active {
                                    action = TabBarAction::Activate(idx);
                                } else if tab_btn.middle_clicked() {
                                    action = TabBarAction::Close(idx);
                                }

                                let (close_rect, close_resp) = ui.allocate_exact_size(
                                    egui::Vec2::new(18.0, 18.0),
                                    egui::Sense::click(),
                                );
                                if ui.is_rect_visible(close_rect) {
                                    let is_hovered = close_resp.hovered();
                                    if is_hovered {
                                        ui.painter().rect_filled(
                                            close_rect,
                                            4.0,
                                            theme.accent_color().gamma_multiply(0.25),
                                        );
                                    }
                                    let cross_color = if is_hovered {
                                        theme.accent_color()
                                    } else {
                                        theme.text_color().gamma_multiply(0.75)
                                    };
                                    let c = close_rect.center();
                                    let r = 3.8;
                                    let stroke = Stroke::new(1.6, cross_color);
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(c.x - r, c.y - r),
                                            egui::pos2(c.x + r, c.y + r),
                                        ],
                                        stroke,
                                    );
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(c.x - r, c.y + r),
                                            egui::pos2(c.x + r, c.y - r),
                                        ],
                                        stroke,
                                    );
                                }
                                let close_resp =
                                    close_resp.on_hover_text(format!("{} (Ctrl+W)", t.close_tab));
                                if close_resp.clicked() {
                                    action = TabBarAction::Close(idx);
                                }
                            });
                        });
                    }

                    let add_btn = ui
                        .add(
                            IconButton::new(Icon::Plus, "")
                                .font_size(15.0)
                                .min_size(egui::Vec2::new(26.0, 24.0)),
                        )
                        .on_hover_text(t.open_file);
                    if add_btn.clicked() {
                        action = TabBarAction::OpenNewFile;
                    }
                });
            });

        action
    }
}

impl super::QuietReaderApp {
    pub fn save_active_to_tab(&mut self) {
        let cur_page = match self.view_kind() {
            DocumentViewKind::Pdf => self.pdf_view.current_page,
            _ => self.comic_view.current_page,
        };
        if let Some(tab) = self.tabs.get_mut(self.active_tab)
            && self.document.is_some()
        {
            tab.current_chapter = self.current_chapter;
            tab.last_rendered_chapter = self.last_rendered_chapter;
            tab.current_scroll_y = self.current_scroll_y;
            tab.viewport_height = self.viewport_height;
            tab.max_scroll_y = self.max_scroll_y;
            tab.comic_page = cur_page;
        }
        ReaderCore::record_progress(
            &mut self.settings,
            &self.document,
            &self.current_file_path,
            self.current_chapter,
            cur_page,
            self.current_scroll_y,
        );
    }

    pub fn activate_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.flush_settings_now();
        self.autoscroll.stop();
        self.apply_tab_state(idx);
    }

    pub(crate) fn apply_tab_state(&mut self, idx: usize) {
        if let Some(tab) = self.tabs.get(idx).cloned() {
            self.active_tab = idx;
            self.document = tab.document;
            self.current_file_path = Some(tab.file_path.clone());
            self.current_chapter = tab.current_chapter;
            self.last_rendered_chapter = tab.current_chapter;
            self.pending_scroll_y = Some(tab.current_scroll_y);
            self.current_scroll_y = tab.current_scroll_y;
            self.viewport_height = tab.viewport_height;
            self.max_scroll_y = tab.max_scroll_y;
            self.comic_view.clear_cache();
            self.comic_view.current_page = tab.comic_page;
            self.pdf_view.clear_cache();
            self.pdf_view.current_page = tab.comic_page;
            self.search.reset_for_document();
            self.tts.stop();
            self.last_tts_paragraph = None;
            self.settings.last_opened_book = Some(tab.file_path);
            self.settings.active_tab_index = idx;
            let t = self.settings.ui_language.t();
            if tab.is_loading {
                self.status_message = Some(crate::i18n::fill(
                    t.extra.status_tab_loading,
                    &[("{fmt}", &tab.format_badge), ("{title}", &tab.title)],
                ));
            } else if let Some(ref err) = tab.load_error {
                self.status_message = Some(crate::i18n::fill(
                    t.extra.status_tab_error,
                    &[("{fmt}", &tab.format_badge), ("{error}", err)],
                ));
            } else {
                self.status_message = Some(crate::i18n::fill(
                    t.extra.status_tab_active,
                    &[("{fmt}", &tab.format_badge), ("{title}", &tab.title)],
                ));
            }
        }
    }

    pub fn close_tab(&mut self, idx: usize) {
        if idx >= self.tabs.len() {
            return;
        }
        self.tts.stop();
        self.last_tts_paragraph = None;
        self.save_active_to_tab();
        self.tabs.remove(idx);

        if self.tabs.is_empty() {
            self.active_tab = 0;
            self.document = None;
            self.current_file_path = None;
            self.current_chapter = 0;
            self.last_rendered_chapter = 0;
            self.pending_scroll_y = None;
            self.current_scroll_y = 0.0;
            self.max_scroll_y = 0.0;
            self.comic_view.clear_cache();
            self.pdf_view.clear_cache();
            self.autoscroll.stop();
            self.status_message = Some(
                self.settings
                    .ui_language
                    .t()
                    .extra
                    .status_all_tabs_closed
                    .to_string(),
            );
        } else {
            let next_idx = if idx < self.active_tab {
                self.active_tab - 1
            } else {
                self.active_tab.min(self.tabs.len() - 1)
            };
            self.apply_tab_state(next_idx);
        }
        self.flush_settings_now();
    }

    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.activate_tab((self.active_tab + 1) % self.tabs.len());
        }
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            let prev = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
            self.activate_tab(prev);
        }
    }
}
