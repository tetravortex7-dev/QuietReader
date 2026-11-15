use crate::app::library_view::rhythm::daily_goal_met;
use crate::app::library_view::shelf::compute_shelf_grid;
use crate::app::library_view::{ShelfMove, shelf_move_target};
use crate::core::library::ShelfViewMode;

#[test]
fn shelf_keyboard_movement_follows_grid_and_list_rules() {
    assert_eq!(shelf_move_target(None, 10, 4, ShelfMove::Right), Some(0));
    assert_eq!(shelf_move_target(None, 10, 4, ShelfMove::End), Some(9));
    assert_eq!(shelf_move_target(Some(0), 10, 4, ShelfMove::Right), Some(1));
    assert_eq!(shelf_move_target(Some(0), 10, 4, ShelfMove::Left), Some(0));
    assert_eq!(shelf_move_target(Some(0), 10, 4, ShelfMove::Down), Some(4));
    assert_eq!(shelf_move_target(Some(6), 10, 4, ShelfMove::Down), Some(9));
    assert_eq!(shelf_move_target(Some(6), 10, 4, ShelfMove::Up), Some(2));
    assert_eq!(shelf_move_target(Some(6), 10, 4, ShelfMove::Home), Some(0));
    assert_eq!(shelf_move_target(Some(6), 10, 4, ShelfMove::End), Some(9));
    assert_eq!(shelf_move_target(Some(0), 10, 1, ShelfMove::Down), Some(1));
    assert_eq!(
        shelf_move_target(Some(99), 10, 4, ShelfMove::Right),
        Some(9)
    );
    assert_eq!(shelf_move_target(Some(0), 0, 4, ShelfMove::Right), None);
}

#[test]
fn shelf_grid_adapts_columns_and_fills_width_across_viewports() {
    let compact = compute_shelf_grid(480.0, ShelfViewMode::Standard);
    assert_eq!(compact.cols, 3);
    assert!(compact.card_inner_w >= 112.0);
    let compact_row_w =
        compact.cols as f32 * compact.card_outer_w + (compact.cols - 1) as f32 * compact.gap;
    assert!(compact_row_w <= 480.0);

    let desktop = compute_shelf_grid(1212.0, ShelfViewMode::Standard);
    assert_eq!(desktop.cols, 6);
    assert!(desktop.card_outer_w >= 166.0);
    let desktop_row_w =
        desktop.cols as f32 * desktop.card_outer_w + (desktop.cols - 1) as f32 * desktop.gap;
    assert!(desktop_row_w <= 1212.0);
    assert!(1212.0 - desktop_row_w <= desktop.cols as f32);

    let maxi = compute_shelf_grid(1212.0, ShelfViewMode::Maxi);
    assert_eq!(maxi.cols, 4);
    assert!(maxi.card_outer_w >= 240.0);
    let maxi_row_w = maxi.cols as f32 * maxi.card_outer_w + (maxi.cols - 1) as f32 * maxi.gap;
    assert!(maxi_row_w <= 1212.0);

    let wide = compute_shelf_grid(2400.0, ShelfViewMode::Standard);
    assert_eq!(wide.cols, 6);
}

#[test]
fn daily_goal_met_requires_thirty_minutes() {
    assert!(!daily_goal_met(0));
    assert!(!daily_goal_met(1799));
    assert!(daily_goal_met(1800));
    assert!(daily_goal_met(2700));
}
