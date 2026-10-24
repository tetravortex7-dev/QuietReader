#[test]
fn test_span_matches_search_normalizes_case_and_yo() {
    use crate::app::pdf_view::span_matches_search;

    assert!(span_matches_search("Привет, мир", "привет"));
    assert!(span_matches_search("Ещё один", "еще"));
    assert!(!span_matches_search("Привет", "мир"));
    assert!(!span_matches_search("Привет", ""));
}

#[test]
fn test_pdf_view_scale_discretization() {
    use crate::app::pdf_view::PdfViewState;
    use crate::settings::PdfSettings;

    let state = PdfViewState::default();
    let mut settings = PdfSettings::default();

    settings.fit_mode = "Original".to_string();
    assert_eq!(state.compute_target_scale(&settings), 2.0);

    settings.fit_mode = "FitWidth".to_string();
    assert_eq!(state.compute_target_scale(&settings), 2.5);

    settings.fit_mode = "FitWindow".to_string();
    assert_eq!(state.compute_target_scale(&settings), 2.25);

    settings.fit_mode = "Custom".to_string();
    settings.zoom_percent = 100.0;
    let s1 = state.compute_target_scale(&settings);
    assert!((s1 * 4.0).fract() < 1e-4);

    settings.zoom_percent = 104.0;
    let s2 = state.compute_target_scale(&settings);
    assert_eq!(s1, s2);
}

#[test]
fn test_pdf_view_clear_cache_and_document_generation_isolation() {
    use crate::app::pdf_view::PdfViewState;

    let mut state = PdfViewState::default();
    assert_eq!(state.doc_generation, 0);

    state.page_input = "42".to_string();
    state.clear_cache();
    assert_eq!(state.doc_generation, 1);
    assert_eq!(state.total_texture_bytes, 0);
    assert!(state.textures.is_empty());
    assert!(state.pending_pages.is_empty());
    assert!(state.page_text_spans.is_empty());
    assert!(!state.selection_dirty);
    assert!(state.selected_text.is_empty());

    state.clear_cache();
    assert_eq!(state.doc_generation, 2);
}

#[test]
fn test_pdf_selection_state_and_dirty_flag() {
    use crate::app::pdf_view::PdfViewState;
    use eframe::egui::Pos2;

    let mut state = PdfViewState::default();
    assert!(!state.selection_dirty);
    assert!(state.selected_text.is_empty());

    state.selection_start = Some(Pos2::new(10.0, 10.0));
    state.selection_current = Some(Pos2::new(50.0, 50.0));
    state.selection_dirty = true;
    state.selected_text = "Sample extracted text".to_string();

    state.clear_cache();
    assert!(state.selection_start.is_none());
    assert!(state.selection_current.is_none());
    assert!(!state.selection_dirty);
    assert!(state.selected_text.is_empty());
}

#[test]
fn test_pdf_transform_span_bbox_all_rotations() {
    use crate::app::paged_common::transform_span_bbox;
    use eframe::egui;
    let rect =
        egui::Rect::from_min_max(egui::Pos2::new(100.0, 200.0), egui::Pos2::new(300.0, 600.0));
    let bbox = [0.1, 0.2, 0.5, 0.4];

    let r0 = transform_span_bbox(bbox, 0, rect);
    assert!((r0.min.x - 120.0).abs() < 1e-3);
    assert!((r0.min.y - 280.0).abs() < 1e-3);
    assert!((r0.max.x - 200.0).abs() < 1e-3);
    assert!((r0.max.y - 360.0).abs() < 1e-3);

    let r90 = transform_span_bbox(bbox, 90, rect);
    assert!((r90.min.x - 220.0).abs() < 1e-3);
    assert!((r90.max.x - 260.0).abs() < 1e-3);
    assert!((r90.min.y - 240.0).abs() < 1e-3);
    assert!((r90.max.y - 400.0).abs() < 1e-3);

    let r180 = transform_span_bbox(bbox, 180, rect);
    assert!((r180.min.x - 200.0).abs() < 1e-3);
    assert!((r180.max.x - 280.0).abs() < 1e-3);
    assert!((r180.min.y - 440.0).abs() < 1e-3);
    assert!((r180.max.y - 520.0).abs() < 1e-3);

    let r270 = transform_span_bbox(bbox, 270, rect);
    assert!((r270.min.x - 140.0).abs() < 1e-3);
    assert!((r270.max.x - 180.0).abs() < 1e-3);
    assert!((r270.min.y - 400.0).abs() < 1e-3);
    assert!((r270.max.y - 560.0).abs() < 1e-3);
}
