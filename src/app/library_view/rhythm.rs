use crate::app::icons::{Icon, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::{LibraryStore, unix_day_index, weekday_index_from_unix_day};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};

pub(crate) const DAILY_GOAL_MINUTES: u64 = 30;

pub(crate) fn daily_goal_met(today_seconds: u64) -> bool {
    today_seconds / 60 >= DAILY_GOAL_MINUTES
}

#[allow(clippy::too_many_arguments)]
pub fn render_reading_rhythm_panel(
    ui: &mut egui::Ui,
    panel_width: f32,
    target_outer_h: Option<f32>,
    library: &LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    container_fill: Color32,
    border_stroke: Stroke,
    is_cozy: bool,
) {
    let target_inner_h = target_outer_h
        .map(|h| (h - 24.0).max(176.0))
        .unwrap_or(180.0);
    egui::Frame::new()
        .fill(container_fill)
        .stroke(border_stroke)
        .corner_radius(12.0)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            let inner_w = (panel_width - 28.0).max(100.0);
            ui.set_width(inner_w);
            ui.set_min_height(target_inner_h);

            let (header_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 22.0), egui::Sense::hover());

            ui.painter().text(
                header_rect.left_center(),
                egui::Align2::LEFT_CENTER,
                t.library_rhythm_title,
                egui::FontId::proportional(15.5),
                if is_cozy {
                    CozyPalette::ON_SURFACE
                } else {
                    theme.text_color()
                },
            );

            let shield_color = if is_cozy {
                CozyPalette::TERTIARY
            } else {
                Color32::from_rgb(72, 187, 120)
            };
            let badge_font = egui::FontId::proportional(10.5);
            let badge_text_w = ui.fonts_mut(|f| {
                f.layout_no_wrap(
                    t.library_rhythm_local.to_string(),
                    badge_font.clone(),
                    shield_color,
                )
                .size()
                .x
            });
            let badge_w = (badge_text_w + 30.0).max(88.0);
            let badge_rect = Rect::from_min_size(
                Pos2::new(header_rect.max.x - badge_w, header_rect.center().y - 10.0),
                Vec2::new(badge_w, 20.0),
            );
            ui.painter()
                .rect_filled(badge_rect, 10.0, shield_color.gamma_multiply(0.14));
            let icon_r = Rect::from_center_size(
                Pos2::new(badge_rect.min.x + 10.0, badge_rect.center().y),
                Vec2::splat(12.0),
            );
            paint_icon(ui.painter(), Icon::Shield, icon_r, shield_color);
            ui.painter().text(
                Pos2::new(badge_rect.min.x + 23.0, badge_rect.center().y),
                egui::Align2::LEFT_CENTER,
                t.library_rhythm_local,
                badge_font,
                shield_color,
            );

            ui.add_space(6.0);

            let today = unix_day_index(crate::core::license::unix_now());
            let streak = library.current_streak_days(today);
            let total_minutes = library.total_reading_minutes();
            let hours_float = (total_minutes as f32 / 60.0 * 10.0).round() / 10.0;
            let completed = library.completed_books_count();

            let stat_gap = 6.0;
            let metric_w = ((inner_w - 2.0 * stat_gap) / 3.0).floor().max(50.0);

            ui.horizontal(|ui| {
                render_stat_box(
                    ui,
                    metric_w,
                    &streak.to_string(),
                    t.library_days_streak,
                    if is_cozy {
                        CozyPalette::SECONDARY
                    } else {
                        Color32::from_rgb(220, 160, 40)
                    },
                    theme,
                    is_cozy,
                );
                ui.add_space(stat_gap);
                render_stat_box(
                    ui,
                    metric_w,
                    &format!("{hours_float}"),
                    t.library_hours_total,
                    if is_cozy {
                        CozyPalette::PRIMARY
                    } else {
                        theme.accent_color()
                    },
                    theme,
                    is_cozy,
                );
                ui.add_space(stat_gap);
                render_stat_box(
                    ui,
                    metric_w,
                    &completed.to_string(),
                    t.library_books_done,
                    if is_cozy {
                        CozyPalette::TERTIARY
                    } else {
                        Color32::from_rgb(72, 187, 120)
                    },
                    theme,
                    is_cozy,
                );
            });

            ui.add_space(6.0);

            render_sparkline_card(ui, inner_w, library, theme, t, is_cozy);

            ui.add_space(6.0);

            let today_secs = library
                .daily_reading_seconds
                .get(&today)
                .copied()
                .unwrap_or(0);
            let today_mins = today_secs / 60;
            let goal_done = daily_goal_met(today_secs);

            let (footer_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 16.0), egui::Sense::hover());
            ui.painter().text(
                footer_rect.left_center(),
                egui::Align2::LEFT_CENTER,
                t.library_daily_goal,
                egui::FontId::proportional(11.0),
                if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                },
            );

            let status_color = if goal_done {
                if is_cozy {
                    CozyPalette::TERTIARY
                } else {
                    Color32::from_rgb(72, 187, 120)
                }
            } else if is_cozy {
                CozyPalette::ON_SURFACE_VARIANT
            } else {
                theme.secondary_text_color()
            };
            let status_text = if goal_done {
                t.library_goal_completed.to_string()
            } else {
                t.extra
                    .library_goal_today_progress
                    .replace("{minutes}", &today_mins.to_string())
            };
            let status_icon = if goal_done {
                Icon::Check
            } else {
                Icon::Hourglass
            };
            let text_font = egui::FontId::proportional(10.5);
            let galley = ui
                .painter()
                .layout_no_wrap(status_text, text_font, status_color);
            let icon_rect = Rect::from_center_size(
                Pos2::new(
                    footer_rect.max.x - galley.size().x - 10.0,
                    footer_rect.center().y,
                ),
                Vec2::splat(11.0),
            );
            paint_icon(ui.painter(), status_icon, icon_rect, status_color);
            ui.painter().galley(
                Pos2::new(
                    footer_rect.max.x - galley.size().x,
                    footer_rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                status_color,
            );
        });
}

fn render_stat_box(
    ui: &mut egui::Ui,
    box_width: f32,
    value: &str,
    label: &str,
    value_color: Color32,
    theme: &ReaderTheme,
    is_cozy: bool,
) {
    let box_fill = if is_cozy {
        CozyPalette::CARD_BG
    } else {
        theme.panel_bg_color().gamma_multiply(1.22)
    };
    let border = Stroke::new(
        1.0,
        if is_cozy {
            CozyPalette::OUTLINE.gamma_multiply(0.3)
        } else {
            theme.accent_color().gamma_multiply(0.22)
        },
    );

    egui::Frame::new()
        .fill(box_fill)
        .stroke(border)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(4, 4))
        .show(ui, |ui| {
            let w = (box_width - 8.0).max(40.0);
            ui.set_width(w);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(value).size(17.0).strong().color(value_color));
                ui.add(
                    egui::Label::new(RichText::new(label).size(9.0).color(if is_cozy {
                        CozyPalette::ON_SURFACE_VARIANT
                    } else {
                        theme.secondary_text_color()
                    }))
                    .truncate(),
                );
            });
        });
}

fn render_sparkline_card(
    ui: &mut egui::Ui,
    width: f32,
    library: &LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    is_cozy: bool,
) {
    let box_fill = if is_cozy {
        CozyPalette::CARD_BG
    } else {
        theme.panel_bg_color().gamma_multiply(1.22)
    };
    let border = Stroke::new(
        1.0,
        if is_cozy {
            CozyPalette::OUTLINE.gamma_multiply(0.3)
        } else {
            theme.accent_color().gamma_multiply(0.22)
        },
    );

    egui::Frame::new()
        .fill(box_fill)
        .stroke(border)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            let inner_w = (width - 20.0).max(80.0);
            ui.set_width(inner_w);

            let (header_r, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 16.0), egui::Sense::hover());
            ui.painter().text(
                header_r.left_center(),
                egui::Align2::LEFT_CENTER,
                t.library_activity_7_days,
                egui::FontId::proportional(11.0),
                if is_cozy {
                    CozyPalette::ON_SURFACE
                } else {
                    theme.text_color()
                },
            );
            ui.painter().text(
                header_r.right_center(),
                egui::Align2::RIGHT_CENTER,
                t.library_minutes_per_day,
                egui::FontId::proportional(9.5),
                if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                },
            );

            ui.add_space(4.0);

            let today = unix_day_index(crate::core::license::unix_now());
            let day_names = [
                t.extra.weekday_mon,
                t.extra.weekday_tue,
                t.extra.weekday_wed,
                t.extra.weekday_thu,
                t.extra.weekday_fri,
                t.extra.weekday_sat,
                t.extra.weekday_sun,
            ];

            let chart_height = 34.0;
            let (chart_rect, _) = ui.allocate_exact_size(
                Vec2::new(inner_w, chart_height + 16.0),
                egui::Sense::hover(),
            );

            let col_w = chart_rect.width() / 7.0;
            let mut max_minutes = 30.0f32;
            for i in 0..7usize {
                let day = today.saturating_sub((6 - i) as u64);
                let mins = library
                    .daily_reading_seconds
                    .get(&day)
                    .copied()
                    .unwrap_or(0) as f32
                    / 60.0;
                if mins > max_minutes {
                    max_minutes = mins;
                }
            }

            for i in 0..7usize {
                let day = today.saturating_sub((6 - i) as u64);
                let secs = library
                    .daily_reading_seconds
                    .get(&day)
                    .copied()
                    .unwrap_or(0);
                let mins = secs as f32 / 60.0;
                let fraction = (mins / max_minutes).clamp(0.08, 1.0);
                let bar_h = chart_height * fraction;

                let col_rect = Rect::from_min_size(
                    Pos2::new(chart_rect.min.x + i as f32 * col_w, chart_rect.min.y),
                    Vec2::new(col_w, chart_height + 16.0),
                );
                let bar_resp = ui.interact(
                    col_rect,
                    ui.scope_id().with(("spark_bar", i)),
                    egui::Sense::hover(),
                );

                let cx = col_rect.center().x;
                let bar_w = (col_w - 6.0).clamp(8.0, 24.0);
                let bar_rect = Rect::from_min_max(
                    Pos2::new(cx - bar_w * 0.5, chart_rect.min.y + chart_height - bar_h),
                    Pos2::new(cx + bar_w * 0.5, chart_rect.min.y + chart_height),
                );

                let is_today = i == 6;
                let base_color = if is_today {
                    if is_cozy {
                        CozyPalette::PRIMARY
                    } else {
                        theme.accent_color()
                    }
                } else if mins > 0.0 {
                    if is_cozy {
                        CozyPalette::PRIMARY.gamma_multiply(0.45)
                    } else {
                        theme.accent_color().gamma_multiply(0.45)
                    }
                } else if is_cozy {
                    CozyPalette::SURFACE_HIGHEST
                } else {
                    theme.panel_bg_color().gamma_multiply(1.5)
                };
                let bar_color = if bar_resp.hovered() {
                    if is_cozy {
                        CozyPalette::PRIMARY_CONTAINER
                    } else {
                        theme.accent_color()
                    }
                } else {
                    base_color
                };

                ui.painter().rect_filled(bar_rect, 2.5, bar_color);

                let wd_idx = weekday_index_from_unix_day(day);
                let day_label = day_names[wd_idx];
                let label_pos = Pos2::new(cx, chart_rect.min.y + chart_height + 8.0);
                let label_color = if is_today {
                    if is_cozy {
                        CozyPalette::PRIMARY
                    } else {
                        theme.accent_color()
                    }
                } else if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                };
                ui.painter().text(
                    label_pos,
                    egui::Align2::CENTER_CENTER,
                    day_label,
                    egui::FontId::proportional(9.5),
                    label_color,
                );

                let rounded_mins = secs.div_ceil(60);
                let mins_str = t
                    .extra
                    .library_minutes_short
                    .replace("{minutes}", &rounded_mins.to_string());
                let tip = format!("{day_label}: {mins_str}");
                bar_resp.on_hover_text(tip);
            }
        });
}
