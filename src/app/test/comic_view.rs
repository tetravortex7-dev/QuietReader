use crate::app::comic_view::{ComicViewState, comic_pages_to_show, texture_byte_size};
use crate::settings::ComicSettings;

#[test]
fn texture_byte_size_accounts_for_rgba_pixels() {
    assert_eq!(texture_byte_size(100, 200), 80_000);
    assert_eq!(texture_byte_size(0, 200), 0);
}

#[test]
fn texture_byte_size_saturates_instead_of_overflowing() {
    assert_eq!(texture_byte_size(u32::MAX, u32::MAX), usize::MAX);
}

#[test]
fn double_page_navigation_shows_cover_alone_then_keeps_spreads() {
    let mut state = ComicViewState::default();
    let settings = ComicSettings {
        page_layout: "Double".to_string(),
        reading_mode: "Comic".to_string(),
        ..Default::default()
    };

    assert_eq!(state.current_page, 0);
    state.next_page(6, &settings);
    assert_eq!(state.current_page, 1);
    state.next_page(6, &settings);
    assert_eq!(state.current_page, 3);
    state.next_page(6, &settings);
    assert_eq!(state.current_page, 5);
    state.next_page(6, &settings);
    assert_eq!(state.current_page, 5);

    state.prev_page(&settings);
    assert_eq!(state.current_page, 3);
    state.prev_page(&settings);
    assert_eq!(state.current_page, 1);
    state.prev_page(&settings);
    assert_eq!(state.current_page, 0);
}

#[test]
fn double_page_spread_shows_cover_alone() {
    assert_eq!(comic_pages_to_show(0, 6, true, false), vec![0]);
    assert_eq!(comic_pages_to_show(1, 6, true, false), vec![1, 2]);
    assert_eq!(comic_pages_to_show(1, 6, true, true), vec![2, 1]);
    assert_eq!(comic_pages_to_show(5, 6, true, false), vec![5]);
    assert_eq!(comic_pages_to_show(5, 7, true, false), vec![5, 6]);
    assert_eq!(comic_pages_to_show(4, 6, false, false), vec![4]);
}

#[test]
fn webtoon_jump_lock_preserves_target_page_until_user_scrolls() {
    let mut state = ComicViewState::default();
    state.go_to_page(12);
    assert_eq!(state.current_page, 12);
    assert_eq!(state.pending_target_page, Some(12));

    let target_y = ComicViewState::webtoon_target_scroll_y(12, 20, 800.0);
    assert_eq!(target_y, 12.0 * (800.0 + 8.0));

    let (page_after_jump, lock) =
        ComicViewState::resolve_webtoon_visible_page(12, Some(11), 20, true, None, target_y);
    assert_eq!(page_after_jump, 12);
    assert_eq!(lock, Some((12, target_y)));

    let (page_next_frame, lock2) = ComicViewState::resolve_webtoon_visible_page(
        page_after_jump,
        Some(11),
        20,
        false,
        lock,
        target_y,
    );
    assert_eq!(page_next_frame, 12);
    assert_eq!(lock2, Some((12, target_y)));

    let (page_after_scroll, lock3) = ComicViewState::resolve_webtoon_visible_page(
        page_next_frame,
        Some(10),
        20,
        false,
        lock2,
        target_y - 200.0,
    );
    assert_eq!(page_after_scroll, 10);
    assert_eq!(lock3, None);
}

#[test]
fn webtoon_next_and_prev_page_set_pending_target() {
    let mut state = ComicViewState::default();
    let settings = ComicSettings {
        reading_mode: "Webtoon".to_string(),
        ..Default::default()
    };

    state.next_page(10, &settings);
    assert_eq!(state.current_page, 1);
    assert_eq!(state.pending_target_page, Some(1));

    state.prev_page(&settings);
    assert_eq!(state.current_page, 0);
    assert_eq!(state.pending_target_page, Some(0));
}
