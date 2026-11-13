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

#[test]
fn normalized_search_index_preserves_utf8_boundaries_across_case_shifts() {
    use crate::core::NormalizedSearchIndex;

    let text = "İstanbul ẞß ΣΑ И слово";
    let idx = NormalizedSearchIndex::new(text);

    let word_ranges = idx.find_ranges("слово");
    assert_eq!(word_ranges.len(), 1);
    assert_eq!(&text[word_ranges[0].0..word_ranges[0].1], "слово");

    let sharp_ranges = idx.find_ranges("ß");
    assert_eq!(
        sharp_ranges
            .iter()
            .map(|&(s, e)| &text[s..e])
            .collect::<Vec<_>>(),
        vec!["ẞ", "ß"]
    );

    let sigma_ranges = idx.find_ranges("σ");
    assert_eq!(sigma_ranges.len(), 1);
    assert_eq!(&text[sigma_ranges[0].0..sigma_ranges[0].1], "Σ");
}

#[test]
fn normalized_search_index_tracks_greek_final_sigma_and_snippet_offsets() {
    use crate::core::NormalizedSearchIndex;

    let greek = "ΟΣ ΑΛΦΑΣ";
    let idx = NormalizedSearchIndex::new(greek);
    let sigma_ranges = idx.find_ranges("ς");
    assert_eq!(
        sigma_ranges
            .iter()
            .map(|&(s, e)| &greek[s..e])
            .collect::<Vec<_>>(),
        vec!["Σ", "Σ"]
    );

    let doc_text = format!("{} начало важный_маркер конец", "İ".repeat(25));
    let doc = TxtDocument::from_text(&doc_text, "Doc");
    let hits = ReaderCore::search_in_document(&doc, "важный_маркер");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains("важный_маркер"));
}
