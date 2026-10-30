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
    _pro_required: &mut Option<ProFeature>,
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

    Panel::top("unified_top_bar")
        .frame(
            egui::Frame::default()
                .fill(bar_bg)
                .stroke(Stroke::new(1.0, border_color))
                .inner_margin(egui::Margin::symmetric(14, 6)),
        )
        .show(ui, |ui| {
            let row_height = 30.0;
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let logo_rect = Rect::from_min_size(ui.cursor().min, Vec2::splat(22.0));
                    let logo_resp = ui.allocate_rect(logo_rect, egui::Sense::click());
                    ui.painter().rect_filled(
                        logo_rect,
                        4.0,
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

                    let brand_resp = ui.add(
                        egui::Label::new(RichText::new("QuietReader").strong().size(14.5).color(
                            if is_cozy {
                                CozyPalette::ON_SURFACE
                            } else {
                                app.theme.text_color()
                            },
                        ))
                        .sense(egui::Sense::click()),
                    );
                    if logo_resp.clicked() || brand_resp.clicked() {
                        app.show_library = !app.show_library;
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    for (idx, tab) in app.tabs.iter().enumerate() {
                        let is_active = idx == app.active_tab && !app.show_library;
                        let tab_bg = if is_active {
                            if is_cozy {
                                CozyPalette::CARD_BG
                            } else {
                                app.theme.panel_bg_color().gamma_multiply(1.3)
                            }
                        } else if is_cozy {
                            CozyPalette::SURFACE_HIGH
                        } else {
                            app.theme.panel_bg_color().gamma_multiply(1.05)
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
                                    let badge_bg = match tab.format_badge.as_str() {
                                        "PDF" => CozyPalette::SECONDARY,
                                        "FB2" | "CBZ" | "CBR" => CozyPalette::TERTIARY_CONTAINER,
                                        _ => CozyPalette::PRIMARY,
                                    };
                                    let (br, _) = ui.allocate_exact_size(
                                        Vec2::new(26.0, 13.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(br, 2.5, badge_bg);
                                    ui.painter().text(
                                        br.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &tab.format_badge,
                                        egui::FontId::proportional(8.5),
                                        Color32::WHITE,
                                    );
                                    ui.add_space(2.0);

                                    let short = tab.short_title(16);
                                    let tab_label = ui.add(
                                        egui::Label::new(RichText::new(short).size(11.5).color(
                                            if is_active {
                                                if is_cozy {
                                                    CozyPalette::PRIMARY
                                                } else {
                                                    app.theme.accent_color()
                                                }
                                            } else if is_cozy {
                                                CozyPalette::ON_SURFACE
                                            } else {
                                                app.theme.text_color()
                                            },
                                        ))
                                        .sense(egui::Sense::click()),
                                    );
                                    if tab_label.clicked() {
                                        tab_to_activate = Some(idx);
                                    }

                                    let (xr, xresp) = ui.allocate_exact_size(
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
                                        Rect::from_center_size(xr.center(), Vec2::splat(8.0)),
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

                    if ui
                        .add(IconButton::new(Icon::Plus, "").font_size(12.0))
                        .clicked()
                    {
                        open_new_file = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(IconButton::new(Icon::Settings, "").font_size(14.0))
                            .on_hover_text(t.tab_about)
                            .clicked()
                        {
                            app.show_settings_dialog = !app.show_settings_dialog;
                        }

                        if ui
                            .add(IconButton::new(Icon::Fullscreen, "").font_size(13.0))
                            .on_hover_text(t.fullscreen)
                            .clicked()
                        {
                            app.set_fullscreen_mode(ctx, !app.is_fullscreen);
                        }

                        let aa_btn = ui.add(
                            egui::Button::new(RichText::new("Aa").strong().size(13.0).color(
                                if is_cozy {
                                    CozyPalette::ON_SURFACE_VARIANT
                                } else {
                                    app.theme.text_color()
                                },
                            ))
                            .frame(false),
                        );
                        if aa_btn.clicked() {
                            app.show_settings_dialog = !app.show_settings_dialog;
                        }

                        ui.separator();

                        let today = unix_day_index(crate::core::license::unix_now());
                        let streak = app.library.current_streak_days(today);
                        let (streak_rect, _) =
                            ui.allocate_exact_size(Vec2::new(76.0, 24.0), egui::Sense::hover());
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
                        ui.painter().text(
                            Pos2::new(streak_rect.min.x + 22.0, streak_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{streak} дней"),
                            egui::FontId::proportional(11.0),
                            if is_cozy {
                                CozyPalette::ON_SURFACE
                            } else {
                                app.theme.text_color()
                            },
                        );

                        let (amb_rect, amb_resp) =
                            ui.allocate_exact_size(Vec2::new(140.0, 24.0), egui::Sense::click());
                        ui.painter().rect_filled(
                            amb_rect,
                            12.0,
                            if amb_resp.hovered() {
                                if is_cozy {
                                    CozyPalette::SURFACE_HIGH
                                } else {
                                    app.theme.panel_bg_color().gamma_multiply(1.3)
                                }
                            } else if is_cozy {
                                CozyPalette::SURFACE_CONTAINER
                            } else {
                                app.theme.panel_bg_color().gamma_multiply(1.2)
                            },
                        );
                        let wave_rect = Rect::from_center_size(
                            Pos2::new(amb_rect.min.x + 12.0, amb_rect.center().y),
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
                        ui.painter().text(
                            Pos2::new(amb_rect.min.x + 22.0, amb_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            t.library_ambient_default,
                            egui::FontId::proportional(11.0),
                            if is_cozy {
                                CozyPalette::ON_SURFACE
                            } else {
                                app.theme.text_color()
                            },
                        );
                        if amb_resp.clicked() {
                            app.ambient.toggle_panel();
                        }

                        ui.separator();

                        let is_cozy_active = app.library.design == UiDesignMode::Cozy;
                        let (capsule_rect, _) =
                            ui.allocate_exact_size(Vec2::new(165.0, 24.0), egui::Sense::hover());
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

                        let cozy_resp = ui.allocate_rect(cozy_r, egui::Sense::click());
                        let studio_resp = ui.allocate_rect(studio_r, egui::Sense::click());

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
                                egui::FontId::proportional(11.0),
                                if is_cozy {
                                    CozyPalette::PRIMARY
                                } else {
                                    app.theme.accent_color()
                                },
                            );
                            ui.painter().text(
                                studio_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "Исследование",
                                egui::FontId::proportional(11.0),
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
                                egui::FontId::proportional(11.0),
                                if is_cozy {
                                    CozyPalette::ON_SURFACE_VARIANT
                                } else {
                                    app.theme.secondary_text_color()
                                },
                            );
                            ui.painter().text(
                                studio_r.center(),
                                egui::Align2::CENTER_CENTER,
                                "Исследование",
                                egui::FontId::proportional(11.0),
                                if is_cozy {
                                    CozyPalette::PRIMARY
                                } else {
                                    app.theme.accent_color()
                                },
                            );
                        }

                        if cozy_resp.clicked() && app.library.design != UiDesignMode::Cozy {
                            app.library.design = UiDesignMode::Cozy;
                            app.mark_settings_dirty();
                        }
                        if studio_resp.clicked() && app.library.design != UiDesignMode::Studio {
                            app.library.design = UiDesignMode::Studio;
                            app.mark_settings_dirty();
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
    if actions.toggle_settings {
        app.show_settings_dialog = !app.show_settings_dialog;
    }
    if actions.open_update {
        if let crate::updater::UpdateStatus::UpdateAvailable(ref info) = app.updater.status() {
            ctx.open_url(egui::OpenUrl::new_tab(&info.release_url));
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
