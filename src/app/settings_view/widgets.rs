use crate::app::icons::{Icon, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme, contrast_fg};
use crate::core::library::UiDesignMode;
use eframe::egui::{self, Color32, Margin, Pos2, Rect, Response, RichText, Sense, Stroke, Vec2};

#[derive(Clone, Copy)]
pub struct SettingsPalette {
    pub is_dark: bool,
    pub modal_bg: Color32,
    pub header_bg: Color32,
    pub rail_bg: Color32,
    pub section_bg: Color32,
    pub card_bg: Color32,
    pub surface_container: Color32,
    pub surface_highest: Color32,
    pub text: Color32,
    pub text_variant: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub accent_on: Color32,
    pub secondary: Color32,
    pub secondary_bg: Color32,
    pub secondary_fg: Color32,
    pub tertiary: Color32,
    pub tertiary_bg: Color32,
    pub error: Color32,
    pub border: Color32,
    pub border_subtle: Color32,
    pub scrim: Color32,
    pub corner_lg: f32,
    pub corner_md: f32,
    pub corner_sm: f32,
}

impl SettingsPalette {
    pub fn from_theme(theme: &ReaderTheme, design: UiDesignMode) -> Self {
        let is_cozy_radius = design == UiDesignMode::Cozy;
        let corner_lg = if is_cozy_radius { 16.0 } else { 8.0 };
        let corner_md = if is_cozy_radius { 12.0 } else { 6.0 };
        let corner_sm = if is_cozy_radius { 8.0 } else { 5.0 };

        if *theme == ReaderTheme::Warm {
            return Self {
                is_dark: false,
                modal_bg: CozyPalette::CARD_BG,
                header_bg: CozyPalette::SURFACE_LOW,
                rail_bg: CozyPalette::SURFACE_LOW,
                section_bg: CozyPalette::SURFACE_LOW,
                card_bg: CozyPalette::CANVAS,
                surface_container: CozyPalette::SURFACE_CONTAINER,
                surface_highest: CozyPalette::SURFACE_HIGHEST,
                text: CozyPalette::ON_SURFACE,
                text_variant: CozyPalette::ON_SURFACE_VARIANT,
                accent: CozyPalette::PRIMARY,
                accent_soft: CozyPalette::PRIMARY_FIXED,
                accent_on: Color32::WHITE,
                secondary: CozyPalette::SECONDARY,
                secondary_bg: CozyPalette::SECONDARY_FIXED,
                secondary_fg: CozyPalette::ON_SECONDARY_FIXED,
                tertiary: CozyPalette::TERTIARY,
                tertiary_bg: CozyPalette::TERTIARY_FIXED,
                error: CozyPalette::ERROR,
                border: CozyPalette::OUTLINE.gamma_multiply(0.55),
                border_subtle: CozyPalette::OUTLINE.gamma_multiply(0.32),
                scrim: Color32::from_rgba_unmultiplied(35, 25, 21, 130),
                corner_lg,
                corner_md,
                corner_sm,
            };
        }

        if !theme.is_dark() {
            let panel = theme.panel_bg_color();
            let canvas = theme.bg_color();
            let accent = theme.accent_color();
            return Self {
                is_dark: false,
                modal_bg: CozyPalette::CARD_BG,
                header_bg: panel,
                rail_bg: panel,
                section_bg: panel,
                card_bg: canvas,
                surface_container: mix_color(panel, theme.text_color(), 0.05),
                surface_highest: mix_color(panel, theme.text_color(), 0.11),
                text: theme.text_color(),
                text_variant: theme.secondary_text_color(),
                accent,
                accent_soft: mix_color(canvas, accent, 0.18),
                accent_on: contrast_fg(accent),
                secondary: Color32::from_rgb(126, 87, 0),
                secondary_bg: CozyPalette::SECONDARY_FIXED,
                secondary_fg: CozyPalette::ON_SECONDARY_FIXED,
                tertiary: CozyPalette::TERTIARY,
                tertiary_bg: CozyPalette::TERTIARY_FIXED,
                error: CozyPalette::ERROR,
                border: theme.text_color().gamma_multiply(0.16),
                border_subtle: theme.text_color().gamma_multiply(0.09),
                scrim: Color32::from_rgba_unmultiplied(28, 22, 18, 130),
                corner_lg,
                corner_md,
                corner_sm,
            };
        }

        let bg = theme.bg_color();
        let panel = theme.panel_bg_color();
        let elevated = match theme {
            ReaderTheme::Oled => Color32::from_rgb(20, 20, 25),
            ReaderTheme::Dark => Color32::from_rgb(44, 44, 48),
            ReaderTheme::Twilight => Color32::from_rgb(76, 67, 61),
            _ => mix_color(panel, Color32::WHITE, 0.06),
        };
        let modal_bg = match theme {
            ReaderTheme::Oled => Color32::from_rgb(12, 12, 15),
            _ => panel,
        };
        let accent = theme.accent_color();
        Self {
            is_dark: true,
            modal_bg,
            header_bg: elevated,
            rail_bg: elevated,
            section_bg: elevated,
            card_bg: bg,
            surface_container: mix_color(elevated, Color32::WHITE, 0.06),
            surface_highest: mix_color(elevated, Color32::WHITE, 0.14),
            text: theme.text_color(),
            text_variant: theme.secondary_text_color(),
            accent,
            accent_soft: mix_color(elevated, accent, 0.25),
            accent_on: contrast_fg(accent),
            secondary: Color32::from_rgb(235, 185, 85),
            secondary_bg: Color32::from_rgba_unmultiplied(235, 185, 85, 42),
            secondary_fg: Color32::from_rgb(250, 215, 130),
            tertiary: Color32::from_rgb(120, 195, 140),
            tertiary_bg: Color32::from_rgba_unmultiplied(120, 195, 140, 38),
            error: Color32::from_rgb(245, 110, 105),
            border: theme.text_color().gamma_multiply(0.18),
            border_subtle: theme.text_color().gamma_multiply(0.10),
            scrim: Color32::from_black_alpha(165),
            corner_lg,
            corner_md,
            corner_sm,
        }
    }
}

pub fn mix_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| -> u8 { ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8 };
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

pub enum SectionBadge<'a> {
    Adaptive(&'a str),
    Pro(&'a str),
}

pub fn section_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    title: &str,
    badge: Option<SectionBadge<'_>>,
    subtitle: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .fill(p.section_bg)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .corner_radius(p.corner_md)
        .inner_margin(Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(title).size(15.5).strong().color(p.text));
                if let Some(b) = badge {
                    ui.add_space(4.0);
                    match b {
                        SectionBadge::Adaptive(text) => {
                            pill_badge(ui, p.tertiary_bg, p.tertiary, text, 10.5);
                        }
                        SectionBadge::Pro(text) => {
                            pill_badge(ui, p.secondary_bg, p.secondary_fg, text, 10.0);
                        }
                    }
                }
            });
            if let Some(sub) = subtitle
                && !sub.is_empty()
            {
                ui.add_space(2.0);
                ui.label(RichText::new(sub).size(11.5).color(p.text_variant));
            }
            ui.add_space(12.0);
            add_contents(ui);
        });
}

pub fn row_card(ui: &mut egui::Ui, p: &SettingsPalette, add_contents: impl FnOnce(&mut egui::Ui)) {
    row_card_active(ui, p, false, add_contents);
}

pub fn row_card_active(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    active: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let stroke = if active {
        Stroke::new(1.5, p.accent)
    } else {
        Stroke::new(1.0, p.border_subtle)
    };
    egui::Frame::new()
        .fill(p.card_bg)
        .stroke(stroke)
        .corner_radius(p.corner_sm)
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

pub fn pill_badge(ui: &mut egui::Ui, fill: Color32, fg: Color32, text: &str, font_size: f32) {
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), egui::FontId::proportional(font_size), fg)
    });
    let size = Vec2::new(galley.size().x + 12.0, (font_size + 7.0).max(18.0));
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, size.y * 0.5, fill);
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        fg,
    );
}

pub fn kbd_badge(ui: &mut egui::Ui, p: &SettingsPalette, keys: &str) {
    let font = egui::FontId::monospace(10.5);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(keys.to_string(), font, p.text_variant));
    let size = Vec2::new(galley.size().x + 12.0, 20.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(
        rect,
        5.0,
        p.surface_container,
        Stroke::new(1.0, p.border_subtle),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        p.text_variant,
    );
}

pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, p: &SettingsPalette) -> Response {
    let desired = Vec2::new(38.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let track_color = mix_color(p.surface_highest, p.accent, how_on);
        let border_color = if response.hovered() {
            p.accent.gamma_multiply(0.7)
        } else {
            p.border
        };
        ui.painter().rect(
            rect,
            rect.height() * 0.5,
            track_color,
            Stroke::new(if *on { 0.0 } else { 1.0 }, border_color),
            egui::StrokeKind::Inside,
        );
        let radius = rect.height() * 0.5 - 3.0;
        let min_x = rect.min.x + 3.0 + radius;
        let max_x = rect.max.x - 3.0 - radius;
        let knob_x = egui::lerp(min_x..=max_x, how_on);
        let knob_center = Pos2::new(knob_x, rect.center().y);
        let knob_fill = if *on { Color32::WHITE } else { p.modal_bg };
        ui.painter().circle_filled(knob_center, radius, knob_fill);
    }
    response
}

pub fn segmented_control<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    items: &[(T, &str)],
    p: &SettingsPalette,
    use_accent_active: bool,
) -> bool {
    let mut changed = false;
    let avail = ui.available_width();
    let count = items.len().max(1);
    let inner_pad = 3.0;
    let outer_w = avail.max(120.0);
    let btn_w = ((outer_w - inner_pad * 2.0) / count as f32).max(32.0);
    let (container_rect, container_resp) =
        ui.allocate_exact_size(Vec2::new(outer_w, 30.0), Sense::hover());

    ui.painter().rect(
        container_rect,
        p.corner_sm + 1.0,
        p.surface_container,
        Stroke::new(1.0, p.border_subtle),
        egui::StrokeKind::Inside,
    );

    for (idx, (val, label)) in items.iter().enumerate() {
        let is_active = *current == *val;
        let btn_rect = Rect::from_min_size(
            Pos2::new(
                container_rect.min.x + inner_pad + idx as f32 * btn_w,
                container_rect.min.y + inner_pad,
            ),
            Vec2::new(btn_w, container_rect.height() - inner_pad * 2.0),
        );
        let id = container_resp.id.with(("seg_opt", idx));
        let resp = ui.interact(btn_rect, id, Sense::click());
        let (fill, fg) = if is_active {
            if use_accent_active {
                (p.accent, p.accent_on)
            } else {
                (p.modal_bg, p.text)
            }
        } else if resp.hovered() {
            (p.modal_bg.gamma_multiply(0.55), p.text)
        } else {
            (Color32::TRANSPARENT, p.text_variant)
        };
        if fill != Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(btn_rect, (p.corner_sm - 2.0).max(4.0), fill);
        }
        let max_text_w = (btn_rect.width() - 10.0).max(20.0);
        let base_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap((*label).to_string(), egui::FontId::proportional(11.5), fg)
        });
        let galley = if base_galley.size().x <= max_text_w {
            base_galley
        } else {
            ui.fonts_mut(|f| {
                f.layout_no_wrap((*label).to_string(), egui::FontId::proportional(10.0), fg)
            })
        };
        let text_x = if galley.size().x <= max_text_w {
            btn_rect.center().x - galley.size().x * 0.5
        } else {
            btn_rect.min.x + 5.0
        };
        let clipped_painter = ui
            .painter()
            .with_clip_rect(btn_rect.shrink2(Vec2::new(3.0, 1.0)));
        clipped_painter.galley(
            Pos2::new(text_x, btn_rect.center().y - galley.size().y * 0.5),
            galley,
            fg,
        );
        if resp.clicked() && !is_active {
            *current = *val;
            changed = true;
        }
    }
    changed
}

pub fn metric_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: Option<f64>,
    label: &str,
    readout: &str,
    tooltip: Option<&str>,
    p: &SettingsPalette,
) -> (bool, bool) {
    let mut changed = false;
    let mut commit = false;
    let col_w = ui.available_width();
    ui.vertical(|ui| {
        ui.set_max_width(col_w);
        let readout_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                readout.to_string(),
                egui::FontId::proportional(12.0),
                p.text,
            )
        });
        let readout_reserve = readout_galley.size().x + 10.0;
        let label_max_w = (col_w - readout_reserve).max(40.0);
        ui.horizontal(|ui| {
            ui.set_max_width(col_w);
            ui.allocate_ui_with_layout(
                Vec2::new(label_max_w, 18.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(label).size(12.0).strong().color(p.text))
                            .wrap(),
                    );
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(readout).size(12.0).strong().color(p.text));
            });
        });
        ui.add_space(4.0);
        let prev = ui.visuals().clone();
        let prev_slider_w = ui.spacing().slider_width;
        ui.spacing_mut().slider_width = (col_w - 8.0).max(40.0);
        {
            let visuals = ui.visuals_mut();
            visuals.widgets.inactive.bg_fill = p.surface_highest;
            visuals.widgets.inactive.fg_stroke = Stroke::new(1.5, p.accent);
            visuals.widgets.hovered.bg_fill = p.surface_highest;
            visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, p.accent);
            visuals.widgets.active.bg_fill = p.accent_soft;
            visuals.widgets.active.fg_stroke = Stroke::new(2.0, p.accent);
            visuals.selection.bg_fill = p.accent;
        }
        let mut slider = egui::Slider::new(value, range).show_value(false);
        if let Some(s) = step {
            slider = slider.step_by(s);
        }
        let mut resp = ui.add_sized([col_w, 22.0], slider);
        if let Some(tip) = tooltip {
            resp = resp.on_hover_text(tip);
        }
        ui.spacing_mut().slider_width = prev_slider_w;
        *ui.visuals_mut() = prev;
        if resp.changed() {
            changed = true;
        }
        if resp.drag_stopped() || resp.lost_focus() {
            commit = true;
        }
    });
    (changed, commit)
}

pub fn stepper_capsule(
    ui: &mut egui::Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    display_text: &str,
    dec_tip: &str,
    inc_tip: &str,
    p: &SettingsPalette,
) -> bool {
    let mut changed = false;
    egui::Frame::new()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0, p.border_subtle))
        .corner_radius(p.corner_sm + 1.0)
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            let w = ui.available_width();
            let h = 26.0;
            let (rect, capsule_resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
            let btn_sz = Vec2::splat(h);
            let dec_rect = Rect::from_min_size(rect.min, btn_sz);
            let inc_rect = Rect::from_min_size(Pos2::new(rect.max.x - h, rect.min.y), btn_sz);

            let dec_resp = ui
                .interact(dec_rect, capsule_resp.id.with("stp_dec"), Sense::click())
                .on_hover_text(dec_tip);
            let inc_resp = ui
                .interact(inc_rect, capsule_resp.id.with("stp_inc"), Sense::click())
                .on_hover_text(inc_tip);

            let can_dec = *value > min + 0.001;
            let can_inc = *value < max - 0.001;

            let paint_btn = |r: Rect, icon: Icon, enabled: bool, hovered: bool| {
                let bg = if !enabled {
                    p.surface_container.gamma_multiply(0.5)
                } else if hovered {
                    p.surface_highest
                } else {
                    p.surface_container
                };
                let fg = if enabled {
                    p.text
                } else {
                    p.text_variant.gamma_multiply(0.45)
                };
                ui.painter()
                    .rect_filled(r, (p.corner_sm - 2.0).max(4.0), bg);
                paint_icon(
                    ui.painter(),
                    icon,
                    Rect::from_center_size(r.center(), Vec2::splat(11.0)),
                    fg,
                );
            };
            paint_btn(dec_rect, Icon::Minus, can_dec, dec_resp.hovered());
            paint_btn(inc_rect, Icon::Plus, can_inc, inc_resp.hovered());

            let galley = ui.fonts_mut(|f| {
                f.layout_no_wrap(
                    display_text.to_string(),
                    egui::FontId::proportional(13.0),
                    p.text,
                )
            });
            ui.painter().galley(
                Pos2::new(
                    rect.center().x - galley.size().x * 0.5,
                    rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                p.text,
            );

            if dec_resp.clicked() && can_dec {
                *value = (*value - step).clamp(min, max);
                changed = true;
            }
            if inc_resp.clicked() && can_inc {
                *value = (*value + step).clamp(min, max);
                changed = true;
            }
        });
    changed
}

pub fn themed_button(
    ui: &mut egui::Ui,
    text: &str,
    primary: bool,
    p: &SettingsPalette,
) -> Response {
    let (bg, fg, stroke) = if primary {
        (p.accent, p.accent_on, Stroke::NONE)
    } else {
        (
            p.surface_container,
            p.text,
            Stroke::new(1.0, p.border_subtle),
        )
    };
    ui.add(
        egui::Button::new(RichText::new(text).size(12.0).strong().color(fg))
            .fill(bg)
            .stroke(stroke)
            .corner_radius(p.corner_sm),
    )
}
