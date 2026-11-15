use super::QuietReaderApp;
use super::SidebarTab;
use super::icons::{Icon, STATUS_ARROW_SEP, paint_arrow_pair, paint_icon};
use super::reader_chrome::{ReaderSurfaceColors, has_quick_bookmark};
use super::theme::ReaderTheme;
use crate::core::Command;
use crate::core::library::{DEFAULT_READING_WPM, UiDesignMode, estimate_minutes_left_in_chapter};
use crate::document::{Document, DocumentViewKind};
use eframe::egui::{self, Color32, FontId, Panel, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};
use std::sync::Arc;

#[derive(Default, Clone, Copy, Debug)]
pub struct CozyReaderBottomActions {
    pub set_spread_mode: Option<bool>,
    pub toggle_tts: bool,
    pub toggle_autoscroll: bool,
    pub toggle_ruler: bool,
    pub toggle_bookmark: bool,
    pub open_bookmarks_panel: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn render_active_app_bottom_bar(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    status_opacity: f32,
    view_kind: DocumentViewKind,
    pdf_info: (usize, usize),
    comic_info: (usize, usize),
    text_info: (usize, usize, f32, f32),
) {
    let status_owned = app.status_message.clone();
    let status_text = status_owned.as_deref();
    let cozy_opds = app.show_opds_dialog && app.library.design == UiDesignMode::Cozy;
    if (app.show_library || cozy_opds) && status_text.is_none() {
        return;
    }
    let text_pages_override = if app.settings.text_paged_scroll
        && view_kind == DocumentViewKind::Text
        && app.viewport_height > 1.0
        && app.max_scroll_y > 0.0
    {
        let chapters = app.document.as_deref().map(|d| d.chapters()).unwrap_or(&[]);
        Some(super::reader_chrome::effective_text_page_pair(
            chapters,
            app.current_chapter,
            app.current_scroll_y,
            app.max_scroll_y,
            app.viewport_height,
            true,
        ))
    } else {
        None
    };

    if !app.show_library
        && !cozy_opds
        && app.document.is_some()
        && app.library.design == UiDesignMode::Cozy
    {
        render_app_cozy_bottom_bar(
            app,
            ui,
            ctx,
            status_text,
            status_opacity,
            view_kind,
            pdf_info,
            comic_info,
            text_info,
            text_pages_override,
        );
    } else {
        super::render_bottom_bar(
            ui,
            &app.document,
            &app.theme,
            status_text,
            status_opacity,
            view_kind,
            pdf_info,
            comic_info,
            text_info,
            text_pages_override,
            app.settings.ui_language,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_app_cozy_bottom_bar(
    app: &mut QuietReaderApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    status_text: Option<&str>,
    status_opacity: f32,
    view_kind: DocumentViewKind,
    pdf_info: (usize, usize),
    comic_info: (usize, usize),
    text_info: (usize, usize, f32, f32),
    text_pages_override: Option<(usize, usize)>,
) {
    let tts_st = app.tts.status();
    let tts_active = tts_st.is_playing || tts_st.is_paused || app.tts.show_panel;
    let page_num = match view_kind {
        DocumentViewKind::Pdf => Some(pdf_info.0),
        DocumentViewKind::Comic => Some(comic_info.0),
        _ => None,
    };
    let has_bookmark = has_quick_bookmark(
        &app.settings,
        app.current_file_path.as_deref(),
        app.current_chapter,
        app.current_scroll_y,
        page_num,
    );
    let eff_two_cols = super::reader_chrome::last_effective_spread(
        ctx,
        ui.available_width(),
        app.settings.text_two_columns,
    );
    let show_spread_range = eff_two_cols && super::reader_chrome::spread_right_page_visible(ctx);
    let actions = render_cozy_reader_bottom_bar(
        ui,
        &app.document,
        &app.theme,
        app.library.design,
        status_text,
        status_opacity,
        view_kind,
        pdf_info,
        comic_info,
        text_info,
        text_pages_override,
        app.settings.text_two_columns,
        show_spread_range,
        tts_active,
        tts_st.rate,
        app.autoscroll.is_active(),
        app.settings.effective_reading_ruler(),
        has_bookmark,
        app.settings.ui_language,
    );
    if let Some(spread_mode) = actions.set_spread_mode
        && app.settings.text_two_columns != spread_mode
    {
        app.settings.text_two_columns = spread_mode;
        app.mark_settings_dirty();
    }
    if actions.toggle_tts {
        app.execute_command(ctx, &Command::ToggleTTS, view_kind);
    }
    if actions.toggle_autoscroll {
        app.execute_command(ctx, &Command::ToggleAutoscroll, view_kind);
    }
    if actions.toggle_ruler {
        app.execute_command(ctx, &Command::ToggleReadingRuler, view_kind);
    }
    if actions.toggle_bookmark {
        app.execute_command(ctx, &Command::ToggleBookmark, view_kind);
    }
    if actions.open_bookmarks_panel {
        app.show_toc = true;
        app.sidebar_tab = SidebarTab::Bookmarks;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_cozy_reader_bottom_bar(
    ui: &mut egui::Ui,
    document: &Option<Arc<dyn Document>>,
    theme: &ReaderTheme,
    design: UiDesignMode,
    status_message: Option<&str>,
    status_opacity: f32,
    view_kind: DocumentViewKind,
    pdf_page: (usize, usize),
    comic_page: (usize, usize),
    text_chapter: (usize, usize, f32, f32),
    text_pages_override: Option<(usize, usize)>,
    want_two_columns: bool,
    two_columns: bool,
    tts_active: bool,
    tts_rate: f32,
    autoscroll_active: bool,
    reading_ruler_active: bool,
    has_bookmark: bool,
    lang: crate::i18n::UiLanguage,
) -> CozyReaderBottomActions {
    let t = lang.t();
    let is_rtl = lang.is_rtl();
    let colors = ReaderSurfaceColors::for_theme(theme, design);
    let mut actions = CozyReaderBottomActions::default();

    let (page_str, pct_str, progress_frac, chapter_eta, section_str) = compute_status_metrics(
        document,
        view_kind,
        pdf_page,
        comic_page,
        text_chapter,
        text_pages_override,
        two_columns,
        lang,
    );

    Panel::bottom("cozy_reader_bottom_bar")
        .frame(
            egui::Frame::new()
                .fill(colors.bar_bg)
                .stroke(Stroke::new(1.0, colors.bar_border))
                .inner_margin(egui::Margin::symmetric(14, 7)),
        )
        .show(ui, |ui| {
            let avail_w = ui.available_width();
            let ultra_compact = avail_w < 720.0;
            let compact = avail_w < 1040.0;
            let show_center_stats = avail_w >= 1060.0 && view_kind == DocumentViewKind::Text;
            let show_keycaps = avail_w >= 1380.0;

            let row_layout = if is_rtl {
                egui::Layout::right_to_left(egui::Align::Center)
            } else {
                egui::Layout::left_to_right(egui::Align::Center)
            };
            let right_layout = if is_rtl {
                egui::Layout::left_to_right(egui::Align::Center)
            } else {
                egui::Layout::right_to_left(egui::Align::Center)
            };

            ui.allocate_ui_with_layout(Vec2::new(avail_w, 28.0), right_layout, |ui| {
                ui.spacing_mut().item_spacing.x = if ultra_compact { 4.0 } else { 6.0 };

                let bm_label = if ultra_compact {
                    ""
                } else {
                    t.extra.reader_bookmark_btn
                };
                let bm_resp = paint_action_pill(
                    ui,
                    &colors,
                    Icon::Bookmark,
                    bm_label,
                    has_bookmark,
                    true,
                    design,
                )
                .on_hover_text(t.add_bookmark);
                if bm_resp.clicked() {
                    actions.toggle_bookmark = true;
                }
                if bm_resp.secondary_clicked() {
                    actions.open_bookmarks_panel = true;
                }

                let ruler_label = if ultra_compact {
                    String::new()
                } else if compact {
                    t.reading_ruler.to_string()
                } else {
                    format!("{} (L)", t.reading_ruler)
                };
                if paint_action_pill(
                    ui,
                    &colors,
                    Icon::Ruler,
                    &ruler_label,
                    reading_ruler_active,
                    false,
                    design,
                )
                .on_hover_text(t.reading_ruler_tooltip)
                .clicked()
                {
                    actions.toggle_ruler = true;
                }

                let auto_label = if ultra_compact {
                    ""
                } else {
                    t.extra.reader_autoscroll_btn
                };
                if paint_action_pill(
                    ui,
                    &colors,
                    Icon::AutoScroll,
                    auto_label,
                    autoscroll_active,
                    false,
                    design,
                )
                .on_hover_text(format!("{} (A)", t.autoscroll_title))
                .clicked()
                {
                    actions.toggle_autoscroll = true;
                }

                let tts_label = if ultra_compact {
                    String::new()
                } else if tts_active {
                    format!("{} ({:.1}x)", t.extra.reader_tts_btn, tts_rate)
                } else {
                    t.extra.reader_tts_btn.to_string()
                };
                if paint_action_pill(
                    ui,
                    &colors,
                    Icon::Speech,
                    &tts_label,
                    tts_active,
                    false,
                    design,
                )
                .on_hover_text(t.speech_tooltip)
                .clicked()
                {
                    actions.toggle_tts = true;
                }

                if view_kind == DocumentViewKind::Text {
                    if let Some(selected) = paint_spread_mode_switch(
                        ui,
                        &colors,
                        want_two_columns,
                        avail_w >= 900.0,
                        design,
                        t,
                    ) {
                        actions.set_spread_mode = Some(selected);
                    }
                }

                let left_avail_w = ui.available_width().max(16.0);
                ui.allocate_ui_with_layout(Vec2::new(left_avail_w, 28.0), row_layout, |ui| {
                    ui.set_clip_rect(ui.available_rect_before_wrap());
                    ui.spacing_mut().item_spacing.x = if ultra_compact { 6.0 } else { 10.0 };

                    if !page_str.is_empty() {
                        ui.label(
                            RichText::new(&page_str)
                                .size(12.0)
                                .strong()
                                .color(colors.page_ink),
                        );
                    }

                    if !pct_str.is_empty() {
                        let read_pill = t.extra.reader_pages_read_pct.replace("{pct}", &pct_str);
                        egui::Frame::new()
                            .fill(colors.accent_soft_bg)
                            .corner_radius(10.0)
                            .inner_margin(egui::Margin::symmetric(7, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(read_pill)
                                        .size(10.5)
                                        .strong()
                                        .color(colors.accent),
                                );
                            });

                        if !ultra_compact && status_message.is_none() {
                            let bar_w = if compact { 56.0 } else { 86.0 };
                            let (bar_rect, _) =
                                ui.allocate_exact_size(Vec2::new(bar_w, 6.0), egui::Sense::hover());
                            ui.painter().rect_filled(bar_rect, 3.0, colors.pill_bg);
                            let fill_w =
                                (bar_rect.width() * progress_frac.clamp(0.0, 1.0)).max(0.0);
                            if fill_w > 0.5 {
                                let fill_rect = if is_rtl {
                                    Rect::from_min_max(
                                        Pos2::new(bar_rect.max.x - fill_w, bar_rect.min.y),
                                        bar_rect.max,
                                    )
                                } else {
                                    Rect::from_min_max(
                                        bar_rect.min,
                                        Pos2::new(bar_rect.min.x + fill_w, bar_rect.max.y),
                                    )
                                };
                                ui.painter().rect_filled(fill_rect, 3.0, colors.accent);
                            }
                        }
                    }

                    if let Some(err) = crate::settings::AppSettings::last_save_error() {
                        ui.add(
                            egui::Label::new(RichText::new(&err).size(11.0).color(Color32::RED))
                                .truncate(),
                        );
                    } else if let Some(msg) = status_message
                        && status_opacity > 0.01
                    {
                        let c = colors.accent.gamma_multiply(status_opacity);
                        if let Some((left, right)) = msg.split_once(STATUS_ARROW_SEP) {
                            paint_arrow_pair(ui, left, right, c, is_rtl);
                        } else {
                            ui.add(
                                egui::Label::new(RichText::new(msg).size(11.5).strong().color(c))
                                    .truncate(),
                            );
                        }
                    } else if show_center_stats {
                        ui.add_space(6.0);
                        if let Some((eta_str, sec_str)) = chapter_eta.zip(Some(section_str)) {
                            let wpm_str = t
                                .extra
                                .reader_wpm_suffix
                                .replace("{wpm}", &DEFAULT_READING_WPM.to_string());
                            ui.label(
                                RichText::new(format!("{eta_str} {wpm_str}"))
                                    .size(11.5)
                                    .color(colors.page_muted),
                            );
                            if !sec_str.is_empty() {
                                paint_small_dot(ui, colors.page_muted);
                                ui.label(
                                    RichText::new(&sec_str).size(11.5).color(colors.page_muted),
                                );
                            }
                            if show_keycaps {
                                paint_small_dot(ui, colors.page_muted);
                                paint_keycap_hint(
                                    ui,
                                    &colors,
                                    t.extra.reader_key_space,
                                    t.extra.reader_page_turn_hint,
                                );
                            }
                        }
                    }
                });
            });
        });

    actions
}

fn paint_spread_mode_switch(
    ui: &mut egui::Ui,
    colors: &ReaderSurfaceColors,
    is_spread: bool,
    show_labels: bool,
    design: UiDesignMode,
    t: &crate::i18n::Translations,
) -> Option<bool> {
    let mut picked = None;
    let height = 26.0;
    let inner_pad = 2.0;
    let icon_sz = 12.5;
    let font = FontId::proportional(11.0);
    let scroll_w = if show_labels {
        ui.painter()
            .layout_no_wrap(
                t.extra.text_mode_scroll.to_string(),
                font.clone(),
                colors.pill_text,
            )
            .size()
            .x
    } else {
        0.0
    };
    let spread_w = if show_labels {
        ui.painter()
            .layout_no_wrap(
                t.extra.text_mode_spread.to_string(),
                font.clone(),
                colors.pill_text,
            )
            .size()
            .x
    } else {
        0.0
    };
    let seg_pad = if show_labels { 16.0 } else { 8.0 };
    let gap = if show_labels { 4.5 } else { 0.0 };
    let seg0_w = seg_pad + icon_sz + gap + scroll_w;
    let seg1_w = seg_pad + icon_sz + gap + spread_w;
    let total_w = seg0_w + seg1_w + inner_pad * 2.0;
    let (rect, base_resp) =
        ui.allocate_exact_size(Vec2::new(total_w, height), egui::Sense::hover());
    let radius = (design.corner_radius().min(10) as f32).max(6.0);

    ui.painter().rect(
        rect,
        radius,
        colors.pill_bg.gamma_multiply(0.82),
        Stroke::new(1.0, colors.bar_border),
        StrokeKind::Inside,
    );

    let items = [
        (
            false,
            Icon::Scroll,
            t.extra.text_mode_scroll,
            t.extra.text_scroll_tip,
            seg0_w,
            rect.min.x + inner_pad,
        ),
        (
            true,
            Icon::Spread,
            t.extra.text_mode_spread,
            t.extra.text_spread_tip,
            seg1_w,
            rect.min.x + inner_pad + seg0_w,
        ),
    ];

    for (idx, (val, icon, label, tip, w, x)) in items.into_iter().enumerate() {
        let active = is_spread == val;
        let seg_r = Rect::from_min_size(
            Pos2::new(x, rect.min.y + inner_pad),
            Vec2::new(w, height - inner_pad * 2.0),
        );
        let resp = ui
            .interact(
                seg_r,
                base_resp.id.with(("text_mode", idx)),
                egui::Sense::click(),
            )
            .on_hover_text(tip);
        if active {
            ui.painter().rect(
                seg_r,
                (radius - 2.0).max(4.0),
                colors.accent_soft_bg,
                Stroke::new(1.0, colors.accent.gamma_multiply(0.45)),
                StrokeKind::Inside,
            );
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(seg_r, (radius - 2.0).max(4.0), colors.pill_bg);
        }
        let fg = if active {
            colors.accent
        } else {
            colors.pill_text
        };
        let content_w = if show_labels {
            icon_sz
                + gap
                + ui.painter()
                    .layout_no_wrap(label.to_string(), font.clone(), fg)
                    .size()
                    .x
        } else {
            icon_sz
        };
        let start_x = seg_r.center().x - content_w * 0.5;
        let icon_rect = Rect::from_center_size(
            Pos2::new(start_x + icon_sz * 0.5, seg_r.center().y),
            Vec2::splat(icon_sz),
        );
        paint_icon(ui.painter(), icon, icon_rect, fg);
        if show_labels {
            ui.painter().text(
                Pos2::new(icon_rect.max.x + gap, seg_r.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                font.clone(),
                fg,
            );
        }
        if resp.clicked() && !active {
            picked = Some(val);
        }
    }
    picked
}

fn paint_action_pill(
    ui: &mut egui::Ui,
    colors: &ReaderSurfaceColors,
    icon: Icon,
    label: &str,
    active: bool,
    accent_active_style: bool,
    design: UiDesignMode,
) -> egui::Response {
    let icon_sz = 13.0;
    let font = FontId::proportional(11.5);
    let label_galley = if label.is_empty() {
        None
    } else {
        Some(ui.painter().layout_no_wrap(
            label.to_string(),
            font,
            if active {
                colors.accent
            } else {
                colors.pill_text
            },
        ))
    };
    let text_w = label_galley.as_ref().map(|g| g.size().x).unwrap_or(0.0);
    let h_pad = if label.is_empty() { 7.0 } else { 10.0 };
    let gap = if label.is_empty() { 0.0 } else { 5.0 };
    let width = h_pad * 2.0 + icon_sz + gap + text_w;
    let height = 26.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());

    let fill = if active && accent_active_style {
        colors.accent_soft_bg
    } else if active {
        colors.accent_soft_bg.gamma_multiply(0.85)
    } else if resp.hovered() {
        colors.pill_bg
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if active {
        Stroke::new(1.0, colors.accent.gamma_multiply(0.45))
    } else if resp.hovered() {
        Stroke::new(1.0, colors.bar_border)
    } else {
        Stroke::NONE
    };
    let radius = design.corner_radius().min(10) as f32;
    if fill != Color32::TRANSPARENT || stroke != Stroke::NONE {
        ui.painter()
            .rect(rect, radius, fill, stroke, StrokeKind::Inside);
    }

    let fg = if active {
        colors.accent
    } else {
        colors.pill_text
    };
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.min.x + h_pad + icon_sz * 0.5, rect.center().y),
        Vec2::splat(icon_sz),
    );
    paint_icon(ui.painter(), icon, icon_rect, fg);

    if let Some(galley) = label_galley {
        let tx = icon_rect.max.x + gap;
        let ty = rect.center().y - galley.size().y * 0.5;
        ui.painter().galley(Pos2::new(tx, ty), galley, fg);
    }
    resp
}

fn paint_small_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(6.0, 12.0), egui::Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 1.5, color.gamma_multiply(0.65));
}

fn paint_keycap_hint(
    ui: &mut egui::Ui,
    colors: &ReaderSurfaceColors,
    space_label: &str,
    hint_label: &str,
) {
    let font = FontId::proportional(10.0);
    let space_galley =
        ui.painter()
            .layout_no_wrap(space_label.to_string(), font, colors.page_muted);
    let space_w = space_galley.size().x + 10.0;
    let (s_rect, _) = ui.allocate_exact_size(Vec2::new(space_w, 18.0), egui::Sense::hover());
    ui.painter().rect(
        s_rect,
        4.0,
        colors.pill_bg.gamma_multiply(0.65),
        Stroke::new(1.0, colors.bar_border),
        StrokeKind::Inside,
    );
    ui.painter().galley(
        Pos2::new(
            s_rect.center().x - space_galley.size().x * 0.5,
            s_rect.center().y - space_galley.size().y * 0.5,
        ),
        space_galley,
        colors.page_muted,
    );

    ui.label(RichText::new("/").size(10.5).color(colors.page_muted));

    for icon in [Icon::ArrowLeft, Icon::ArrowRight] {
        let (kr, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), egui::Sense::hover());
        ui.painter().rect(
            kr,
            4.0,
            colors.pill_bg.gamma_multiply(0.65),
            Stroke::new(1.0, colors.bar_border),
            StrokeKind::Inside,
        );
        paint_icon(
            ui.painter(),
            icon,
            Rect::from_center_size(kr.center(), Vec2::splat(10.0)),
            colors.page_muted,
        );
    }

    ui.label(
        RichText::new(hint_label)
            .size(11.0)
            .color(colors.page_muted),
    );
}

#[allow(clippy::too_many_arguments)]
fn compute_status_metrics(
    document: &Option<Arc<dyn Document>>,
    view_kind: DocumentViewKind,
    pdf_page: (usize, usize),
    comic_page: (usize, usize),
    text_chapter: (usize, usize, f32, f32),
    text_pages_override: Option<(usize, usize)>,
    two_columns: bool,
    lang: crate::i18n::UiLanguage,
) -> (String, String, f32, Option<String>, String) {
    let t = lang.t();
    match view_kind {
        DocumentViewKind::Pdf | DocumentViewKind::Comic => {
            let (cur_idx, total) = if view_kind == DocumentViewKind::Pdf {
                pdf_page
            } else {
                comic_page
            };
            let cur = if total > 0 {
                (cur_idx + 1).min(total)
            } else {
                0
            };
            let pct = if total > 0 {
                (cur as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            (
                format!("{} {cur} {} {total}", t.page_prefix, t.page_of),
                crate::i18n::format::format_percent(pct, lang),
                (pct / 100.0).clamp(0.0, 1.0),
                None,
                String::new(),
            )
        }
        DocumentViewKind::Text => {
            let Some(doc) = document else {
                return (String::new(), String::new(), 0.0, None, String::new());
            };
            let (ch_idx, total_chapters, cur_scroll, max_scroll) = text_chapter;
            let (cur_page, total_pages, pct) =
                super::bars::calculate_text_pages(doc.chapters(), ch_idx, cur_scroll, max_scroll);
            let (bar_page, bar_total) = text_pages_override.unwrap_or((cur_page, total_pages));
            let page_str = if two_columns && bar_total > 1 && bar_page < bar_total {
                format!(
                    "{} {bar_page}-{} {} {bar_total}",
                    t.page_prefix,
                    (bar_page + 1).min(bar_total),
                    t.page_of
                )
            } else {
                format!("{} {bar_page} {} {bar_total}", t.page_prefix, t.page_of)
            };
            let section_str = if total_chapters > 1 {
                t.section_x_of_y
                    .replacen("{}", &(ch_idx + 1).to_string(), 1)
                    .replacen("{}", &total_chapters.to_string(), 1)
            } else {
                String::new()
            };
            let eta_str = doc.chapters().get(ch_idx).and_then(|ch| {
                let chars = ch.title.chars().count()
                    + ch.paragraphs
                        .iter()
                        .map(|p| p.chars().count())
                        .sum::<usize>();
                if chars == 0 {
                    return None;
                }
                let progress = if max_scroll > 1.0 {
                    (cur_scroll / max_scroll).clamp(0.0, 1.0)
                } else if ch_idx + 1 == doc.chapters().len() {
                    1.0
                } else {
                    0.0
                };
                let mins = estimate_minutes_left_in_chapter(chars, progress, DEFAULT_READING_WPM);
                Some(t.chapter_time_left.replace("{minutes}", &mins.to_string()))
            });
            (
                page_str,
                crate::i18n::format::format_percent(pct, lang),
                (pct / 100.0).clamp(0.0, 1.0),
                eta_str,
                section_str,
            )
        }
    }
}
