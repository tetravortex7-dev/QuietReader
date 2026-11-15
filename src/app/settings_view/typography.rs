use super::widgets::{
    SettingsPalette, metric_slider, mix_color, row_card, section_card, segmented_control,
    stepper_capsule, toggle_switch,
};
use crate::app::fonts::{AVAILABLE_FONTS, FontOptionInfo, apply_font_family};
use crate::app::license_view::ProUpgradeState;
use crate::app::text_view::create_sample_preview_job;
use crate::app::theme::ReaderTheme;
use crate::core::license::ProFeature;
use crate::i18n::{Translations, UiLanguage};
use crate::settings::{AppSettings, OPEN_DYSLEXIC_FONT};
use eframe::egui::{self, Color32, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub fn compute_font_grid_cols(avail_w: f32) -> usize {
    if avail_w >= 620.0 {
        3
    } else if avail_w >= 380.0 {
        2
    } else {
        1
    }
}

pub(super) fn render_typography_tab(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: &SettingsPalette,
    t: &Translations,
    theme: &ReaderTheme,
    settings: &mut AppSettings,
    font_size: &mut f32,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    render_typography_section(
        ui,
        ctx,
        p,
        t,
        theme,
        settings,
        font_size,
        on_save_settings,
        pro,
    );
}

pub(super) fn render_typography_section(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: &SettingsPalette,
    t: &Translations,
    theme: &ReaderTheme,
    settings: &mut AppSettings,
    font_size: &mut f32,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let lang = settings.ui_language;
    section_card(
        ui,
        p,
        t.extra.settings_typography_title,
        None,
        Some(t.extra.settings_typography_subtitle),
        |ui| {
            let avail_w = ui.available_width();
            let cols = compute_font_grid_cols(avail_w);
            let gap = 10.0;
            let card_w = ((avail_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32)
                .clamp(160.0, 380.0);
            let pad_x = 12.0;
            let desc_max_w = (card_w - pad_x * 2.0).max(40.0);

            for chunk in AVAILABLE_FONTS.chunks(cols) {
                let row_desc_h = chunk
                    .iter()
                    .map(|font_info| {
                        ui.fonts_mut(|f| {
                            f.layout(
                                font_info.desc_for_lang(lang).to_string(),
                                egui::FontId::proportional(10.8),
                                p.text_variant,
                                desc_max_w,
                            )
                            .size()
                            .y
                        })
                    })
                    .fold(32.0_f32, f32::max);
                let row_card_h = (34.0 + row_desc_h + 36.0).max(118.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for font_info in chunk {
                        let is_selected = settings.font_family == font_info.id;
                        render_font_card(
                            ui,
                            p,
                            t,
                            lang,
                            font_info,
                            card_w,
                            row_card_h,
                            is_selected,
                            || {
                                select_font(settings, ctx, font_info.id, on_save_settings, pro);
                            },
                        );
                    }
                });
                ui.add_space(8.0);
            }

            ui.add_space(6.0);
            let divider_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
            ui.painter().hline(
                divider_rect.min.x..=divider_rect.max.x,
                divider_rect.center().y,
                Stroke::new(1.0, p.border_subtle),
            );
            ui.add_space(12.0);

            let metrics_w = ui.available_width();
            if metrics_w >= 620.0 {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.columns(3, |cols_ui| {
                        render_font_size_control(
                            &mut cols_ui[0],
                            p,
                            t,
                            font_size,
                            settings,
                            on_save_settings,
                        );
                        render_line_height_control(
                            &mut cols_ui[1],
                            p,
                            t,
                            settings,
                            on_save_settings,
                        );
                        render_column_width_control(
                            &mut cols_ui[2],
                            p,
                            t,
                            settings,
                            on_save_settings,
                        );
                    });
                });
            } else {
                render_font_size_control(ui, p, t, font_size, settings, on_save_settings);
                ui.add_space(10.0);
                render_line_height_control(ui, p, t, settings, on_save_settings);
                ui.add_space(10.0);
                render_column_width_control(ui, p, t, settings, on_save_settings);
            }

            ui.add_space(12.0);
            render_reading_layout_selector(
                ui,
                p,
                t,
                &mut settings.text_two_columns,
                on_save_settings,
            );
            ui.add_space(8.0);
            let toggle_cols = if ui.available_width() >= 580.0 { 2 } else { 1 };
            if toggle_cols == 2 {
                ui.columns(2, |cols_ui| {
                    render_reading_switch_row(
                        &mut cols_ui[0],
                        p,
                        t.paged_scroll_title,
                        t.paged_scroll_desc,
                        &mut settings.text_paged_scroll,
                        on_save_settings,
                    );
                    render_reading_switch_row(
                        &mut cols_ui[1],
                        p,
                        t.paragraph_mode,
                        t.paragraph_mode_desc,
                        &mut settings.text_paragraph_mode,
                        on_save_settings,
                    );
                });
            } else {
                render_reading_switch_row(
                    ui,
                    p,
                    t.paged_scroll_title,
                    t.paged_scroll_desc,
                    &mut settings.text_paged_scroll,
                    on_save_settings,
                );
                ui.add_space(8.0);
                render_reading_switch_row(
                    ui,
                    p,
                    t.paragraph_mode,
                    t.paragraph_mode_desc,
                    &mut settings.text_paragraph_mode,
                    on_save_settings,
                );
            }

            ui.add_space(12.0);
            row_card(ui, p, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.live_preview)
                            .size(11.0)
                            .strong()
                            .color(p.accent),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} · {:.0} px · {:.2}×",
                                settings.effective_font_family(),
                                *font_size,
                                settings.effective_line_height_scale()
                            ))
                            .size(11.0)
                            .color(p.text_variant),
                        );
                    });
                });
                ui.add_space(8.0);
                egui::Frame::new()
                    .fill(theme.bg_color())
                    .stroke(Stroke::new(1.0, p.border_subtle))
                    .corner_radius(p.corner_sm)
                    .inner_margin(Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        let preview_job = create_sample_preview_job(
                            t.sample_text,
                            *font_size,
                            theme.text_color(),
                            settings.effective_line_height_scale(),
                            settings.effective_bionic_reading(),
                            settings.ui_language.is_rtl(),
                        );
                        ui.vertical_centered(|ui| {
                            ui.set_max_width(settings.effective_max_column_width());
                            ui.label(preview_job);
                        });
                    });
            });
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn render_font_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    lang: UiLanguage,
    font_info: &FontOptionInfo,
    width: f32,
    card_h: f32,
    selected: bool,
    on_click: impl FnOnce(),
) {
    let is_pro_font = font_info.id == OPEN_DYSLEXIC_FONT;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, card_h), Sense::click());
    let hovered = resp.hovered();

    let fill = if selected {
        mix_color(p.card_bg, p.accent_soft, 0.28)
    } else if hovered {
        mix_color(p.card_bg, p.surface_container, 0.45)
    } else {
        p.card_bg
    };
    let stroke = if selected {
        Stroke::new(1.5, p.accent)
    } else if hovered {
        Stroke::new(1.0, p.accent.gamma_multiply(0.55))
    } else {
        Stroke::new(1.0, p.border_subtle)
    };
    ui.painter()
        .rect(rect, p.corner_md, fill, stroke, egui::StrokeKind::Inside);

    let pad_x = 12.0;
    let pad_y = 10.0;
    let font_family = egui::FontFamily::Name(font_info.id.into());
    let title_galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            font_info.name.to_string(),
            egui::FontId::new(14.5, font_family),
            p.text,
        )
    });
    let title_pos = Pos2::new(rect.min.x + pad_x, rect.min.y + pad_y);
    ui.painter().galley(title_pos, title_galley.clone(), p.text);

    if is_pro_font {
        let pro_galley = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                t.license_pro_badge.to_string(),
                egui::FontId::proportional(9.5),
                p.secondary_fg,
            )
        });
        let pill_rect = Rect::from_min_size(
            Pos2::new(title_pos.x + title_galley.size().x + 6.0, title_pos.y + 1.0),
            Vec2::new(pro_galley.size().x + 10.0, 16.0),
        );
        if pill_rect.max.x < rect.max.x - 24.0 {
            ui.painter()
                .rect_filled(pill_rect, pill_rect.height() * 0.5, p.secondary_bg);
            ui.painter().galley(
                Pos2::new(
                    pill_rect.center().x - pro_galley.size().x * 0.5,
                    pill_rect.center().y - pro_galley.size().y * 0.5,
                ),
                pro_galley,
                p.secondary_fg,
            );
        }
    }

    let radio_center = Pos2::new(rect.max.x - pad_x - 7.0, rect.min.y + pad_y + 8.0);
    if selected {
        ui.painter().circle_filled(radio_center, 7.0, p.accent);
        ui.painter()
            .circle_filled(radio_center, 2.5, Color32::WHITE);
    } else {
        ui.painter()
            .circle_stroke(radio_center, 6.5, Stroke::new(1.4, p.border));
    }

    let desc_max_w = (rect.width() - pad_x * 2.0).max(40.0);
    let desc_galley = ui.fonts_mut(|f| {
        f.layout(
            font_info.desc_for_lang(lang).to_string(),
            egui::FontId::proportional(10.8),
            p.text_variant,
            desc_max_w,
        )
    });
    ui.painter().galley(
        Pos2::new(rect.min.x + pad_x, rect.min.y + pad_y + 22.0),
        desc_galley,
        p.text_variant,
    );

    let (badge_tag, _) = font_info.badge_parts_for_lang(lang);
    let badge_galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            badge_tag.to_string(),
            egui::FontId::proportional(9.5),
            p.text_variant,
        )
    });
    let tag_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + pad_x, rect.max.y - 25.0),
        Vec2::new(badge_galley.size().x + 10.0, 16.0),
    );
    ui.painter().rect_filled(tag_rect, 4.0, p.surface_container);
    ui.painter().galley(
        Pos2::new(
            tag_rect.center().x - badge_galley.size().x * 0.5,
            tag_rect.center().y - badge_galley.size().y * 0.5,
        ),
        badge_galley,
        p.text_variant,
    );

    if resp.clicked() {
        on_click();
    }
}

fn render_font_size_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    font_size: &mut f32,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    ui.label(RichText::new(t.font_size).size(12.0).strong().color(p.text));
    ui.add_space(4.0);
    let display = format!("{:.0} px", *font_size);
    if stepper_capsule(
        ui,
        font_size,
        12.0,
        36.0,
        1.0,
        &display,
        t.font_decrease_tip,
        t.font_increase_tip,
        p,
    ) {
        settings.font_size = *font_size;
        *on_save_settings = true;
    }
}

fn render_line_height_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let readout = format!("{:.2}×", settings.line_height_scale.clamp(1.1, 2.0));
    let (changed, commit) = metric_slider(
        ui,
        &mut settings.line_height_scale,
        1.1..=2.0,
        Some(0.05),
        t.line_height_label,
        &readout,
        Some(t.line_height_tooltip),
        p,
    );
    if changed || commit {
        *on_save_settings = true;
    }
}

fn render_column_width_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let readout = format!("{:.0} px", settings.max_column_width.clamp(520.0, 1200.0));
    let (changed, commit) = metric_slider(
        ui,
        &mut settings.max_column_width,
        520.0..=1200.0,
        Some(20.0),
        t.column_width_label,
        &readout,
        Some(t.column_width_tooltip),
        p,
    );
    if changed || commit {
        *on_save_settings = true;
    }
}

fn render_reading_layout_selector(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    two_columns: &mut bool,
    on_save_settings: &mut bool,
) {
    let options = [
        (false, t.extra.text_mode_scroll),
        (true, t.extra.text_mode_spread),
    ];
    row_card(ui, p, |ui| {
        if ui.available_width() >= 520.0 {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 230.0).max(120.0));
                    ui.label(
                        RichText::new(t.extra.text_layout_title)
                            .size(13.0)
                            .strong()
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(t.extra.text_layout_desc)
                            .size(11.0)
                            .color(p.text_variant),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(210.0, 30.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            if segmented_control(ui, two_columns, &options, p, true) {
                                *on_save_settings = true;
                            }
                        },
                    );
                });
            });
        } else {
            ui.label(
                RichText::new(t.extra.text_layout_title)
                    .size(13.0)
                    .strong()
                    .color(p.text),
            );
            ui.label(
                RichText::new(t.extra.text_layout_desc)
                    .size(11.0)
                    .color(p.text_variant),
            );
            ui.add_space(6.0);
            if segmented_control(ui, two_columns, &options, p, true) {
                *on_save_settings = true;
            }
        }
    });
}

fn render_reading_switch_row(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    title: &str,
    desc: &str,
    value: &mut bool,
    on_save_settings: &mut bool,
) {
    row_card(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 48.0).max(80.0));
                ui.label(RichText::new(title).size(13.0).strong().color(p.text));
                ui.label(RichText::new(desc).size(11.0).color(p.text_variant));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, value, p).changed() {
                    *on_save_settings = true;
                }
            });
        });
    });
}

fn select_font(
    settings: &mut AppSettings,
    ctx: &egui::Context,
    font_id: &str,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    if font_id == OPEN_DYSLEXIC_FONT && !settings.can_use(ProFeature::FocusTools) {
        pro.request(ProFeature::FocusTools);
        return;
    }
    settings.font_family = font_id.to_string();
    let effective_font_family = settings.effective_font_family();
    apply_font_family(ctx, &effective_font_family);
    *on_save_settings = true;
}
