use eframe::egui::{self, Color32, LayerId, Order};

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
