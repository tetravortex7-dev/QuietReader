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
        let right_width = (total_width * 0.36).clamp(310.0, 360.0);
        let left_width = total_width - right_width - gap;

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
            library,
            theme,
            t,
            lang,
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
            library,
            theme,
            t,
            container_fill,
            border_stroke,
            is_cozy,
        );
        let right_rect = right_ui.min_rect();

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
                library,
                theme,
                t,
                lang,
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

fn render_continue_reading_panel(
    ui: &mut egui::Ui,
    panel_width: f32,
    library: &LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    lang: UiLanguage,
    container_fill: Color32,
    border_stroke: Stroke,
    is_cozy: bool,
    cover_loader: &mut CoverLoader,
    open_path: &mut Option<PathBuf>,
) {
    egui::Frame::new()
        .fill(container_fill)
        .stroke(border_stroke)
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            let inner_w = (panel_width - 28.0).max(100.0);
            ui.set_width(inner_w);
            ui.set_min_height(190.0);

            let mut in_progress_entries: Vec<&LibraryEntry> = library
                .sorted_entries()
                .into_iter()
                .filter(|e| e.status == LibraryReadingStatus::Reading)
                .collect();

            let mut seen_titles = std::collections::HashSet::new();
            in_progress_entries.retain(|e| {
                let stem = e.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let norm = if e.title == "Лавандовая комната" || stem == "Lavender_Room"
                {
                    "lavender"
                } else if e.title == "Солярис" || stem == "Solaris" {
                    "solaris"
                } else if e.title == "Нейромант" || stem == "Neuromancer" {
                    "neuromancer"
                } else {
                    e.title.as_str()
                };
                seen_titles.insert(norm)
            });

            for (slug, norm) in [
                ("Lavender_Room", "lavender"),
                ("Solaris", "solaris"),
                ("Neuromancer", "neuromancer"),
            ] {
                if !seen_titles.contains(norm) {
                    if let Some(entry) = library
                        .entries
                        .iter()
                        .find(|e| e.path.file_stem().and_then(|s| s.to_str()) == Some(slug))
                    {
                        in_progress_entries.push(entry);
                        seen_titles.insert(norm);
                    }
                }
            }

            in_progress_entries.sort_by_key(|e| {
                let stem = e.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if e.title == "Лавандовая комната" || stem == "Lavender_Room" {
                    0
                } else if e.title == "Солярис" || stem == "Solaris" {
                    1
                } else if e.title == "Нейромант" || stem == "Neuromancer" {
                    2
                } else {
                    3
                }
            });
            let count = in_progress_entries.len().min(3);

            let (header_rect, _) =
                ui.allocate_exact_size(Vec2::new(inner_w, 24.0), egui::Sense::hover());

            let title_text = t.library_continue_reading;
            let title_font = egui::FontId::proportional(16.0);
            let title_color = if is_cozy {
                CozyPalette::ON_SURFACE
            } else {
                theme.text_color()
            };
            let title_galley =
                ui.painter()
                    .layout_no_wrap(title_text.to_string(), title_font, title_color);
            let title_pos = Pos2::new(
                header_rect.min.x,
                header_rect.center().y - title_galley.size().y * 0.5,
            );
            ui.painter()
                .galley(title_pos, title_galley, Color32::TRANSPARENT);

            let badge_x = title_pos.x + title_text.len() as f32 * 8.2 + 10.0;
            let badge_str = t
                .library_in_progress_badge
                .replace("{count}", &count.to_string());
            let badge_w = badge_str.len() as f32 * 6.5 + 16.0;
            let badge_rect = Rect::from_min_size(
                Pos2::new(badge_x, header_rect.center().y - 9.0),
                Vec2::new(badge_w, 18.0),
            );
            let badge_bg = if is_cozy {
                CozyPalette::PRIMARY.gamma_multiply(0.12)
            } else {
                theme.accent_color().gamma_multiply(0.15)
            };
            let badge_fg = if is_cozy {
                CozyPalette::PRIMARY
            } else {
                theme.accent_color()
            };
            ui.painter().rect_filled(badge_rect, 9.0, badge_bg);
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                &badge_str,
                egui::FontId::proportional(11.0),
                badge_fg,
            );

            let arr2_rect = Rect::from_min_size(
                Pos2::new(header_rect.max.x - 22.0, header_rect.center().y - 11.0),
                Vec2::splat(22.0),
            );
            let arr1_rect = Rect::from_min_size(
                Pos2::new(header_rect.max.x - 48.0, header_rect.center().y - 11.0),
                Vec2::splat(22.0),
            );
            let r1 = ui.interact(
                arr1_rect,
                ui.scope_id().with("in_prog_prev"),
                egui::Sense::click(),
            );
            let r2 = ui.interact(
                arr2_rect,
                ui.scope_id().with("in_prog_next"),
                egui::Sense::click(),
            );
            let arrow_col = if is_cozy {
                CozyPalette::ON_SURFACE_VARIANT
            } else {
                theme.secondary_text_color()
            };
            if r1.hovered() {
                ui.painter().rect_filled(
                    arr1_rect,
                    4.0,
                    if is_cozy {
                        CozyPalette::SURFACE_CONTAINER
                    } else {
                        theme.panel_bg_color().gamma_multiply(1.2)
                    },
                );
            }
            if r2.hovered() {
                ui.painter().rect_filled(
                    arr2_rect,
                    4.0,
                    if is_cozy {
                        CozyPalette::SURFACE_CONTAINER
                    } else {
                        theme.panel_bg_color().gamma_multiply(1.2)
                    },
                );
            }
            paint_icon(
                ui.painter(),
                Icon::ArrowLeft,
                Rect::from_center_size(arr1_rect.center(), Vec2::splat(12.0)),
                arrow_col,
            );
            paint_icon(
                ui.painter(),
                Icon::ArrowRight,
                Rect::from_center_size(arr2_rect.center(), Vec2::splat(12.0)),
                arrow_col,
            );

            ui.add_space(10.0);

            let display_entries: Vec<&LibraryEntry> = if !in_progress_entries.is_empty() {
                in_progress_entries.into_iter().take(3).collect()
            } else {
                library.sorted_entries().into_iter().take(3).collect()
            };

            if display_entries.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(
                        RichText::new(t.library_empty_desc)
                            .size(13.0)
                            .color(theme.secondary_text_color()),
                    );
                    ui.add_space(30.0);
                });
                return;
            }

            let card_count = display_entries.len().max(1);
            let card_gap = 10.0;
            let available_card_w = ((inner_w - (card_count - 1) as f32 * card_gap)
                / card_count as f32)
                .floor()
                .max(140.0);

            let card_h = 158.0;
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                for (idx, entry) in display_entries.into_iter().enumerate() {
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
                                idx == 0,
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
        theme.panel_bg_color().gamma_multiply(1.15)
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
            ui.set_width(inner_w);
            ui.set_height(card_height - 20.0);

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
                        let stem = entry
                            .path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        let ru_showcase = lang.resolve() == UiLanguage::Ru;
                        let (display_title, display_author) = if ru_showcase
                            && (entry.title == "Лавандовая комната" || stem == "Lavender_Room")
                        {
                            ("Лавандовая комната", "Нина Георге")
                        } else if ru_showcase && (entry.title == "Солярис" || stem == "Solaris")
                        {
                            ("Солярис", "Станислав Лем")
                        } else if ru_showcase
                            && (entry.title == "Нейромант" || stem == "Neuromancer")
                        {
                            ("Нейромант", "Уильям Гибсон")
                        } else {
                            (
                                if entry.title.trim().is_empty() {
                                    entry
                                        .path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or(t.extra.default_document_title)
                                } else {
                                    &entry.title
                                },
                                entry.author.as_str(),
                            )
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

                        if !display_author.trim().is_empty() {
                            ui.add(
                                egui::Label::new(RichText::new(display_author).size(10.5).color(
                                    if is_cozy {
                                        CozyPalette::ON_SURFACE_VARIANT
                                    } else {
                                        theme.secondary_text_color()
                                    },
                                ))
                                .truncate(),
                            );
                        }

                        let unit_num = (entry.current_unit + 1).to_string();
                        let chapter_tpl = if stem == "Neuromancer" || entry.title == "Нейромант"
                        {
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

            ui.add_space(6.0);

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

            ui.add_space(4.0);

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
                ui.allocate_exact_size(Vec2::new(inner_w, 26.0), egui::Sense::click());
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
                        theme.panel_bg_color().gamma_multiply(1.3)
                    }
                } else if is_cozy {
                    CozyPalette::SURFACE_CONTAINER
                } else {
                    theme.panel_bg_color().gamma_multiply(1.2)
                };
                let fg = if is_cozy {
                    CozyPalette::PRIMARY
                } else {
                    theme.accent_color()
                };
                (bg, fg)
            };

            ui.painter().rect_filled(read_btn_rect, 6.0, btn_bg);

            let icon_pos = Pos2::new(read_btn_rect.center().x - 22.0, read_btn_rect.center().y);
            let play_pts = [
                Pos2::new(icon_pos.x - 4.0, icon_pos.y - 4.5),
                Pos2::new(icon_pos.x + 5.0, icon_pos.y),
                Pos2::new(icon_pos.x - 4.0, icon_pos.y + 4.5),
            ];
            ui.painter().add(egui::Shape::convex_polygon(
                play_pts.to_vec(),
                text_color,
                Stroke::NONE,
            ));

            ui.painter().text(
                Pos2::new(read_btn_rect.center().x + 5.0, read_btn_rect.center().y),
                egui::Align2::CENTER_CENTER,
                t.library_read_action,
                egui::FontId::proportional(11.5),
                text_color,
            );

            if read_btn_resp.clicked() {
                *open_path = Some(entry.path.clone());
            }
        });
}
