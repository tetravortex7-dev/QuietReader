use super::super::icons::{Icon, paint_icon};
use super::super::theme::{CozyPalette, ReaderTheme};
use super::{CoverLoader, LibraryActions};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use std::path::Path;

pub(super) fn book_cover_palette(title: &str, path: &Path) -> (Color32, Color32, Color32) {
    let mut h: u64 = 5381;
    for b in title.bytes().chain(path.to_string_lossy().bytes()) {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    const PALETTES: [(Color32, Color32, Color32); 8] = [
        (
            Color32::from_rgb(107, 45, 45),
            Color32::from_rgb(142, 60, 60),
            Color32::from_rgb(250, 220, 185),
        ),
        (
            Color32::from_rgb(37, 72, 54),
            Color32::from_rgb(49, 94, 71),
            Color32::from_rgb(235, 245, 225),
        ),
        (
            Color32::from_rgb(32, 53, 82),
            Color32::from_rgb(42, 70, 108),
            Color32::from_rgb(225, 235, 250),
        ),
        (
            Color32::from_rgb(148, 69, 44),
            Color32::from_rgb(175, 82, 52),
            Color32::from_rgb(255, 238, 225),
        ),
        (
            Color32::from_rgb(142, 106, 42),
            Color32::from_rgb(170, 128, 52),
            Color32::from_rgb(255, 248, 220),
        ),
        (
            Color32::from_rgb(45, 68, 77),
            Color32::from_rgb(60, 90, 102),
            Color32::from_rgb(220, 240, 245),
        ),
        (
            Color32::from_rgb(85, 42, 78),
            Color32::from_rgb(110, 55, 102),
            Color32::from_rgb(248, 225, 245),
        ),
        (
            Color32::from_rgb(105, 62, 42),
            Color32::from_rgb(130, 78, 52),
            Color32::from_rgb(250, 232, 215),
        ),
    ];
    PALETTES[(h as usize) % PALETTES.len()]
}

pub(super) fn shelf_cover(
    ui: &mut egui::Ui,
    path: &Path,
    title: &str,
    author: &str,
    size: Vec2,
    radius: f32,
    theme: &ReaderTheme,
    cover_loader: &mut CoverLoader,
    default_title: &str,
) -> egui::Response {
    let is_cozy = !theme.is_dark();
    let (cover_rect, cover_response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect_filled(
        cover_rect,
        radius,
        if is_cozy {
            CozyPalette::SURFACE_CONTAINER
        } else {
            theme.accent_color().gamma_multiply(0.08)
        },
    );

    if let Some(texture) = cover_loader.texture(path) {
        let texture_size = texture.size_vec2();
        if texture_size.x > 0.0 && texture_size.y > 0.0 {
            let available = cover_rect.shrink(1.0);
            let card_aspect = available.width() / available.height();
            let tex_aspect = texture_size.x / texture_size.y;
            let (uv_min, uv_max) = if tex_aspect > card_aspect {
                let crop_frac = card_aspect / tex_aspect;
                let x_offset = (1.0 - crop_frac) * 0.5;
                (Pos2::new(x_offset, 0.0), Pos2::new(1.0 - x_offset, 1.0))
            } else {
                let crop_frac = tex_aspect / card_aspect;
                let y_offset = (1.0 - crop_frac) * 0.5;
                (Pos2::new(0.0, y_offset), Pos2::new(1.0, 1.0 - y_offset))
            };
            ui.painter().image(
                texture.id(),
                available,
                Rect::from_min_max(uv_min, uv_max),
                Color32::WHITE,
            );
        }
    } else {
        paint_procedural_cover(
            ui.painter(),
            cover_rect,
            title,
            author,
            path,
            radius,
            default_title,
        );
    }

    ui.painter().rect_stroke(
        cover_rect,
        radius,
        Stroke::new(
            1.0,
            if is_cozy {
                CozyPalette::OUTLINE.gamma_multiply(0.3)
            } else {
                theme.accent_color().gamma_multiply(0.3)
            },
        ),
        StrokeKind::Inside,
    );

    let format_str = crate::document::registry::format_badge_for_path(path);
    let badge_bg = match format_str.as_str() {
        "PDF" => CozyPalette::SECONDARY,
        "FB2" | "CBZ" | "CBR" => CozyPalette::TERTIARY,
        _ => CozyPalette::PRIMARY,
    };
    let badge_rect = if cover_rect.width() < 70.0 {
        Rect::from_min_size(
            Pos2::new(cover_rect.min.x + 3.0, cover_rect.min.y + 3.0),
            Vec2::new(24.0, 12.0),
        )
    } else {
        Rect::from_min_size(
            Pos2::new(cover_rect.max.x - 34.0, cover_rect.min.y + 6.0),
            Vec2::new(28.0, 14.0),
        )
    };
    ui.painter().rect_filled(badge_rect, 3.0, badge_bg);
    ui.painter().text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        &format_str,
        egui::FontId::proportional(8.5),
        Color32::WHITE,
    );

    cover_response
}

pub(crate) fn paint_procedural_cover(
    painter: &egui::Painter,
    rect: egui::Rect,
    title: &str,
    author: &str,
    path: &Path,
    radius: f32,
    default_title: &str,
) {
    let (bg, spine_col, foil) = book_cover_palette(title, path);

    painter.rect_filled(rect, radius, bg);

    let spine_w = (rect.width() * 0.12).clamp(4.0, 18.0);
    let spine_rect = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + spine_w, rect.max.y));
    painter.rect_filled(spine_rect, radius, spine_col);

    painter.line_segment(
        [
            Pos2::new(rect.min.x + spine_w, rect.min.y),
            Pos2::new(rect.min.x + spine_w, rect.max.y),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 70)),
    );
    painter.line_segment(
        [
            Pos2::new(rect.min.x + spine_w + 1.0, rect.min.y),
            Pos2::new(rect.min.x + spine_w + 1.0, rect.max.y),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 40)),
    );

    let frame_inset = (rect.width() * 0.06).clamp(3.0, 8.0);
    let frame_rect = rect.shrink(frame_inset);
    painter.rect_stroke(
        frame_rect,
        radius * 0.7,
        Stroke::new(1.0, foil.gamma_multiply(0.35)),
        StrokeKind::Inside,
    );

    if rect.width() >= 96.0 {
        let inner_frame = frame_rect.shrink(3.0);
        painter.rect_stroke(
            inner_frame,
            radius * 0.5,
            Stroke::new(0.5, foil.gamma_multiply(0.2)),
            StrokeKind::Inside,
        );
    }

    let text_left = rect.min.x + spine_w + frame_inset + 6.0;
    let text_right = rect.max.x - frame_inset - 6.0;
    let max_text_w = (text_right - text_left).max(32.0);
    let center_x = (text_left + text_right) * 0.5;

    let icon_size = (rect.width() * 0.20).clamp(16.0, 26.0);
    let icon_y = if rect.width() >= 96.0 {
        rect.min.y + rect.height() * 0.25
    } else {
        rect.center().y
    };
    paint_icon(
        painter,
        Icon::Book,
        Rect::from_center_size(Pos2::new(center_x, icon_y), Vec2::splat(icon_size)),
        foil.gamma_multiply(0.85),
    );

    if rect.width() >= 96.0 {
        let display_title = if title.trim().is_empty() {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(default_title)
        } else {
            title
        };

        let title_clip = Rect::from_min_max(
            Pos2::new(text_left, rect.min.y + rect.height() * 0.36),
            Pos2::new(text_right, rect.min.y + rect.height() * 0.70),
        );
        let title_galley = painter.layout(
            display_title.to_string(),
            egui::FontId::proportional(11.0),
            foil,
            max_text_w,
        );
        let title_y = (title_clip.center().y - title_galley.size().y * 0.5).max(title_clip.min.y);
        let title_x = (center_x - title_galley.size().x * 0.5).max(title_clip.min.x);
        painter
            .with_clip_rect(title_clip)
            .galley(Pos2::new(title_x, title_y), title_galley, foil);

        if !author.trim().is_empty() {
            let author_clip = Rect::from_min_max(
                Pos2::new(text_left, rect.min.y + rect.height() * 0.73),
                Pos2::new(text_right, rect.max.y - frame_inset - 4.0),
            );
            let author_color = foil.gamma_multiply(0.82);
            let author_galley = painter.layout(
                author.to_string(),
                egui::FontId::proportional(9.2),
                author_color,
                max_text_w,
            );
            let author_y =
                (author_clip.center().y - author_galley.size().y * 0.5).max(author_clip.min.y);
            let author_x = (center_x - author_galley.size().x * 0.5).max(author_clip.min.x);
            painter.with_clip_rect(author_clip).galley(
                Pos2::new(author_x, author_y),
                author_galley,
                author_color,
            );
        }
    }
}

pub(super) fn render_add_book_card(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    t: &Translations,
    is_cozy: bool,
    theme: &ReaderTheme,
    actions: &mut LibraryActions,
) {
    let card_fill = if is_cozy {
        CozyPalette::SURFACE_LOW.gamma_multiply(0.65)
    } else {
        theme.panel_bg_color()
    };
    let border_stroke = Stroke::new(
        1.5,
        if is_cozy {
            CozyPalette::OUTLINE.gamma_multiply(0.55)
        } else {
            theme.accent_color().gamma_multiply(0.32)
        },
    );

    let (card_rect, card_resp) =
        ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    let is_hovered = card_resp.hovered();

    ui.painter().rect_filled(
        card_rect,
        12.0,
        if is_hovered {
            card_fill.gamma_multiply(1.25)
        } else {
            card_fill
        },
    );
    ui.painter()
        .rect_stroke(card_rect, 12.0, border_stroke, StrokeKind::Inside);

    let center_x = card_rect.center().x;
    let circle_cy = card_rect.min.y + card_rect.height() * 0.36;
    let circle_rect = Rect::from_center_size(Pos2::new(center_x, circle_cy), Vec2::splat(40.0));
    let circle_bg = if is_hovered {
        if is_cozy {
            CozyPalette::PRIMARY
        } else {
            theme.accent_color()
        }
    } else if is_cozy {
        CozyPalette::SURFACE_CONTAINER
    } else {
        theme.panel_bg_color().gamma_multiply(1.3)
    };
    let icon_color = if is_hovered {
        Color32::WHITE
    } else if is_cozy {
        CozyPalette::PRIMARY
    } else {
        theme.accent_color()
    };

    ui.painter()
        .circle_filled(circle_rect.center(), 20.0, circle_bg);
    paint_icon(
        ui.painter(),
        Icon::Plus,
        Rect::from_center_size(circle_rect.center(), Vec2::splat(18.0)),
        icon_color,
    );

    let wrap_w = (width - 24.0).max(80.0);
    let title_col = if is_cozy {
        CozyPalette::ON_SURFACE
    } else {
        theme.text_color()
    };
    let title_galley = ui.painter().layout(
        t.library_add_books.to_string(),
        egui::FontId::proportional(13.0),
        title_col,
        wrap_w,
    );
    let title_y = circle_cy + 30.0;
    let title_h = title_galley.size().y;
    ui.painter().galley(
        Pos2::new(center_x - title_galley.size().x * 0.5, title_y),
        title_galley,
        title_col,
    );

    let hint_col = if is_cozy {
        CozyPalette::ON_SURFACE_VARIANT
    } else {
        theme.secondary_text_color()
    };
    let hint_galley = ui.painter().layout(
        t.library_drop_hint.to_string(),
        egui::FontId::proportional(10.5),
        hint_col,
        wrap_w,
    );
    ui.painter().galley(
        Pos2::new(
            center_x - hint_galley.size().x * 0.5,
            title_y + title_h + 6.0,
        ),
        hint_galley,
        hint_col,
    );

    if card_resp.clicked() {
        actions.open_file = true;
    }

    let badge_rect = Rect::from_center_size(
        Pos2::new(center_x, card_rect.max.y - 26.0),
        Vec2::new((width - 24.0).max(96.0), 20.0),
    );
    ui.painter().rect_filled(
        badge_rect,
        4.0,
        if is_cozy {
            CozyPalette::SURFACE_CONTAINER
        } else {
            theme.panel_bg_color().gamma_multiply(1.22)
        },
    );
    let badge_text_color = if is_cozy {
        CozyPalette::ON_SURFACE_VARIANT
    } else {
        theme.secondary_text_color()
    };
    crate::app::icons::paint_dotted_tokens(
        ui.painter(),
        badge_rect,
        &["EPUB", "FB2", "PDF", "CBZ"],
        egui::FontId::proportional(9.5),
        badge_text_color,
        badge_text_color.gamma_multiply(0.7),
    );
}
