use super::widgets::{SettingsPalette, kbd_badge, mix_color, pill_badge, themed_button};
use crate::app::dialogs::SettingsTab;
use crate::app::icons::{Icon, paint_icon};
use crate::i18n::Translations;
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub(super) fn nav_items<'a>(t: &'a Translations) -> [(SettingsTab, Icon, &'a str); 7] {
    [
        (
            SettingsTab::Appearance,
            Icon::Appearance,
            t.extra.settings_nav_appearance,
        ),
        (
            SettingsTab::Fonts,
            Icon::Fonts,
            t.extra.settings_nav_typography,
        ),
        (
            SettingsTab::Reading,
            Icon::Ruler,
            t.extra.settings_nav_reading,
        ),
        (
            SettingsTab::AmbienceSpeech,
            Icon::Headphones,
            t.extra.settings_nav_ambience_tts,
        ),
        (
            SettingsTab::DictionariesTranslation,
            Icon::Globe,
            t.extra.settings_nav_dict_translate,
        ),
        (
            SettingsTab::System,
            Icon::Settings,
            t.extra.settings_nav_system,
        ),
        (
            SettingsTab::About,
            Icon::Pro,
            t.extra.settings_nav_pro_about,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_modal_header(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    modal_w: f32,
    search_query: &mut String,
    focus_search: bool,
    reset_confirm: &mut bool,
    close: &mut bool,
) {
    let stacked_header = modal_w < 740.0;
    egui::Frame::new()
        .fill(p.header_bg)
        .corner_radius(egui::CornerRadius {
            nw: p.corner_lg as u8,
            ne: p.corner_lg as u8,
            sw: 0,
            se: 0,
        })
        .inner_margin(Margin::symmetric(18, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                ui.painter()
                    .rect_filled(badge_rect, p.corner_sm + 2.0, p.accent_soft);
                paint_icon(
                    ui.painter(),
                    Icon::Settings,
                    badge_rect.shrink(8.0),
                    p.accent,
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(
                        RichText::new(t.settings_title)
                            .size(16.0)
                            .strong()
                            .color(p.text),
                    );
                    if modal_w >= 460.0 {
                        ui.label(
                            RichText::new(t.extra.settings_subtitle)
                                .size(11.0)
                                .color(p.text_variant),
                        );
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (close_rect, close_resp) =
                        ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
                    let close_bg = if close_resp.hovered() {
                        p.surface_highest
                    } else {
                        p.surface_container
                    };
                    ui.painter()
                        .circle_filled(close_rect.center(), 15.0, close_bg);
                    paint_icon(
                        ui.painter(),
                        Icon::Cross,
                        Rect::from_center_size(close_rect.center(), Vec2::splat(13.0)),
                        p.text,
                    );
                    if close_resp.on_hover_text(t.close_btn).clicked() {
                        *close = true;
                    }

                    if !stacked_header {
                        if themed_button(ui, t.extra.settings_reset_btn, false, p)
                            .on_hover_text(t.extra.settings_reset_tip)
                            .clicked()
                        {
                            *reset_confirm = !*reset_confirm;
                        }
                        render_search_box(
                            ui,
                            p,
                            t,
                            search_query,
                            focus_search,
                            (modal_w * 0.25).clamp(210.0, 290.0),
                        );
                    }
                });
            });

            if stacked_header {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let box_w = (ui.available_width() - 120.0).max(140.0);
                    render_search_box(ui, p, t, search_query, focus_search, box_w);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if themed_button(ui, t.extra.settings_reset_btn, false, p)
                            .on_hover_text(t.extra.settings_reset_tip)
                            .clicked()
                        {
                            *reset_confirm = !*reset_confirm;
                        }
                    });
                });
            }
        });

    let hr = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
    ui.painter().hline(
        hr.min.x..=hr.max.x,
        hr.center().y,
        Stroke::new(1.0, p.border_subtle),
    );
}

fn render_search_box(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    search_query: &mut String,
    focus_search: bool,
    width: f32,
) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        egui::Frame::new()
            .fill(p.card_bg)
            .stroke(Stroke::new(1.0, p.border_subtle))
            .corner_radius(p.corner_sm + 2.0)
            .inner_margin(Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.set_width(width);
                ui.set_max_width(width);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                    paint_icon(ui.painter(), Icon::Search, icon_rect, p.text_variant);

                    let right_badge_reserve = if !search_query.is_empty() {
                        20.0
                    } else if width >= 220.0 {
                        56.0
                    } else {
                        0.0
                    };
                    let edit = ui.add(
                        egui::TextEdit::singleline(search_query)
                            .hint_text(t.extra.settings_search_hint)
                            .frame(egui::Frame::NONE)
                            .desired_width((width - 36.0 - right_badge_reserve).max(60.0)),
                    );
                    if focus_search {
                        edit.request_focus();
                    }

                    if !search_query.is_empty() {
                        let (clr_rect, clr_resp) =
                            ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                        paint_icon(
                            ui.painter(),
                            Icon::Cross,
                            clr_rect.shrink(2.0),
                            if clr_resp.hovered() {
                                p.text
                            } else {
                                p.text_variant
                            },
                        );
                        if clr_resp.clicked() {
                            search_query.clear();
                        }
                    } else if width >= 220.0 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            kbd_badge(ui, p, "Ctrl+F");
                        });
                    }
                });
            });
    });
}

pub(super) fn render_reset_confirm_banner(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    reset_confirm: &mut bool,
    reset_done: &mut bool,
    on_confirm: impl FnOnce(),
) {
    egui::Frame::new()
        .fill(p.secondary_bg)
        .inner_margin(Margin::symmetric(18, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.label(
                    RichText::new(t.extra.settings_reset_confirm)
                        .size(12.0)
                        .strong()
                        .color(p.secondary_fg),
                );
                if themed_button(ui, t.extra.settings_reset_confirm_btn, true, p).clicked() {
                    on_confirm();
                    *reset_confirm = false;
                    *reset_done = true;
                }
                if themed_button(ui, t.extra.settings_cancel_btn, false, p).clicked() {
                    *reset_confirm = false;
                }
            });
        });
    let hr = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
    ui.painter().hline(
        hr.min.x..=hr.max.x,
        hr.center().y,
        Stroke::new(1.0, p.border_subtle),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_left_rail(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings_tab: &mut SettingsTab,
    search_query: &mut String,
    reset_done: &mut bool,
    settings: &AppSettings,
    rail_h: f32,
) {
    egui::Frame::new()
        .fill(p.rail_bg)
        .inner_margin(Margin::symmetric(12, 14))
        .show(ui, |ui| {
            ui.set_min_height((rail_h - 28.0).max(120.0));
            ui.set_width(ui.available_width());

            let search_active = !search_query.trim().is_empty();
            for (tab, icon, label) in nav_items(t) {
                let is_active = !search_active && *settings_tab == tab;
                let row_w = ui.available_width();
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(row_w, 36.0), Sense::click());
                let fill = if is_active {
                    p.accent
                } else if resp.hovered() {
                    mix_color(p.rail_bg, p.surface_container, 0.7)
                } else {
                    Color32::TRANSPARENT
                };
                if fill != Color32::TRANSPARENT {
                    ui.painter().rect_filled(rect, p.corner_sm + 1.0, fill);
                }

                let fg = if is_active {
                    p.accent_on
                } else if resp.hovered() {
                    p.text
                } else {
                    p.text_variant
                };
                let icon_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + 12.0, rect.center().y - 7.5),
                    Vec2::splat(15.0),
                );
                paint_icon(ui.painter(), icon, icon_rect, fg);

                let font = egui::FontId::proportional(12.5);
                let galley = ui.fonts_mut(|f| {
                    f.layout(label.to_string(), font, fg, (rect.width() - 40.0).max(60.0))
                });
                ui.painter().galley(
                    Pos2::new(rect.min.x + 34.0, rect.center().y - galley.size().y * 0.5),
                    galley,
                    fg,
                );

                if resp.clicked() {
                    *settings_tab = tab;
                    search_query.clear();
                    *reset_done = false;
                }
                ui.add_space(4.0);
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                egui::Frame::new()
                    .fill(p.card_bg)
                    .stroke(Stroke::new(1.0, p.border_subtle))
                    .corner_radius(p.corner_sm)
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let profile = if AppSettings::is_portable() {
                            t.extra.settings_profile_portable
                        } else {
                            t.extra.settings_profile_local
                        };
                        ui.label(RichText::new(profile).size(10.5).color(p.text_variant));
                        ui.add_space(3.0);
                        let ver = format!(
                            "{} v{}",
                            t.extra.settings_version_prefix,
                            crate::updater::CURRENT_VERSION
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let (dot_r, _) =
                                ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                            ui.painter().circle_filled(dot_r.center(), 3.2, p.tertiary);
                            ui.label(RichText::new(ver).size(11.0).strong().color(p.text));
                            if settings.license_status().is_pro_active() {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        pill_badge(
                                            ui,
                                            p.secondary_bg,
                                            p.secondary_fg,
                                            t.license_pro_badge,
                                            9.0,
                                        );
                                    },
                                );
                            }
                        });
                    });
            });
        });
}

pub(super) fn render_compact_nav_bar(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings_tab: &mut SettingsTab,
    search_query: &mut String,
    reset_done: &mut bool,
) {
    let outer_w = ui.available_width().max(200.0);
    let inner_w = (outer_w - 28.0).max(160.0);
    egui::Frame::new()
        .fill(p.rail_bg)
        .inner_margin(Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.set_width(inner_w);
            ui.set_max_width(inner_w);
            let search_active = !search_query.trim().is_empty();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                let font = egui::FontId::proportional(11.5);
                for (tab, icon, label) in nav_items(t) {
                    let is_active = !search_active && *settings_tab == tab;
                    let (fill, fg, stroke) = if is_active {
                        (p.accent, p.accent_on, Stroke::new(1.0, p.accent))
                    } else {
                        (p.card_bg, p.text_variant, Stroke::new(1.0, p.border_subtle))
                    };
                    let galley =
                        ui.fonts_mut(|f| f.layout_no_wrap(label.to_string(), font.clone(), fg));
                    let pill_w = (10.0 + 13.0 + 5.0 + galley.size().x + 10.0).min(inner_w);
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(pill_w, 28.0), Sense::click());
                    let bg = if !is_active && resp.hovered() {
                        mix_color(p.card_bg, p.surface_container, 0.7)
                    } else {
                        fill
                    };
                    ui.painter()
                        .rect(rect, p.corner_sm, bg, stroke, egui::StrokeKind::Inside);
                    let icon_rect = Rect::from_min_size(
                        Pos2::new(rect.min.x + 9.0, rect.center().y - 6.5),
                        Vec2::splat(13.0),
                    );
                    paint_icon(ui.painter(), icon, icon_rect, fg);
                    ui.painter().galley(
                        Pos2::new(rect.min.x + 27.0, rect.center().y - galley.size().y * 0.5),
                        galley,
                        fg,
                    );
                    if resp.clicked() {
                        *settings_tab = tab;
                        search_query.clear();
                        *reset_done = false;
                    }
                }
            });
        });
    let hr = ui.allocate_space(Vec2::new(outer_w, 1.0)).1;
    ui.painter().hline(
        hr.min.x..=hr.max.x,
        hr.center().y,
        Stroke::new(1.0, p.border_subtle),
    );
}

pub(super) fn render_modal_footer(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    modal_w: f32,
    footer_h: f32,
    close: &mut bool,
) {
    let hr = ui.allocate_space(Vec2::new(modal_w, 1.0)).1;
    ui.painter().hline(
        hr.min.x..=hr.max.x,
        hr.center().y,
        Stroke::new(1.0, p.border_subtle),
    );

    let inner_w = (modal_w - 36.0).max(160.0);
    let inner_h = (footer_h - 17.0).max(28.0);
    egui::Frame::new()
        .fill(p.header_bg)
        .corner_radius(egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: p.corner_lg as u8,
            se: p.corner_lg as u8,
        })
        .inner_margin(Margin::symmetric(18, 8))
        .show(ui, |ui| {
            ui.set_width(inner_w);
            ui.set_max_width(inner_w);
            ui.set_min_height(inner_h);
            ui.horizontal(|ui| {
                ui.set_width(inner_w);
                if modal_w >= 860.0 {
                    let (dot_r, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                    ui.painter().circle_filled(dot_r.center(), 3.2, p.tertiary);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(t.extra.settings_auto_saved_note)
                            .size(11.5)
                            .color(p.text_variant.gamma_multiply(0.85)),
                    );
                    ui.add_space(10.0);
                    kbd_badge(ui, p, "Esc");
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(t.extra.settings_esc_hint)
                            .size(11.5)
                            .color(p.text_variant),
                    );
                } else if modal_w >= 460.0 {
                    kbd_badge(ui, p, "Esc");
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(t.extra.settings_esc_hint)
                            .size(11.5)
                            .color(p.text_variant),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = ui.add(
                        egui::Button::new(
                            RichText::new(t.extra.settings_done_btn)
                                .size(12.5)
                                .strong()
                                .color(p.accent_on),
                        )
                        .fill(p.accent)
                        .stroke(Stroke::NONE)
                        .corner_radius(p.corner_sm)
                        .min_size(Vec2::new(104.0, 30.0)),
                    );
                    if btn.clicked() {
                        *close = true;
                    }
                });
            });
        });
}
