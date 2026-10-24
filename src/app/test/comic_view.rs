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
