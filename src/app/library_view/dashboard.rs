use super::CoverLoader;
use super::rhythm::render_reading_rhythm_panel;
use crate::app::icons::{Icon, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::{LibraryEntry, LibraryReadingStatus, LibraryStore};
use crate::i18n::{Translations, UiLanguage};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use std::path::PathBuf;

pub fn render_dashboard(
    ui: &mut egui::Ui,
    library: &LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    lang: UiLanguage,
    continue_page: &mut usize,
    cover_loader: &mut CoverLoader,
    open_path: &mut Option<PathBuf>,
) {
    let is_cozy = !theme.is_dark();
    let container_fill = if is_cozy {
        CozyPalette::SURFACE_LOW
    } else {
        theme.panel_bg_color()
    };
    let border_stroke = Stroke::new(
        1.0,
        if is_cozy {
            CozyPalette::OUTLINE.gamma_multiply(0.35)
        } else {
            theme.accent_color().gamma_multiply(0.2)
        },
    );

    let total_width = ui.available_width();
    let gap = 16.0;

    if total_width >= 840.0 {
        let right_width = (total_width * 0.36).clamp(310.0, 380.0);
        let left_width = total_width - right_width - gap;

        let sync_id = ui.scope_id().with("dashboard_right_panel_h");
        let target_panel_h = ui
            .ctx()
            .memory(|m| m.data.get_temp::<f32>(sync_id))
            .unwrap_or(206.0)
            .clamp(196.0, 340.0);

        let start_pos = ui.cursor().min;
        let mut left_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    start_pos,
                    Vec2::new(left_width, 1000.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        render_continue_reading_panel(
            &mut left_ui,
            left_width,
            Some(target_panel_h),
            library,
            theme,
            t,
            lang,
            continue_page,
            container_fill,
            border_stroke,
            is_cozy,
            cover_loader,
            open_path,
        );
        let left_rect = left_ui.min_rect();

        let right_pos = Pos2::new(start_pos.x + left_width + gap, start_pos.y);
        let mut right_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    right_pos,
                    Vec2::new(right_width, 1000.0),
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        render_reading_rhythm_panel(
            &mut right_ui,
            right_width,
            Some(left_rect.height()),
            library,
            theme,
            t,
            container_fill,
            border_stroke,
            is_cozy,
        );
        let right_rect = right_ui.min_rect();

        let measured_right_h = right_rect.height();
        if (measured_right_h - target_panel_h).abs() > 1.0 {
            ui.ctx()
                .memory_mut(|m| m.data.insert_temp(sync_id, measured_right_h));
            ui.ctx().request_repaint();
        }

        let total_h = left_rect.height().max(right_rect.height());
        ui.advance_cursor_after_rect(Rect::from_min_size(
            start_pos,
            Vec2::new(total_width, total_h),
        ));
    } else {
        ui.vertical(|ui| {
            render_continue_reading_panel(
                ui,
                total_width,
                None,
                library,
                theme,
                t,
                lang,
                continue_page,
                container_fill,
                border_stroke,
                is_cozy,
                cover_loader,
                open_path,
            );
            ui.add_space(gap);
            render_reading_rhythm_panel(
                ui,
                total_width,
                None,
                library,
                theme,
                t,
                container_fill,
                border_stroke,
                is_cozy,
            );
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn render_continue_reading_panel(
    ui: &mut egui::Ui,
    panel_width: f32,
    target_outer_h: Option<f32>,
    library: &LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    lang: UiLanguage,
    continue_page: &mut usize,
    container_fill: Color32,
    border_stroke: Stroke,
    is_cozy: bool,
    cover_loader: &mut CoverLoader,
    open_path: &mut Option<PathBuf>,
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

            let in_progress_entries: Vec<&LibraryEntry> = library
                .sorted_entries()
                .into_iter()
                .filter(|e| e.status == LibraryReadingStatus::Reading)
                .collect();
            let in_progress_count = in_progress_entries.len();

            let pool: Vec<&LibraryEntry> = if !in_progress_entries.is_empty() {
                in_progress_entries
            } else {
                library.sorted_entries()
            };

            let per_page = if inner_w >= 600.0 {
                3usize
            } else if inner_w >= 380.0 {
                2usize
            } else {
                1usize
            };
            let total_pages = pool.len().div_ceil(per_page).max(1);
            if *continue_page >= total_pages {
                *continue_page = total_pages - 1;
            }
            let can_prev = *continue_page > 0;
            let can_next = *continue_page + 1 < total_pages;

            let (header_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 24.0), egui::Sense::hover());

            let title_font = egui::FontId::proportional(15.5);
            let title_color = if is_cozy {
                CozyPalette::ON_SURFACE
            } else {
                theme.text_color()
            };
            let title_galley = ui.painter().layout_no_wrap(
                t.library_continue_reading.to_string(),
                title_font,
                title_color,
            );
            let title_w = title_galley.size().x;
            let title_pos = Pos2::new(
                header_rect.min.x,
                header_rect.center().y - title_galley.size().y * 0.5,
            );
            ui.painter()
                .galley(title_pos, title_galley, Color32::TRANSPARENT);

            let mut nav_reserved_w = 54.0;
            let page_text = if total_pages > 1 {
                Some(format!("{}/{}", *continue_page + 1, total_pages))
            } else {
                None
            };
            let page_font = egui::FontId::proportional(10.5);
            let page_text_color = if is_cozy {
                CozyPalette::ON_SURFACE_VARIANT
            } else {
                theme.secondary_text_color()
            };
            let page_galley = page_text.as_ref().map(|txt| {
                ui.painter()
                    .layout_no_wrap(txt.clone(), page_font.clone(), page_text_color)
            });
            if let Some(ref pg) = page_galley {
                nav_reserved_w += pg.size().x + 8.0;
            }

            let badge_str = t
                .library_in_progress_badge
                .replace("{count}", &in_progress_count.to_string());
            let badge_font = egui::FontId::proportional(10.5);
            let badge_fg = if is_cozy {
                CozyPalette::PRIMARY
            } else {
                theme.accent_color()
            };
            let badge_bg = if is_cozy {
                CozyPalette::PRIMARY.gamma_multiply(0.12)
            } else {
                theme.accent_color().gamma_multiply(0.15)
            };
            let badge_galley = ui.painter().layout_no_wrap(badge_str, badge_font, badge_fg);
            let badge_w = badge_galley.size().x + 14.0;
            let badge_x = title_pos.x + title_w + 10.0;
            let badge_max_right = header_rect.max.x - nav_reserved_w - 6.0;
            if badge_x + badge_w <= badge_max_right {
                let badge_rect = Rect::from_min_size(
                    Pos2::new(badge_x, header_rect.center().y - 9.0),
                    Vec2::new(badge_w, 18.0),
                );
                ui.painter().rect_filled(badge_rect, 9.0, badge_bg);
                let text_pos = Pos2::new(
                    badge_rect.center().x - badge_galley.size().x * 0.5,
                    badge_rect.center().y - badge_galley.size().y * 0.5,
                );
                ui.painter().galley(text_pos, badge_galley, badge_fg);
            }

            if let Some(pg) = page_galley {
                let pg_pos = Pos2::new(
                    header_rect.max.x - 54.0 - pg.size().x - 6.0,
                    header_rect.center().y - pg.size().y * 0.5,
                );
                ui.painter().galley(pg_pos, pg, page_text_color);
            }

            let arr2_rect = Rect::from_min_size(
                Pos2::new(header_rect.max.x - 22.0, header_rect.center().y - 11.0),
                Vec2::splat(22.0),
            );
            let arr1_rect = Rect::from_min_size(
                Pos2::new(header_rect.max.x - 48.0, header_rect.center().y - 11.0),
                Vec2::splat(22.0),
            );
            let prev_sense = if can_prev {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let next_sense = if can_next {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let r1 = ui.interact(arr1_rect, ui.scope_id().with("in_prog_prev"), prev_sense);
            let r2 = ui.interact(arr2_rect, ui.scope_id().with("in_prog_next"), next_sense);
            let arrow_col = if is_cozy {
                CozyPalette::ON_SURFACE_VARIANT
            } else {
                theme.secondary_text_color()
            };
            let hover_fill = if is_cozy {
                CozyPalette::SURFACE_CONTAINER
            } else {
                theme.panel_bg_color().gamma_multiply(1.3)
            };

            if can_prev && r1.hovered() {
                ui.painter().rect_filled(arr1_rect, 5.0, hover_fill);
            }
            if can_next && r2.hovered() {
                ui.painter().rect_filled(arr2_rect, 5.0, hover_fill);
            }
            paint_icon(
                ui.painter(),
                Icon::ArrowLeft,
                Rect::from_center_size(arr1_rect.center(), Vec2::splat(12.0)),
                if can_prev {
                    arrow_col
                } else {
                    arrow_col.gamma_multiply(0.32)
                },
            );
            paint_icon(
                ui.painter(),
                Icon::ArrowRight,
                Rect::from_center_size(arr2_rect.center(), Vec2::splat(12.0)),
                if can_next {
                    arrow_col
                } else {
                    arrow_col.gamma_multiply(0.32)
                },
            );

            if r1.on_hover_text(t.prev_page).clicked() && can_prev {
                *continue_page = continue_page.saturating_sub(1);
            }
            if r2.on_hover_text(t.next_page).clicked() && can_next {
                *continue_page = (*continue_page + 1).min(total_pages - 1);
            }

            ui.add_space(10.0);

            if pool.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(36.0);
                    ui.label(
                        RichText::new(t.library_empty_desc)
                            .size(13.0)
                            .color(theme.secondary_text_color()),
                    );
                    ui.add_space(36.0);
                });
                return;
            }

            let start = (*continue_page * per_page).min(pool.len());
            let end = (start + per_page).min(pool.len());
            let display_entries = &pool[start..end];

            let slot_cols = pool.len().min(per_page).max(1);
            let card_gap = 10.0;
            let available_card_w = ((inner_w - (slot_cols - 1) as f32 * card_gap)
                / slot_cols as f32)
                .floor()
                .max(140.0);

            let card_h = (target_inner_h - 32.0).clamp(150.0, 195.0);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                for (idx, entry) in display_entries.iter().enumerate() {
                    if idx > 0 {
                        ui.add_space(card_gap);
                    }
                    cover_loader.queue(&entry.path);
                    ui.allocate_ui_with_layout(
                        Vec2::new(available_card_w, card_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            render_in_progress_card(
                                ui,
                                available_card_w,
                                card_h,
                                entry,
                                *continue_page == 0 && idx == 0,
                                theme,
                                t,
                                lang,
                                is_cozy,
                                cover_loader,
                                open_path,
                            );
                        },
                    );
                }
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn render_in_progress_card(
    ui: &mut egui::Ui,
    card_width: f32,
    card_height: f32,
    entry: &LibraryEntry,
    is_primary: bool,
    theme: &ReaderTheme,
    t: &Translations,
    lang: UiLanguage,
    is_cozy: bool,
    cover_loader: &mut CoverLoader,
    open_path: &mut Option<PathBuf>,
) {
    let card_fill = if is_cozy {
        CozyPalette::CARD_BG
    } else {
        theme.panel_bg_color().gamma_multiply(1.18)
    };
    let card_border = Stroke::new(
        1.0,
        if is_cozy {
            CozyPalette::OUTLINE.gamma_multiply(0.35)
        } else {
            theme.accent_color().gamma_multiply(0.25)
        },
    );

    egui::Frame::new()
        .fill(card_fill)
        .stroke(card_border)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            let inner_w = (card_width - 20.0).max(80.0);
            let inner_h = (card_height - 20.0).max(120.0);
            ui.set_width(inner_w);
            ui.set_height(inner_h);

            let cover_w = 52.0;
            let cover_h = 74.0;
            let info_w = (inner_w - cover_w - 10.0).max(60.0);

            ui.horizontal(|ui| {
                let (cover_rect, cover_resp) =
                    ui.allocate_exact_size(Vec2::new(cover_w, cover_h), egui::Sense::click());
                if cover_resp.clicked() {
                    *open_path = Some(entry.path.clone());
                }

                if let Some(tex) = cover_loader.texture(&entry.path) {
                    let ts = tex.size_vec2();
                    if ts.x > 0.0 && ts.y > 0.0 {
                        let available = cover_rect.shrink(1.0);
                        let card_aspect = available.width() / available.height();
                        let tex_aspect = ts.x / ts.y;
                        let (uv_min, uv_max) = if tex_aspect > card_aspect {
                            let crop_frac = card_aspect / tex_aspect;
                            let x_offset = (1.0 - crop_frac) * 0.5;
                            (Pos2::new(x_offset, 0.0), Pos2::new(1.0 - x_offset, 1.0))
                        } else {
                            let crop_frac = tex_aspect / card_aspect;
                            let y_offset = (1.0 - crop_frac) * 0.5;
                            (Pos2::new(0.0, y_offset), Pos2::new(1.0, 1.0 - y_offset))
                        };
                        ui.painter().image(
                            tex.id(),
                            available,
                            Rect::from_min_max(uv_min, uv_max),
                            Color32::WHITE,
                        );
                    }
                } else {
                    super::shelf_cover::paint_procedural_cover(
                        ui.painter(),
                        cover_rect,
                        &entry.title,
                        &entry.author,
                        &entry.path,
                        4.0,
                        t.extra.default_book_title,
                    );
                }

                let format_ext = crate::document::registry::format_badge_for_path(&entry.path);
                let badge_bg = match format_ext.as_str() {
                    "PDF" => CozyPalette::SECONDARY,
                    "FB2" | "CBZ" | "CBR" => CozyPalette::TERTIARY,
                    _ => CozyPalette::PRIMARY,
                };
                let badge_rect = Rect::from_min_size(
                    Pos2::new(cover_rect.min.x + 2.0, cover_rect.min.y + 2.0),
                    Vec2::new(24.0, 12.0),
                );
                ui.painter().rect_filled(badge_rect, 2.5, badge_bg);
                ui.painter().text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format_ext,
                    egui::FontId::proportional(8.0),
                    Color32::WHITE,
                );

                ui.add_space(8.0);

                ui.allocate_ui_with_layout(
                    Vec2::new(info_w, cover_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let fallback_title = entry
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(t.extra.default_document_title);
                        let display_title = if entry.title.trim().is_empty() {
                            fallback_title
                        } else {
                            entry.title.as_str()
                        };

                        let title_resp = ui.add(
                            egui::Label::new(
                                RichText::new(display_title).size(12.5).strong().color(
                                    if is_cozy {
                                        CozyPalette::ON_SURFACE
                                    } else {
                                        theme.text_color()
                                    },
                                ),
                            )
                            .truncate()
                            .sense(egui::Sense::click()),
                        );
                        if title_resp.clicked() {
                            *open_path = Some(entry.path.clone());
                        }

                        if !entry.author.trim().is_empty() {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(entry.author.as_str()).size(10.5).color(
                                        if is_cozy {
                                            CozyPalette::ON_SURFACE_VARIANT
                                        } else {
                                            theme.secondary_text_color()
                                        },
                                    ),
                                )
                                .truncate(),
                            );
                        }

                        let unit_num = (entry.current_unit + 1).to_string();
                        let is_pdf = entry
                            .path
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
                        let chapter_tpl = if is_pdf {
                            t.extra.section_short
                        } else {
                            t.extra.chapter_short
                        };
                        let chapter_str = crate::i18n::fill(chapter_tpl, &[("{n}", &unit_num)]);
                        ui.label(RichText::new(chapter_str).size(10.0).color(if is_cozy {
                            CozyPalette::PRIMARY
                        } else {
                            theme.accent_color()
                        }));

                        let now = crate::core::license::unix_now();
                        let time_str = crate::i18n::format::format_relative_time(
                            entry.last_opened_unix,
                            now,
                            lang,
                        );
                        ui.label(RichText::new(time_str).size(9.5).color(if is_cozy {
                            CozyPalette::ON_SURFACE_VARIANT.gamma_multiply(0.7)
                        } else {
                            theme.secondary_text_color()
                        }));
                    },
                );
            });

            let footer_h = 58.0;
            let remaining_h = (inner_h - cover_h - footer_h).max(4.0);
            ui.add_space(remaining_h);

            let (sep_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 1.0), egui::Sense::hover());
            ui.painter().rect_filled(
                sep_rect,
                0.0,
                if is_cozy {
                    CozyPalette::OUTLINE.gamma_multiply(0.2)
                } else {
                    theme.accent_color().gamma_multiply(0.15)
                },
            );

            ui.add_space(5.0);

            let percent = entry.progress_percent().round() as u32;
            let fraction = percent as f32 / 100.0;

            let (prog_text_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 14.0), egui::Sense::hover());
            ui.painter().text(
                prog_text_rect.left_center(),
                egui::Align2::LEFT_CENTER,
                t.library_progress_label,
                egui::FontId::proportional(10.0),
                if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                },
            );
            ui.painter().text(
                prog_text_rect.right_center(),
                egui::Align2::RIGHT_CENTER,
                format!("{percent}%"),
                egui::FontId::proportional(10.0),
                if is_cozy {
                    CozyPalette::ON_SURFACE
                } else {
                    theme.text_color()
                },
            );

            ui.add_space(2.0);
            let bar_height = 5.0;
            let (bar_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, bar_height), egui::Sense::hover());
            ui.painter().rect_filled(
                bar_rect,
                2.5,
                if is_cozy {
                    CozyPalette::SURFACE_CONTAINER
                } else {
                    theme.panel_bg_color().gamma_multiply(1.4)
                },
            );
            if fraction > 0.0 {
                let fill_w = bar_rect.width() * fraction;
                let active_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_height));
                ui.painter().rect_filled(
                    active_rect,
                    2.5,
                    if is_cozy {
                        CozyPalette::PRIMARY
                    } else {
                        theme.accent_color()
                    },
                );
            }

            ui.add_space(6.0);

            let (read_btn_rect, read_btn_resp) =
                ui.allocate_exact_size(Vec2::new(inner_w, 27.0), egui::Sense::click());
            let is_h = read_btn_resp.hovered();

            let (btn_bg, text_color) = if is_primary {
                let bg = if is_h {
                    if is_cozy {
                        CozyPalette::PRIMARY_CONTAINER
                    } else {
                        theme.accent_color().gamma_multiply(0.85)
                    }
                } else if is_cozy {
                    CozyPalette::PRIMARY
                } else {
                    theme.accent_color()
                };
                (bg, Color32::WHITE)
            } else {
                let bg = if is_h {
                    if is_cozy {
                        CozyPalette::SURFACE_HIGH
                    } else {
                        theme.panel_bg_color().gamma_multiply(1.35)
                    }
                } else if is_cozy {
                    CozyPalette::SURFACE_CONTAINER
                } else {
                    theme.panel_bg_color().gamma_multiply(1.22)
                };
                let fg = if is_cozy {
                    CozyPalette::PRIMARY
                } else {
                    theme.accent_color()
                };
                (bg, fg)
            };

            ui.painter().rect_filled(read_btn_rect, 6.0, btn_bg);

            let btn_font = egui::FontId::proportional(11.5);
            let read_galley = ui.painter().layout_no_wrap(
                t.library_read_action.to_string(),
                btn_font,
                text_color,
            );
            let tri_w = 8.0;
            let tri_gap = 6.0;
            let content_w = tri_w + tri_gap + read_galley.size().x;
            let start_x = read_btn_rect.center().x - content_w * 0.5;
            let tri_cx = start_x + 3.5;
            let tri_cy = read_btn_rect.center().y;
            let play_pts = [
                Pos2::new(tri_cx - 3.5, tri_cy - 4.2),
                Pos2::new(tri_cx + 4.5, tri_cy),
                Pos2::new(tri_cx - 3.5, tri_cy + 4.2),
            ];
            ui.painter().add(egui::Shape::convex_polygon(
                play_pts.to_vec(),
                text_color,
                Stroke::NONE,
            ));
            ui.painter().galley(
                Pos2::new(
                    start_x + tri_w + tri_gap,
                    tri_cy - read_galley.size().y * 0.5,
                ),
                read_galley,
                text_color,
            );

            if read_btn_resp.clicked() {
                *open_path = Some(entry.path.clone());
            }
        });
}
