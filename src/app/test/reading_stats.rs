use crate::app::reading_stats::is_reading_active;

#[test]
fn test_reading_active_requires_open_book_and_signal() {
    assert!(!is_reading_active(false, false, false, false, false, false));
    assert!(!is_reading_active(true, true, false, true, true, true));
    assert!(!is_reading_active(true, false, true, true, true, true));
    assert!(!is_reading_active(true, false, false, false, false, false));
    assert!(is_reading_active(true, false, false, true, false, false));
    assert!(is_reading_active(true, false, false, false, true, false));
    assert!(is_reading_active(true, false, false, false, false, true));
}
