use crate::app::library_view::cover_cache_needs_eviction;

#[test]
fn cover_cache_evicts_at_texture_limit() {
    assert!(!cover_cache_needs_eviction(255, 0, 1));
    assert!(cover_cache_needs_eviction(256, 0, 1));
}

#[test]
fn cover_cache_evicts_at_byte_limit() {
    assert!(cover_cache_needs_eviction(0, 128 * 1024 * 1024, 1));
    assert!(!cover_cache_needs_eviction(0, 128 * 1024 * 1024 - 1, 1));
}
