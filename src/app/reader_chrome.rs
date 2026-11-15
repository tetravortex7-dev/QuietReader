use super::icons::{Icon, paint_icon};
use super::theme::{CozyPalette, ReaderTheme};
use crate::core::Command;
use crate::core::library::UiDesignMode;
use crate::document::{Document, strip_formatting_tags};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct ReaderSurfaceColors {
    pub outer_canvas_bg: Color32,
    pub cover_rim_bg: Color32,
    pub cover_rim_border: Color32,
    pub paper_deck_outer: Color32,
    pub paper_deck_inner: Color32,
    pub page_bg: Color32,
    pub page_ink: Color32,
    pub page_muted: Color32,
    pub accent: Color32,
    pub accent_soft_bg: Color32,
    pub ribbon_active: Color32,
    pub ribbon_fold: Color32,
    pub ribbon_inactive: Color32,
    pub quote_fill: Color32,
    pub quote_border: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub bar_bg: Color32,
    pub bar_border: Color32,
    pub pill_bg: Color32,
    pub pill_text: Color32,
    pub spine_shadow_alpha: u8,
    pub is_dark: bool,
}

impl ReaderSurfaceColors {
    pub fn for_theme(theme: &ReaderTheme, design: UiDesignMode) -> Self {
        let is_cozy = design == UiDesignMode::Cozy;
        match theme {
            ReaderTheme::Warm => Self {
                outer_canvas_bg: if is_cozy {
                    Color32::from_rgb(245, 239, 232)
                } else {
                    theme.bg_color()
                },
                cover_rim_bg: Color32::from_rgb(45, 36, 32),
                cover_rim_border: Color32::from_rgb(65, 53, 47),
                paper_deck_outer: Color32::from_rgb(229, 215, 202),
                paper_deck_inner: Color32::from_rgb(239, 228, 216),
                page_bg: if is_cozy {
                    Color32::from_rgb(250, 246, 240)
                } else {
                    theme.bg_color()
                },
                page_ink: Color32::from_rgb(47, 39, 35),
                page_muted: Color32::from_rgb(126, 111, 100),
                accent: Color32::from_rgb(163, 82, 55),
                accent_soft_bg: Color32::from_rgb(239, 226, 218),
                ribbon_active: Color32::from_rgb(200, 109, 81),
                ribbon_fold: Color32::from_rgb(159, 79, 54),
                ribbon_inactive: Color32::from_rgba_unmultiplied(200, 109, 81, 80),
                quote_fill: Color32::from_rgba_unmultiplied(243, 235, 224, 180),
                quote_border: Color32::from_rgba_unmultiplied(200, 109, 81, 125),
                card_bg: CozyPalette::CARD_BG,
                card_border: Color32::from_rgb(235, 224, 213),
                bar_bg: Color32::from_rgb(253, 251, 247),
                bar_border: Color32::from_rgba_unmultiplied(219, 193, 186, 140),
                pill_bg: Color32::from_rgb(239, 231, 222),
                pill_text: Color32::from_rgb(74, 63, 58),
                spine_shadow_alpha: 22,
                is_dark: false,
            },
            ReaderTheme::Sepia => Self {
                outer_canvas_bg: Color32::from_rgb(234, 224, 206),
                cover_rim_bg: Color32::from_rgb(56, 42, 30),
                cover_rim_border: Color32::from_rgb(78, 60, 44),
                paper_deck_outer: Color32::from_rgb(218, 205, 180),
                paper_deck_inner: Color32::from_rgb(231, 220, 198),
                page_bg: theme.bg_color(),
                page_ink: theme.text_color(),
                page_muted: Color32::from_rgb(122, 104, 84),
                accent: theme.accent_color(),
                accent_soft_bg: Color32::from_rgba_unmultiplied(140, 90, 40, 34),
                ribbon_active: Color32::from_rgb(168, 92, 52),
                ribbon_fold: Color32::from_rgb(130, 68, 36),
                ribbon_inactive: Color32::from_rgba_unmultiplied(168, 92, 52, 75),
                quote_fill: Color32::from_rgba_unmultiplied(228, 217, 194, 175),
                quote_border: theme.accent_color().gamma_multiply(0.55),
                card_bg: Color32::from_rgb(251, 246, 236),
                card_border: Color32::from_rgb(215, 200, 175),
                bar_bg: Color32::from_rgb(248, 242, 230),
                bar_border: Color32::from_rgb(214, 198, 172),
                pill_bg: Color32::from_rgb(232, 221, 200),
                pill_text: Color32::from_rgb(75, 60, 46),
                spine_shadow_alpha: 20,
                is_dark: false,
            },
            ReaderTheme::Light => Self {
                outer_canvas_bg: Color32::from_rgb(237, 240, 244),
                cover_rim_bg: Color32::from_rgb(44, 52, 64),
                cover_rim_border: Color32::from_rgb(66, 76, 92),
                paper_deck_outer: Color32::from_rgb(216, 222, 230),
                paper_deck_inner: Color32::from_rgb(232, 236, 242),
                page_bg: Color32::from_rgb(253, 254, 255),
                page_ink: theme.text_color(),
                page_muted: Color32::from_rgb(104, 114, 128),
                accent: theme.accent_color(),
                accent_soft_bg: theme.accent_color().gamma_multiply(0.14),
                ribbon_active: theme.accent_color(),
                ribbon_fold: Color32::from_rgb(0, 74, 152),
                ribbon_inactive: theme.accent_color().gamma_multiply(0.28),
                quote_fill: Color32::from_rgba_unmultiplied(238, 242, 248, 190),
                quote_border: theme.accent_color().gamma_multiply(0.55),
                card_bg: Color32::WHITE,
                card_border: Color32::from_rgb(214, 220, 228),
                bar_bg: Color32::from_rgb(250, 251, 253),
                bar_border: Color32::from_rgb(218, 224, 232),
                pill_bg: Color32::from_rgb(232, 236, 242),
                pill_text: Color32::from_rgb(54, 62, 74),
                spine_shadow_alpha: 18,
                is_dark: false,
            },
            ReaderTheme::Mint => Self {
                outer_canvas_bg: Color32::from_rgb(225, 236, 226),
                cover_rim_bg: Color32::from_rgb(38, 52, 42),
                cover_rim_border: Color32::from_rgb(56, 74, 60),
                paper_deck_outer: Color32::from_rgb(202, 220, 204),
                paper_deck_inner: Color32::from_rgb(220, 234, 221),
                page_bg: Color32::from_rgb(243, 249, 243),
                page_ink: theme.text_color(),
                page_muted: Color32::from_rgb(84, 108, 90),
                accent: theme.accent_color(),
                accent_soft_bg: theme.accent_color().gamma_multiply(0.15),
                ribbon_active: theme.accent_color(),
                ribbon_fold: Color32::from_rgb(30, 92, 58),
                ribbon_inactive: theme.accent_color().gamma_multiply(0.28),
                quote_fill: Color32::from_rgba_unmultiplied(226, 239, 227, 190),
                quote_border: theme.accent_color().gamma_multiply(0.55),
                card_bg: Color32::from_rgb(249, 253, 249),
                card_border: Color32::from_rgb(198, 218, 200),
                bar_bg: Color32::from_rgb(242, 248, 242),
                bar_border: Color32::from_rgb(198, 216, 200),
                pill_bg: Color32::from_rgb(220, 232, 221),
                pill_text: Color32::from_rgb(44, 66, 50),
                spine_shadow_alpha: 18,
                is_dark: false,
            },
            ReaderTheme::Twilight => Self {
                outer_canvas_bg: Color32::from_rgb(38, 32, 29),
                cover_rim_bg: Color32::from_rgb(26, 21, 19),
                cover_rim_border: Color32::from_rgb(76, 65, 58),
                paper_deck_outer: Color32::from_rgb(44, 37, 34),
                paper_deck_inner: Color32::from_rgb(49, 42, 38),
                page_bg: theme.bg_color(),
                page_ink: theme.text_color(),
                page_muted: Color32::from_rgb(164, 150, 136),
                accent: theme.accent_color(),
                accent_soft_bg: theme.accent_color().gamma_multiply(0.2),
                ribbon_active: theme.accent_color(),
                ribbon_fold: Color32::from_rgb(168, 108, 42),
                ribbon_inactive: theme.accent_color().gamma_multiply(0.32),
                quote_fill: Color32::from_rgba_unmultiplied(68, 59, 53, 185),
                quote_border: theme.accent_color().gamma_multiply(0.6),
                card_bg: Color32::from_rgb(64, 56, 51),
                card_border: Color32::from_rgb(92, 80, 72),
                bar_bg: Color32::from_rgb(48, 41, 37),
                bar_border: Color32::from_rgb(80, 69, 62),
                pill_bg: Color32::from_rgb(66, 57, 52),
                pill_text: Color32::from_rgb(228, 216, 200),
                spine_shadow_alpha: 38,
                is_dark: true,
            },
            ReaderTheme::Dark => Self {
                outer_canvas_bg: Color32::from_rgb(22, 22, 25),
                cover_rim_bg: Color32::from_rgb(14, 14, 16),
                cover_rim_border: Color32::from_rgb(56, 56, 62),
                paper_deck_outer: Color32::from_rgb(24, 24, 27),
                paper_deck_inner: Color32::from_rgb(29, 29, 32),
                page_bg: theme.bg_color(),
                page_ink: theme.text_color(),
                page_muted: Color32::from_rgb(146, 150, 158),
                accent: theme.accent_color(),
                accent_soft_bg: theme.accent_color().gamma_multiply(0.2),
                ribbon_active: theme.accent_color(),
                ribbon_fold: Color32::from_rgb(65, 114, 196),
                ribbon_inactive: theme.accent_color().gamma_multiply(0.3),
                quote_fill: Color32::from_rgba_unmultiplied(40, 40, 45, 195),
                quote_border: theme.accent_color().gamma_multiply(0.6),
                card_bg: Color32::from_rgb(38, 38, 42),
                card_border: Color32::from_rgb(68, 68, 76),
                bar_bg: Color32::from_rgb(30, 30, 33),
                bar_border: Color32::from_rgb(58, 58, 64),
                pill_bg: Color32::from_rgb(44, 44, 50),
                pill_text: Color32::from_rgb(220, 222, 228),
                spine_shadow_alpha: 42,
                is_dark: true,
            },
            ReaderTheme::Oled => Self {
                outer_canvas_bg: Color32::BLACK,
                cover_rim_bg: Color32::from_rgb(10, 10, 12),
                cover_rim_border: Color32::from_rgb(40, 40, 46),
                paper_deck_outer: Color32::from_rgb(6, 6, 8),
                paper_deck_inner: Color32::from_rgb(11, 11, 14),
                page_bg: Color32::from_rgb(4, 4, 6),
                page_ink: theme.text_color(),
                page_muted: Color32::from_rgb(138, 142, 150),
                accent: theme.accent_color(),
                accent_soft_bg: theme.accent_color().gamma_multiply(0.22),
                ribbon_active: theme.accent_color(),
                ribbon_fold: Color32::from_rgb(56, 100, 185),
                ribbon_inactive: theme.accent_color().gamma_multiply(0.28),
                quote_fill: Color32::from_rgba_unmultiplied(20, 20, 24, 210),
                quote_border: theme.accent_color().gamma_multiply(0.55),
                card_bg: Color32::from_rgb(16, 16, 20),
                card_border: Color32::from_rgb(46, 46, 54),
                bar_bg: Color32::from_rgb(8, 8, 10),
                bar_border: Color32::from_rgb(38, 38, 44),
                pill_bg: Color32::from_rgb(22, 22, 26),
                pill_text: Color32::from_rgb(210, 210, 218),
                spine_shadow_alpha: 36,
                is_dark: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BookSpreadFrameLayout {
    pub cover_rect: Rect,
    pub outer_deck_rect: Rect,
    pub inner_deck_rect: Rect,
    pub sheet_rect: Rect,
    pub running_header_rect: Rect,
    pub content_rect: Rect,
    pub running_footer_rect: Rect,
    pub left_nav_center: Pos2,
    pub right_nav_center: Pos2,
    pub effective_two_columns: bool,
    pub compact: bool,
}

pub fn compute_book_spread_layout(
    avail: Rect,
    max_column_width: f32,
    want_two_columns: bool,
) -> BookSpreadFrameLayout {
    let w = avail.width().max(260.0);
    let h = avail.height().max(220.0);
    let compact = w < 820.0 || h < 540.0;
    let outer_chevrons = w >= 980.0;

    let side_gutter = if outer_chevrons {
        58.0
    } else if compact {
        6.0
    } else {
        16.0
    };
    let v_pad = if h >= 720.0 {
        (h * 0.028).clamp(14.0, 24.0)
    } else {
        (h * 0.015).clamp(4.0, 10.0)
    };

    let max_usable_w = (w - side_gutter * 2.0).max(240.0);
    let casing_h_pad = if compact { 4.0 } else { 10.0 };
    let effective_two_columns = want_two_columns && (max_usable_w - casing_h_pad * 2.0) >= 840.0;

    let cover_w = if effective_two_columns {
        (max_column_width * 1.48)
            .clamp(900.0, 1180.0)
            .min(max_usable_w)
    } else {
        (max_column_width + 96.0)
            .clamp(680.0, 900.0)
            .min(max_usable_w)
    };
    let cover_h = (h - v_pad * 2.0).clamp(200.0, 1040.0);

    let cover_rect = Rect::from_center_size(avail.center(), Vec2::new(cover_w, cover_h));
    let outer_deck_rect = Rect::from_min_max(
        Pos2::new(
            cover_rect.min.x + casing_h_pad,
            cover_rect.min.y + if compact { 3.0 } else { 6.0 },
        ),
        Pos2::new(
            cover_rect.max.x - casing_h_pad,
            cover_rect.max.y - if compact { 4.0 } else { 10.0 },
        ),
    );
    let inner_deck_rect = Rect::from_min_max(
        Pos2::new(
            outer_deck_rect.min.x + if compact { 1.5 } else { 3.0 },
            outer_deck_rect.min.y,
        ),
        Pos2::new(
            outer_deck_rect.max.x - if compact { 1.5 } else { 3.0 },
            outer_deck_rect.max.y - if compact { 1.5 } else { 3.0 },
        ),
    );
    let sheet_rect = Rect::from_min_max(
        Pos2::new(
            inner_deck_rect.min.x + if compact { 1.5 } else { 3.0 },
            inner_deck_rect.min.y,
        ),
        Pos2::new(
            inner_deck_rect.max.x - if compact { 1.5 } else { 3.0 },
            inner_deck_rect.max.y - if compact { 1.5 } else { 3.0 },
        ),
    );

    let sheet_h_pad = if compact { 16.0 } else { 36.0 };
    let header_h = if compact { 24.0 } else { 32.0 };
    let footer_h = if compact { 22.0 } else { 28.0 };

    let running_header_rect = Rect::from_min_max(
        Pos2::new(
            sheet_rect.min.x + sheet_h_pad,
            sheet_rect.min.y + if compact { 6.0 } else { 12.0 },
        ),
        Pos2::new(
            sheet_rect.max.x - sheet_h_pad,
            sheet_rect.min.y + if compact { 6.0 } else { 12.0 } + header_h,
        ),
    );
    let running_footer_rect = Rect::from_min_max(
        Pos2::new(
            sheet_rect.min.x + sheet_h_pad,
            sheet_rect.max.y - if compact { 6.0 } else { 10.0 } - footer_h,
        ),
        Pos2::new(
            sheet_rect.max.x - sheet_h_pad,
            sheet_rect.max.y - if compact { 6.0 } else { 10.0 },
        ),
    );
    let footer_gap = if compact { 8.0 } else { 12.0 };
    let header_gap = if compact { 4.0 } else { 6.0 };
    let content_rect = Rect::from_min_max(
        Pos2::new(
            sheet_rect.min.x + 12.0,
            running_header_rect.max.y + header_gap,
        ),
        Pos2::new(
            sheet_rect.max.x - 12.0,
            (running_footer_rect.min.y - footer_gap).max(running_header_rect.max.y + 40.0),
        ),
    );

    let (left_nav_x, right_nav_x) = if outer_chevrons {
        (
            ((cover_rect.min.x + avail.min.x) * 0.5).min(cover_rect.min.x - 24.0),
            ((cover_rect.max.x + avail.max.x) * 0.5).max(cover_rect.max.x + 24.0),
        )
    } else {
        (cover_rect.min.x + 18.0, cover_rect.max.x - 18.0)
    };
    let nav_y = sheet_rect.center().y;

    BookSpreadFrameLayout {
        cover_rect,
        outer_deck_rect,
        inner_deck_rect,
        sheet_rect,
        running_header_rect,
        content_rect,
        running_footer_rect,
        left_nav_center: Pos2::new(left_nav_x, nav_y),
        right_nav_center: Pos2::new(right_nav_x, nav_y),
        effective_two_columns,
        compact,
    }
}

#[derive(Default, Clone, Debug)]
pub struct BookChromeResponse {
    pub toggle_bookmark: bool,
    pub nav_command: Option<Command>,
}

pub fn paint_book_backdrop(
    ui: &mut egui::Ui,
    layout: &BookSpreadFrameLayout,
    colors: &ReaderSurfaceColors,
) {
    let painter = ui.painter();
    painter.rect_filled(
        layout.cover_rect.expand2(Vec2::new(5.0, 6.0)),
        18.0,
        Color32::from_black_alpha(12),
    );
    painter.rect_filled(
        layout.cover_rect.translate(Vec2::new(0.0, 5.0)).expand(2.0),
        16.0,
        Color32::from_black_alpha(24),
    );

    let cover_r = if layout.compact { 8.0 } else { 14.0 };
    painter.rect(
        layout.cover_rect,
        cover_r,
        colors.cover_rim_bg,
        Stroke::new(1.0, colors.cover_rim_border),
        StrokeKind::Inside,
    );

    let deck_r = if layout.compact { 4.0 } else { 6.0 };
    painter.rect_filled(layout.outer_deck_rect, deck_r, colors.paper_deck_outer);
    painter.rect_filled(layout.inner_deck_rect, deck_r, colors.paper_deck_inner);
    painter.rect(
        layout.sheet_rect,
        deck_r,
        colors.page_bg,
        Stroke::new(1.0, colors.bar_border.gamma_multiply(0.55)),
        StrokeKind::Inside,
    );

    if layout.effective_two_columns {
        let cx = layout.sheet_rect.center().x;
        let top = layout.sheet_rect.min.y;
        let bot = layout.sheet_rect.max.y;
        let bands: [(f32, u8); 4] = [
            (22.0, colors.spine_shadow_alpha / 4),
            (12.0, colors.spine_shadow_alpha / 2),
            (5.0, (colors.spine_shadow_alpha as f32 * 0.78) as u8),
            (1.5, colors.spine_shadow_alpha),
        ];
        for (half_w, alpha) in bands {
            let r = Rect::from_min_max(Pos2::new(cx - half_w, top), Pos2::new(cx + half_w, bot));
            painter.rect_filled(r, 0.0, Color32::from_black_alpha(alpha));
        }
        painter.line_segment(
            [Pos2::new(cx, top), Pos2::new(cx, bot)],
            Stroke::new(
                1.0,
                Color32::from_black_alpha(colors.spine_shadow_alpha.saturating_add(10)),
            ),
        );
    }
}

pub fn paint_book_foreground(
    ui: &mut egui::Ui,
    layout: &BookSpreadFrameLayout,
    colors: &ReaderSurfaceColors,
    is_rtl: bool,
    meta_author: &str,
    meta_title: &str,
    chapter_title: &str,
    format_badge: &str,
    cur_page: usize,
    total_pages: usize,
    has_bookmark: bool,
    bookmark_tooltip: &str,
    prev_tooltip: &str,
    next_tooltip: &str,
) -> BookChromeResponse {
    let mut result = BookChromeResponse::default();

    let clean_author = meta_author.trim();
    let left_hdr = if !clean_author.is_empty() {
        truncate_upper(clean_author, 32)
    } else {
        format_badge.to_ascii_uppercase()
    };
    let right_hdr = truncate_upper(meta_title.trim(), 38);
    let hdr_font = FontId::proportional(if layout.compact { 9.5 } else { 10.5 });

    let painter = ui.painter();
    let left_anchor = Pos2::new(
        layout.running_header_rect.min.x,
        layout.running_header_rect.center().y,
    );
    let right_anchor = Pos2::new(
        layout.running_header_rect.max.x,
        layout.running_header_rect.center().y,
    );

    if !left_hdr.is_empty() {
        painter.text(
            left_anchor,
            egui::Align2::LEFT_CENTER,
            &left_hdr,
            hdr_font.clone(),
            colors.page_muted,
        );
    }
    if !right_hdr.is_empty() && layout.running_header_rect.width() >= 340.0 {
        painter.text(
            right_anchor,
            egui::Align2::RIGHT_CENTER,
            &right_hdr,
            hdr_font,
            colors.page_muted,
        );
    }

    let ftr = layout.running_footer_rect;
    let ftr_font = FontId::proportional(if layout.compact { 10.0 } else { 11.0 });
    let ftr_y = ftr.center().y;
    let total = total_pages.max(1);
    let cur = cur_page.clamp(1, total);

    if layout.effective_two_columns {
        painter.text(
            Pos2::new(ftr.min.x, ftr_y),
            egui::Align2::LEFT_CENTER,
            cur.to_string(),
            ftr_font.clone(),
            colors.page_muted,
        );

        let left_mid_end = layout.sheet_rect.center().x - 24.0;
        let short_book = truncate_plain(meta_title.trim(), 22);
        if !short_book.is_empty() && left_mid_end - ftr.min.x > 160.0 {
            let brand_galley = painter.layout_no_wrap(
                "QuietReader".to_string(),
                ftr_font.clone(),
                colors.page_muted.gamma_multiply(0.75),
            );
            let title_galley = painter.layout_no_wrap(
                short_book,
                ftr_font.clone(),
                colors.page_muted.gamma_multiply(0.75),
            );
            let total_w = brand_galley.size().x + 12.0 + title_galley.size().x;
            let start_x = left_mid_end - total_w;
            painter.galley(
                Pos2::new(start_x, ftr_y - brand_galley.size().y * 0.5),
                brand_galley.clone(),
                colors.page_muted.gamma_multiply(0.75),
            );
            painter.circle_filled(
                Pos2::new(start_x + brand_galley.size().x + 6.0, ftr_y),
                1.4,
                colors.page_muted.gamma_multiply(0.65),
            );
            painter.galley(
                Pos2::new(
                    start_x + brand_galley.size().x + 12.0,
                    ftr_y - title_galley.size().y * 0.5,
                ),
                title_galley,
                colors.page_muted.gamma_multiply(0.75),
            );
        }

        let right_mid_start = layout.sheet_rect.center().x + 24.0;
        let clean_chap = truncate_plain(chapter_title.trim(), 30);
        if !clean_chap.is_empty() && ftr.max.x - right_mid_start > 160.0 {
            painter.text(
                Pos2::new(right_mid_start, ftr_y),
                egui::Align2::LEFT_CENTER,
                clean_chap,
                ftr_font.clone(),
                colors.page_muted.gamma_multiply(0.75),
            );
        }
        if spread_right_page_visible(ui.ctx()) {
            let right_page = (cur + 1).min(total);
            painter.text(
                Pos2::new(ftr.max.x, ftr_y),
                egui::Align2::RIGHT_CENTER,
                right_page.to_string(),
                ftr_font,
                colors.page_muted,
            );
        }
    } else {
        let clean_chap = truncate_plain(chapter_title.trim(), 28);
        if !clean_chap.is_empty() && ftr.width() > 260.0 {
            painter.text(
                Pos2::new(ftr.min.x, ftr_y),
                egui::Align2::LEFT_CENTER,
                clean_chap,
                ftr_font.clone(),
                colors.page_muted.gamma_multiply(0.75),
            );
        }
        painter.text(
            Pos2::new(ftr.max.x, ftr_y),
            egui::Align2::RIGHT_CENTER,
            format!("{cur} / {total}"),
            ftr_font,
            colors.page_muted,
        );
    }

    let rib_w = if layout.compact { 18.0 } else { 22.0 };
    let rib_h = if has_bookmark {
        if layout.compact { 26.0 } else { 36.0 }
    } else if layout.compact {
        16.0
    } else {
        22.0
    };
    let rib_top = layout.sheet_rect.min.y - 2.0;
    let rib_x = layout.sheet_rect.center().x;
    let rib_rect = Rect::from_min_max(
        Pos2::new(rib_x - rib_w * 0.5, rib_top),
        Pos2::new(rib_x + rib_w * 0.5, rib_top + rib_h),
    );
    let rib_resp = ui
        .allocate_rect(rib_rect.expand(4.0), egui::Sense::click())
        .on_hover_text(bookmark_tooltip);
    let rib_color = if has_bookmark || rib_resp.hovered() {
        colors.ribbon_active
    } else {
        colors.ribbon_inactive
    };
    let notch = if has_bookmark { 6.0 } else { 4.5 };
    let left_half = vec![
        Pos2::new(rib_rect.min.x, rib_rect.min.y),
        Pos2::new(rib_rect.center().x, rib_rect.min.y),
        Pos2::new(rib_rect.center().x, rib_rect.max.y - notch),
        Pos2::new(rib_rect.min.x, rib_rect.max.y),
    ];
    let right_half = vec![
        Pos2::new(rib_rect.center().x, rib_rect.min.y),
        Pos2::new(rib_rect.max.x, rib_rect.min.y),
        Pos2::new(rib_rect.max.x, rib_rect.max.y),
        Pos2::new(rib_rect.center().x, rib_rect.max.y - notch),
    ];
    let p = ui.painter();
    p.add(egui::Shape::convex_polygon(
        left_half,
        rib_color,
        Stroke::NONE,
    ));
    p.add(egui::Shape::convex_polygon(
        right_half,
        rib_color,
        Stroke::NONE,
    ));
    if has_bookmark || rib_resp.hovered() {
        p.rect_filled(
            Rect::from_min_max(
                rib_rect.min,
                Pos2::new(rib_rect.max.x, rib_rect.min.y + 4.0),
            ),
            0.0,
            colors.ribbon_fold,
        );
    }
    if rib_resp.clicked() {
        result.toggle_bookmark = true;
    }

    let btn_r = if layout.compact { 15.0 } else { 19.0 };
    let draw_nav_btn = |ui: &mut egui::Ui, center: Pos2, icon: Icon, tip: &str| -> bool {
        let hit = Rect::from_center_size(center, Vec2::splat(btn_r * 2.0));
        let resp = ui
            .allocate_rect(hit, egui::Sense::click())
            .on_hover_text(tip);
        let fill = if resp.hovered() {
            colors.card_bg
        } else {
            colors.card_bg.gamma_multiply(0.92)
        };
        let stroke = if resp.hovered() {
            colors.accent.gamma_multiply(0.6)
        } else {
            colors.card_border
        };
        let p = ui.painter();
        p.circle_filled(
            center + Vec2::new(0.0, 1.5),
            btn_r,
            Color32::from_black_alpha(14),
        );
        p.circle(center, btn_r, fill, Stroke::new(1.0, stroke));
        paint_icon(
            p,
            icon,
            Rect::from_center_size(center, Vec2::splat(btn_r * 0.92)),
            if resp.hovered() {
                colors.accent
            } else {
                colors.page_muted
            },
        );
        resp.clicked()
    };

    if draw_nav_btn(ui, layout.left_nav_center, Icon::ArrowLeft, prev_tooltip) {
        result.nav_command = Some(if is_rtl {
            Command::NextPage
        } else {
            Command::PrevPage
        });
    }
    if draw_nav_btn(ui, layout.right_nav_center, Icon::ArrowRight, next_tooltip) {
        result.nav_command = Some(if is_rtl {
            Command::PrevPage
        } else {
            Command::NextPage
        });
    }

    result
}

pub fn begin_cozy_spread_frame(
    ui: &mut egui::Ui,
    theme: &ReaderTheme,
    design: UiDesignMode,
    settings: &crate::settings::AppSettings,
) -> (ReaderSurfaceColors, BookSpreadFrameLayout) {
    let colors = ReaderSurfaceColors::for_theme(theme, design);
    let layout = compute_book_spread_layout(
        ui.available_rect_before_wrap(),
        settings.effective_max_column_width(),
        settings.text_two_columns,
    );
    set_effective_spread(ui.ctx(), layout.effective_two_columns);
    paint_book_backdrop(ui, &layout, &colors);
    (colors, layout)
}

pub fn set_effective_spread(ctx: &egui::Context, effective: bool) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::unique("quietreader_effective_spread"), effective);
    });
}

pub fn compute_studio_two_columns(avail_w: f32, want_two_columns: bool) -> bool {
    want_two_columns && avail_w >= 840.0
}

pub fn set_spread_right_visible(ctx: &egui::Context, visible: bool) {
    ctx.data_mut(|d| {
        d.insert_temp(
            egui::Id::unique("quietreader_spread_right_visible"),
            visible,
        );
    });
}

pub fn spread_right_page_visible(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(egui::Id::unique("quietreader_spread_right_visible"))
            .unwrap_or(true)
    })
}

pub fn last_effective_spread(ctx: &egui::Context, avail_w: f32, want: bool) -> bool {
    if !want {
        return false;
    }
    ctx.data(|d| d.get_temp::<bool>(egui::Id::unique("quietreader_effective_spread")))
        .unwrap_or_else(|| avail_w >= 960.0)
}

#[allow(clippy::too_many_arguments)]
pub fn finish_cozy_spread_frame(
    ui: &mut egui::Ui,
    layout: &BookSpreadFrameLayout,
    colors: &ReaderSurfaceColors,
    doc: &Option<Arc<dyn Document>>,
    file_path: Option<&std::path::Path>,
    settings: &crate::settings::AppSettings,
    current_chapter: usize,
    current_scroll_y: f32,
    max_scroll_y: f32,
    viewport_height: f32,
) -> BookChromeResponse {
    let Some(doc_ref) = doc.as_ref() else {
        return BookChromeResponse::default();
    };
    let t = settings.ui_language.t();
    let is_rtl = settings.ui_language.is_rtl();
    let meta = doc_ref.metadata();
    let format_badge = doc_ref.format_name();
    let chapter_title = current_chapter_title(doc, current_chapter);
    let (cur_page, total_pages) = effective_text_page_pair(
        doc_ref.chapters(),
        current_chapter,
        current_scroll_y,
        max_scroll_y,
        viewport_height,
        settings.text_paged_scroll,
    );
    let has_bookmark =
        has_quick_bookmark(settings, file_path, current_chapter, current_scroll_y, None);
    paint_book_foreground(
        ui,
        layout,
        colors,
        is_rtl,
        &meta.author,
        &meta.title,
        &chapter_title,
        format_badge,
        cur_page,
        total_pages,
        has_bookmark,
        t.add_bookmark,
        t.prev_section,
        t.next_section,
    )
}

pub fn effective_text_page_pair(
    chapters: &[crate::document::BookChapter],
    current_chapter: usize,
    current_scroll_y: f32,
    max_scroll_y: f32,
    viewport_height: f32,
    text_paged_scroll: bool,
) -> (usize, usize) {
    let (cur_page, total_pages, _) = super::bars::calculate_text_pages(
        chapters,
        current_chapter,
        current_scroll_y,
        max_scroll_y,
    );
    if text_paged_scroll && viewport_height > 1.0 && max_scroll_y > 0.0 {
        let total = ((max_scroll_y + viewport_height) / viewport_height)
            .ceil()
            .max(1.0) as usize;
        let current = ((current_scroll_y / viewport_height).round() as usize + 1).min(total);
        (current, total)
    } else {
        (cur_page, total_pages)
    }
}

pub fn current_chapter_title(doc: &Option<Arc<dyn Document>>, ch_idx: usize) -> String {
    doc.as_ref()
        .and_then(|d| d.chapters().get(ch_idx))
        .map(|ch| strip_formatting_tags(&ch.title))
        .unwrap_or_default()
}

pub fn has_quick_bookmark(
    settings: &crate::settings::AppSettings,
    file_path: Option<&std::path::Path>,
    chapter: usize,
    scroll_y: f32,
    page_num: Option<usize>,
) -> bool {
    let Some(path) = file_path else {
        return false;
    };
    settings.bookmarks.iter().any(|b| {
        b.book_path == path
            && b.chapter == chapter
            && (b.scroll_y - scroll_y).abs() < 25.0
            && b.page_number == page_num
    })
}

fn truncate_upper(s: &str, max_chars: usize) -> String {
    let truncated = truncate_plain(s, max_chars);
    truncated.to_uppercase()
}

fn truncate_plain(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let taken: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        let shorter: String = taken.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{shorter}...")
    } else {
        taken
    }
}

pub fn is_blockquote_paragraph(paragraph: &str) -> bool {
    let trimmed = paragraph.trim();
    if trimmed.len() < 32 {
        return false;
    }
    let Some(inner) = trimmed
        .strip_prefix("<i>")
        .and_then(|s| s.strip_suffix("</i>"))
    else {
        return false;
    };
    !inner.contains("<i>") && !inner.contains("</i>") && !inner.contains("{{image:")
}
