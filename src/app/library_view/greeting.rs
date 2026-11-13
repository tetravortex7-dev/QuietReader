use super::LibraryActions;
use crate::app::icons::{Icon, IconButton, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::LibraryStore;
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};

pub fn render_greeting(
    ui: &mut egui::Ui,
    library: &LibraryStore,
    theme: &ReaderTheme,
    t: &Translations,
    actions: &mut LibraryActions,
) {
    let is_cozy = !theme.is_dark();
    let border_color = if is_cozy {
        CozyPalette::OUTLINE.gamma_multiply(0.4)
    } else {
        theme.accent_color().gamma_multiply(0.2)
    };

    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let dot_color = if is_cozy {
                            CozyPalette::PRIMARY
                        } else {
                            theme.accent_color()
                        };
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 3.5, dot_color);
                        ui.label(
                            RichText::new(t.library_cozy_label)
                                .size(11.0)
                                .strong()
                                .color(dot_color),
                        );
                    });

                    ui.add_space(2.0);

                    let greeting_text = {
                        let now_secs = crate::core::license::unix_now();
                        let hour = ((now_secs % 86400) / 3600 + 3) % 24;
                        if (5..12).contains(&hour) {
                            t.library_greeting_morning
                        } else if (12..18).contains(&hour) {
                            t.library_greeting_afternoon
                        } else {
                            t.library_greeting_evening
                        }
                    };

                    ui.label(RichText::new(greeting_text).size(20.0).color(if is_cozy {
                        CozyPalette::ON_SURFACE
                    } else {
                        theme.text_color()
                    }));

                    ui.add_space(2.0);

                    let count_str = library.entries.len().to_string();
                    let count_text = t
                        .extra
                        .library_collection_count
                        .replace("{count}", &count_str);
                    let sub_color = if is_cozy {
                        CozyPalette::ON_SURFACE_VARIANT
                    } else {
                        theme.secondary_text_color()
                    };
                    crate::app::icons::dotted_responsive_row(
                        ui,
                        &[t.library_collection_sub, &count_text],
                        11.5,
                        sub_color,
                        sub_color.gamma_multiply(0.7),
                    );
                });

                let remaining = ui.available_width();
                let buttons_w = 480.0;
                if remaining > buttons_w {
                    ui.add_space(remaining - buttons_w);
                }

                let btn_height = 32.0;

                let primary_fill = if is_cozy {
                    CozyPalette::PRIMARY
                } else {
                    theme.accent_color()
                };
                let (btn_rect, btn_resp) =
                    ui.allocate_exact_size(Vec2::new(120.0, btn_height), egui::Sense::click());
                let is_hovered = btn_resp.hovered();
                let fill = if is_hovered {
                    primary_fill.gamma_multiply(0.85)
                } else {
                    primary_fill
                };
                ui.painter().rect_filled(btn_rect, 8.0, fill);
                let icon_rect = Rect::from_center_size(
                    Pos2::new(btn_rect.min.x + 18.0, btn_rect.center().y),
                    Vec2::splat(14.0),
                );
                paint_icon(ui.painter(), Icon::OpenFile, icon_rect, Color32::WHITE);
                ui.painter().text(
                    Pos2::new(btn_rect.min.x + 32.0, btn_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    t.open_file,
                    egui::FontId::proportional(12.0),
                    Color32::WHITE,
                );
                if btn_resp.clicked() {
                    actions.open_file = true;
                }

                ui.add_space(4.0);

                let folder_resp = ui.add(
                    IconButton::new(Icon::Folder, t.open_folder)
                        .font_size(12.0)
                        .min_size(Vec2::new(0.0, btn_height)),
                );
                if folder_resp.clicked() {
                    actions.open_folder = true;
                }

                ui.add_space(4.0);

                let opds_resp = ui.add(
                    IconButton::new(Icon::Globe, t.opds_catalogs)
                        .font_size(12.0)
                        .min_size(Vec2::new(0.0, btn_height)),
                );
                if opds_resp.clicked() {
                    actions.open_file = false;
                }

                ui.separator();

                if ui
                    .add(IconButton::new(Icon::Download, t.library_export_all).font_size(12.0))
                    .on_hover_text(t.library_export_all_hint)
                    .clicked()
                {
                    actions.export_all = true;
                }

                ui.add_space(4.0);

                if ui
                    .add(IconButton::new(Icon::Refresh, t.library_restore_snapshot).font_size(12.0))
                    .on_hover_text(t.library_restore_snapshot_hint)
                    .clicked()
                {
                    actions.restore_snapshot = true;
                }
            });

            ui.add_space(4.0);
            let sep_stroke = Stroke::new(1.0, border_color);
            let y = ui.cursor().top();
            let min_x = ui.min_rect().min.x;
            let max_x = ui.max_rect().max.x;
            ui.painter()
                .line_segment([Pos2::new(min_x, y), Pos2::new(max_x, y)], sep_stroke);
            ui.add_space(8.0);
        },
    );
}
