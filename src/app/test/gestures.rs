use crate::app::gestures::{SWIPE_THRESHOLD, swipe_forward};

#[test]
fn swipe_ignores_small_movements() {
    assert_eq!(swipe_forward(0.0, false), None);
    assert_eq!(swipe_forward(SWIPE_THRESHOLD - 1.0, false), None);
    assert_eq!(swipe_forward(-(SWIPE_THRESHOLD - 1.0), true), None);
}

#[test]
fn swipe_left_turns_forward_for_ltr() {
    assert_eq!(swipe_forward(-SWIPE_THRESHOLD, false), Some(true));
    assert_eq!(swipe_forward(SWIPE_THRESHOLD, false), Some(false));
}

#[test]
fn swipe_right_turns_forward_for_manga() {
    assert_eq!(swipe_forward(SWIPE_THRESHOLD, true), Some(true));
    assert_eq!(swipe_forward(-SWIPE_THRESHOLD, true), Some(false));
}
