use super::LibraryActions;
use crate::app::icons::{Icon, IconButton, paint_icon};
use crate::app::theme::{CozyPalette, ReaderTheme};
use crate::core::library::LibraryStore;
use crate::i18n::Translations;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, Vec2};

fn measure_button_w(ui: &mut egui::Ui, text: &str, font_size: f32) -> f32 {
    let font_id = egui::FontId::proportional(font_size);
    let text_w = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font_id, Color32::WHITE)
            .size()
            .x
    });
    let padding_x = ui.spacing().button_padding.x * 2.0;
    let gap = if text.is_empty() { 0.0 } else { 6.0 };
    padding_x + font_size + gap + text_w
}

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
    let dot_color = if is_cozy {
        CozyPalette::PRIMARY
    } else {
        theme.accent_color()
    };
    let title_color = if is_cozy {
        CozyPalette::ON_SURFACE
    } else {
        theme.text_color()
    };
    let sub_color = if is_cozy {
        CozyPalette::ON_SURFACE_VARIANT
    } else {
        theme.secondary_text_color()
    };
    let primary_fill = if is_cozy {
        CozyPalette::PRIMARY
    } else {
        theme.accent_color()
    };

    let greeting_text = {
        let hour = crate::platform::local_hour();
        if (5..12).contains(&hour) {
            t.library_greeting_morning
        } else if (12..18).contains(&hour) {
            t.library_greeting_afternoon
        } else {
            t.library_greeting_evening
        }
    };

    let count_str = library.entries.len().to_string();
    let count_text = t
        .extra
        .library_collection_count
        .replace("{count}", &count_str);

    let avail_w = ui.available_width();
    let btn_font = 12.0;
    let btn_height = 30.0;

    let open_pad = 12.0;
    let open_text_w = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            t.open_file.to_string(),
            egui::FontId::proportional(btn_font),
            Color32::WHITE,
        )
        .size()
        .x
    });
    let primary_w = (open_pad + 14.0 + 7.0 + open_text_w + open_pad).max(100.0);
    let folder_w = measure_button_w(ui, t.open_folder, btn_font);
    let opds_w = measure_button_w(ui, t.opds_catalogs, btn_font);
    let export_w = measure_button_w(ui, t.library_export_all, btn_font);
    let restore_w = measure_button_w(ui, t.library_restore_snapshot, btn_font);
    let item_gap = ui.spacing().item_spacing.x;
    let buttons_total_w =
        primary_w + folder_w + opds_w + export_w + restore_w + 8.0 + item_gap * 5.0 + 16.0;

    let greeting_text_w = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            greeting_text.to_string(),
            egui::FontId::proportional(19.0),
            title_color,
        )
        .size()
        .x
    });
    let two_col =
        avail_w >= 900.0 && (greeting_text_w.max(300.0) + buttons_total_w + 16.0) <= avail_w;

    let render_left = |ui: &mut egui::Ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 3.5, dot_color);
                ui.label(
                    RichText::new(t.library_cozy_label)
                        .size(11.0)
                        .strong()
                        .color(dot_color),
                );
            });
            ui.add_space(1.0);
            ui.label(RichText::new(greeting_text).size(19.0).color(title_color));
            ui.add_space(1.0);
            crate::app::icons::dotted_responsive_row(
                ui,
                &[t.library_collection_sub, &count_text],
                11.5,
                sub_color,
                sub_color.gamma_multiply(0.7),
            );
        });
    };

    let container_resp = ui.allocate_ui_with_layout(
        Vec2::new(avail_w, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            if two_col {
                ui.horizontal(|ui| {
                    render_left(ui);
                    let remaining = ui.available_width() - buttons_total_w;
                    if remaining > 0.0 {
                        ui.add_space(remaining);
                    }
                    ui.vertical(|ui| {
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            render_greeting_buttons(
                                ui,
                                primary_w,
                                btn_height,
                                open_pad,
                                btn_font,
                                primary_fill,
                                t,
                                actions,
                            );
                        });
                    });
                });
            } else {
                render_left(ui);
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    render_greeting_buttons(
                        ui,
                        primary_w,
                        btn_height,
                        open_pad,
                        btn_font,
                        primary_fill,
                        t,
                        actions,
                    );
                });
            }

            ui.add_space(4.0);
        },
    );

    let y = container_resp.response.rect.bottom();
    let sep_stroke = Stroke::new(1.0, border_color);
    ui.painter().line_segment(
        [
            Pos2::new(container_resp.response.rect.left(), y),
            Pos2::new(container_resp.response.rect.right(), y),
        ],
        sep_stroke,
    );
    ui.add_space(8.0);
}

#[allow(clippy::too_many_arguments)]
fn render_greeting_buttons(
    ui: &mut egui::Ui,
    primary_w: f32,
    btn_height: f32,
    open_pad: f32,
    btn_font: f32,
    primary_fill: Color32,
    t: &Translations,
    actions: &mut LibraryActions,
) {
    let (btn_rect, btn_resp) =
        ui.allocate_exact_size(Vec2::new(primary_w, btn_height), egui::Sense::click());
    let fill = if btn_resp.hovered() {
        primary_fill.gamma_multiply(0.86)
    } else {
        primary_fill
    };
    ui.painter().rect_filled(btn_rect, 8.0, fill);
    let icon_rect = Rect::from_center_size(
        Pos2::new(btn_rect.min.x + open_pad + 7.0, btn_rect.center().y),
        Vec2::splat(14.0),
    );
    paint_icon(ui.painter(), Icon::OpenFile, icon_rect, Color32::WHITE);
    ui.painter().text(
        Pos2::new(btn_rect.min.x + open_pad + 21.0, btn_rect.center().y),
        egui::Align2::LEFT_CENTER,
        t.open_file,
        egui::FontId::proportional(btn_font),
        Color32::WHITE,
    );
    if btn_resp.on_hover_text(t.open_file_tooltip).clicked() {
        actions.open_file = true;
    }

    ui.add_space(2.0);
    if ui
        .add(
            IconButton::new(Icon::Folder, t.open_folder)
                .font_size(btn_font)
                .min_size(Vec2::new(0.0, btn_height)),
        )
        .on_hover_text(t.open_folder_tooltip)
        .clicked()
    {
        actions.open_folder = true;
    }

    ui.add_space(2.0);
    if ui
        .add(
            IconButton::new(Icon::Globe, t.opds_catalogs)
                .font_size(btn_font)
                .min_size(Vec2::new(0.0, btn_height)),
        )
        .on_hover_text(t.opds_catalogs_tooltip)
        .clicked()
    {
        actions.open_opds = true;
    }

    ui.separator();

    if ui
        .add(
            IconButton::new(Icon::Download, t.library_export_all)
                .font_size(btn_font)
                .min_size(Vec2::new(0.0, btn_height)),
        )
        .on_hover_text(t.library_export_all_hint)
        .clicked()
    {
        actions.export_all = true;
    }

    ui.add_space(2.0);
    if ui
        .add(
            IconButton::new(Icon::Refresh, t.library_restore_snapshot)
                .font_size(btn_font)
                .min_size(Vec2::new(0.0, btn_height)),
        )
        .on_hover_text(t.library_restore_snapshot_hint)
        .clicked()
    {
        actions.restore_snapshot = true;
    }
}
