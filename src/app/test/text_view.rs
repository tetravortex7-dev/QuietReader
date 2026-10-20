use crate::document::Fb2Document;

#[test]
fn test_search_and_copy_with_formatted_text() {
    let xml = r#"<FictionBook><body>
        <section>
            <title><p>Раздел</p></title>
            <p>Здесь есть <strong>секретное</strong> слово.</p>
        </section>
    </body></FictionBook>"#;

    let doc = Fb2Document::from_bytes(xml.as_bytes()).unwrap();
    let hits = crate::core::ReaderCore::search_in_document(&doc, "секретное");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].snippet, "Здесь есть секретное слово.");

    let full_text = crate::core::ReaderCore::extract_chapter_text(&doc, 0).unwrap();
    assert_eq!(full_text, "Раздел\n\nЗдесь есть секретное слово.");
}

#[test]
fn test_parse_formatted_spans_edge_cases() {
    let text = "5 < 10 and 10 > 5 and <b>bold</b> and unclosed <i>italic";
    let spans = crate::document::parse_formatted_spans(text);
    assert_eq!(spans[0].text, "5 < 10 and 10 > 5 and ");
    assert!(!spans[0].bold && !spans[0].italic);
    assert_eq!(spans[1].text, "bold");
    assert!(spans[1].bold && !spans[1].italic);
    assert_eq!(spans[2].text, " and unclosed ");
    assert!(!spans[2].bold && !spans[2].italic);
    assert_eq!(spans[3].text, "italic");
    assert!(!spans[3].bold && spans[3].italic);

    let stripped = crate::document::strip_formatting_tags(text);
    assert_eq!(stripped, "5 < 10 and 10 > 5 and bold and unclosed italic");
}

#[test]
fn test_text_pages_calculation_and_progress() {
    let empty_chapters: Vec<crate::document::BookChapter> = Vec::new();
    let (cur, total, pct) = crate::app::bars::calculate_text_pages(&empty_chapters, 0, 0.0, 0.0);
    assert_eq!(cur, 1);
    assert_eq!(total, 1);
    assert_eq!(pct, 0.0);

    let ch1_text = "a".repeat(1800 * 200);
    let ch2_text = "b".repeat(1800 * 299);
    let chapters = vec![
        crate::document::BookChapter {
            id: None,
            title: String::new(),
            paragraphs: vec![ch1_text],
        },
        crate::document::BookChapter {
            id: None,
            title: String::new(),
            paragraphs: vec![ch2_text],
        },
    ];

    let (cur, total, pct) = crate::app::bars::calculate_text_pages(&chapters, 0, 0.0, 1000.0);
    assert_eq!(cur, 1);
    assert_eq!(total, 499);
    assert_eq!(pct, 0.0);

    let (cur, total, _) =
        crate::app::bars::calculate_text_pages(&chapters, 1, (100.0 / 299.0) * 1000.0, 1000.0);
    assert_eq!(total, 499);
    assert_eq!(cur, 301);

    let (cur, total, pct) = crate::app::bars::calculate_text_pages(&chapters, 1, 1000.0, 1000.0);
    assert_eq!(cur, 499);
    assert_eq!(total, 499);
    assert_eq!(pct, 100.0);
}

#[test]
fn test_text_search_multi_chapter_and_title() {
    use crate::core::ReaderCore;
    use crate::document::txt::TxtDocument;

    let text = "Chapter 1\nIntroductory text here\n\nChapter 2\nSecond section with target keyword\n\nSpecial Title\nFinal content";
    let doc = TxtDocument::from_text(text, "SearchDoc");

    let hits = ReaderCore::search_in_document(&doc, "target");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].chapter_index, 0);
    assert!(hits[0].snippet.contains("target"));

    let title_hits = ReaderCore::search_in_document(&doc, "SearchDoc");
    assert!(!title_hits.is_empty());
    assert_eq!(title_hits[0].paragraph_index, None);
}

#[test]
fn test_search_state_cycle_and_scroll_estimate() {
    use crate::app::search_view::SearchState;
    use crate::core::ReaderCore;
    use crate::document::txt::TxtDocument;

    let mut state = SearchState::default();
    assert!(!state.is_open);
    state.open();
    assert!(state.is_open);
    assert!(state.should_focus);

    let doc = TxtDocument::from_text("Alpha\n\nBeta\n\nGamma\n\nDelta", "Doc");
    let y0 = ReaderCore::estimate_paragraph_scroll_y(&doc, 0, None, 18.0);
    assert_eq!(y0, 0.0);

    let y_para = ReaderCore::estimate_paragraph_scroll_y(&doc, 0, Some(2), 18.0);
    assert!(y_para >= 0.0);

    state.hits = ReaderCore::search_in_document(&doc, "Beta");
    state
        .hits
        .extend(ReaderCore::search_in_document(&doc, "Delta"));
    assert_eq!(state.hits.len(), 2);
    let first = state.current_hit().cloned();
    let second = state.next_hit().cloned();
    assert_ne!(first, second);
    let wrapped = state.prev_hit().cloned();
    assert_eq!(first, wrapped);
}

#[test]
fn test_make_safe_snippet_utf8_boundaries() {
    use crate::core::ReaderCore;

    let cyrillic = "Строка с кириллицей и множеством многобайтовых символов UTF-8 для проверки";
    for i in 0..cyrillic.len() {
        let snippet = ReaderCore::make_safe_snippet(cyrillic, i, 4);
        assert!(!snippet.is_empty());
    }
}

#[test]
fn test_text_view_virtualization_bounds_estimation() {
    let font_size = 19.0f32;
    let line_height = font_size * 1.45;
    let para_spacing = font_size * 0.4;
    let chars_per_line = (760.0 / (font_size * 0.52)).max(10.0);

    let short_text = "Short single line.";
    let char_count = short_text.chars().count();
    let num_lines = (char_count as f32 / chars_per_line).ceil().max(1.0);
    assert_eq!(num_lines, 1.0);
    let est_height = num_lines * line_height + para_spacing;
    assert!(est_height > 20.0 && est_height < 50.0);

    let view_top = 100.0f32;
    let view_bot = 800.0f32;

    let top_y_offscreen_above = 0.0f32;
    let bot_y_offscreen_above = top_y_offscreen_above + est_height;
    assert!(bot_y_offscreen_above < view_top);

    let top_y_visible = 300.0f32;
    let bot_y_visible = top_y_visible + est_height;
    assert!(!(bot_y_visible < view_top || top_y_visible > view_bot));

    let top_y_offscreen_below = 1200.0f32;
    assert!(top_y_offscreen_below > view_bot);
}
