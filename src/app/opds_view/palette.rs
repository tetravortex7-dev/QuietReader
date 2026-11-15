use crate::app::theme::{CozyPalette, ReaderTheme, contrast_fg};
use crate::core::library::UiDesignMode;
use eframe::egui::{self, Color32, Stroke, Vec2};

#[derive(Clone, Copy)]
pub(super) struct OpdsPalette {
    pub canvas_bg: Color32,
    pub sidebar_bg: Color32,
    pub surface_container: Color32,
    pub surface_high: Color32,
    pub surface_highest: Color32,
    pub card_bg: Color32,
    pub card_footer_bg: Color32,
    pub card_border: Color32,
    pub primary: Color32,
    pub primary_soft: Color32,
    pub on_primary: Color32,
    pub secondary: Color32,
    pub secondary_soft: Color32,
    pub tertiary_sage: Color32,
    pub tertiary_sage_bg: Color32,
    pub tertiary_sage_border: Color32,
    pub on_surface: Color32,
    pub on_surface_variant: Color32,
    pub error_fg: Color32,
    pub error_bg: Color32,
    pub category_tints: [(Color32, Color32); 3],
    pub is_dark: bool,
}

impl OpdsPalette {
    pub fn for_theme(theme: &ReaderTheme, design: UiDesignMode) -> Self {
        if !theme.is_dark() && design == UiDesignMode::Cozy {
            Self {
                canvas_bg: CozyPalette::CANVAS,
                sidebar_bg: CozyPalette::SURFACE_LOW,
                surface_container: CozyPalette::SURFACE_CONTAINER,
                surface_high: CozyPalette::SURFACE_HIGH,
                surface_highest: CozyPalette::SURFACE_HIGHEST,
                card_bg: CozyPalette::CARD_BG,
                card_footer_bg: CozyPalette::SURFACE_LOW,
                card_border: CozyPalette::OUTLINE.gamma_multiply(0.45),
                primary: CozyPalette::PRIMARY,
                primary_soft: CozyPalette::PRIMARY_FIXED,
                on_primary: CozyPalette::ON_PRIMARY,
                secondary: CozyPalette::SECONDARY,
                secondary_soft: CozyPalette::SECONDARY_FIXED,
                tertiary_sage: CozyPalette::TERTIARY,
                tertiary_sage_bg: CozyPalette::TERTIARY_FIXED,
                tertiary_sage_border: CozyPalette::TERTIARY.gamma_multiply(0.35),
                on_surface: CozyPalette::ON_SURFACE,
                on_surface_variant: CozyPalette::ON_SURFACE_VARIANT,
                error_fg: CozyPalette::ERROR,
                error_bg: CozyPalette::ERROR_CONTAINER,
                category_tints: [
                    (CozyPalette::SECONDARY_FIXED, CozyPalette::SECONDARY),
                    (CozyPalette::PRIMARY_FIXED, CozyPalette::PRIMARY),
                    (CozyPalette::TERTIARY_FIXED, CozyPalette::TERTIARY),
                ],
                is_dark: false,
            }
        } else if !theme.is_dark() {
            let panel = theme.panel_bg_color();
            let accent = theme.accent_color();
            let text = theme.text_color();
            let muted = theme.secondary_text_color();
            let card = Color32::WHITE;
            Self {
                canvas_bg: theme.bg_color(),
                sidebar_bg: panel,
                surface_container: mix_color(panel, text, 0.05),
                surface_high: mix_color(panel, text, 0.10),
                surface_highest: mix_color(panel, text, 0.14),
                card_bg: card,
                card_footer_bg: mix_color(card, panel, 0.45),
                card_border: muted.gamma_multiply(0.28),
                primary: accent,
                primary_soft: mix_color(card, accent, 0.16),
                on_primary: contrast_fg(accent),
                secondary: CozyPalette::SECONDARY,
                secondary_soft: CozyPalette::SECONDARY_FIXED,
                tertiary_sage: CozyPalette::TERTIARY,
                tertiary_sage_bg: CozyPalette::TERTIARY_FIXED,
                tertiary_sage_border: CozyPalette::TERTIARY.gamma_multiply(0.35),
                on_surface: text,
                on_surface_variant: muted,
                error_fg: CozyPalette::ERROR,
                error_bg: CozyPalette::ERROR_CONTAINER,
                category_tints: [
                    (CozyPalette::SECONDARY_FIXED, CozyPalette::SECONDARY),
                    (mix_color(card, accent, 0.16), accent),
                    (CozyPalette::TERTIARY_FIXED, CozyPalette::TERTIARY),
                ],
                is_dark: false,
            }
        } else {
            let canvas = theme.bg_color();
            let sidebar = theme.panel_bg_color();
            let text = theme.text_color();
            let muted = theme.secondary_text_color();
            let accent = theme.accent_color();
            let card = mix_color(sidebar, text, 0.04);
            let sage = Color32::from_rgb(106, 160, 112);
            let amber = Color32::from_rgb(225, 168, 74);
            let err = Color32::from_rgb(235, 110, 105);
            Self {
                canvas_bg: canvas,
                sidebar_bg: sidebar,
                surface_container: mix_color(sidebar, text, 0.08),
                surface_high: mix_color(sidebar, text, 0.13),
                surface_highest: mix_color(sidebar, text, 0.18),
                card_bg: card,
                card_footer_bg: mix_color(sidebar, text, 0.06),
                card_border: text.gamma_multiply(0.14),
                primary: accent,
                primary_soft: mix_color(sidebar, accent, 0.24),
                on_primary: contrast_fg(accent),
                secondary: amber,
                secondary_soft: mix_color(sidebar, amber, 0.22),
                tertiary_sage: sage,
                tertiary_sage_bg: mix_color(sidebar, sage, 0.22),
                tertiary_sage_border: sage.gamma_multiply(0.45),
                on_surface: text,
                on_surface_variant: muted,
                error_fg: err,
                error_bg: mix_color(sidebar, err, 0.22),
                category_tints: [
                    (mix_color(sidebar, amber, 0.20), amber),
                    (mix_color(sidebar, accent, 0.22), accent),
                    (mix_color(sidebar, sage, 0.20), sage),
                ],
                is_dark: true,
            }
        }
    }
}

pub(super) fn mix_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let inv = 1.0 - t;
    Color32::from_rgb(
        (a.r() as f32 * inv + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * inv + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * inv + b.b() as f32 * t).round() as u8,
    )
}

pub(super) fn pill_badge(
    ui: &mut egui::Ui,
    text: &str,
    bg: Color32,
    fg: Color32,
    font_size: f32,
    stroke: Option<Stroke>,
) {
    let mut frame = egui::Frame::new()
        .fill(bg)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(7, 2));
    if let Some(s) = stroke {
        frame = frame.stroke(s);
    }
    frame.show(ui, |ui| {
        ui.label(egui::RichText::new(text).size(font_size).color(fg));
    });
}

pub(super) fn pill_button(
    ui: &mut egui::Ui,
    text: &str,
    bg: Color32,
    fg: Color32,
    stroke: Stroke,
    font_size: f32,
    min_height: f32,
) -> egui::Response {
    let btn = egui::Button::new(egui::RichText::new(text).size(font_size).color(fg))
        .fill(bg)
        .stroke(stroke)
        .corner_radius(8.0)
        .min_size(Vec2::new(0.0, min_height));
    ui.add(btn)
}
