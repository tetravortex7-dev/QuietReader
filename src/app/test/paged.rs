#[test]
fn test_paged_geometry_bounded_and_centered() {
    use crate::app::paged_common::compute_paged_geometry;
    use eframe::egui::Vec2;

    let avail_size = Vec2::new(1200.0, 800.0);
    let page_tex = Vec2::new(600.0, 900.0);

    let geom_fit_window = compute_paged_geometry(avail_size, "FitWindow", 100.0, 0, &[page_tex]);
    assert_eq!(geom_fit_window.max_pan, Vec2::ZERO);
    assert!(geom_fit_window.pad_x > 200.0);
    assert!(geom_fit_window.page_sizes[0].y <= avail_size.y);
    assert!(geom_fit_window.page_sizes[0].x <= avail_size.x);

    let geom_fit_width = compute_paged_geometry(avail_size, "FitWidth", 100.0, 0, &[page_tex]);
    assert_eq!(geom_fit_width.max_pan.x, 0.0);
    assert!(geom_fit_width.max_pan.y > 0.0);
    assert_eq!(
        geom_fit_width.max_pan.y,
        (geom_fit_width.page_sizes[0].y - avail_size.y) / 2.0
    );

    let geom_double =
        compute_paged_geometry(avail_size, "FitWindow", 100.0, 0, &[page_tex, page_tex]);
    assert_eq!(geom_double.max_pan, Vec2::ZERO);
    assert_eq!(geom_double.page_sizes.len(), 2);
    assert!(geom_double.pad_x >= 8.0);
}

#[test]
fn test_paged_common_helpers() {
    use crate::app::paged_common::{
        calculate_paged_dimensions, parse_page_input, zoom_in, zoom_out, zoom_reset,
    };
    use eframe::egui::Vec2;

    assert_eq!(parse_page_input("1", 10), Some(0));
    assert_eq!(parse_page_input("5", 10), Some(4));
    assert_eq!(parse_page_input("10", 10), Some(9));
    assert_eq!(parse_page_input("99", 10), Some(9));
    assert_eq!(parse_page_input("0", 10), Some(0));
    assert_eq!(parse_page_input("abc", 10), None);

    let mut zoom = 100.0;
    let mut fit = "FitWidth".to_string();
    let mut pan = Vec2::new(10.0, 20.0);

    zoom_in(&mut zoom, &mut fit);
    assert_eq!(zoom, 115.0);
    assert_eq!(fit, "Custom");

    zoom_out(&mut zoom, &mut fit);
    assert_eq!(zoom, 100.0);

    zoom_reset(&mut zoom, &mut fit, &mut pan);
    assert_eq!(zoom, 100.0);
    assert_eq!(fit, "Original");
    assert_eq!(pan, Vec2::ZERO);

    let (w_fit_w, _) = calculate_paged_dimensions(800.0, 1200.0, 1000.0, 800.0, "FitWidth", 100.0);
    assert_eq!(w_fit_w, 984.0);

    let (_, h_fit_h) = calculate_paged_dimensions(800.0, 1200.0, 1000.0, 800.0, "FitHeight", 100.0);
    assert_eq!(h_fit_h, 784.0);
}
