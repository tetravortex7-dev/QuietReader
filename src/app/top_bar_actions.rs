use super::dialogs::SettingsTab;
use super::icons::{Icon, IconButton, paint_icon};
use super::theme::CozyPalette;
use super::{QuietReaderApp, TabBarAction, TabsBar, render_top_bar};
use crate::app::toc_view::SidebarTab;
use crate::core::library::{LibraryMode, UiDesignMode, unix_day_index};
use crate::core::license::ProFeature;
use crate::document::DocumentViewKind;
use eframe::egui::{self, Color32, Panel, Pos2, Rect, RichText, Stroke, Vec2};

pub(super) fn render(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pro_required: &mut Option<ProFeature>,
) {
    let ptr_top = ctx
        .input(|i| i.pointer.latest_pos().map(|p| p.y < 72.0))
        .unwrap_or(false);
    if !app.is_fullscreen || ptr_top || app.show_settings_dialog {
        if app.show_library || app.library.design == UiDesignMode::Cozy {
            render_unified_bar(app, ui, ctx, pro_required);
        } else {
            render_classic_bar(app, ui, ctx, pro_required);
        }
    }
}

fn render_unified_bar(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pro_required: &mut Option<ProFeature>,
) {
    let t = app.settings.ui_language.t();
    let is_cozy = !app.theme.is_dark();
    let bar_bg = if is_cozy {
        CozyPalette::SURFACE_LOW
    } else {
        app.theme.panel_bg_color()
    };
    let border_color = if is_cozy {
        CozyPalette::OUTLINE.gamma_multiply(0.35)
    } else {
        app.theme.accent_color().gamma_multiply(0.2)
    };

    let mut tab_to_activate = None;
    let mut tab_to_close = None;
    let mut open_new_file = false;

    let label_font = egui::FontId::proportional(11.0);
    let today = unix_day_index(crate::core::license::unix_now());
    let streak = app.library.current_streak_days(today);
    let streak_label = format!("{streak} {}", t.library_days_streak);
    let today_mins = app
        .library
        .daily_reading_seconds
        .get(&today)
        .copied()
        .unwrap_or(0)
        / 60;
    let ambient_label = active_ambient_label(app, t);
    let show_back_to_lib = !app.show_library && app.library.mode != LibraryMode::ReaderOnly;
    let show_reader_tools = !app.show_library && app.document.is_some();
    let update_tag = match app.updater.status() {
        crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
            Some(info.latest_version.clone())
        }
        _ => None,
    };

    let (cozy_w, studio_w, streak_text_w, amb_text_w, brand_w, back_w, update_w) =
        ui.fonts_mut(|f| {
            (
                f.layout_no_wrap(
                    t.library_design_cozy.to_string(),
                    label_font.clone(),
                    Color32::WHITE,
                )
                .size()
                .x,
                f.layout_no_wrap(
                    t.library_design_studio.to_string(),
                    label_font.clone(),
                    Color32::WHITE,
                )
                .size()
                .x,
                f.layout_no_wrap(streak_label.clone(), label_font.clone(), Color32::WHITE)
                    .size()
                    .x,
                f.layout_no_wrap(
                    ambient_label.to_string(),
                    label_font.clone(),
                    Color32::WHITE,
                )
                .size()
                .x,
                f.layout_no_wrap(
                    "QuietReader".to_string(),
                    egui::FontId::proportional(14.5),
                    Color32::WHITE,
                )
                .size()
                .x,
                f.layout_no_wrap(
                    t.extra.reader_back_to_library.to_string(),
                    label_font.clone(),
                    Color32::WHITE,
                )
                .size()
                .x,
                update_tag
                    .as_ref()
                    .map(|v| {
                        f.layout_no_wrap(format!("v{v}"), label_font.clone(), Color32::WHITE)
                            .size()
                            .x
                            + 24.0
                    })
                    .unwrap_or(0.0),
            )
        });

    Panel::top("unified_top_bar")
        .frame(
            egui::Frame::default()
                .fill(bar_bg)
                .stroke(Stroke::new(1.0, border_color))
                .inner_margin(egui::Margin::symmetric(14, 6)),
        )
        .show(ui, |ui| {
            let full_w = ui.available_width();
            let compact = full_w < 1040.0;
            let ultra_compact = full_w < 740.0;
            let show_brand_text = full_w >= 860.0;
            let show_back_text = full_w >= 960.0;

            let side_w = (cozy_w.max(studio_w) + 20.0).clamp(62.0, 96.0);
            let capsule_w = side_w * 2.0;
            let amb_w = if ultra_compact {
                0.0
            } else if compact {
                34.0
            } else {
                (amb_text_w + 42.0).clamp(118.0, 224.0)
            };
            let streak_w = if ultra_compact {
                0.0
            } else if compact {
                44.0
            } else {
                (streak_text_w + 34.0).clamp(86.0, 175.0)
            };
            let reader_tools_w = if show_reader_tools && !ultra_compact {
                58.0
            } else {
                0.0
            };
            let util_w = 102.0 + reader_tools_w + if update_w > 0.0 { update_w + 8.0 } else { 0.0 };
            let right_reserved_w = capsule_w
                + amb_w
                + streak_w
                + util_w
                + if ultra_compact { 24.0 } else { 54.0 };
            let back_btn_w = if show_back_to_lib {
                if show_back_text {
                    back_w + 36.0
                } else {
                    32.0
                }
            } else {
                0.0
            };
            let left_brand_w =
                back_btn_w + 22.0 + if show_brand_text { 6.0 + brand_w } else { 0.0 } + 24.0;
            let tabs_budget_w = (full_w - left_brand_w - right_reserved_w - 36.0).max(0.0);

            let row_height = 30.0;
            ui.allocate_ui_with_layout(
                Vec2::new(full_w, row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    if show_back_to_lib {
                        let btn_w = if show_back_text { back_w + 26.0 } else { 28.0 };
                        let (br, back_resp) =
                            ui.allocate_exact_size(Vec2::new(btn_w, 26.0), egui::Sense::click());
                        if back_resp.hovered() {
                            ui.painter().rect_filled(
                                br,
                                8.0,
                                if is_cozy {
                                    CozyPalette::SURFACE_CONTAINER
                                } else {
                                    app.theme.panel_bg_color().gamma_multiply(1.3)
                                },
                            );
                        }
                        let fg = if back_resp.hovered() {
                            if is_cozy {
                                CozyPalette::ON_SURFACE
                            } else {
                                app.theme.text_color()
                            }
                        } else if is_cozy {
                            CozyPalette::ON_SURFACE_VARIANT
                        } else {
                            app.theme.secondary_text_color()
                        };
                        paint_icon(
                            ui.painter(),
                            Icon::ArrowLeft,
                            Rect::from_center_size(
                                Pos2::new(
                                    if show_back_text {
                                        br.min.x + 11.0
                                    } else {
                                        br.center().x
                                    },
                                    br.center().y,
                                ),
                                Vec2::splat(12.0),
                            ),
                            fg,
                        );
                        if show_back_text {
                            ui.painter().text(
                                Pos2::new(br.min.x + 21.0, br.center().y),
                                egui::Align2::LEFT_CENTER,
                                t.extra.reader_back_to_library,
                                egui::FontId::proportional(11.5),
                                fg,
                            );
                        }
                        if back_resp.on_hover_text(t.library_open_tooltip).clicked() {
                            app.show_library = true;
                            app.show_recent_dialog = false;
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }

                    let logo_rect = Rect::from_min_size(ui.cursor().min, Vec2::splat(22.0));
                    let logo_resp = ui
                        .allocate_rect(logo_rect, egui::Sense::click())
                        .on_hover_text(t.library_open_tooltip);
                    ui.painter().rect_filled(
                        logo_rect,
                        5.0,
                        if is_cozy {
                            CozyPalette::PRIMARY
                        } else {
                            app.theme.accent_color()
                        },
                    );
                    paint_icon(
                        ui.painter(),
                        Icon::Book,
                        Rect::from_center_size(logo_rect.center(), Vec2::splat(13.0)),
                        Color32::WHITE,
                    );
                    ui.add_space(6.0);

                    let brand_clicked = if show_brand_text {
                        let brand_resp = ui
                            .add(
                                egui::Label::new(
                                    RichText::new("QuietReader").strong().size(14.5).color(
                                        if is_cozy {
                                            CozyPalette::ON_SURFACE
                                        } else {
                                            app.theme.text_color()
                                        },
                                    ),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_text(t.library_open_tooltip);
                        brand_resp.clicked()
                    } else {
                        false
                    };
                    if logo_resp.clicked() || brand_clicked {
                        app.show_library = !app.show_library;
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    if !app.tabs.is_empty() && tabs_budget_w > 60.0 {
                        ui.allocate_ui_with_layout(
                            Vec2::new(tabs_budget_w, 26.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                egui::ScrollArea::horizontal()
                                    .id_salt("unified_tabs_scroll")
                                    .scroll_bar_visibility(
                                        egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                    )
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            for (idx, tab) in app.tabs.iter().enumerate() {
                                                let is_active =
                                                    idx == app.active_tab && !app.show_library;
                                                let tab_bg = if is_active {
                                                    if is_cozy {
                                                        CozyPalette::CARD_BG
                                                    } else {
                                                        app.theme
                                                            .panel_bg_color()
                                                            .gamma_multiply(1.3)
                                                    }
                                                } else if is_cozy {
                                                    CozyPalette::SURFACE_HIGH
                                                } else {
                                                    app.theme
                                                        .panel_bg_color()
                                                        .gamma_multiply(1.05)
                                                };

                                                let stroke = if is_active {
                                                    Stroke::new(
                                                        1.0,
                                                        if is_cozy {
                                                            CozyPalette::OUTLINE.gamma_multiply(0.4)
                                                        } else {
                                                            app.theme.accent_color()
                                                        },
                                                    )
                                                } else {
                                                    Stroke::NONE
                                                };

                                                egui::Frame::new()
                                                    .fill(tab_bg)
                                                    .stroke(stroke)
                                                    .corner_radius(6.0)
                                                    .inner_margin(egui::Margin::symmetric(8, 3))
                                                    .show(ui, |ui| {
                                                        ui.horizontal(|ui| {
                                                            let badge_bg =
                                                                match tab.format_badge.as_str() {
                                                                    "PDF" => CozyPalette::SECONDARY,
                                                                    "FB2" | "CBZ" | "CBR" => {
                                                                        CozyPalette::TERTIARY_CONTAINER
                                                                    }
                                                                    _ => CozyPalette::PRIMARY,
                                                                };
                                                            let (br, _) = ui.allocate_exact_size(
                                                                Vec2::new(26.0, 13.0),
                                                                egui::Sense::hover(),
                                                            );
                                                            ui.painter().rect_filled(
                                                                br,
                                                                2.5,
                                                                badge_bg,
                                                            );
                                                            ui.painter().text(
                                                                br.center(),
                                                                egui::Align2::CENTER_CENTER,
                                                                &tab.format_badge,
                                                                egui::FontId::proportional(8.5),
                                                                Color32::WHITE,
                                                            );
                                                            ui.add_space(2.0);

                                                            let short = tab.short_title(16);
                                                            let tab_label = ui
                                                                .add(
                                                                    egui::Label::new(
                                                                        RichText::new(short)
                                                                            .size(11.5)
                                                                            .color(if is_active {
                                                                                if is_cozy {
                                                                                    CozyPalette::PRIMARY
                                                                                } else {
                                                                                    app.theme.accent_color()
                                                                                }
                                                                            } else if is_cozy {
                                                                                CozyPalette::ON_SURFACE
                                                                            } else {
                                                                                app.theme.text_color()
                                                                            }),
                                                                    )
                                                                    .sense(egui::Sense::click()),
                                                                )
                                                                .on_hover_text(&tab.title);
                                                            if tab_label.clicked() {
                                                                tab_to_activate = Some(idx);
                                                            }

                                                            let (xr, xresp) = ui
                                                                .allocate_exact_size(
                                                                    Vec2::splat(14.0),
                                                                    egui::Sense::click(),
                                                                );
                                                            if xresp.hovered() {
                                                                ui.painter().circle_filled(
                                                                    xr.center(),
                                                                    6.5,
                                                                    Color32::from_black_alpha(25),
                                                                );
                                                            }
                                                            paint_icon(
                                                                ui.painter(),
                                                                Icon::Cross,
                                                                Rect::from_center_size(
                                                                    xr.center(),
                                                                    Vec2::splat(8.0),
                                                                ),
                                                                if is_cozy {
                                                                    CozyPalette::ON_SURFACE_VARIANT
                                                                } else {
                                                                    app.theme.secondary_text_color()
                                                                },
                                                            );
                                                            if xresp.clicked() {
                                                                tab_to_close = Some(idx);
                                                            }
                                                        });
                                                    });
                                                ui.add_space(4.0);
                                            }
                                        });
                                    });
                            },
                        );
                    }

                    if ui
                        .add(IconButton::new(Icon::Plus, "").font_size(12.0))
                        .on_hover_text(t.open_file_tooltip)
                        .clicked()
                    {
                        open_new_file = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let settings_active =
                            app.show_settings_dialog && app.settings_tab != SettingsTab::Fonts;
                        if ui
                            .add(
                                IconButton::new(Icon::Settings, "")
                                    .font_size(14.0)
                                    .selected(settings_active),
                            )
                            .on_hover_text(t.settings_title)
                            .clicked()
                        {
                            if app.show_settings_dialog && app.settings_tab == SettingsTab::Fonts {
                                app.settings_tab = SettingsTab::Appearance;
                            } else {
                                app.show_settings_dialog = !app.show_settings_dialog;
                            }
                        }

                        let fs_icon = if app.is_fullscreen {
                            Icon::Windowed
                        } else {
                            Icon::Fullscreen
                        };
                        if ui
                            .add(
                                IconButton::new(fs_icon, "")
                                    .font_size(13.0)
                                    .selected(app.is_fullscreen),
                            )
                            .on_hover_text(t.fullscreen)
                            .clicked()
                        {
                            app.set_fullscreen_mode(ctx, !app.is_fullscreen);
                        }

                        let fonts_active =
                            app.show_settings_dialog && app.settings_tab == SettingsTab::Fonts;
                        let aa_btn = ui
                            .add(
                                egui::Button::new(RichText::new("Aa").strong().size(13.0).color(
                                    if fonts_active {
                                        if is_cozy {
                                            CozyPalette::PRIMARY
                                        } else {
                                            app.theme.accent_color()
                                        }
                                    } else if is_cozy {
                                        CozyPalette::ON_SURFACE_VARIANT
                                    } else {
                                        app.theme.text_color()
                                    },
                                ))
                                .selected(fonts_active)
                                .corner_radius(6.0),
                            )
                            .on_hover_text(t.tab_fonts);
                        if aa_btn.clicked() {
                            if fonts_active {
                                app.show_settings_dialog = false;
                            } else {
                                app.settings_tab = SettingsTab::Fonts;
                                app.show_settings_dialog = true;
                            }
                        }

                        if show_reader_tools && !ultra_compact {
                            let act_fg = if is_cozy {
                                CozyPalette::PRIMARY
                            } else {
                                app.theme.accent_color()
                            };
                            let mut s_btn = IconButton::new(Icon::Search, "").font_size(13.0);
                            if app.search.is_open {
                                s_btn = s_btn.color(act_fg).fill(act_fg.gamma_multiply(0.16));
                            }
                            if ui.add(s_btn).on_hover_text(t.search_tooltip).clicked()
                                && matches!(
                                    app.view_kind(),
                                    DocumentViewKind::Text | DocumentViewKind::Pdf
                                )
                            {
                                app.search.toggle();
                            }

                            let mut toc_btn = IconButton::new(Icon::Toc, "").font_size(13.0);
                            if app.show_toc {
                                toc_btn = toc_btn.color(act_fg).fill(act_fg.gamma_multiply(0.16));
                            }
                            if ui.add(toc_btn).on_hover_text(t.toc_tooltip).clicked() {
                                app.show_toc = !app.show_toc;
                                if app.show_toc {
                                    app.sidebar_tab = SidebarTab::Toc;
                                }
                            }
                        }

                        if let Some(version) = update_tag.as_deref() {
                            let label = format!("v{version}");
                            let upd_btn = ui
                                .add(
                                    IconButton::new(Icon::Download, &label)
                                        .font_size(11.0)
                                        .color(if is_cozy {
                                            CozyPalette::PRIMARY
                                        } else {
                                            app.theme.accent_color()
                                        }),
                                )
                                .on_hover_text(t.tab_system);
                            if upd_btn.clicked() {
                                app.settings_tab = SettingsTab::System;
                                app.show_settings_dialog = true;
                            }
                        }

                        if !ultra_compact {
                            ui.separator();

                            let (streak_rect, streak_resp) = ui.allocate_exact_size(
                                Vec2::new(streak_w, 24.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(
                                streak_rect,
                                12.0,
                                if is_cozy {
                                    CozyPalette::SURFACE_CONTAINER
                                } else {
                                    app.theme.panel_bg_color().gamma_multiply(1.2)
                                },
                            );
                            let flame_rect = Rect::from_center_size(
                                Pos2::new(streak_rect.min.x + 12.0, streak_rect.center().y),
                                Vec2::splat(12.0),
                            );
                            paint_icon(
                                ui.painter(),
                                Icon::Flame,
                                flame_rect,
                                if is_cozy {
                                    CozyPalette::SECONDARY
                                } else {
                                    Color32::from_rgb(220, 160, 40)
                                },
                            );
                            if compact {
                                ui.painter().text(
                                    Pos2::new(streak_rect.min.x + 22.0, streak_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    streak.to_string(),
                                    label_font.clone(),
                                    if is_cozy {
                                        CozyPalette::ON_SURFACE
                                    } else {
                                        app.theme.text_color()
                                    },
                                );
                            } else {
                                ui.painter().text(
                                    Pos2::new(streak_rect.min.x + 22.0, streak_rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    &streak_label,
                                    label_font.clone(),
                                    if is_cozy {
                                        CozyPalette::ON_SURFACE
                                    } else {
                                        app.theme.text_color()
                                    },
                                );
                            }
                            let goal_hint = t
                                .extra
                                .library_goal_today_progress
                                .replace("{minutes}", &today_mins.to_string());
                            streak_resp.on_hover_text(goal_hint);

                            let (amb_rect, amb_resp) =
                                ui.allocate_exact_size(Vec2::new(amb_w, 24.0), egui::Sense::click());
                            let amb_playing = app.ambient.is_playing();
                            let amb_fill = if amb_playing {
                                if is_cozy {
                                    CozyPalette::PRIMARY.gamma_multiply(0.18)
                                } else {
                                    app.theme.accent_color().gamma_multiply(0.22)
                                }
                            } else if amb_resp.hovered() {
                                if is_cozy {
                                    CozyPalette::SURFACE_HIGH
                                } else {
                                    app.theme.panel_bg_color().gamma_multiply(1.3)
                                }
                            } else if is_cozy {
                                CozyPalette::SURFACE_CONTAINER
                            } else {
                                app.theme.panel_bg_color().gamma_multiply(1.2)
                            };
                            ui.painter().rect_filled(amb_rect, 12.0, amb_fill);
                            let wave_center = if compact {
                                amb_rect.center()
                            } else {
                                Pos2::new(amb_rect.min.x + 14.0, amb_rect.center().y)
                            };
                            if amb_playing {
                                let dot_pos = if compact {
                                    Pos2::new(amb_rect.max.x - 6.0, amb_rect.min.y + 6.0)
                                } else {
                                    Pos2::new(amb_rect.min.x + 9.0, amb_rect.center().y)
                                };
                                ui.painter().circle_filled(
                                    dot_pos,
                                    3.0,
                                    Color32::from_rgb(52, 168, 83),
                                );
                            }
                            let wave_offset = if amb_playing && !compact { 9.0 } else { 0.0 };
                            let wave_rect = Rect::from_center_size(
                                Pos2::new(wave_center.x + wave_offset, wave_center.y),
                                Vec2::splat(12.0),
                            );
                            paint_icon(
                                ui.painter(),
                                Icon::Soundwave,
                                wave_rect,
                                if is_cozy {
                                    CozyPalette::PRIMARY
                                } else {
                                    app.theme.accent_color()
                                },
                            );
                            if !compact {
                                ui.painter().text(
                                    Pos2::new(
                                        amb_rect.min.x + 23.0 + wave_offset,
                                        amb_rect.center().y,
                                    ),
                                    egui::Align2::LEFT_CENTER,
                                    ambient_label,
                                    label_font.clone(),
                                    if is_cozy {
                                        CozyPalette::ON_SURFACE
                                    } else {
                                        app.theme.text_color()
                                    },
                                );
                            }
                            if amb_resp.on_hover_text(t.ambient_title).clicked() {
                                if app.settings.can_use(ProFeature::AmbientAudio) {
                                    app.ambient.toggle_panel();
                                } else {
                                    *pro_required = Some(ProFeature::AmbientAudio);
                                }
                            }

                            ui.separator();
                        }

                        let is_cozy_active = app.library.design == UiDesignMode::Cozy;
                        let (capsule_rect, _) = ui.allocate_exact_size(
                            Vec2::new(capsule_w, 24.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            capsule_rect,
                            12.0,
                            if is_cozy {
                                CozyPalette::SURFACE_CONTAINER
                            } else {
                                app.theme.panel_bg_color().gamma_multiply(1.2)
                            },
                        );

                        let half_w = capsule_rect.width() * 0.5;
                        let cozy_r = Rect::from_min_size(capsule_rect.min, Vec2::new(half_w, 24.0));
                        let studio_r = Rect::from_min_size(
                            Pos2::new(capsule_rect.min.x + half_w, capsule_rect.min.y),
                            Vec2::new(half_w, 24.0),
                        );

                        let cozy_resp = ui
                            .allocate_rect(cozy_r, egui::Sense::click())
                            .on_hover_text(t.library_design_tooltip);
                        let studio_resp = ui
                            .allocate_rect(studio_r, egui::Sense::click())
                            .on_hover_text(t.library_design_tooltip);

                        if is_cozy_active {
                            ui.painter().rect_filled(
                                cozy_r.shrink(1.5),
                                10.0,
                                if is_cozy {
                                    CozyPalette::CARD_BG
                                } else {
                                    app.theme.panel_bg_color().gamma_multiply(1.4)
                                },
                            );
                            ui.painter().text(
                                cozy_r.center(),
                                egui::Align2::CENTER_CENTER,
                                t.library_design_cozy,
                                label_font.clone(),
                                if is_cozy {
                                    CozyPalette::PRIMARY
                                } else {
                                    app.theme.accent_color()
                                },
                            );
                            ui.painter().text(
                                studio_r.center(),
                                egui::Align2::CENTER_CENTER,
                                t.library_design_studio,
                                label_font,
                                if is_cozy {
                                    CozyPalette::ON_SURFACE_VARIANT
                                } else {
                                    app.theme.secondary_text_color()
                                },
                            );
                        } else {
                            ui.painter().rect_filled(
                                studio_r.shrink(1.5),
                                10.0,
                                if is_cozy {
                                    CozyPalette::CARD_BG
                                } else {
                                    app.theme.panel_bg_color().gamma_multiply(1.4)
                                },
                            );
                            ui.painter().text(
                                cozy_r.center(),
                                egui::Align2::CENTER_CENTER,
                                t.library_design_cozy,
                                label_font.clone(),
                                if is_cozy {
                                    CozyPalette::ON_SURFACE_VARIANT
                                } else {
                                    app.theme.secondary_text_color()
                                },
                            );
                            ui.painter().text(
                                studio_r.center(),
                                egui::Align2::CENTER_CENTER,
                                t.library_design_studio,
                                label_font,
                                if is_cozy {
                                    CozyPalette::PRIMARY
                                } else {
                                    app.theme.accent_color()
                                },
                            );
                        }

                        if cozy_resp.clicked() {
                            app.set_ui_design(UiDesignMode::Cozy);
                        }
                        if studio_resp.clicked() {
                            app.set_ui_design(UiDesignMode::Studio);
                        }
                    });
                },
            );
        });

    if let Some(idx) = tab_to_activate {
        app.show_library = false;
        app.activate_tab(idx);
    }
    if let Some(idx) = tab_to_close {
        app.close_tab(idx);
    }
    if open_new_file {
        app.open_file_dialog();
    }
}

fn active_ambient_label<'a>(app: &QuietReaderApp, t: &'a crate::i18n::Translations) -> &'a str {
    if app.ambient.is_playing()
        && let Some(preset) = app.ambient.settings.selected_preset.as_deref()
    {
        return match preset {
            "laser_focus" => t.preset_laser_focus,
            "rainy_evening" => t.preset_rainy_evening,
            "night_lofi" => t.preset_night_lofi,
            "deep_space" => t.preset_deep_space,
            _ => t.library_ambient_default,
        };
    }
    t.library_ambient_default
}

fn render_classic_bar(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pro_required: &mut Option<ProFeature>,
) {
    let license_active = app.settings.can_use(ProFeature::NeuralTts);
    let update_tag = match app.updater.status() {
        crate::updater::UpdateStatus::UpdateAvailable(ref info) => {
            Some(info.latest_version.clone())
        }
        _ => None,
    };
    let actions = render_top_bar(
        ui,
        app.document.is_some() && !app.show_library,
        app.document.is_some() && !app.show_library && app.view_kind() == DocumentViewKind::Text,
        app.settings.text_two_columns,
        app.settings.text_paragraph_mode && !app.focus_read.rsvp.active,
        app.focus_read.rsvp.active,
        app.library.mode != LibraryMode::ReaderOnly,
        app.is_fullscreen,
        app.show_toc,
        app.settings.effective_reading_ruler(),
        license_active,
        app.settings.ui_language,
        update_tag.as_deref(),
    );
    if actions.open_file {
        app.open_file_dialog();
    }
    if actions.open_folder {
        app.open_folder_dialog();
    }
    if actions.toggle_recent {
        app.show_recent_dialog = !app.show_recent_dialog;
    }
    if actions.toggle_library {
        app.show_library = !app.show_library;
        if app.show_library {
            app.show_recent_dialog = false;
        }
    }
    if actions.toggle_opds {
        app.show_opds_dialog = !app.show_opds_dialog;
    }
    if !app.show_library {
        if actions.toggle_toc {
            app.show_toc = !app.show_toc;
            if app.show_toc {
                app.sidebar_tab = SidebarTab::Toc;
            }
        }
        if actions.toggle_bookmark {
            app.toggle_quick_bookmark();
        }
        if actions.toggle_bookmarks_panel {
            if !app.show_toc {
                app.show_toc = true;
                app.sidebar_tab = SidebarTab::Bookmarks;
            } else if app.sidebar_tab == SidebarTab::Bookmarks {
                app.show_toc = false;
            } else {
                app.sidebar_tab = SidebarTab::Bookmarks;
            }
        }
        if actions.toggle_search {
            if matches!(
                app.view_kind(),
                DocumentViewKind::Text | DocumentViewKind::Pdf
            ) && app.document.is_some()
            {
                app.search.toggle();
            }
        }
        if actions.open_dictionary {
            let selection = crate::app::selection::get_selected_text(ctx);
            if let Some(text) = selection.filter(|text| !text.trim().is_empty()) {
                app.dictionary
                    .open_word(text, app.settings.ui_language, false);
            } else {
                app.dictionary.visible = true;
                app.dictionary.refresh_installed();
            }
        }
        if actions.toggle_tts {
            app.close_focus_reading();
            app.tts.toggle_panel();
            if app.tts.show_panel && !app.tts.status().is_playing {
                if let Some(doc) = &app.document {
                    let visible_para = crate::core::ReaderCore::estimate_visible_paragraph(
                        doc.as_ref(),
                        app.current_chapter,
                        app.current_scroll_y,
                        app.font_size,
                    );
                    app.last_tts_paragraph = Some(visible_para);
                }
            }
        }
        if actions.toggle_reading_ruler {
            if app.settings.can_use(ProFeature::FocusTools) {
                app.settings.reading_ruler = !app.settings.reading_ruler;
                app.mark_settings_dirty();
            } else {
                *pro_required = Some(ProFeature::FocusTools);
            }
        }
        if let Some(spread_mode) = actions.set_text_spread
            && app.settings.text_two_columns != spread_mode
        {
            app.settings.text_two_columns = spread_mode;
            app.mark_settings_dirty();
        }
        if actions.toggle_paragraph_mode {
            app.toggle_paragraph_mode();
        }
        if actions.toggle_rsvp {
            if app.focus_read.rsvp.active {
                app.close_speed_reading();
            } else {
                app.start_speed_reading();
            }
        }
        if actions.toggle_ambient {
            if app.settings.can_use(ProFeature::AmbientAudio) {
                app.ambient.toggle_panel();
            } else {
                *pro_required = Some(ProFeature::AmbientAudio);
            }
        }
    }
    if actions.open_license {
        app.pro_upgrade.open(None);
    }
    if actions.toggle_fullscreen {
        app.set_fullscreen_mode(ctx, !app.is_fullscreen);
    }
    if actions.toggle_design {
        app.set_ui_design(UiDesignMode::Cozy);
    }
    if actions.toggle_settings {
        app.show_settings_dialog = !app.show_settings_dialog;
    }
    if actions.open_update {
        if let crate::updater::UpdateStatus::UpdateAvailable(ref info) = app.updater.status() {
            crate::platform::open_url(&info.release_url);
        }
    }

    if !app.tabs.is_empty() {
        let mut tab_action = TabBarAction::None;
        Panel::top("tabs_bar").show(ui, |ui| {
            tab_action = TabsBar::render(
                ui,
                &app.tabs,
                app.active_tab,
                &app.theme,
                app.settings.ui_language,
            );
        });
        match tab_action {
            TabBarAction::None => {}
            TabBarAction::Activate(idx) => app.activate_tab(idx),
            TabBarAction::Close(idx) => app.close_tab(idx),
            TabBarAction::OpenNewFile => app.open_file_dialog(),
        }
    }
}
