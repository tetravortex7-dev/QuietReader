use crate::app::library_view::{ShelfMove, shelf_move_target};

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
