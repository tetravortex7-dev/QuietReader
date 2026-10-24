use crate::core::{ReaderCore, normalize_search_text};
use crate::document::TxtDocument;

#[test]
fn normalize_search_text_is_case_insensitive_and_folds_yo() {
    assert_eq!(normalize_search_text("  ЁЛКА  "), "  елка  ");
    assert_eq!(normalize_search_text("Привет"), "привет");
}

#[test]
fn search_finds_yo_with_e_query() {
    let doc = TxtDocument::from_text("На ёлке висели шары", "Doc");
    let hits = ReaderCore::search_in_document(&doc, "елке");
    assert_eq!(hits.len(), 1);
}

#[test]
fn search_finds_e_with_yo_query() {
    let doc = TxtDocument::from_text("На елке висели шары", "Doc");
    let hits = ReaderCore::search_in_document(&doc, "ёлке");
    assert_eq!(hits.len(), 1);
}
