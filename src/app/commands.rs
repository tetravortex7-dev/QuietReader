use crate::app::QuietReaderApp;
use crate::core::{Command, ReaderCore};
use crate::document::DocumentViewKind;
use eframe::egui;

pub(crate) fn next_unique_bookmark_id(
    existing: &[crate::core::Bookmark],
    timestamp: u64,
    chapter: usize,
) -> String {
    let mut seq = existing.len();
    loop {
        let candidate = format!("{timestamp}_{chapter}_{seq}");
        if !existing.iter().any(|b| b.id == candidate) {
            return candidate;
        }
        seq += 1;
    }
}

impl QuietReaderApp {
    pub(crate) fn execute_command(
        &mut self,
        ctx: &egui::Context,
        cmd: &Command,
        view_kind: DocumentViewKind,
    ) {
        match cmd {
            Command::CopyChapterText => {
                if let Some(doc) = &self.document {
                    if !doc.can_copy() {
                        self.status_message = Some(
                            self.settings
                                .ui_language
                                .t()
                                .extra
                                .copy_restricted
                                .to_string(),
                        );
                        return;
                    }
                    let text_opt = if view_kind == DocumentViewKind::Pdf {
                        doc.page_content()
                            .and_then(|c| c.extract_page_text(self.pdf_view.current_page))
                    } else {
                        ReaderCore::extract_chapter_text(doc.as_ref(), self.current_chapter)
                    };
                    if let Some(text) = text_opt {
                        ctx.copy_text(text);
                        let t = self.settings.ui_language.t();
                        let msg = if view_kind == DocumentViewKind::Pdf {
                            t.extra
                                .copy_page_done
                                .replace("{page}", &(self.pdf_view.current_page + 1).to_string())
                        } else {
                            t.extra.copy_section_done.to_string()
                        };
                        self.status_message = Some(msg);
                    }
                }
            }
            Command::ZoomIn | Command::ZoomOut | Command::ResetZoom => {
                match view_kind {
                    DocumentViewKind::Pdf => match cmd {
                        Command::ZoomIn => super::paged_common::zoom_in(
                            &mut self.settings.pdf.zoom_percent,
                            &mut self.settings.pdf.fit_mode,
                        ),
                        Command::ZoomOut => super::paged_common::zoom_out(
                            &mut self.settings.pdf.zoom_percent,
                            &mut self.settings.pdf.fit_mode,
                        ),
                        _ => super::paged_common::zoom_reset(
                            &mut self.settings.pdf.zoom_percent,
                            &mut self.settings.pdf.fit_mode,
                            &mut self.pdf_view.pan_offset,
                        ),
                    },
                    DocumentViewKind::Comic => match cmd {
                        Command::ZoomIn => super::paged_common::zoom_in(
                            &mut self.settings.comic.zoom_percent,
                            &mut self.settings.comic.fit_mode,
                        ),
                        Command::ZoomOut => super::paged_common::zoom_out(
                            &mut self.settings.comic.zoom_percent,
                            &mut self.settings.comic.fit_mode,
                        ),
                        _ => super::paged_common::zoom_reset(
                            &mut self.settings.comic.zoom_percent,
                            &mut self.settings.comic.fit_mode,
                            &mut self.comic_view.pan_offset,
                        ),
                    },
                    DocumentViewKind::Text => {
                        self.font_size = match cmd {
                            Command::ZoomIn => (self.font_size + 1.0).clamp(12.0, 36.0),
                            Command::ZoomOut => (self.font_size - 1.0).clamp(12.0, 36.0),
                            _ => 19.0,
                        };
                    }
                }
                self.mark_settings_dirty();
            }
            Command::ToggleAutoscroll => {
                self.close_focus_reading();
                if self.document.is_some() {
                    self.autoscroll.toggle(
                        self.current_scroll_y,
                        self.max_scroll_y,
                        self.settings.auto_scroll.step_distance,
                    );
                }
            }
            Command::ToggleFullscreen => {
                self.set_fullscreen_mode(ctx, !self.is_fullscreen);
            }
            Command::ToggleSearch => {
                if self.show_settings_dialog {
                    ctx.data_mut(|d| {
                        d.insert_temp(egui::Id::unique("settings_focus_search_requested"), true);
                    });
                } else if matches!(view_kind, DocumentViewKind::Text | DocumentViewKind::Pdf)
                    && self.document.is_some()
                {
                    self.search.toggle();
                }
            }
            Command::ToggleTTS => {
                self.close_focus_reading();
                self.tts.toggle_panel();
            }
            Command::ToggleToc => {
                if self.document.is_some() {
                    self.show_toc = !self.show_toc;
                }
            }
            Command::ToggleSettings => {
                self.show_settings_dialog = !self.show_settings_dialog;
            }
            Command::ToggleReadingRuler => {
                if self
                    .settings
                    .can_use(crate::core::license::ProFeature::FocusTools)
                {
                    self.settings.reading_ruler = !self.settings.reading_ruler;
                    self.mark_settings_dirty();
                } else {
                    self.pro_upgrade
                        .request(crate::core::license::ProFeature::FocusTools);
                }
            }
            Command::ToggleBookmark => {
                self.toggle_quick_bookmark();
            }
            Command::ToggleAmbient => {
                if self
                    .settings
                    .can_use(crate::core::license::ProFeature::AmbientAudio)
                {
                    self.ambient.toggle_panel();
                } else {
                    self.pro_upgrade
                        .request(crate::core::license::ProFeature::AmbientAudio);
                }
            }
            Command::ExitFullscreen => {
                if self.focus_read.rsvp.active {
                    self.close_speed_reading();
                } else if self.settings.text_paragraph_mode {
                    self.toggle_paragraph_mode();
                } else if self.search.is_open {
                    self.search.close();
                } else if self.ambient.is_open {
                    self.ambient.close();
                } else if self.autoscroll.is_active() {
                    self.autoscroll.stop();
                } else if self.show_settings_dialog {
                    self.show_settings_dialog = false;
                } else if self.show_toc {
                    self.show_toc = false;
                } else if self.show_recent_dialog {
                    self.show_recent_dialog = false;
                } else if self.show_opds_dialog {
                    self.show_opds_dialog = false;
                } else if self.dictionary.visible {
                    self.dictionary.visible = false;
                } else if self.translate.is_open {
                    self.translate.close();
                } else if self.pro_upgrade.is_open {
                    self.pro_upgrade.close();
                } else if self.show_association_dialog {
                    self.show_association_dialog = false;
                } else if self.is_fullscreen {
                    self.set_fullscreen_mode(ctx, false);
                }
            }
            nav_cmd => {
                if matches!(
                    nav_cmd,
                    Command::PageDown
                        | Command::PageUp
                        | Command::LineDown
                        | Command::LineUp
                        | Command::NextChapter
                        | Command::PrevChapter
                ) {
                    self.autoscroll.stop();
                }
                match view_kind {
                    DocumentViewKind::Pdf => {
                        let total_pages = self.pdf_view.page_count(&self.document);
                        let auto_crop = self.settings.effective_pdf_auto_crop();
                        match nav_cmd {
                            Command::LineDown => {
                                if self.settings.pdf.layout == "Continuous" {
                                    let cur =
                                        self.pending_scroll_y.unwrap_or(self.current_scroll_y);
                                    self.pending_scroll_y =
                                        Some((cur + 60.0).clamp(0.0, self.max_scroll_y));
                                } else {
                                    let max_pan_y = self
                                        .pdf_view
                                        .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                        .y;
                                    if max_pan_y > 1.0
                                        && self.pdf_view.pan_offset.y > -max_pan_y + 1.0
                                    {
                                        self.pdf_view.pan_offset.y =
                                            (self.pdf_view.pan_offset.y - 60.0).max(-max_pan_y);
                                    } else {
                                        self.pdf_view.next_page(total_pages, &self.settings.pdf);
                                        let next_max_y = self
                                            .pdf_view
                                            .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                            .y;
                                        self.pdf_view.pan_offset.y = next_max_y;
                                    }
                                }
                                self.mark_settings_dirty();
                            }
                            Command::LineUp => {
                                if self.settings.pdf.layout == "Continuous" {
                                    let cur =
                                        self.pending_scroll_y.unwrap_or(self.current_scroll_y);
                                    self.pending_scroll_y =
                                        Some((cur - 60.0).clamp(0.0, self.max_scroll_y));
                                } else {
                                    let max_pan_y = self
                                        .pdf_view
                                        .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                        .y;
                                    if max_pan_y > 1.0
                                        && self.pdf_view.pan_offset.y < max_pan_y - 1.0
                                    {
                                        self.pdf_view.pan_offset.y =
                                            (self.pdf_view.pan_offset.y + 60.0).min(max_pan_y);
                                    } else {
                                        self.pdf_view.prev_page(&self.settings.pdf);
                                        let prev_max_y = self
                                            .pdf_view
                                            .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                            .y;
                                        self.pdf_view.pan_offset.y = -prev_max_y;
                                    }
                                }
                                self.mark_settings_dirty();
                            }
                            Command::PageDown | Command::NextPage => {
                                if self.settings.pdf.layout == "Continuous" {
                                    let cur =
                                        self.pending_scroll_y.unwrap_or(self.current_scroll_y);
                                    self.pending_scroll_y = Some(
                                        (cur + self.viewport_height * 0.85)
                                            .clamp(0.0, self.max_scroll_y),
                                    );
                                } else {
                                    let max_pan_y = self
                                        .pdf_view
                                        .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                        .y;
                                    if *nav_cmd == Command::PageDown
                                        && max_pan_y > 1.0
                                        && self.pdf_view.pan_offset.y > -max_pan_y + 1.0
                                    {
                                        self.pdf_view.pan_offset.y = (self.pdf_view.pan_offset.y
                                            - self.viewport_height * 0.85)
                                            .max(-max_pan_y);
                                    } else {
                                        self.pdf_view.next_page(total_pages, &self.settings.pdf);
                                        let next_max_y = self
                                            .pdf_view
                                            .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                            .y;
                                        self.pdf_view.pan_offset.y = next_max_y;
                                    }
                                }
                                self.mark_settings_dirty();
                            }
                            Command::PageUp | Command::PrevPage => {
                                if self.settings.pdf.layout == "Continuous" {
                                    let cur =
                                        self.pending_scroll_y.unwrap_or(self.current_scroll_y);
                                    self.pending_scroll_y = Some(
                                        (cur - self.viewport_height * 0.85)
                                            .clamp(0.0, self.max_scroll_y),
                                    );
                                } else {
                                    let max_pan_y = self
                                        .pdf_view
                                        .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                        .y;
                                    if *nav_cmd == Command::PageUp
                                        && max_pan_y > 1.0
                                        && self.pdf_view.pan_offset.y < max_pan_y - 1.0
                                    {
                                        self.pdf_view.pan_offset.y = (self.pdf_view.pan_offset.y
                                            + self.viewport_height * 0.85)
                                            .min(max_pan_y);
                                    } else {
                                        self.pdf_view.prev_page(&self.settings.pdf);
                                        let prev_max_y = self
                                            .pdf_view
                                            .max_pan(&self.settings.pdf, total_pages, auto_crop)
                                            .y;
                                        self.pdf_view.pan_offset.y = -prev_max_y;
                                    }
                                }
                                self.mark_settings_dirty();
                            }
                            Command::Home => {
                                self.pdf_view.go_to_page(0);
                                if self.settings.pdf.layout == "Continuous" {
                                    self.pending_scroll_y = Some(0.0);
                                }
                                self.mark_settings_dirty();
                            }
                            Command::End => {
                                if total_pages > 0 {
                                    self.pdf_view.go_to_page(total_pages.saturating_sub(1));
                                }
                                self.mark_settings_dirty();
                            }
                            _ => {}
                        }
                    }
                    DocumentViewKind::Comic => {
                        let is_webtoon = self.settings.comic.reading_mode == "Webtoon";
                        let prev_page = self.comic_view.current_page;
                        if ReaderCore::apply_paged_command(
                            nav_cmd,
                            &self.document,
                            is_webtoon,
                            self.settings.comic.page_layout == "Double",
                            &mut self.comic_view.current_page,
                            self.current_scroll_y,
                            self.max_scroll_y,
                            self.viewport_height,
                            &mut self.pending_scroll_y,
                        ) {
                            if !is_webtoon && self.comic_view.current_page != prev_page {
                                self.comic_view.pan_offset = egui::Vec2::ZERO;
                            }
                            self.mark_settings_dirty();
                        }
                    }
                    DocumentViewKind::Text => {
                        if !self.focus_nav_command(nav_cmd) {
                            if self.settings.text_two_columns
                                && self.viewport_height > 1.0
                                && matches!(
                                    nav_cmd,
                                    Command::PageDown
                                        | Command::NextPage
                                        | Command::PageUp
                                        | Command::PrevPage
                                        | Command::Home
                                        | Command::End
                                )
                            {
                                if matches!(nav_cmd, Command::Home) {
                                    self.pending_scroll_y = Some(0.0);
                                    self.current_scroll_y = 0.0;
                                    self.mark_settings_dirty();
                                } else if matches!(nav_cmd, Command::End) {
                                    self.pending_scroll_y = Some(self.max_scroll_y);
                                    self.current_scroll_y = self.max_scroll_y;
                                    self.mark_settings_dirty();
                                } else if let Some(doc) = &self.document {
                                    let forward =
                                        matches!(nav_cmd, Command::PageDown | Command::NextPage);
                                    let two_cols = super::reader_chrome::last_effective_spread(
                                        ctx, 1200.0, true,
                                    );
                                    if forward && self.current_scroll_y + 2.0 >= self.max_scroll_y {
                                        if let Some(next_idx) =
                                            ReaderCore::find_next_non_empty_chapter(
                                                doc.as_ref(),
                                                self.current_chapter,
                                            )
                                        {
                                            self.current_chapter = next_idx;
                                            self.last_rendered_chapter = next_idx;
                                            self.pending_scroll_y = Some(0.0);
                                            self.current_scroll_y = 0.0;
                                            self.mark_settings_dirty();
                                        }
                                    } else if !forward && self.current_scroll_y <= 2.0 {
                                        if let Some(prev_idx) =
                                            ReaderCore::find_prev_non_empty_chapter(
                                                doc.as_ref(),
                                                self.current_chapter,
                                            )
                                        {
                                            self.current_chapter = prev_idx;
                                            self.last_rendered_chapter = prev_idx;
                                            self.pending_scroll_y = Some(999_999.0);
                                            self.mark_settings_dirty();
                                        }
                                    } else {
                                        let step =
                                            self.viewport_height * if two_cols { 2.0 } else { 1.0 };
                                        let target = if forward {
                                            (self.current_scroll_y + step).min(self.max_scroll_y)
                                        } else {
                                            (self.current_scroll_y - step).max(0.0)
                                        };
                                        self.pending_scroll_y = Some(target);
                                        self.current_scroll_y = target;
                                        self.mark_settings_dirty();
                                    }
                                }
                            } else if self.settings.text_paged_scroll
                                && matches!(
                                    nav_cmd,
                                    Command::PageDown
                                        | Command::NextPage
                                        | Command::PageUp
                                        | Command::PrevPage
                                )
                                && self.viewport_height > 1.0
                            {
                                let forward =
                                    matches!(nav_cmd, Command::PageDown | Command::NextPage);
                                if let Some(doc) = &self.document {
                                    if forward && self.current_scroll_y + 2.0 >= self.max_scroll_y {
                                        if let Some(next_idx) =
                                            ReaderCore::find_next_non_empty_chapter(
                                                doc.as_ref(),
                                                self.current_chapter,
                                            )
                                        {
                                            self.current_chapter = next_idx;
                                            self.last_rendered_chapter = next_idx;
                                            self.pending_scroll_y = Some(0.0);
                                            self.current_scroll_y = 0.0;
                                            self.mark_settings_dirty();
                                        }
                                    } else if !forward && self.current_scroll_y <= 2.0 {
                                        if let Some(prev_idx) =
                                            ReaderCore::find_prev_non_empty_chapter(
                                                doc.as_ref(),
                                                self.current_chapter,
                                            )
                                        {
                                            self.current_chapter = prev_idx;
                                            self.last_rendered_chapter = prev_idx;
                                            self.pending_scroll_y = Some(999_999.0);
                                            self.mark_settings_dirty();
                                        }
                                    } else {
                                        let step = self.viewport_height;
                                        let target = if forward {
                                            (self.current_scroll_y + step).min(self.max_scroll_y)
                                        } else {
                                            (self.current_scroll_y - step).max(0.0)
                                        };
                                        self.pending_scroll_y = Some(target);
                                        self.current_scroll_y = target;
                                        self.mark_settings_dirty();
                                    }
                                }
                            } else if let Some(doc) = &self.document {
                                ReaderCore::apply_text_command(
                                    nav_cmd,
                                    doc.as_ref(),
                                    &mut self.current_chapter,
                                    &mut self.last_rendered_chapter,
                                    self.current_scroll_y,
                                    self.max_scroll_y,
                                    self.viewport_height,
                                    &mut self.pending_scroll_y,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_quick_bookmark(&mut self) {
        let Some(file_path) = self.current_file_path.clone() else {
            return;
        };
        let Some(doc) = self.document.clone() else {
            return;
        };
        let t = self.settings.ui_language.t();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let view_kind = self.view_kind();
        let (label, text, progress_pct, page_num) = match view_kind {
            DocumentViewKind::Pdf => {
                let cur = self.pdf_view.current_page;
                let total = self.pdf_view.page_count(&self.document).max(1);
                let pct = ((cur + 1) as f32 / total as f32 * 100.0).clamp(0.0, 100.0);
                let page_text = doc
                    .page_content()
                    .and_then(|c| c.extract_page_text(cur))
                    .unwrap_or_default();
                let preview: String = page_text.trim().chars().take(95).collect();
                (
                    format!("{} {}", t.page_prefix, cur + 1),
                    preview,
                    pct,
                    Some(cur),
                )
            }
            DocumentViewKind::Comic => {
                let cur = self.comic_view.current_page;
                let total = doc
                    .page_content()
                    .map(|c| c.page_count())
                    .unwrap_or(1)
                    .max(1);
                let pct = ((cur + 1) as f32 / total as f32 * 100.0).clamp(0.0, 100.0);
                (
                    format!("{} {}", t.page_prefix, cur + 1),
                    format!("{} {}", t.page_prefix, cur + 1),
                    pct,
                    Some(cur),
                )
            }
            _ => {
                let total_ch = doc.chapters().len().max(1);
                let ch = doc.chapters().get(self.current_chapter);
                let clean_title = ch
                    .map(|c| crate::document::strip_formatting_tags(&c.title))
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| {
                        t.section_num
                            .replace("{}", &(self.current_chapter + 1).to_string())
                    });

                let visible_p = ReaderCore::estimate_visible_paragraph(
                    doc.as_ref(),
                    self.current_chapter,
                    self.current_scroll_y,
                    self.font_size,
                );
                let para_text = ch
                    .and_then(|c| c.paragraphs.get(visible_p))
                    .map(|p| crate::document::strip_formatting_tags(p))
                    .unwrap_or_default();
                let mut preview: String = para_text.trim().chars().take(95).collect();
                if para_text.trim().chars().count() > 95 {
                    preview.push_str("...");
                }

                let ch_frac = self.current_chapter as f32 / total_ch as f32;
                let scroll_frac = if self.max_scroll_y > 0.0 {
                    (self.current_scroll_y / self.max_scroll_y).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let pct = ((ch_frac + (scroll_frac / total_ch as f32)) * 100.0).clamp(0.0, 100.0);

                (clean_title, preview, pct, None)
            }
        };

        let existing_idx = self.settings.bookmarks.iter().position(|b| {
            b.book_path == file_path
                && b.chapter == self.current_chapter
                && (b.scroll_y - self.current_scroll_y).abs() < 25.0
                && b.page_number == page_num
        });

        if let Some(idx) = existing_idx {
            self.settings.bookmarks.remove(idx);
            self.status_message = Some(t.bookmark_removed.to_string());
        } else {
            let id =
                next_unique_bookmark_id(&self.settings.bookmarks, timestamp, self.current_chapter);
            let bm = crate::core::Bookmark {
                id,
                book_path: file_path,
                chapter: self.current_chapter,
                scroll_y: self.current_scroll_y,
                label,
                text,
                note: String::new(),
                color: crate::core::BookmarkColor::Yellow,
                kind: crate::core::BookmarkKind::Bookmark,
                timestamp,
                progress_percent: progress_pct,
                page_number: page_num,
            };
            self.settings.bookmarks.push(bm);
            self.status_message = Some(t.bookmark_added.to_string());
        }
        self.mark_settings_dirty();
    }

    pub fn save_quote_bookmark(&mut self, text: String) {
        let Some(file_path) = self.current_file_path.clone() else {
            return;
        };
        let Some(doc) = self.document.clone() else {
            return;
        };
        let t = self.settings.ui_language.t();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let total_ch = doc.chapters().len().max(1);
        let ch = doc.chapters().get(self.current_chapter);
        let clean_title = ch
            .map(|c| crate::document::strip_formatting_tags(&c.title))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                t.section_num
                    .replace("{}", &(self.current_chapter + 1).to_string())
            });

        let ch_frac = self.current_chapter as f32 / total_ch as f32;
        let scroll_frac = if self.max_scroll_y > 0.0 {
            (self.current_scroll_y / self.max_scroll_y).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let pct = ((ch_frac + (scroll_frac / total_ch as f32)) * 100.0).clamp(0.0, 100.0);

        let id = next_unique_bookmark_id(&self.settings.bookmarks, timestamp, self.current_chapter);
        let bm = crate::core::Bookmark {
            id,
            book_path: file_path,
            chapter: self.current_chapter,
            scroll_y: self.current_scroll_y,
            label: clean_title,
            text,
            note: String::new(),
            color: crate::core::BookmarkColor::Yellow,
            kind: crate::core::BookmarkKind::Quote,
            timestamp,
            progress_percent: pct,
            page_number: None,
        };
        self.settings.bookmarks.push(bm);
        self.status_message = Some(t.quote_saved.to_string());
        self.mark_settings_dirty();
    }
}
