use crate::app::focus_read::{
    collect_words, delay_factor, is_text_paragraph, nearest_text_paragraph, pivot_char_index,
    step_paragraph, text_paragraph_position,
};
use crate::document::{footnote_token, image_token};

fn paragraphs() -> Vec<String> {
    vec![
        "Первый абзац.".to_string(),
        String::new(),
        format!("Второй {} абзац с <b>выделением</b>.", footnote_token("1")),
        image_token("cover").to_string(),
        "Третий абзац!".to_string(),
    ]
}

#[test]
fn collects_words_without_markup_and_tracks_paragraphs() {
    let words = collect_words(&paragraphs());
    assert_eq!(words.len(), 8);
    assert_eq!(words[0].text, "Первый");
    assert_eq!(words[0].paragraph, 0);
    assert_eq!(words[1].text, "абзац.");
    assert!(words.iter().all(|word| !word.text.contains("{{")));
    assert!(words.iter().any(|word| word.paragraph == 2));
    assert!(words.iter().all(|word| word.paragraph != 3));
}

#[test]
fn paragraph_navigation_skips_empty_and_image_paragraphs() {
    let paragraphs = paragraphs();
    assert!(is_text_paragraph(&paragraphs[0]));
    assert!(!is_text_paragraph(&paragraphs[1]));
    assert!(!is_text_paragraph(&paragraphs[3]));
    assert_eq!(step_paragraph(&paragraphs, 0, true), Some(2));
    assert_eq!(step_paragraph(&paragraphs, 4, false), Some(2));
    assert_eq!(step_paragraph(&paragraphs, 4, true), None);
    assert_eq!(nearest_text_paragraph(&paragraphs, 0, true), Some(0));
    assert_eq!(nearest_text_paragraph(&paragraphs, 3, true), Some(4));
    assert_eq!(nearest_text_paragraph(&paragraphs, 3, false), Some(2));
}

#[test]
fn text_paragraph_position_ignores_non_text_paragraphs() {
    let paragraphs = paragraphs();
    assert_eq!(text_paragraph_position(&paragraphs, 0), (1, 3));
    assert_eq!(text_paragraph_position(&paragraphs, 2), (2, 3));
    assert_eq!(text_paragraph_position(&paragraphs, 4), (3, 3));
}

#[test]
fn pivot_and_pause_heuristics_are_stable() {
    assert_eq!(pivot_char_index(""), 0);
    assert_eq!(pivot_char_index("Я"), 0);
    assert_eq!(pivot_char_index("да"), 0);
    assert_eq!(pivot_char_index("слово"), 2);
    assert_eq!(pivot_char_index("гипотетический"), 5);
    assert!(delay_factor("конец.") > delay_factor("конец,"));
    assert!(delay_factor("конец,") > delay_factor("конец"));
    assert_eq!(delay_factor("конец"), 1.0);
    assert_eq!(delay_factor("«конец!»"), 2.0);
}
