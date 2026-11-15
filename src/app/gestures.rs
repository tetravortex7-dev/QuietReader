use super::QuietReaderApp;
use crate::document::DocumentViewKind;
use eframe::egui::{self, Color32, LayerId, Order};
use std::time::Instant;

pub(super) const SWIPE_THRESHOLD: f32 = 60.0;

pub(super) fn swipe_forward(accumulated: f32, is_manga: bool) -> Option<bool> {
    if accumulated.abs() < SWIPE_THRESHOLD {
        return None;
    }
    if is_manga {
        Some(accumulated > 0.0)
    } else {
        Some(accumulated < 0.0)
    }
}

pub(super) fn paint_brightness(ctx: &egui::Context, brightness: f32) {
    let level = brightness.clamp(0.0, 1.0);
    if level >= 0.999 {
        return;
    }
    let alpha = ((1.0 - level) * 255.0).round().clamp(0.0, 255.0) as u8;
    let painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        egui::Id::unique("quietreader_brightness"),
    ));
    painter.rect_filled(ctx.content_rect(), 0.0, Color32::from_black_alpha(alpha));
}

impl QuietReaderApp {
    pub(super) fn handle_trackpad_swipe(
        &mut self,
        ctx: &egui::Context,
        view_kind: DocumentViewKind,
        is_manga: bool,
    ) {
        let active = self.settings.trackpad_swipe_pages
            && !self.show_library
            && !self.show_settings_dialog
            && !self.show_association_dialog
            && !self.show_recent_dialog
            && self.document.is_some();
        let paged = active
            && match view_kind {
                DocumentViewKind::Pdf => self.settings.pdf.layout != "Continuous",
                DocumentViewKind::Comic => self.settings.comic.reading_mode != "Webtoon",
                DocumentViewKind::Text => self.settings.text_two_columns,
            };
        if !paged {
            self.swipe_accum = 0.0;
            return;
        }

        let delta = ctx.input(|input| input.smooth_scroll_delta);
        let now = Instant::now();
        if let Some(last) = self.last_swipe_event
            && now.duration_since(last).as_secs_f32() > 0.4
        {
            self.swipe_accum = 0.0;
        }
        let horizontal = delta.x.abs() > delta.y.abs() * 1.5 && delta.x.abs() > 0.5;
        if horizontal {
            self.swipe_accum += delta.x;
            self.last_swipe_event = Some(now);
        } else if delta != egui::Vec2::ZERO {
            self.swipe_accum = 0.0;
        }

        let Some(forward) = swipe_forward(self.swipe_accum, is_manga) else {
            return;
        };
        if let Some(last) = self.last_swipe_turn
            && now.duration_since(last).as_secs_f32() < 0.3
        {
            return;
        }
        self.swipe_accum = 0.0;
        self.last_swipe_turn = Some(now);

        match view_kind {
            DocumentViewKind::Pdf => {
                let total_pages = self.pdf_view.page_count(&self.document);
                if forward {
                    self.pdf_view.next_page(total_pages, &self.settings.pdf);
                } else {
                    self.pdf_view.prev_page(&self.settings.pdf);
                }
            }
            DocumentViewKind::Comic => {
                let total_pages = self.comic_view.page_count(&self.document);
                if forward {
                    self.comic_view.next_page(total_pages, &self.settings.comic);
                } else {
                    self.comic_view.prev_page(&self.settings.comic);
                }
            }
            DocumentViewKind::Text => {
                let cmd = if forward {
                    crate::core::Command::NextPage
                } else {
                    crate::core::Command::PrevPage
                };
                self.execute_command(ctx, &cmd, view_kind);
            }
        }
    }
}
