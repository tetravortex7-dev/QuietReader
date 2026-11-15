use crate::app::reader_bottom_bar::render_cozy_reader_bottom_bar;
use crate::app::reader_chrome::{
    ReaderSurfaceColors, compute_book_spread_layout, is_blockquote_paragraph, paint_book_backdrop,
    paint_book_foreground,
};
use crate::app::theme::ReaderTheme;
use crate::core::library::UiDesignMode;
use crate::core::{Command, ReaderCore};
use crate::document::txt::TxtDocument;
use crate::document::{Document, DocumentViewKind};
use crate::i18n::UiLanguage;
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::sync::Arc;

#[test]
fn test_reader_surface_colors_cover_all_seven_themes() {
    let all_themes = [
        (ReaderTheme::Warm, false),
        (ReaderTheme::Sepia, false),
        (ReaderTheme::Light, false),
        (ReaderTheme::Mint, false),
        (ReaderTheme::Dark, true),
        (ReaderTheme::Twilight, true),
        (ReaderTheme::Oled, true),
    ];

    for (theme, expected_dark) in all_themes {
        for design in [UiDesignMode::Cozy, UiDesignMode::Studio] {
            let colors = ReaderSurfaceColors::for_theme(&theme, design);
            assert_eq!(colors.is_dark, expected_dark);
            assert_ne!(colors.page_bg, colors.page_ink);
            assert_ne!(colors.accent, colors.page_bg);
            assert_ne!(colors.cover_rim_bg, colors.page_bg);
        }
        let warm_cozy = ReaderSurfaceColors::for_theme(&theme, UiDesignMode::Cozy);
        assert_ne!(warm_cozy.outer_canvas_bg, warm_cozy.page_bg);
    }
}

#[test]
fn test_book_spread_layout_is_responsive() {
    let compact_avail = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 480.0));
    let compact_layout = compute_book_spread_layout(compact_avail, 720.0, true);
    assert!(compact_layout.compact);
    assert!(!compact_layout.effective_two_columns);
    assert!(compact_avail.contains_rect(compact_layout.cover_rect));
    assert!(
        compact_layout
            .cover_rect
            .contains_rect(compact_layout.sheet_rect)
    );
    assert!(
        compact_layout
            .sheet_rect
            .contains_rect(compact_layout.content_rect)
    );
    assert!(compact_layout.content_rect.width() > 180.0);
    assert!(compact_layout.content_rect.height() > 120.0);

    let desktop_avail = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 920.0));
    let desktop_layout = compute_book_spread_layout(desktop_avail, 720.0, true);
    assert!(!desktop_layout.compact);
    assert!(desktop_layout.effective_two_columns);
    assert!(desktop_avail.contains_rect(desktop_layout.cover_rect));
    assert!(desktop_layout.left_nav_center.x < desktop_layout.cover_rect.min.x);
    assert!(desktop_layout.right_nav_center.x > desktop_layout.cover_rect.max.x);

    let ultrawide_avail = Rect::from_min_size(Pos2::ZERO, Vec2::new(2560.0, 1080.0));
    let ultrawide_layout = compute_book_spread_layout(ultrawide_avail, 720.0, true);
    assert!(ultrawide_layout.cover_rect.width() <= 1180.0);
    assert!(ultrawide_layout.cover_rect.height() <= 1040.0);
    assert!((ultrawide_layout.cover_rect.center().x - ultrawide_avail.center().x).abs() < 1.0);
}

#[test]
fn test_blockquote_detection_rules() {
    assert!(!is_blockquote_paragraph("<i>Short note</i>"));
    assert!(!is_blockquote_paragraph(
        "Normal unitalicized paragraph that is quite long."
    ));
    assert!(!is_blockquote_paragraph(
        "<i>First italic segment</i> mixed with plain text and <i>another one</i>"
    ));
    assert!(!is_blockquote_paragraph(
        "<i>{{image:cover}} caption text that exceeds thirty two bytes</i>"
    ));
    assert!(is_blockquote_paragraph(
        "<i>Standing by the ocean of Solaris, we measure the silence of our own reflection.</i>"
    ));
}

#[test]
fn test_next_prev_page_command_moves_scroll_and_chapters() {
    let text = "Chapter 1\nFirst line\n\nChapter 2\nSecond line";
    let doc = TxtDocument::from_text(text, "NavBook");

    let mut chapter = 0usize;
    let mut last_chapter = 0usize;
    let mut pending = None;

    assert!(ReaderCore::apply_text_command(
        &Command::NextPage,
        &doc,
        &mut chapter,
        &mut last_chapter,
        0.0,
        1000.0,
        500.0,
        &mut pending,
    ));
    assert_eq!(chapter, 0);
    assert_eq!(pending, Some(425.0));

    assert!(ReaderCore::apply_text_command(
        &Command::NextPage,
        &doc,
        &mut chapter,
        &mut last_chapter,
        990.0,
        1000.0,
        500.0,
        &mut pending,
    ));
    assert_eq!(chapter, 1);
    assert_eq!(last_chapter, 1);
    assert_eq!(pending, Some(0.0));

    assert!(ReaderCore::apply_text_command(
        &Command::PrevPage,
        &doc,
        &mut chapter,
        &mut last_chapter,
        0.0,
        500.0,
        500.0,
        &mut pending,
    ));
    assert_eq!(chapter, 0);
    assert_eq!(last_chapter, 0);
}

#[test]
fn test_cozy_reader_bottom_bar_and_book_chrome_render_across_locales_and_widths() {
    let ctx = egui::Context::default();
    let doc: Arc<dyn Document> = Arc::new(TxtDocument::from_text(
        "Section One\nParagraph content for testing reader chrome.",
        "Solaris",
    ));
    let langs = [UiLanguage::En, UiLanguage::Ru, UiLanguage::Ar];
    let widths = [560.0f32, 920.0, 1440.0];

    for lang in langs {
        for &width in &widths {
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 820.0)));
            let doc_ref = Some(Arc::clone(&doc));
            let mut out = ctx.run_ui(raw, |ui| {
                render_cozy_reader_bottom_bar(
                    ui,
                    &doc_ref,
                    &ReaderTheme::Warm,
                    UiDesignMode::Cozy,
                    None,
                    0.0,
                    DocumentViewKind::Text,
                    (0, 0),
                    (0, 0),
                    (0, 2, 120.0, 480.0),
                    None,
                    width >= 1000.0,
                    width >= 1000.0,
                    true,
                    1.2,
                    true,
                    true,
                    true,
                    lang,
                );
                egui::CentralPanel::default().show(ui, |ui| {
                    let colors =
                        ReaderSurfaceColors::for_theme(&ReaderTheme::Warm, UiDesignMode::Cozy);
                    let layout = compute_book_spread_layout(
                        ui.available_rect_before_wrap(),
                        720.0,
                        width >= 1000.0,
                    );
                    paint_book_backdrop(ui, &layout, &colors);
                    let resp = paint_book_foreground(
                        ui,
                        &layout,
                        &colors,
                        lang.is_rtl(),
                        "Stanislaw Lem",
                        "Solaris",
                        "Chapter 1",
                        "FB2",
                        12,
                        240,
                        true,
                        "Bookmark",
                        "Prev",
                        "Next",
                    );
                    assert!(!resp.toggle_bookmark);
                    assert!(resp.nav_command.is_none());
                });
            });
            assert!(!out.shapes.is_empty());
            out.textures_delta.clear();
        }
    }
}

#[test]
fn test_split_paragraphs_weighted_balances_with_header_weight() {
    use crate::app::text_view::layout::split_paragraphs_weighted;

    let even = vec![
        "a".repeat(120),
        "b".repeat(120),
        "c".repeat(120),
        "d".repeat(120),
    ];
    assert_eq!(split_paragraphs_weighted(&even, 0), 2);
    assert_eq!(split_paragraphs_weighted(&even, 240), 1);

    assert_eq!(split_paragraphs_weighted(&[], 240), 0);
    assert_eq!(split_paragraphs_weighted(&["solo".to_string()], 240), 1);
}

#[test]
fn test_last_effective_spread_uses_recorded_state_and_fallback() {
    use crate::app::reader_chrome::last_effective_spread;

    let ctx = egui::Context::default();
    assert!(!last_effective_spread(&ctx, 1440.0, false));
    assert!(!last_effective_spread(&ctx, 820.0, true));
    assert!(last_effective_spread(&ctx, 1280.0, true));

    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::unique("quietreader_effective_spread"), false);
    });
    assert!(!last_effective_spread(&ctx, 1440.0, true));
}

fn collect_texts_in_shape(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
    match shape {
        egui::epaint::Shape::Text(text_shape) => out.push(text_shape.galley.text().to_string()),
        egui::epaint::Shape::Vec(children) => {
            for child in children {
                collect_texts_in_shape(child, out);
            }
        }
        _ => {}
    }
}

fn find_text_rect_in_shape(shape: &egui::epaint::Shape, needle: &str) -> Option<egui::Rect> {
    match shape {
        egui::epaint::Shape::Text(text_shape) => {
            if text_shape.galley.text() == needle {
                Some(egui::Rect::from_min_size(
                    text_shape.pos,
                    text_shape.galley.size(),
                ))
            } else {
                None
            }
        }
        egui::epaint::Shape::Vec(children) => children
            .iter()
            .find_map(|child| find_text_rect_in_shape(child, needle)),
        _ => None,
    }
}

#[test]
fn test_classic_top_bar_exposes_and_triggers_cozy_design_toggle() {
    use crate::app::bars::render_top_bar;
    use crate::i18n::UiLanguage;

    for (width, lang) in [
        (1440.0, UiLanguage::Ru),
        (820.0, UiLanguage::En),
        (1024.0, UiLanguage::Ar),
    ] {
        let ctx = egui::Context::default();
        let needle = lang.t().library_design_cozy;
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::new(width, 720.0),
            )),
            ..Default::default()
        };

        let mut btn_rect = None;
        let run_bar = |ui: &mut egui::Ui| {
            let actions = render_top_bar(
                ui, true, true, true, false, false, true, false, false, false, false, lang, None,
            );
            egui::CentralPanel::default().show(ui, |_| {});
            actions
        };

        let mut warm = ctx.run_ui(raw.clone(), |ui| {
            let _ = run_bar(ui);
        });
        warm.textures_delta.clear();

        let mut out1 = ctx.run_ui(raw.clone(), |ui| {
            let actions = run_bar(ui);
            assert!(!actions.toggle_design);
        });
        out1.textures_delta.clear();
        for clipped in &out1.shapes {
            if let Some(rect) = find_text_rect_in_shape(&clipped.shape, needle) {
                btn_rect = Some(rect);
                break;
            }
        }
        let rect = match btn_rect {
            Some(r) => r,
            None => {
                let mut found = Vec::new();
                for clipped in &out1.shapes {
                    collect_texts_in_shape(&clipped.shape, &mut found);
                }
                panic!("classic top bar missing {needle:?}; texts={found:?}");
            }
        };
        assert!(rect.min.x >= 0.0 && rect.max.x <= width);

        let click_pos = rect.center();
        let raw_press = egui::RawInput {
            screen_rect: raw.screen_rect,
            events: vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut out_press = ctx.run_ui(raw_press, |ui| {
            let _ = run_bar(ui);
        });
        out_press.textures_delta.clear();

        let mut triggered = false;
        let raw_release = egui::RawInput {
            screen_rect: raw.screen_rect,
            events: vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        let mut out_release = ctx.run_ui(raw_release, |ui| {
            let actions = run_bar(ui);
            triggered = actions.toggle_design;
        });
        out_release.textures_delta.clear();
        assert!(triggered);
    }
}

fn collect_clipped_text_rects(
    shape: &egui::epaint::Shape,
    clip_rect: egui::Rect,
    out: &mut Vec<(String, egui::Rect, egui::Rect)>,
) {
    match shape {
        egui::epaint::Shape::Text(text_shape) => {
            let raw_rect = text_shape.galley.rect.translate(text_shape.pos.to_vec2());
            let clipped_rect = raw_rect.intersect(clip_rect);
            if clipped_rect.width() > 0.5 && clipped_rect.height() > 0.5 {
                out.push((text_shape.galley.text().to_string(), raw_rect, clipped_rect));
            }
        }
        egui::epaint::Shape::Vec(children) => {
            for child in children {
                collect_clipped_text_rects(child, clip_rect, out);
            }
        }
        _ => {}
    }
}

#[test]
fn test_classic_bottom_bar_prevents_overlap_across_widths_and_locales() {
    use crate::app::bars::render_bottom_bar;

    let doc: Option<Arc<dyn Document>> = Some(Arc::new(TxtDocument::from_text(
        "Chapter 1\nBody text",
        "Very Long Book Title That Should Truncate Cleanly In Studio Mode",
    )));

    for (width, lang) in [
        (640.0, UiLanguage::Ru),
        (820.0, UiLanguage::En),
        (1440.0, UiLanguage::Ar),
    ] {
        let ctx = egui::Context::default();
        let t = lang.t();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(width, 720.0));
        let raw = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let status = Some("Opened book: Very Long File Name And Author Notification Message");
        let run_bar = |ui: &mut egui::Ui| {
            render_bottom_bar(
                ui,
                &doc,
                &ReaderTheme::Warm,
                status,
                1.0,
                DocumentViewKind::Text,
                (0, 0),
                (0, 0),
                (0, 4, 0.0, 100.0),
                None,
                lang,
            );
            egui::CentralPanel::default().show(ui, |_| {});
        };

        let mut warm = ctx.run_ui(raw.clone(), run_bar);
        warm.textures_delta.clear();

        let mut out = ctx.run_ui(raw, run_bar);
        out.textures_delta.clear();

        let mut texts = Vec::new();
        for clipped in &out.shapes {
            collect_clipped_text_rects(&clipped.shape, clipped.clip_rect, &mut texts);
        }

        let metric = texts
            .iter()
            .find(|(text, _, _)| text.contains(t.page_prefix) && text.contains('|'))
            .unwrap_or_else(|| {
                panic!("missing bottom bar page metric at width={width} lang={lang:?}: {texts:?}")
            });
        assert!(
            metric.1.min.x >= 0.0 && metric.1.max.x <= width,
            "metric out of bounds at width={width} lang={lang:?}: metric={metric:?}, all_texts={texts:?}"
        );
        assert!(
            (metric.1.width() - metric.2.width()).abs() < 1.0,
            "metric clipped at width={width} lang={lang:?}: metric={metric:?}, all_texts={texts:?}"
        );

        for (text, _, visible_rect) in &texts {
            if std::ptr::eq(text, &metric.0) {
                continue;
            }
            assert!(
                !visible_rect.intersects(metric.2),
                "bottom bar text '{text}' ({visible_rect:?}) overlapped metric '{}' ({:?}) at width={width} lang={lang:?}",
                metric.0,
                metric.2,
            );
        }
    }
}

#[test]
fn test_cozy_bottom_bar_scroll_and_spread_switcher() {
    let doc: Arc<dyn Document> = Arc::new(TxtDocument::from_text(
        "Section One\nParagraph content for testing reader mode switching.",
        "Solaris",
    ));
    let doc_ref = Some(doc);

    for (start_spread, lang) in [
        (true, UiLanguage::Ru),
        (false, UiLanguage::En),
        (true, UiLanguage::Ar),
    ] {
        let ctx = egui::Context::default();
        let t = lang.t();
        let needle = if start_spread {
            t.extra.text_mode_scroll
        } else {
            t.extra.text_mode_spread
        };
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 820.0))),
            ..Default::default()
        };
        let run_bar = |ui: &mut egui::Ui| {
            let actions = render_cozy_reader_bottom_bar(
                ui,
                &doc_ref,
                &ReaderTheme::Warm,
                UiDesignMode::Cozy,
                None,
                0.0,
                DocumentViewKind::Text,
                (0, 0),
                (0, 0),
                (0, 2, 0.0, 400.0),
                None,
                start_spread,
                start_spread,
                false,
                1.0,
                false,
                false,
                false,
                lang,
            );
            egui::CentralPanel::default().show(ui, |_| {});
            actions
        };

        let mut warm = ctx.run_ui(raw.clone(), |ui| {
            let _ = run_bar(ui);
        });
        warm.textures_delta.clear();

        let mut out1 = ctx.run_ui(raw.clone(), |ui| {
            let actions = run_bar(ui);
            assert!(actions.set_spread_mode.is_none());
        });
        out1.textures_delta.clear();
        let mut rect = None;
        for clipped in &out1.shapes {
            if let Some(r) = find_text_rect_in_shape(&clipped.shape, needle) {
                rect = Some(r);
                break;
            }
        }
        let btn_rect = rect.unwrap_or_else(|| panic!("missing mode label {needle:?}"));
        let click_pos = btn_rect.center();

        let mut out_press = ctx.run_ui(
            egui::RawInput {
                screen_rect: raw.screen_rect,
                events: vec![
                    egui::Event::PointerMoved(click_pos),
                    egui::Event::PointerButton {
                        pos: click_pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                let _ = run_bar(ui);
            },
        );
        out_press.textures_delta.clear();

        let mut switched = None;
        let mut out_release = ctx.run_ui(
            egui::RawInput {
                screen_rect: raw.screen_rect,
                events: vec![
                    egui::Event::PointerMoved(click_pos),
                    egui::Event::PointerButton {
                        pos: click_pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ui| {
                switched = run_bar(ui).set_spread_mode;
            },
        );
        out_release.textures_delta.clear();
        assert_eq!(switched, Some(!start_spread));
    }
}

#[test]
fn test_classic_top_bar_scroll_and_spread_buttons() {
    use crate::app::bars::render_top_bar;

    let ctx = egui::Context::default();
    let lang = UiLanguage::Ru;
    let needle = lang.t().extra.text_mode_scroll;
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 720.0))),
        ..Default::default()
    };
    let run_bar = |ui: &mut egui::Ui| {
        let actions = render_top_bar(
            ui, true, true, true, false, false, true, false, false, false, false, lang, None,
        );
        egui::CentralPanel::default().show(ui, |_| {});
        actions
    };

    let mut warm = ctx.run_ui(raw.clone(), |ui| {
        let _ = run_bar(ui);
    });
    warm.textures_delta.clear();

    let mut out1 = ctx.run_ui(raw.clone(), |ui| {
        let _ = run_bar(ui);
    });
    out1.textures_delta.clear();
    let click_pos = out1
        .shapes
        .iter()
        .find_map(|c| find_text_rect_in_shape(&c.shape, needle))
        .expect("classic top bar missing scroll mode button")
        .center();

    let mut out_press = ctx.run_ui(
        egui::RawInput {
            screen_rect: raw.screen_rect,
            events: vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| {
            let _ = run_bar(ui);
        },
    );
    out_press.textures_delta.clear();

    let mut chosen = None;
    let mut out_release = ctx.run_ui(
        egui::RawInput {
            screen_rect: raw.screen_rect,
            events: vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ui| {
            chosen = run_bar(ui).set_text_spread;
        },
    );
    out_release.textures_delta.clear();
    assert_eq!(chosen, Some(false));
}
