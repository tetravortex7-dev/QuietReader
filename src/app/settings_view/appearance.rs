use super::widgets::{
    SectionBadge, SettingsPalette, metric_slider, mix_color, section_card, segmented_control,
};
use crate::app::icons::{Icon, paint_icon};
use crate::app::theme::ReaderTheme;
use crate::core::library::{LibraryMode, LibraryStore, UiDesignMode};
use crate::i18n::{Translations, UiLanguage, fill};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub fn compute_theme_swatch_cols(avail_w: f32) -> usize {
    if avail_w >= 680.0 {
        7
    } else if avail_w >= 400.0 {
        4
    } else {
        2
    }
}

pub(super) fn theme_short_name<'a>(theme: &ReaderTheme, t: &'a Translations) -> &'a str {
    match theme {
        ReaderTheme::Sepia => t.extra.theme_sepia_short,
        ReaderTheme::Twilight => t.extra.theme_twilight_short,
        ReaderTheme::Warm => t.extra.theme_warm_short,
        ReaderTheme::Mint => t.extra.theme_mint_short,
        ReaderTheme::Light => t.extra.theme_light_short,
        ReaderTheme::Dark => t.extra.theme_dark_short,
        ReaderTheme::Oled => t.extra.theme_oled_short,
    }
}

fn theme_swatch_tag<'a>(theme: &ReaderTheme, t: &'a Translations) -> &'a str {
    match theme {
        ReaderTheme::Sepia => t.extra.settings_swatch_sepia_tag,
        ReaderTheme::Twilight => t.extra.settings_swatch_twilight_tag,
        ReaderTheme::Warm => t.extra.settings_swatch_warm_tag,
        ReaderTheme::Mint => t.extra.settings_swatch_mint_tag,
        ReaderTheme::Light => t.extra.settings_swatch_light_tag,
        ReaderTheme::Dark => t.extra.settings_swatch_dark_tag,
        ReaderTheme::Oled => t.extra.settings_swatch_oled_tag,
    }
}

fn theme_storage_name(theme: &ReaderTheme) -> &'static str {
    match theme {
        ReaderTheme::Dark => "Dark",
        ReaderTheme::Light => "Light",
        ReaderTheme::Sepia => "Sepia",
        ReaderTheme::Oled => "Oled",
        ReaderTheme::Warm => "Warm",
        ReaderTheme::Mint => "Mint",
        ReaderTheme::Twilight => "Twilight",
    }
}

pub(super) fn render_appearance_tab(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    theme: &mut ReaderTheme,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    render_theme_and_display_section(
        ui,
        p,
        t,
        theme,
        library,
        library_dirty,
        settings,
        on_save_settings,
    );
    ui.add_space(14.0);
    render_language_section(ui, p, t, settings, on_save_settings);
}

pub(super) fn render_theme_and_display_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    theme: &mut ReaderTheme,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    section_card(
        ui,
        p,
        t.extra.settings_themes_title,
        Some(SectionBadge::Adaptive(t.extra.settings_themes_adaptive)),
        Some(t.extra.settings_themes_subtitle),
        |ui| {
            let avail_w = ui.available_width();
            let cols = compute_theme_swatch_cols(avail_w);
            let gap = 10.0;
            let card_w = ((avail_w - gap * (cols.saturating_sub(1) as f32)) / cols as f32)
                .clamp(78.0, 220.0);

            let themes = [
                ReaderTheme::Sepia,
                ReaderTheme::Twilight,
                ReaderTheme::Warm,
                ReaderTheme::Mint,
                ReaderTheme::Light,
                ReaderTheme::Dark,
                ReaderTheme::Oled,
            ];

            for chunk in themes.chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    for &variant in chunk {
                        let is_selected = *theme == variant;
                        render_theme_swatch_card(ui, p, t, &variant, card_w, is_selected, || {
                            *theme = variant;
                            settings.theme = theme_storage_name(&variant).to_string();
                            *on_save_settings = true;
                        });
                    }
                });
                ui.add_space(10.0);
            }

            ui.add_space(4.0);
            let divider_rect = ui.allocate_space(Vec2::new(ui.available_width(), 1.0)).1;
            ui.painter().hline(
                divider_rect.min.x..=divider_rect.max.x,
                divider_rect.center().y,
                Stroke::new(1.0, p.border_subtle),
            );
            ui.add_space(12.0);

            let controls_avail = ui.available_width();
            if controls_avail >= 680.0 {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.columns(3, |cols_ui| {
                        cols_ui[0].spacing_mut().item_spacing.y = 4.0;
                        render_brightness_control(
                            &mut cols_ui[0],
                            p,
                            t,
                            settings,
                            on_save_settings,
                        );

                        cols_ui[1].spacing_mut().item_spacing.y = 4.0;
                        render_rounding_control(
                            &mut cols_ui[1],
                            p,
                            t,
                            library,
                            library_dirty,
                            on_save_settings,
                        );

                        cols_ui[2].spacing_mut().item_spacing.y = 4.0;
                        render_startup_mode_control(
                            &mut cols_ui[2],
                            p,
                            t,
                            library,
                            library_dirty,
                            on_save_settings,
                        );
                    });
                });
            } else {
                render_brightness_control(ui, p, t, settings, on_save_settings);
                ui.add_space(10.0);
                render_rounding_control(ui, p, t, library, library_dirty, on_save_settings);
                ui.add_space(10.0);
                render_startup_mode_control(ui, p, t, library, library_dirty, on_save_settings);
            }
        },
    );
}

fn render_theme_swatch_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    variant: &ReaderTheme,
    width: f32,
    selected: bool,
    on_click: impl FnOnce(),
) {
    let swatch_bg = variant.bg_color();
    let swatch_fg = variant.text_color();
    let swatch_accent = variant.accent_color();
    let hex = format!(
        "#{:02X}{:02X}{:02X}",
        swatch_bg.r(),
        swatch_bg.g(),
        swatch_bg.b()
    );
    let title = theme_short_name(variant, t);
    let tag = if selected {
        t.extra.settings_theme_active
    } else {
        theme_swatch_tag(variant, t)
    };

    let card_h = 114.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, card_h), Sense::click());
    let hovered = resp.hovered();

    if selected {
        let outer = rect.expand(2.0);
        ui.painter().rect_stroke(
            outer,
            p.corner_md + 2.0,
            Stroke::new(2.0, p.accent),
            egui::StrokeKind::Outside,
        );
    }

    let card_fill = if selected {
        mix_color(p.card_bg, p.accent_soft, 0.25)
    } else if hovered {
        mix_color(p.card_bg, p.surface_container, 0.45)
    } else {
        p.card_bg
    };
    let card_stroke = if selected {
        Stroke::new(1.5, p.accent)
    } else if hovered {
        Stroke::new(1.0, p.accent.gamma_multiply(0.5))
    } else {
        Stroke::new(1.0, p.border_subtle)
    };
    ui.painter().rect(
        rect,
        p.corner_md,
        card_fill,
        card_stroke,
        egui::StrokeKind::Inside,
    );

    let pad = 6.0;
    let swatch_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + pad, rect.min.y + pad),
        Vec2::new(rect.width() - pad * 2.0, 56.0),
    );
    let swatch_stroke = Stroke::new(1.0, p.border);
    ui.painter().rect(
        swatch_rect,
        p.corner_sm,
        swatch_bg,
        swatch_stroke,
        egui::StrokeKind::Inside,
    );

    let aa_galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            t.extra.settings_theme_sample.to_string(),
            egui::FontId::proportional(17.0),
            swatch_fg,
        )
    });
    ui.painter().galley(
        Pos2::new(
            swatch_rect.center().x - aa_galley.size().x * 0.5,
            swatch_rect.min.y + (swatch_rect.height() - 16.0 - aa_galley.size().y) * 0.5 + 1.0,
        ),
        aa_galley,
        swatch_fg,
    );

    let hex_galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            hex,
            egui::FontId::monospace(8.5),
            swatch_fg.gamma_multiply(0.68),
        )
    });
    ui.painter().galley(
        Pos2::new(
            swatch_rect.min.x + 6.0,
            swatch_rect.max.y - hex_galley.size().y - 4.0,
        ),
        hex_galley,
        swatch_fg,
    );

    let accent_dot_center = Pos2::new(swatch_rect.max.x - 10.0, swatch_rect.max.y - 9.0);
    ui.painter()
        .circle_filled(accent_dot_center, 3.5, swatch_accent);

    if selected {
        let badge_center = Pos2::new(swatch_rect.max.x - 10.0, swatch_rect.min.y + 10.0);
        ui.painter().circle_filled(badge_center, 8.0, p.accent);
        paint_icon(
            ui.painter(),
            Icon::Check,
            Rect::from_center_size(badge_center, Vec2::splat(9.5)),
            Color32::WHITE,
        );
    }

    let title_galley = ui.fonts_mut(|f| {
        f.layout(
            title.to_string(),
            egui::FontId::proportional(11.5),
            p.text,
            rect.width() - 8.0,
        )
    });
    ui.painter().galley(
        Pos2::new(
            rect.center().x - title_galley.size().x * 0.5,
            swatch_rect.max.y + 7.0,
        ),
        title_galley,
        p.text,
    );

    let tag_color = if selected { p.accent } else { p.text_variant };
    let tag_galley = ui.fonts_mut(|f| {
        f.layout(
            tag.to_string(),
            egui::FontId::proportional(9.8),
            tag_color,
            rect.width() - 8.0,
        )
    });
    ui.painter().galley(
        Pos2::new(
            rect.center().x - tag_galley.size().x * 0.5,
            swatch_rect.max.y + 24.0,
        ),
        tag_galley,
        tag_color,
    );

    if resp.clicked() {
        on_click();
    }
}

fn render_brightness_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let mut percent = (settings.reading_brightness.clamp(0.35, 1.0) * 100.0).round();
    let readout = format!("{:.0}%", percent);
    let (changed, commit) = metric_slider(
        ui,
        &mut percent,
        35.0..=100.0,
        Some(1.0),
        t.brightness_title,
        &readout,
        Some(t.brightness_desc),
        p,
    );
    if changed {
        settings.reading_brightness = (percent / 100.0).clamp(0.35, 1.0);
    }
    if commit {
        *on_save_settings = true;
    }
}

fn render_rounding_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    on_save_settings: &mut bool,
) {
    ui.label(
        RichText::new(t.extra.settings_rounding_label)
            .size(12.0)
            .strong()
            .color(p.text),
    );
    ui.add_space(4.0);
    let cozy_label = fill(
        t.extra.settings_rounding_cozy,
        &[("{px}", &UiDesignMode::Cozy.corner_radius().to_string())],
    );
    let studio_label = fill(
        t.extra.settings_rounding_studio,
        &[("{px}", &UiDesignMode::Studio.corner_radius().to_string())],
    );
    let items = [
        (UiDesignMode::Cozy, cozy_label.as_str()),
        (UiDesignMode::Studio, studio_label.as_str()),
    ];
    if segmented_control(ui, &mut library.design, &items, p, false) {
        *library_dirty = true;
        *on_save_settings = true;
    }
}

fn render_startup_mode_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    library: &mut LibraryStore,
    library_dirty: &mut bool,
    on_save_settings: &mut bool,
) {
    ui.label(
        RichText::new(t.extra.settings_startup_mode_label)
            .size(12.0)
            .strong()
            .color(p.text),
    );
    ui.add_space(4.0);
    let items = [
        (LibraryMode::Shelf, t.library_mode_shelf),
        (LibraryMode::Folder, t.library_mode_folder),
        (LibraryMode::ReaderOnly, t.library_mode_reader),
    ];
    if segmented_control(ui, &mut library.mode, &items, p, false) {
        *library_dirty = true;
        *on_save_settings = true;
    }
}

pub(super) fn render_language_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    section_card(
        ui,
        p,
        t.interface_language,
        None,
        Some(t.interface_language_desc),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                for &lang in UiLanguage::ALL.iter() {
                    let is_sel = settings.ui_language == lang;
                    let text = lang.label(t);
                    let (fill, fg, stroke) = if is_sel {
                        (
                            p.accent_soft,
                            if p.is_dark { Color32::WHITE } else { p.accent },
                            Stroke::new(1.5, p.accent),
                        )
                    } else {
                        (p.card_bg, p.text, Stroke::new(1.0, p.border_subtle))
                    };
                    let btn = ui.add(
                        egui::Button::new(RichText::new(text).size(12.5).strong().color(fg))
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(p.corner_sm)
                            .min_size(Vec2::new(0.0, 30.0)),
                    );
                    if btn.clicked() && !is_sel {
                        settings.ui_language = lang;
                        *on_save_settings = true;
                    }
                }
            });
        },
    );
}
