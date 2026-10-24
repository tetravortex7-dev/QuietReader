use crate::app::search_view::SearchState;

#[test]
fn schedule_search_waits_for_debounce_before_starting() {
    let mut state = SearchState::default();
    state.query = "hello".to_string();
    let ctx = eframe::egui::Context::default();

    state.schedule_search(&ctx);

    assert!(state.is_searching);
    assert!(state.hits.is_empty());
    assert!(state.last_searched_query.is_empty());
}

#[test]
fn schedule_search_clears_state_for_empty_query() {
    let mut state = SearchState::default();
    state.query = "temporary".to_string();
    state.hits.push(crate::core::SearchHit {
        chapter_index: 1,
        paragraph_index: Some(2),
        page_index: None,
        snippet: "temporary".to_string(),
    });
    state.current_hit_index = 1;
    state.last_searched_query = "temporary".to_string();
    let ctx = eframe::egui::Context::default();

    state.query.clear();
    state.schedule_search(&ctx);

    assert!(!state.is_searching);
    assert!(state.hits.is_empty());
    assert_eq!(state.current_hit_index, 0);
    assert!(state.last_searched_query.is_empty());
}
