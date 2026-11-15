use super::QuietReaderApp;
use super::icons::{Icon, paint_icon};
use super::theme::CozyPalette;
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};

pub(super) fn measure_opds_nav_width(
    ui: &mut egui::Ui,
    t: &Translations,
    compact: bool,
    ultra_compact: bool,
) -> f32 {
    if ultra_compact {
        return 30.0;
    }
    let label = if compact {
        "OPDS"
    } else {
        t.extra.opds_nav_label
    };
    let text_w = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        )
        .size()
        .x
    });
    text_w + 32.0
}

pub(super) fn render_opds_nav_pill(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    is_cozy: bool,
    compact: bool,
    ultra_compact: bool,
) {
    let t = app.settings.ui_language.t();
    let label = if ultra_compact {
        ""
    } else if compact {
        "OPDS"
    } else {
        t.extra.opds_nav_label
    };
    let active = app.show_opds_dialog;
    let w = measure_opds_nav_width(ui, t, compact, ultra_compact);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 26.0), egui::Sense::click());

    let primary = if is_cozy {
        CozyPalette::PRIMARY
    } else {
        app.theme.accent_color()
    };
    let bg = if active {
        primary
    } else if resp.hovered() {
        if is_cozy {
            CozyPalette::SURFACE_HIGH
        } else {
            app.theme.panel_bg_color().gamma_multiply(1.25)
        }
    } else if is_cozy {
        CozyPalette::SURFACE_CONTAINER
    } else {
        app.theme.panel_bg_color().gamma_multiply(1.1)
    };

    let fg = if active {
        Color32::WHITE
    } else if is_cozy {
        CozyPalette::ON_SURFACE
    } else {
        app.theme.text_color()
    };

    ui.painter().rect_filled(rect, 8.0, bg);
    if !active {
        ui.painter().rect_stroke(
            rect,
            8.0,
            Stroke::new(
                1.0,
                if is_cozy {
                    CozyPalette::OUTLINE.gamma_multiply(0.45)
                } else {
                    app.theme.accent_color().gamma_multiply(0.25)
                },
            ),
            StrokeKind::Inside,
        );
    }

    if label.is_empty() {
        paint_icon(
            ui.painter(),
            Icon::Globe,
            Rect::from_center_size(rect.center(), Vec2::splat(13.0)),
            fg,
        );
    } else {
        let icon_center = Pos2::new(rect.min.x + 12.5, rect.center().y);
        paint_icon(
            ui.painter(),
            Icon::Globe,
            Rect::from_center_size(icon_center, Vec2::splat(12.5)),
            fg,
        );
        ui.painter().text(
            Pos2::new(rect.min.x + 22.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(11.5),
            fg,
        );
    }

    if resp.on_hover_text(t.opds_catalogs_tooltip).clicked() {
        app.show_opds_dialog = !app.show_opds_dialog;
        if app.show_opds_dialog {
            app.show_recent_dialog = false;
            app.opds_state.ensure_initial_catalog_loaded(&app.settings);
        }
    }
}
