use super::widgets::{
    SectionBadge, SettingsPalette, kbd_badge, metric_slider, mix_color, pill_badge, row_card,
    section_card, segmented_control, themed_button, toggle_switch,
};
use crate::app::icons::{Icon, IconButton, paint_icon};
use crate::app::license_view::ProUpgradeState;
use crate::core::license::ProFeature;
use crate::i18n::{Translations, fill};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};

pub(super) fn render_reading_tab(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    keep_screen_on: &mut bool,
    is_autoscrolling: &mut bool,
    on_toggle_autoscroll: &mut bool,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    render_focus_tools_section(ui, p, t, settings, on_save_settings, pro);
    ui.add_space(14.0);
    render_navigation_display_section(ui, p, t, settings, keep_screen_on, on_save_settings);
    ui.add_space(14.0);
    render_autoscroll_section(
        ui,
        p,
        t,
        settings,
        is_autoscrolling,
        on_toggle_autoscroll,
        on_save_settings,
    );
    ui.add_space(14.0);
    render_comics_and_pdf_section(ui, p, t, settings, on_save_settings, pro);
    ui.add_space(14.0);
    render_hotkeys_section(ui, p, t);
}

pub(super) fn render_focus_tools_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let focus_allowed = settings.can_use(ProFeature::FocusTools);
    section_card(
        ui,
        p,
        t.extra.settings_focus_title,
        Some(SectionBadge::Pro(t.license_pro_badge)),
        Some(t.extra.settings_focus_subtitle),
        |ui| {
            row_card(ui, p, |ui| {
                let mut ruler = settings.effective_reading_ruler();
                ui.horizontal(|ui| {
                    paint_small_row_icon(ui, Icon::Ruler, p);
                    ui.vertical(|ui| {
                        ui.set_max_width((ui.available_width() - 56.0).max(80.0));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(t.reading_ruler_title)
                                    .size(13.0)
                                    .strong()
                                    .color(p.text),
                            );
                            kbd_badge(ui, p, "L");
                            if !focus_allowed {
                                pill_badge(
                                    ui,
                                    p.secondary_bg,
                                    p.secondary_fg,
                                    t.license_pro_badge,
                                    9.5,
                                );
                            }
                        });
                        ui.label(
                            RichText::new(t.reading_ruler_desc)
                                .size(11.0)
                                .color(p.text_variant),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if toggle_switch(ui, &mut ruler, p).changed() {
                            if ruler && !focus_allowed {
                                pro.request(ProFeature::FocusTools);
                            } else {
                                settings.reading_ruler = ruler;
                                *on_save_settings = true;
                            }
                        }
                    });
                });

                if ruler {
                    ui.add_space(10.0);
                    let avail = ui.available_width();
                    if avail >= 480.0 {
                        ui.columns(2, |cols| {
                            render_ruler_lines_control(
                                &mut cols[0],
                                p,
                                t,
                                settings,
                                on_save_settings,
                            );
                            render_ruler_opacity_control(
                                &mut cols[1],
                                p,
                                t,
                                settings,
                                on_save_settings,
                            );
                        });
                    } else {
                        render_ruler_lines_control(ui, p, t, settings, on_save_settings);
                        ui.add_space(8.0);
                        render_ruler_opacity_control(ui, p, t, settings, on_save_settings);
                    }
                }
            });

            ui.add_space(8.0);
            let avail = ui.available_width();
            if avail >= 560.0 {
                ui.columns(2, |cols| {
                    render_bionic_card(&mut cols[0], p, t, settings, on_save_settings, pro);
                    render_rsvp_card(&mut cols[1], p, t, settings, on_save_settings);
                });
            } else {
                render_bionic_card(ui, p, t, settings, on_save_settings, pro);
                ui.add_space(8.0);
                render_rsvp_card(ui, p, t, settings, on_save_settings);
            }
        },
    );
}

fn render_ruler_lines_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(t.reading_ruler_height_label)
                .size(12.0)
                .strong()
                .color(p.text),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{}", settings.reading_ruler_lines.clamp(1, 5)))
                    .size(12.0)
                    .strong()
                    .color(p.accent),
            );
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for n in 1..=5u32 {
            let selected = settings.reading_ruler_lines == n;
            let (fill, fg, stroke) = if selected {
                (p.accent, p.accent_on, Stroke::NONE)
            } else {
                (
                    p.surface_container,
                    p.text,
                    Stroke::new(1.0, p.border_subtle),
                )
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(n.to_string()).size(11.5).strong().color(fg))
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(p.corner_sm)
                        .min_size(Vec2::new(28.0, 26.0)),
                )
                .clicked()
            {
                settings.reading_ruler_lines = n;
                *on_save_settings = true;
            }
        }
    });
}

fn render_ruler_opacity_control(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let mut pct = (settings.reading_ruler_opacity.clamp(0.20, 0.80) * 100.0).round();
    let readout = format!("{:.0}%", pct);
    let (changed, commit) = metric_slider(
        ui,
        &mut pct,
        20.0..=80.0,
        Some(1.0),
        t.reading_ruler_dim_label,
        &readout,
        None,
        p,
    );
    if changed {
        settings.reading_ruler_opacity = (pct / 100.0).clamp(0.20, 0.80);
    }
    if commit {
        *on_save_settings = true;
    }
}

fn render_bionic_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let focus_allowed = settings.can_use(ProFeature::FocusTools);
    row_card(ui, p, |ui| {
        let mut bionic = settings.effective_bionic_reading();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 48.0).max(80.0));
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(t.bionic_reading_title)
                            .size(13.0)
                            .strong()
                            .color(p.text),
                    );
                    if !focus_allowed {
                        pill_badge(ui, p.secondary_bg, p.secondary_fg, t.license_pro_badge, 9.5);
                    }
                });
                ui.label(
                    RichText::new(t.bionic_reading_desc)
                        .size(11.0)
                        .color(p.text_variant),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, &mut bionic, p).changed() {
                    if bionic && !focus_allowed {
                        pro.request(ProFeature::FocusTools);
                    } else {
                        settings.bionic_reading = bionic;
                        *on_save_settings = true;
                    }
                }
            });
        });
        ui.add_space(6.0);
        let status_label = if bionic {
            t.extra.settings_state_enabled
        } else {
            t.extra.settings_state_disabled
        };
        let (badge_bg, badge_fg) = if bionic {
            (p.tertiary_bg, p.tertiary)
        } else {
            (p.surface_container, p.text_variant)
        };
        pill_badge(ui, badge_bg, badge_fg, status_label, 10.0);
    });
}

fn render_rsvp_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    row_card(ui, p, |ui| {
        let mut wpm = (settings.rsvp_wpm.clamp(100, 1000)) as f32;
        let readout = fill(
            t.extra.settings_rsvp_wpm_fmt,
            &[("{wpm}", &format!("{:.0}", wpm))],
        );
        let (changed, commit) = metric_slider(
            ui,
            &mut wpm,
            100.0..=1000.0,
            Some(25.0),
            t.rsvp_speed,
            &readout,
            Some(t.rsvp_desc),
            p,
        );
        if changed {
            settings.rsvp_wpm = (wpm.round() as u32).clamp(100, 1000);
        }
        if commit {
            *on_save_settings = true;
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 46.0).max(80.0));
                ui.label(
                    RichText::new(t.rsvp_auto_accelerate)
                        .size(12.0)
                        .strong()
                        .color(p.text),
                );
                ui.label(
                    RichText::new(t.rsvp_auto_accelerate_desc)
                        .size(10.5)
                        .color(p.text_variant),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, &mut settings.rsvp_auto_accelerate, p).changed() {
                    *on_save_settings = true;
                }
            });
        });
    });
}

pub(super) fn render_navigation_display_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    keep_screen_on: &mut bool,
    on_save_settings: &mut bool,
) {
    section_card(ui, p, t.extra.settings_navigation_title, None, None, |ui| {
        let avail = ui.available_width();
        if avail >= 560.0 {
            ui.columns(2, |cols| {
                render_trackpad_card(&mut cols[0], p, t, settings, on_save_settings);
                render_keep_screen_on_card(
                    &mut cols[1],
                    p,
                    t,
                    settings,
                    keep_screen_on,
                    on_save_settings,
                );
            });
        } else {
            render_trackpad_card(ui, p, t, settings, on_save_settings);
            ui.add_space(8.0);
            render_keep_screen_on_card(ui, p, t, settings, keep_screen_on, on_save_settings);
        }
    });
}

fn render_trackpad_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    row_card(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 48.0).max(80.0));
                ui.label(
                    RichText::new(t.trackpad_swipes_title)
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.label(
                    RichText::new(t.trackpad_swipes_desc)
                        .size(11.0)
                        .color(p.text_variant),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, &mut settings.trackpad_swipe_pages, p).changed() {
                    *on_save_settings = true;
                }
            });
        });
    });
}

fn render_keep_screen_on_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    keep_screen_on: &mut bool,
    on_save_settings: &mut bool,
) {
    row_card(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 48.0).max(80.0));
                ui.label(
                    RichText::new(t.keep_screen_on)
                        .size(13.0)
                        .strong()
                        .color(p.text),
                );
                ui.label(
                    RichText::new(t.keep_screen_on_desc)
                        .size(11.0)
                        .color(p.text_variant),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, keep_screen_on, p).changed() {
                    crate::platform::set_keep_screen_on(*keep_screen_on);
                    settings.keep_screen_on = *keep_screen_on;
                    *on_save_settings = true;
                }
            });
        });
    });
}

pub(super) fn render_autoscroll_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    is_autoscrolling: &mut bool,
    on_toggle_autoscroll: &mut bool,
    on_save_settings: &mut bool,
) {
    section_card(
        ui,
        p,
        t.autoscroll_title,
        None,
        Some(t.autoscroll_desc),
        |ui| {
            row_card(ui, p, |ui| {
                let (scroll_icon, scroll_text) = if *is_autoscrolling {
                    (Icon::Pause, t.autoscroll_stop)
                } else {
                    (Icon::Play, t.autoscroll_start)
                };
                if ui
                    .add(IconButton::new(scroll_icon, scroll_text).font_size(12.5))
                    .clicked()
                {
                    *on_toggle_autoscroll = true;
                }
                ui.add_space(10.0);
                ui.label(
                    RichText::new(t.motion_type)
                        .size(12.0)
                        .strong()
                        .color(p.text),
                );
                ui.add_space(4.0);
                let items = [(true, t.mode_pause), (false, t.mode_continuous)];
                if segmented_control(ui, &mut settings.auto_scroll.pause_mode, &items, p, false) {
                    *on_save_settings = true;
                }
            });

            ui.add_space(10.0);
            if settings.auto_scroll.pause_mode {
                let avail = ui.available_width();
                if avail >= 520.0 {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        ui.columns(2, |cols| {
                            render_pause_duration_slider(
                                &mut cols[0],
                                p,
                                t,
                                settings,
                                on_save_settings,
                            );
                            render_step_distance_slider(
                                &mut cols[1],
                                p,
                                t,
                                settings,
                                on_save_settings,
                            );
                        });
                    });
                } else {
                    render_pause_duration_slider(ui, p, t, settings, on_save_settings);
                    ui.add_space(8.0);
                    render_step_distance_slider(ui, p, t, settings, on_save_settings);
                }
            } else {
                let readout = format!(
                    "{:.0}{}",
                    settings.auto_scroll.continuous_speed, t.extra.pixels_per_second_suffix
                );
                let (changed, commit) = metric_slider(
                    ui,
                    &mut settings.auto_scroll.continuous_speed,
                    10.0..=250.0,
                    Some(2.0),
                    t.scroll_speed,
                    &readout,
                    None,
                    p,
                );
                if changed || commit {
                    *on_save_settings = true;
                }
            }
        },
    );
}

fn render_pause_duration_slider(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let readout = format!(
        "{:.1}{}",
        settings.auto_scroll.pause_duration, t.extra.seconds_suffix
    );
    let (changed, commit) = metric_slider(
        ui,
        &mut settings.auto_scroll.pause_duration,
        0.5..=15.0,
        Some(0.5),
        t.pause_duration,
        &readout,
        None,
        p,
    );
    if changed || commit {
        *on_save_settings = true;
    }
}

fn render_step_distance_slider(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let readout = format!("{:.0} px", settings.auto_scroll.step_distance);
    let (changed, commit) = metric_slider(
        ui,
        &mut settings.auto_scroll.step_distance,
        30.0..=350.0,
        Some(10.0),
        t.step_distance,
        &readout,
        None,
        p,
    );
    if changed || commit {
        *on_save_settings = true;
    }
}

pub(super) fn render_comics_and_pdf_section(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    section_card(ui, p, t.comics_title, None, None, |ui| {
        let modes = [
            ("Auto", t.mode_auto, t.mode_auto_desc),
            ("Manga", t.mode_manga, t.mode_manga_desc),
            ("Comic", t.mode_comic, t.mode_comic_desc),
            ("Webtoon", t.mode_webtoon, t.mode_webtoon_desc),
        ];

        let cols = if ui.available_width() >= 560.0 { 2 } else { 1 };
        for chunk in modes.chunks(cols) {
            if cols == 2 {
                ui.columns(2, |cols_ui| {
                    for (idx, &(mode_key, title, desc)) in chunk.iter().enumerate() {
                        render_comic_mode_card(
                            &mut cols_ui[idx],
                            p,
                            mode_key,
                            title,
                            desc,
                            settings,
                            on_save_settings,
                        );
                    }
                });
            } else {
                for &(mode_key, title, desc) in chunk {
                    render_comic_mode_card(
                        ui,
                        p,
                        mode_key,
                        title,
                        desc,
                        settings,
                        on_save_settings,
                    );
                }
            }
            ui.add_space(6.0);
        }

        ui.add_space(6.0);
        if ui.available_width() >= 560.0 {
            ui.columns(2, |cols_ui| {
                render_comic_layout_card(&mut cols_ui[0], p, t, settings, on_save_settings);
                render_comic_fit_card(&mut cols_ui[1], p, t, settings, on_save_settings);
            });
        } else {
            render_comic_layout_card(ui, p, t, settings, on_save_settings);
            ui.add_space(8.0);
            render_comic_fit_card(ui, p, t, settings, on_save_settings);
        }

        ui.add_space(10.0);
        render_smart_scan_card(ui, p, t, settings, on_save_settings, pro);
    });
}

fn render_comic_mode_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    mode_key: &str,
    title: &str,
    desc: &str,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    let is_sel = settings.comic.reading_mode == mode_key;
    let fill = if is_sel {
        mix_color(p.card_bg, p.accent_soft, 0.25)
    } else {
        p.card_bg
    };
    let stroke = if is_sel {
        Stroke::new(1.5, p.accent)
    } else {
        Stroke::new(1.0, p.border_subtle)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(p.corner_sm)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if ui
                .radio(
                    is_sel,
                    RichText::new(title).size(13.0).strong().color(p.text),
                )
                .clicked()
            {
                settings.comic.reading_mode = mode_key.to_string();
                *on_save_settings = true;
            }
            ui.label(RichText::new(desc).size(11.0).color(p.text_variant));
        });
}

fn render_comic_layout_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    row_card(ui, p, |ui| {
        ui.label(
            RichText::new(t.layout_title)
                .size(12.5)
                .strong()
                .color(p.text),
        );
        ui.add_space(6.0);
        let mut is_double = settings.comic.page_layout == "Double";
        let items = [(false, t.layout_single), (true, t.layout_double)];
        if segmented_control(ui, &mut is_double, &items, p, false) {
            settings.comic.page_layout = if is_double { "Double" } else { "Single" }.to_string();
            *on_save_settings = true;
        }
    });
}

fn render_comic_fit_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
) {
    row_card(ui, p, |ui| {
        ui.label(RichText::new(t.fit_title).size(12.5).strong().color(p.text));
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for (val, label) in [
                ("FitWindow", t.fit_window),
                ("FitWidth", t.fit_width),
                ("FitHeight", t.fit_height),
                ("Original", "100%"),
            ] {
                let selected = settings.comic.fit_mode == val;
                if themed_button(ui, label, selected, p).clicked() {
                    settings.comic.fit_mode = val.to_string();
                    *on_save_settings = true;
                }
            }
        });
    });
}

fn render_smart_scan_card(
    ui: &mut egui::Ui,
    p: &SettingsPalette,
    t: &Translations,
    settings: &mut AppSettings,
    on_save_settings: &mut bool,
    pro: &mut ProUpgradeState,
) {
    let allowed = settings.can_use(ProFeature::SmartScanProcessing);
    row_card(ui, p, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(t.extra.settings_pdf_smart_section)
                    .size(13.0)
                    .strong()
                    .color(p.text),
            );
            if !allowed {
                pill_badge(ui, p.secondary_bg, p.secondary_fg, t.license_pro_badge, 9.5);
            }
        });
        ui.add_space(8.0);

        let mut smart_dark = (settings.pdf.smart_dark || settings.comic.smart_dark) && allowed;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 48.0).max(80.0));
                ui.label(
                    RichText::new(t.smart_dark_mode)
                        .size(12.5)
                        .strong()
                        .color(p.text),
                );
                ui.label(
                    RichText::new(t.smart_dark_mode_tooltip)
                        .size(11.0)
                        .color(p.text_variant),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, &mut smart_dark, p).changed() {
                    if smart_dark && !allowed {
                        pro.request(ProFeature::SmartScanProcessing);
                    } else {
                        settings.pdf.smart_dark = smart_dark;
                        settings.comic.smart_dark = smart_dark;
                        *on_save_settings = true;
                    }
                }
            });
        });

        ui.add_space(8.0);
        let mut auto_crop = (settings.pdf.auto_crop || settings.comic.auto_crop) && allowed;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 48.0).max(80.0));
                ui.label(
                    RichText::new(t.auto_crop_margins)
                        .size(12.5)
                        .strong()
                        .color(p.text),
                );
                ui.label(
                    RichText::new(t.auto_crop_margins_tooltip)
                        .size(11.0)
                        .color(p.text_variant),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if toggle_switch(ui, &mut auto_crop, p).changed() {
                    if auto_crop && !allowed {
                        pro.request(ProFeature::SmartScanProcessing);
                    } else {
                        settings.pdf.auto_crop = auto_crop;
                        settings.comic.auto_crop = auto_crop;
                        *on_save_settings = true;
                    }
                }
            });
        });

        if auto_crop {
            ui.add_space(8.0);
            let mut tol = settings.pdf.auto_crop_tolerance.clamp(5, 80) as f32;
            let readout = format!("{:.0}", tol);
            let (changed, commit) = metric_slider(
                ui,
                &mut tol,
                5.0..=80.0,
                Some(1.0),
                t.crop_tolerance,
                &readout,
                Some(t.crop_sensitivity),
                p,
            );
            if changed {
                let v = (tol.round() as u8).clamp(5, 80);
                settings.pdf.auto_crop_tolerance = v;
                settings.comic.auto_crop_tolerance = v;
            }
            if commit {
                *on_save_settings = true;
            }
        }
    });
}

pub(super) fn render_hotkeys_section(ui: &mut egui::Ui, p: &SettingsPalette, t: &Translations) {
    section_card(ui, p, t.tab_hotkeys, None, None, |ui| {
        let cols = if ui.available_width() >= 580.0 { 2 } else { 1 };
        for chunk in t.hotkeys.chunks(cols) {
            if cols == 2 {
                ui.columns(2, |cols_ui| {
                    for (idx, hk) in chunk.iter().enumerate() {
                        render_hotkey_row(&mut cols_ui[idx], p, hk.keys, hk.description);
                    }
                });
            } else {
                for hk in chunk {
                    render_hotkey_row(ui, p, hk.keys, hk.description);
                }
            }
            ui.add_space(6.0);
        }
    });
}

fn render_hotkey_row(ui: &mut egui::Ui, p: &SettingsPalette, keys: &str, description: &str) {
    row_card(ui, p, |ui| {
        ui.horizontal(|ui| {
            kbd_badge(ui, p, keys);
            ui.add_space(6.0);
            ui.label(RichText::new(description).size(12.0).color(p.text));
        });
    });
}

fn paint_small_row_icon(ui: &mut egui::Ui, icon: Icon, p: &SettingsPalette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::hover());
    paint_icon(
        ui.painter(),
        icon,
        rect.shrink(2.0),
        if p.is_dark {
            p.accent
        } else {
            Color32::from_rgb(133, 83, 66)
        },
    );
}
