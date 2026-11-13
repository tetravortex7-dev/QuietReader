use crate::app::opds_view::{OpdsBrowserState, summary_exceeds_chars, truncate_summary_chars};
use crate::i18n::UiLanguage;
use crate::opds::{OpdsBreadcrumb, OpdsError, OpdsFeed};

#[test]
fn test_opds_summary_char_truncation_handles_multibyte_utf8() {
    let ru_120: String = "А".repeat(120);
    assert!(ru_120.len() > 180);
    assert!(!summary_exceeds_chars(&ru_120, 180));
    assert_eq!(truncate_summary_chars(&ru_120, 180), None);

    let ru_190: String = "Я".repeat(190);
    assert!(summary_exceeds_chars(&ru_190, 180));
    let truncated = truncate_summary_chars(&ru_190, 180).expect("must truncate");
    assert_eq!(truncated.chars().count(), 183);
    assert!(truncated.ends_with("..."));
}

fn empty_test_feed(title: &str) -> OpdsFeed {
    OpdsFeed {
        title: title.to_string(),
        id: title.to_string(),
        icon: None,
        links: Vec::new(),
        entries: Vec::new(),
        next_url: None,
        prev_url: None,
        search_template: None,
        search_descriptor_url: None,
    }
}

#[test]
fn test_opds_browser_state_filters_stale_request_ids_and_cancels_cleanly() {
    let mut state = OpdsBrowserState::new();
    state.is_loading = true;
    state.pending_request_id = Some(2);
    state.breadcrumbs = vec![OpdsBreadcrumb {
        title: "Root".to_string(),
        url: "https://example.com/root".to_string(),
    }];

    let accepted_stale = state.apply_feed_event(
        1,
        "https://example.com/old".to_string(),
        Ok(empty_test_feed("Stale")),
        UiLanguage::En,
    );
    assert!(!accepted_stale);
    assert!(state.is_loading);
    assert_eq!(state.pending_request_id, Some(2));
    assert!(state.current_feed.is_none());

    let accepted_current = state.apply_feed_event(
        2,
        "https://example.com/root".to_string(),
        Ok(empty_test_feed("Root")),
        UiLanguage::En,
    );
    assert!(accepted_current);
    assert!(!state.is_loading);
    assert_eq!(state.pending_request_id, None);
    assert_eq!(
        state.current_feed.as_ref().map(|f| f.title.as_str()),
        Some("Root")
    );

    state.history.push((
        "https://example.com/root".to_string(),
        state.breadcrumbs.clone(),
    ));
    state.breadcrumbs.push(OpdsBreadcrumb {
        title: "Sub".to_string(),
        url: "https://example.com/sub".to_string(),
    });
    state.is_loading = true;
    state.pending_request_id = Some(3);
    state.cancel_loading();

    assert!(!state.is_loading);
    assert_eq!(state.pending_request_id, None);
    assert_eq!(state.breadcrumbs.len(), 1);
    assert!(state.history.is_empty());

    let accepted_cancelled = state.apply_feed_event(
        3,
        "https://example.com/sub".to_string(),
        Err(OpdsError::HttpStatus(500)),
        UiLanguage::En,
    );
    assert!(!accepted_cancelled);
    assert!(state.error_message.is_none());
    assert_eq!(
        state.current_feed.as_ref().map(|f| f.title.as_str()),
        Some("Root")
    );
}

#[test]
fn test_all_languages_have_opds_open_section_label() {
    for lang in UiLanguage::ALL {
        assert!(!lang.t().extra.opds_open_section.trim().is_empty());
    }
}

#[test]
fn test_opds_window_adapts_bounds_across_compact_laptop_and_ultrawide_screens() {
    use crate::app::theme::ReaderTheme;
    use crate::opds::OpdsAcquisitionLink;
    use crate::opds::OpdsEntry;
    use crate::settings::AppSettings;
    use eframe::egui;

    let feed = OpdsFeed {
        title: "Catalog".to_string(),
        id: "cat".to_string(),
        icon: None,
        links: Vec::new(),
        entries: vec![
            OpdsEntry {
                id: "c1".to_string(),
                title: "Fiction".to_string(),
                authors: Vec::new(),
                summary: Some("Section summary".to_string()),
                categories: Vec::new(),
                thumbnail_url: None,
                acquisition_links: Vec::new(),
                navigation_link: Some("https://example.com/fiction".to_string()),
                updated: None,
            },
            OpdsEntry {
                id: "b1".to_string(),
                title: "Very Long Responsive Ebook Title For Multi Monitor Layout".to_string(),
                authors: vec!["Author Name".to_string()],
                summary: Some("Long summary text ".repeat(14)),
                categories: vec![
                    "Novel".to_string(),
                    "Classic".to_string(),
                    "SciFi".to_string(),
                    "Drama".to_string(),
                ],
                thumbnail_url: None,
                acquisition_links: vec![OpdsAcquisitionLink {
                    href: "https://example.com/b1.epub".to_string(),
                    mime_type: "application/epub+zip".to_string(),
                    title: "Download EPUB".to_string(),
                    format_label: "EPUB".to_string(),
                    extension: "epub".to_string(),
                }],
                navigation_link: None,
                updated: None,
            },
            OpdsEntry {
                id: "b2".to_string(),
                title: "Second Book".to_string(),
                authors: vec!["Second Author".to_string()],
                summary: Some("Short description".to_string()),
                categories: vec!["History".to_string()],
                thumbnail_url: None,
                acquisition_links: vec![],
                navigation_link: None,
                updated: None,
            },
        ],
        next_url: Some("https://example.com/next".to_string()),
        prev_url: Some("https://example.com/prev".to_string()),
        search_template: None,
        search_descriptor_url: None,
    };

    for (w, h, ppp) in [
        (540.0_f32, 420.0_f32, 1.5_f32),
        (1366.0, 768.0, 1.25),
        (3440.0, 1440.0, 2.0),
    ] {
        let mut state = OpdsBrowserState::new();
        state.current_feed = Some(feed.clone());
        state.breadcrumbs = vec![
            OpdsBreadcrumb {
                title: "Home".to_string(),
                url: "https://example.com".to_string(),
            },
            OpdsBreadcrumb {
                title: "Browse".to_string(),
                url: "https://example.com/browse".to_string(),
            },
        ];
        let mut settings = AppSettings::default();
        let mut show = true;
        let mut on_open_file: Option<std::path::PathBuf> = None;
        let mut on_pro = None;

        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));
        for _ in 0..2 {
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(screen);
            raw.viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(ppp);
            let mut out = ctx.run_ui(raw, |ui| {
                state.render(
                    ui.ctx(),
                    &mut show,
                    &mut settings,
                    &ReaderTheme::Light,
                    &mut on_open_file,
                    &mut on_pro,
                );
            });
            out.textures_delta.clear();
        }

        let rect = ctx
            .memory(|m| m.area_rect(egui::Id::unique("opds_storefront_window")))
            .expect("opds storefront rect");
        assert!(rect.width() <= w + 1.0, "opds {w}x{h}@{ppp}: rect={rect:?}");
        assert!(
            rect.height() <= h + 1.0,
            "opds {w}x{h}@{ppp}: rect={rect:?}"
        );
        assert!(
            rect.left() >= -1.0 && rect.right() <= w + 1.0,
            "opds {w}x{h}@{ppp}: rect={rect:?}"
        );
        assert!(
            rect.top() >= -1.0 && rect.bottom() <= h + 1.0,
            "opds {w}x{h}@{ppp}: rect={rect:?}"
        );
        if w >= 2000.0 {
            assert!(
                rect.width() <= 1600.0,
                "opds ultrawide {w}x{h}@{ppp}: rect={rect:?}"
            );
        }
    }
}
