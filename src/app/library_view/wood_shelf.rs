use crate::app::theme::ReaderTheme;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};

pub fn paint_wood_shelf(
    painter: &egui::Painter,
    min_x: f32,
    max_x: f32,
    top_y: f32,
    theme: &ReaderTheme,
) {
    let plank_rect = Rect::from_min_max(Pos2::new(min_x, top_y), Pos2::new(max_x, top_y + 11.0));
    let is_dark = theme.is_dark();

    let shadow_rect = Rect::from_min_max(
        Pos2::new(min_x, plank_rect.max.y),
        Pos2::new(max_x, plank_rect.max.y + 10.0),
    );
    for i in 0..10 {
        let alpha = ((10 - i) as f32 * if is_dark { 6.5 } else { 4.8 }) as u8;
        let y0 = shadow_rect.min.y + i as f32;
        painter.line_segment(
            [Pos2::new(min_x, y0), Pos2::new(max_x, y0)],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(24, 16, 12, alpha)),
        );
    }

    let stops = if is_dark {
        [
            Color32::from_rgb(92, 66, 50),
            Color32::from_rgb(70, 49, 37),
            Color32::from_rgb(48, 33, 25),
            Color32::from_rgb(28, 19, 15),
        ]
    } else {
        [
            Color32::from_rgb(212, 163, 115),
            Color32::from_rgb(184, 115, 51),
            Color32::from_rgb(140, 83, 60),
            Color32::from_rgb(90, 50, 34),
        ]
    };

    let h = plank_rect.height();
    for (idx, win) in stops.windows(2).enumerate() {
        let (c0, c1) = (win[0], win[1]);
        let y_start = plank_rect.min.y + (idx as f32 / 3.0) * h;
        let y_end = plank_rect.min.y + ((idx + 1) as f32 / 3.0) * h;
        let strip = Rect::from_min_max(
            Pos2::new(plank_rect.min.x, y_start),
            Pos2::new(plank_rect.max.x, y_end),
        );
        painter.rect_filled(strip, 2.0, c0.lerp_to_gamma(c1, 0.5));
    }

    painter.line_segment(
        [
            Pos2::new(plank_rect.min.x + 2.0, plank_rect.min.y + 0.5),
            Pos2::new(plank_rect.max.x - 2.0, plank_rect.min.y + 0.5),
        ],
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, if is_dark { 45 } else { 120 }),
        ),
    );
    painter.line_segment(
        [
            Pos2::new(plank_rect.min.x + 2.0, plank_rect.max.y - 0.5),
            Pos2::new(plank_rect.max.x - 2.0, plank_rect.max.y - 0.5),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 145)),
    );
}
