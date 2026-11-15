use super::CoverLoader;
use super::shelf_cover::shelf_cover;
use crate::app::icons::{Icon, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme, contrast_fg};
use crate::core::library::{LibraryEntry, LibraryReadingStatus};
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(crate) struct ShelfCardLayout {
    pub inner_w: f32,
    pub cover_h: f32,
    pub inner_h: f32,
    pub title_size: f32,
    pub secondary_size: f32,
    pub status_size: f32,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_shelf_card(
    ui: &mut egui::Ui,
    outer_card_rect: Rect,
    entry: &LibraryEntry,
    layout: ShelfCardLayout,
    theme: &ReaderTheme,
    t: &Translations,
    cover_loader: &mut CoverLoader,
    open_path: &mut Option<PathBuf>,
    remove_path: &mut Option<PathBuf>,
    status_change: &mut Option<(PathBuf, LibraryReadingStatus)>,
) {
    let is_cozy = !theme.is_dark();
    let file_name = entry
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let display_title = if entry.title.trim().is_empty() {
        file_name.as_str()
    } else {
        entry.title.as_str()
    };

    let cover_response = shelf_cover(
        ui,
        &entry.path,
        display_title,
        &entry.author,
        Vec2::new(layout.inner_w, layout.cover_h),
        8.0,
        theme,
        cover_loader,
        t.extra.default_book_title,
    );
    let cover_rect = cover_response.rect;

    let mut action_consumed = false;
    let card_hovered = ui.rect_contains_pointer(outer_card_rect);
    let (status_icon, status_label, next_status) = if entry.status == LibraryReadingStatus::Finished
    {
        (
            Icon::Book,
            t.library_mark_reading,
            LibraryReadingStatus::Reading,
        )
    } else {
        (
            Icon::Check,
            t.library_mark_finished,
            LibraryReadingStatus::Finished,
        )
    };

    if card_hovered {
        let pill_bg = Color32::from_rgba_unmultiplied(24, 20, 18, 195);
        let mark_rect = Rect::from_min_size(
            Pos2::new(cover_rect.min.x + 6.0, cover_rect.min.y + 6.0),
            Vec2::splat(22.0),
        );
        let trash_rect = Rect::from_min_size(
            Pos2::new(mark_rect.max.x + 4.0, cover_rect.min.y + 6.0),
            Vec2::splat(22.0),
        );

        let mark_resp = ui.interact(
            mark_rect,
            ui.scope_id().with(("card_status", &entry.path)),
            egui::Sense::click(),
        );
        let trash_resp = ui.interact(
            trash_rect,
            ui.scope_id().with(("card_trash", &entry.path)),
            egui::Sense::click(),
        );

        ui.painter().rect_filled(
            mark_rect,
            5.0,
            if mark_resp.hovered() {
                theme.accent_color()
            } else {
                pill_bg
            },
        );
        paint_icon(
            ui.painter(),
            status_icon,
            Rect::from_center_size(mark_rect.center(), Vec2::splat(12.0)),
            Color32::WHITE,
        );

        ui.painter().rect_filled(
            trash_rect,
            5.0,
            if trash_resp.hovered() {
                Color32::from_rgb(190, 60, 50)
            } else {
                pill_bg
            },
        );
        paint_icon(
            ui.painter(),
            Icon::Trash,
            Rect::from_center_size(trash_rect.center(), Vec2::splat(12.0)),
            Color32::WHITE,
        );

        if mark_resp.hovered() || trash_resp.hovered() {
            action_consumed = true;
        }
        if mark_resp.on_hover_text(status_label).clicked() {
            *status_change = Some((entry.path.clone(), next_status));
            action_consumed = true;
        }
        if trash_resp.on_hover_text(t.library_remove).clicked() {
            *remove_path = Some(entry.path.clone());
            action_consumed = true;
        }
    }

    if !action_consumed && cover_response.clicked() {
        *open_path = Some(entry.path.clone());
    }

    cover_response.context_menu(|ui| {
        if ui.button(t.library_open_book).clicked() {
            *open_path = Some(entry.path.clone());
            ui.close();
        }
        if ui.button(status_label).clicked() {
            *status_change = Some((entry.path.clone(), next_status));
            ui.close();
        }
        if ui.button(t.library_remove).clicked() {
            *remove_path = Some(entry.path.clone());
            ui.close();
        }
    });

    ui.add_space(7.0);
    let title_response = ui.add(
        egui::Label::new(
            RichText::new(display_title)
                .strong()
                .size(layout.title_size)
                .color(if is_cozy {
                    CozyPalette::ON_SURFACE
                } else {
                    theme.text_color()
                }),
        )
        .sense(egui::Sense::click())
        .truncate(),
    );
    if title_response.clicked() {
        *open_path = Some(entry.path.clone());
    }
    title_response.context_menu(|ui| {
        if ui.button(t.library_open_book).clicked() {
            *open_path = Some(entry.path.clone());
            ui.close();
        }
        if ui.button(status_label).clicked() {
            *status_change = Some((entry.path.clone(), next_status));
            ui.close();
        }
        if ui.button(t.library_remove).clicked() {
            *remove_path = Some(entry.path.clone());
            ui.close();
        }
    });

    let secondary = if entry.author.trim().is_empty() {
        file_name
    } else {
        entry.author.clone()
    };
    ui.add(
        egui::Label::new(
            RichText::new(secondary)
                .size(layout.secondary_size)
                .color(if is_cozy {
                    CozyPalette::ON_SURFACE_VARIANT
                } else {
                    theme.secondary_text_color()
                }),
        )
        .truncate(),
    );

    let footer_h = 56.0;
    let filler_h = (layout.inner_h - layout.cover_h - footer_h - 46.0).max(6.0);
    ui.add_space(filler_h);

    let percent = entry.progress_percent().round() as u32;
    let is_finished = entry.status == LibraryReadingStatus::Finished || percent == 100;
    let fraction = percent as f32 / 100.0;
    let status_color = if is_finished {
        if is_cozy {
            CozyPalette::TERTIARY
        } else {
            Color32::from_rgb(72, 187, 120)
        }
    } else if is_cozy {
        CozyPalette::PRIMARY
    } else {
        theme.accent_color()
    };

    let (prog_r, _) = ui.allocate_exact_size(Vec2::new(layout.inner_w, 14.0), egui::Sense::hover());
    let label = if is_finished {
        t.library_status_finished
    } else {
        t.library_progress_label
    };
    let font_id = egui::FontId::proportional(layout.status_size);
    ui.painter().text(
        prog_r.left_center(),
        egui::Align2::LEFT_CENTER,
        label,
        font_id.clone(),
        if is_cozy {
            CozyPalette::ON_SURFACE_VARIANT
        } else {
            theme.secondary_text_color()
        },
    );
    ui.painter().text(
        prog_r.right_center(),
        egui::Align2::RIGHT_CENTER,
        format!("{percent}%"),
        font_id,
        status_color,
    );

    ui.add_space(3.0);
    let bar_h = 4.0;
    let (bar_rect, _) =
        ui.allocate_exact_size(Vec2::new(layout.inner_w, bar_h), egui::Sense::hover());
    ui.painter().rect_filled(
        bar_rect,
        2.0,
        if is_cozy {
            CozyPalette::SURFACE_CONTAINER
        } else {
            theme.panel_bg_color().gamma_multiply(1.35)
        },
    );
    if fraction > 0.0 {
        let fill_w = bar_rect.width() * fraction;
        ui.painter().rect_filled(
            Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_h)),
            2.0,
            status_color,
        );
    }

    ui.add_space(7.0);
    let (read_btn_rect, read_btn_resp) =
        ui.allocate_exact_size(Vec2::new(layout.inner_w, 26.0), egui::Sense::click());
    let is_h = read_btn_resp.hovered();

    let is_primary_btn = !is_finished && entry.status == LibraryReadingStatus::Reading;
    let btn_fill = if is_primary_btn {
        if is_h {
            if is_cozy {
                CozyPalette::PRIMARY_CONTAINER
            } else {
                theme.accent_color().gamma_multiply(0.86)
            }
        } else if is_cozy {
            CozyPalette::PRIMARY
        } else {
            theme.accent_color()
        }
    } else if is_h {
        if is_cozy {
            CozyPalette::SURFACE_HIGH
        } else {
            theme.panel_bg_color().gamma_multiply(1.6)
        }
    } else if is_cozy {
        CozyPalette::SURFACE_CONTAINER
    } else {
        theme.panel_bg_color().gamma_multiply(1.38)
    };

    let btn_text_color = if is_primary_btn {
        contrast_fg(btn_fill)
    } else if is_cozy {
        CozyPalette::ON_SURFACE
    } else {
        theme.text_color()
    };

    ui.painter().rect_filled(read_btn_rect, 6.0, btn_fill);
    if !is_primary_btn && !is_cozy {
        ui.painter().rect_stroke(
            read_btn_rect,
            6.0,
            Stroke::new(1.0, theme.accent_color().gamma_multiply(0.22)),
            egui::StrokeKind::Inside,
        );
    }

    let read_font = egui::FontId::proportional(11.5);
    let read_galley =
        ui.painter()
            .layout_no_wrap(t.library_read_action.to_string(), read_font, btn_text_color);
    let icon_size = 11.0;
    let gap = 5.0;
    let content_w = icon_size + gap + read_galley.size().x;
    let start_x = read_btn_rect.center().x - content_w * 0.5;
    paint_icon(
        ui.painter(),
        Icon::Play,
        Rect::from_center_size(
            Pos2::new(start_x + icon_size * 0.5, read_btn_rect.center().y),
            Vec2::splat(icon_size),
        ),
        btn_text_color,
    );
    ui.painter().galley(
        Pos2::new(
            start_x + icon_size + gap,
            read_btn_rect.center().y - read_galley.size().y * 0.5,
        ),
        read_galley,
        btn_text_color,
    );

    if read_btn_resp.clicked() {
        *open_path = Some(entry.path.clone());
    }
}
