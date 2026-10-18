use crate::app::QuietReaderApp;
use crate::core::{Command, ReaderCore};
use crate::document::DocumentViewKind;
use eframe::egui;

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
                    let text_opt = if view_kind == DocumentViewKind::Pdf {
                        doc.page_content()
                            .and_then(|c| c.extract_page_text(self.pdf_view.current_page))
                    } else {
                        ReaderCore::extract_chapter_text(doc.as_ref(), self.current_chapter)
                    };
                    if let Some(text) = text_opt {
                        ctx.copy_text(text);
                        let msg = if view_kind == DocumentViewKind::Pdf {
                            format!(
                                "Текст страницы {} скопирован в буфер обмена (Ctrl+A)",
                                self.pdf_view.current_page + 1
                            )
                        } else {
                            "Весь раздел скопирован в буфер обмена (Ctrl+A)".to_string()
                        };
                        self.status_message = Some(msg);
                    }
                }
            }
            Command::ZoomIn | Command::ZoomOut | Command::ResetZoom => {
                self.font_size = match cmd {
                    Command::ZoomIn => (self.font_size + 1.0).clamp(12.0, 36.0),
                    Command::ZoomOut => (self.font_size - 1.0).clamp(12.0, 36.0),
                    _ => 19.0,
                };
                self.mark_settings_dirty();
            }
            Command::ToggleAutoscroll => {
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
                if view_kind == DocumentViewKind::Text && self.document.is_some() {
                    self.search.toggle();
                }
            }
            Command::ToggleTTS => {
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
            Command::ExitFullscreen => {
                if self.search.is_open {
                    self.search.close();
                } else if self.autoscroll.is_active() {
                    self.autoscroll.stop();
                } else if self.show_settings_dialog {
                    self.show_settings_dialog = false;
                } else if self.show_toc {
                    self.show_toc = false;
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
                        match nav_cmd {
                            Command::LineDown => {
                                if self.settings.pdf.layout == "Continuous" {
                                    let cur =
                                        self.pending_scroll_y.unwrap_or(self.current_scroll_y);
                                    self.pending_scroll_y =
                                        Some((cur + 60.0).clamp(0.0, self.max_scroll_y));
                                } else {
                                    let max_pan_y =
                                        self.pdf_view.max_pan(&self.settings.pdf, total_pages).y;
                                    if max_pan_y > 1.0
                                        && self.pdf_view.pan_offset.y > -max_pan_y + 1.0
                                    {
                                        self.pdf_view.pan_offset.y =
                                            (self.pdf_view.pan_offset.y - 60.0).max(-max_pan_y);
                                    } else {
                                        self.pdf_view.next_page(total_pages, &self.settings.pdf);
                                        let next_max_y = self
                                            .pdf_view
                                            .max_pan(&self.settings.pdf, total_pages)
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
                                    let max_pan_y =
                                        self.pdf_view.max_pan(&self.settings.pdf, total_pages).y;
                                    if max_pan_y > 1.0
                                        && self.pdf_view.pan_offset.y < max_pan_y - 1.0
                                    {
                                        self.pdf_view.pan_offset.y =
                                            (self.pdf_view.pan_offset.y + 60.0).min(max_pan_y);
                                    } else {
                                        self.pdf_view.prev_page(&self.settings.pdf);
                                        let prev_max_y = self
                                            .pdf_view
                                            .max_pan(&self.settings.pdf, total_pages)
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
                                    let max_pan_y =
                                        self.pdf_view.max_pan(&self.settings.pdf, total_pages).y;
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
                                            .max_pan(&self.settings.pdf, total_pages)
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
                                    let max_pan_y =
                                        self.pdf_view.max_pan(&self.settings.pdf, total_pages).y;
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
                                            .max_pan(&self.settings.pdf, total_pages)
                                            .y;
                                        self.pdf_view.pan_offset.y = -prev_max_y;
                                    }
                                }
                                self.mark_settings_dirty();
                            }
                            _ => {}
                        }
                    }
                    DocumentViewKind::Comic => {
                        if ReaderCore::apply_paged_command(
                            nav_cmd,
                            &self.document,
                            &self.settings.comic,
                            &mut self.comic_view.current_page,
                            self.current_scroll_y,
                            self.max_scroll_y,
                            self.viewport_height,
                            &mut self.pending_scroll_y,
                        ) {
                            self.mark_settings_dirty();
                        }
                    }
                    DocumentViewKind::Text => {
                        if let Some(doc) = &self.document {
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
