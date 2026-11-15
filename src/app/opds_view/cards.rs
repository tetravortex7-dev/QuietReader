use super::OpdsBrowserState;
use super::palette::{OpdsPalette, pill_badge, pill_button};
use crate::app::icons::{Icon, IconLabel, paint_dot_separator, paint_icon};
use crate::app::library_view::shelf_cover::paint_procedural_cover;
use crate::i18n::{Translations, fill};
use crate::opds::{DownloadStatus, OpdsAcquisitionLink, OpdsAuth, OpdsEntry};
use crate::settings::AppSettings;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use std::path::{Path, PathBuf};

impl OpdsBrowserState {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_book_card(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        entry: &OpdsEntry,
        catalog_name: &str,
        settings: &AppSettings,
        palette: &OpdsPalette,
        card_width: f32,
        on_open_file: &mut Option<PathBuf>,
    ) {
        let t = settings.ui_language.t();
        let compact = card_width < 430.0;
        let cover_w = if compact { 82.0 } else { 104.0 };
        let cover_h = (cover_w * 1.5).round();

        egui::Frame::new()
            .fill(palette.card_bg)
            .stroke(Stroke::new(1.0, palette.card_border))
            .corner_radius(12.0)
            .inner_margin(egui::Margin::ZERO)
            .show(ui, |ui| {
                ui.set_width(card_width);
                ui.spacing_mut().item_spacing.y = 0.0;

                egui::Frame::new()
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            self.render_book_cover(
                                ctx,
                                ui,
                                entry,
                                settings,
                                palette,
                                Vec2::new(cover_w, cover_h),
                                t,
                            );
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                let eyebrow_cat = entry.categories.first().map(|s| s.as_str());
                                if eyebrow_cat.is_some() || !catalog_name.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        if let Some(cat) = eyebrow_cat {
                                            ui.label(
                                                RichText::new(cat)
                                                    .size(10.5)
                                                    .color(palette.on_surface_variant),
                                            );
                                            if !catalog_name.is_empty() {
                                                paint_dot_separator(
                                                    ui,
                                                    10.0,
                                                    palette.on_surface_variant,
                                                );
                                            }
                                        }
                                        if !catalog_name.is_empty() {
                                            ui.label(
                                                RichText::new(catalog_name)
                                                    .size(10.5)
                                                    .color(palette.on_surface_variant),
                                            );
                                        }
                                    });
                                    ui.add_space(2.0);
                                }

                                ui.label(
                                    RichText::new(&entry.title)
                                        .strong()
                                        .size(if compact { 14.0 } else { 15.0 })
                                        .color(palette.on_surface),
                                );

                                if !entry.authors.is_empty() {
                                    ui.label(
                                        RichText::new(entry.authors.join(", "))
                                            .size(12.0)
                                            .color(palette.secondary),
                                    );
                                }

                                if !entry.categories.is_empty() {
                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                                        for cat in entry.categories.iter().take(4) {
                                            pill_badge(
                                                ui,
                                                cat,
                                                palette.surface_container,
                                                palette.on_surface_variant,
                                                10.0,
                                                None,
                                            );
                                        }
                                    });
                                }

                                if let Some(summary) = &entry.summary {
                                    ui.add_space(6.0);
                                    let max_chars = 140;
                                    let is_expanded = self.expanded_summaries.contains(&entry.id);
                                    let clipped = if is_expanded {
                                        None
                                    } else {
                                        super::truncate_summary_chars(summary, max_chars)
                                    };
                                    let display_text = clipped.as_deref().unwrap_or(summary);
                                    ui.label(
                                        RichText::new(display_text)
                                            .size(11.5)
                                            .color(palette.on_surface_variant),
                                    );
                                    if super::summary_exceeds_chars(summary, max_chars) {
                                        let toggle_lbl = if is_expanded {
                                            t.opds_collapse
                                        } else {
                                            t.opds_expand
                                        };
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new(toggle_lbl)
                                                        .size(11.0)
                                                        .color(palette.primary),
                                                )
                                                .frame(false),
                                            )
                                            .clicked()
                                        {
                                            if is_expanded {
                                                self.expanded_summaries.remove(&entry.id);
                                            } else {
                                                self.expanded_summaries.insert(entry.id.clone());
                                            }
                                        }
                                    }
                                }
                            });
                        });
                    });

                self.render_book_card_footer(ui, entry, settings, palette, on_open_file);
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn render_book_cover(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        entry: &OpdsEntry,
        settings: &AppSettings,
        palette: &OpdsPalette,
        size: Vec2,
        t: &Translations,
    ) {
        let (cover_rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let radius = 8.0;
        ui.painter()
            .rect_filled(cover_rect, radius, palette.surface_container);

        let auth = self.active_auth(settings).ok().flatten();
        let tex = entry
            .thumbnail_url
            .as_deref()
            .and_then(|url| self.cover_manager.get_texture(ctx, url, auth));

        if let Some(handle) = tex {
            let tex_sz = handle.size_vec2();
            if tex_sz.x > 0.0 && tex_sz.y > 0.0 {
                let inner = cover_rect.shrink(1.0);
                let rect_aspect = inner.width() / inner.height();
                let tex_aspect = tex_sz.x / tex_sz.y;
                let (uv_min, uv_max) = if tex_aspect > rect_aspect {
                    let frac = rect_aspect / tex_aspect;
                    let off = (1.0 - frac) * 0.5;
                    (Pos2::new(off, 0.0), Pos2::new(1.0 - off, 1.0))
                } else {
                    let frac = tex_aspect / rect_aspect;
                    let off = (1.0 - frac) * 0.5;
                    (Pos2::new(0.0, off), Pos2::new(1.0, 1.0 - off))
                };
                ui.painter().image(
                    handle.id(),
                    inner,
                    Rect::from_min_max(uv_min, uv_max),
                    Color32::WHITE,
                );
            }
        } else {
            let author = entry.authors.first().map(|s| s.as_str()).unwrap_or("");
            paint_procedural_cover(
                ui.painter(),
                cover_rect,
                &entry.title,
                author,
                Path::new(&entry.id),
                radius,
                t.extra.default_book_title,
            );
        }

        ui.painter().rect_stroke(
            cover_rect,
            radius,
            Stroke::new(1.0, palette.card_border),
            StrokeKind::Inside,
        );

        if let Some(year) = super::extract_entry_year(entry.updated.as_deref()) {
            let badge_rect = Rect::from_min_size(
                Pos2::new(cover_rect.min.x + 6.0, cover_rect.max.y - 22.0),
                Vec2::new(36.0, 16.0),
            );
            ui.painter().rect_filled(
                badge_rect,
                4.0,
                Color32::from_rgba_unmultiplied(20, 16, 14, 195),
            );
            ui.painter().text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                year,
                egui::FontId::proportional(9.5),
                Color32::WHITE,
            );
        }
    }

    fn render_book_card_footer(
        &mut self,
        ui: &mut egui::Ui,
        entry: &OpdsEntry,
        settings: &AppSettings,
        palette: &OpdsPalette,
        on_open_file: &mut Option<PathBuf>,
    ) {
        let t = settings.ui_language.t();
        let width = ui.available_width();
        let (div_rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), egui::Sense::hover());
        ui.painter().rect_filled(div_rect, 0.0, palette.card_border);

        egui::Frame::new()
            .fill(palette.card_footer_bg)
            .corner_radius(egui::CornerRadius {
                nw: 0,
                ne: 0,
                sw: 12,
                se: 12,
            })
            .inner_margin(egui::Margin::symmetric(14, 9))
            .show(ui, |ui| {
                ui.set_width((width - 28.0).max(80.0));

                if let Some((dl_id, status, fmt_label, size_bytes)) =
                    self.find_entry_download_state(entry)
                {
                    match status {
                        DownloadStatus::Completed { file_path } => {
                            ui.horizontal_wrapped(|ui| {
                                if ui
                                    .add(
                                        IconButton::new(Icon::Check, t.extra.opds_open_in_reader)
                                            .font_size(11.5)
                                            .fill(palette.tertiary_sage)
                                            .color(Color32::WHITE)
                                            .min_size(Vec2::new(0.0, 28.0)),
                                    )
                                    .clicked()
                                {
                                    *on_open_file = Some(file_path);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if let Some(bytes) = size_bytes {
                                            ui.label(
                                                RichText::new(super::format_byte_size(bytes, t))
                                                    .size(10.5)
                                                    .color(palette.on_surface_variant),
                                            );
                                            paint_dot_separator(
                                                ui,
                                                10.0,
                                                palette.on_surface_variant,
                                            );
                                        }
                                        let saved = fill(
                                            t.extra.opds_saved_format,
                                            &[("{format}", &fmt_label)],
                                        );
                                        ui.label(
                                            RichText::new(saved)
                                                .size(10.5)
                                                .color(palette.tertiary_sage),
                                        );
                                    },
                                );
                            });
                            return;
                        }
                        DownloadStatus::Queued | DownloadStatus::Downloading { .. } => {
                            let (frac, label_text) = match status {
                                DownloadStatus::Queued => (
                                    None,
                                    format!("{} ({fmt_label})", t.opds_downloading),
                                ),
                                DownloadStatus::Downloading { downloaded, total } => {
                                    let f = total
                                        .filter(|&tot| tot > 0)
                                        .map(|tot| downloaded as f32 / tot as f32);
                                    let txt = match (f, total) {
                                        (Some(frac), Some(tot)) => format!(
                                            "{} {}% ({} / {})",
                                            t.opds_downloading,
                                            (frac * 100.0).round() as u32,
                                            super::format_byte_size(downloaded as u64, t),
                                            super::format_byte_size(tot as u64, t)
                                        ),
                                        _ => format!(
                                            "{} ({})",
                                            t.opds_downloading,
                                            super::format_byte_size(downloaded as u64, t)
                                        ),
                                    };
                                    (f, txt)
                                }
                                _ => (None, String::new()),
                            };
                            ui.horizontal(|ui| {
                                let bar_avail = (ui.available_width() - 34.0).max(80.0);
                                ui.vertical(|ui| {
                                    ui.set_width(bar_avail);
                                    ui.label(
                                        RichText::new(&label_text)
                                            .size(11.0)
                                            .color(palette.primary),
                                    );
                                    ui.add_space(4.0);
                                    let (bar_rect, _) = ui.allocate_exact_size(
                                        Vec2::new(bar_avail, 6.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter()
                                        .rect_filled(bar_rect, 3.0, palette.surface_high);
                                    let progress = frac.unwrap_or(0.32);
                                    let fill_w = (bar_avail * progress.clamp(0.05, 1.0)).min(bar_avail);
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, 6.0)),
                                        3.0,
                                        palette.primary,
                                    );
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (xr, xresp) = ui.allocate_exact_size(
                                            Vec2::splat(26.0),
                                            egui::Sense::click(),
                                        );
                                        ui.painter().rect_filled(
                                            xr,
                                            6.0,
                                            if xresp.hovered() {
                                                palette.error_bg
                                            } else {
                                                palette.card_bg
                                            },
                                        );
                                        ui.painter().rect_stroke(
                                            xr,
                                            6.0,
                                            Stroke::new(1.0, palette.card_border),
                                            StrokeKind::Inside,
                                        );
                                        paint_icon(
                                            ui.painter(),
                                            Icon::Cross,
                                            Rect::from_center_size(xr.center(), Vec2::splat(11.0)),
                                            if xresp.hovered() {
                                                palette.error_fg
                                            } else {
                                                palette.on_surface_variant
                                            },
                                        );
                                        if xresp.on_hover_text(t.opds_cancel_btn).clicked() {
                                            self.cancel_download(&dl_id);
                                        }
                                    },
                                );
                            });
                            return;
                        }
                        DownloadStatus::Failed(err) => {
                            ui.add(
                                IconLabel::new(Icon::Warning, &format!("[{fmt_label}] {err}"))
                                    .color(palette.error_fg)
                                    .size(10.5),
                            );
                            ui.add_space(4.0);
                        }
                    }
                }

                let mut link_to_download: Option<OpdsAcquisitionLink> = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 5.0);
                    for (idx, acq) in entry.acquisition_links.iter().enumerate() {
                        let is_primary = idx == 0;
                        let clicked = if is_primary {
                            let label = format!("{} {}", t.opds_download_format, acq.format_label);
                            ui.add(
                                IconButton::new(Icon::Download, &label)
                                    .font_size(11.5)
                                    .fill(palette.primary)
                                    .color(palette.on_primary)
                                    .min_size(Vec2::new(0.0, 28.0)),
                            )
                            .on_hover_text(&acq.title)
                            .clicked()
                        } else {
                            pill_button(
                                ui,
                                &acq.format_label,
                                palette.card_bg,
                                palette.on_surface,
                                Stroke::new(1.0, palette.card_border),
                                11.0,
                                26.0,
                            )
                            .on_hover_text(&acq.title)
                            .clicked()
                        };
                        if clicked {
                            link_to_download = Some(acq.clone());
                        }
                    }

                    if let Some(first_len) = entry.acquisition_links.first().and_then(|l| l.length)
                    {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("~{}", super::format_byte_size(first_len, t)))
                                    .size(10.5)
                                    .color(palette.on_surface_variant),
                            );
                        });
                    }
                });

                if let Some(acq) = link_to_download {
                    self.start_download(entry, &acq, settings);
                }
            });
    }

    fn find_entry_download_state(
        &self,
        entry: &OpdsEntry,
    ) -> Option<(String, DownloadStatus, String, Option<u64>)> {
        for acq in &entry.acquisition_links {
            let dl_id = format!("{}_{}", entry.id, acq.format_label);
            if let Some(item) = self.downloads.get(&dl_id) {
                return Some((
                    dl_id,
                    item.status.clone(),
                    item.format_label.clone(),
                    item.size_bytes,
                ));
            }
        }
        None
    }
}
