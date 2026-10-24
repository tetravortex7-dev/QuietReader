use crate::document::{
    BookChapter, NOTES_CHAPTER_ID, NOTES_CHAPTER_TITLE, NoteLocation, build_notes_appendix,
    locate_notes_in_chapters,
};
use std::collections::HashMap;

#[test]
fn test_build_notes_appendix_orders_and_locates() {
    let mut chapters = vec![BookChapter {
        id: None,
        title: "Text".to_string(),
        paragraphs: vec!["One{{note:b}} two{{note:a}}".to_string()],
    }];
    let mut notes = HashMap::new();
    notes.insert("a".to_string(), "Note A".to_string());
    notes.insert("b".to_string(), "Note B".to_string());

    let locations = build_notes_appendix(&mut chapters, &notes);
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[1].id.as_deref(), Some(NOTES_CHAPTER_ID));
    assert_eq!(chapters[1].title, NOTES_CHAPTER_TITLE);
    assert!(chapters[1].paragraphs[0].contains("Note B"));
    assert!(chapters[1].paragraphs[1].contains("Note A"));
    assert_eq!(
        locations.get("b"),
        Some(&NoteLocation {
            chapter_index: 1,
            paragraph_index: 0,
        })
    );
    assert_eq!(
        locations.get("a"),
        Some(&NoteLocation {
            chapter_index: 1,
            paragraph_index: 1,
        })
    );
}

#[test]
fn test_build_notes_appendix_skips_empty_notes() {
    let mut chapters = vec![BookChapter {
        id: None,
        title: "Text".to_string(),
        paragraphs: vec!["Body".to_string()],
    }];
    let locations = build_notes_appendix(&mut chapters, &HashMap::new());
    assert!(locations.is_empty());
    assert_eq!(chapters.len(), 1);
}

#[test]
fn test_locate_notes_in_chapters_finds_text() {
    let chapters = vec![
        BookChapter {
            id: None,
            title: "Chapter".to_string(),
            paragraphs: vec!["Some text".to_string()],
        },
        BookChapter {
            id: None,
            title: "Notes".to_string(),
            paragraphs: vec!["[1] Note body with words".to_string()],
        },
    ];
    let mut notes = HashMap::new();
    notes.insert("n1".to_string(), "1. Note body with words".to_string());

    let locations = locate_notes_in_chapters(&chapters, &notes);
    assert_eq!(
        locations.get("n1"),
        Some(&NoteLocation {
            chapter_index: 1,
            paragraph_index: 0,
        })
    );
}
