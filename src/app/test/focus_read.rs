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

#[test]
fn rsvp_session_tracks_words_and_time() {
    let mut session = crate::app::focus_read::RsvpSession::default();
    assert_eq!(session.words_read_in_session, 0);
    assert_eq!(session.elapsed_reading_secs, 0.0);

    session.words_read_in_session = 50;
    session.elapsed_reading_secs = 10.0;
    let actual_wpm = ((session.words_read_in_session as f32)
        / (session.elapsed_reading_secs / 60.0))
        .round() as usize;
    assert_eq!(actual_wpm, 300);
}

#[test]
fn rsvp_chapter_navigation_preserves_session_and_skips_empty_chapters() {
    use crate::app::QuietReaderApp;
    use crate::document::{BookChapter, BookMetadata, Document};
    use crate::settings::AppSettings;
    use std::path::PathBuf;
    use std::sync::Arc;

    struct MockChaptersDoc {
        chapters: Vec<BookChapter>,
        meta: BookMetadata,
    }

    impl Document for MockChaptersDoc {
        fn metadata(&self) -> &BookMetadata {
            &self.meta
        }

        fn chapters(&self) -> &[BookChapter] {
            &self.chapters
        }
    }

    let doc: Arc<dyn Document> = Arc::new(MockChaptersDoc {
        chapters: vec![
            BookChapter {
                id: None,
                title: String::new(),
                paragraphs: vec![image_token("cover").to_string()],
            },
            BookChapter {
                id: None,
                title: String::new(),
                paragraphs: vec!["One two".to_string()],
            },
            BookChapter {
                id: None,
                title: String::new(),
                paragraphs: vec![image_token("intermission").to_string()],
            },
            BookChapter {
                id: None,
                title: String::new(),
                paragraphs: vec!["Three four".to_string()],
            },
        ],
        meta: BookMetadata {
            title: "RSVP Book".to_string(),
            author: "Tester".to_string(),
        },
    });

    let mut app = QuietReaderApp::from_settings(AppSettings {
        check_updates_on_startup: false,
        dismiss_association_prompt: true,
        ..Default::default()
    });
    app.document = Some(doc);
    app.current_file_path = Some(PathBuf::from("rsvp.epub"));
    app.current_chapter = 0;

    app.start_speed_reading();
    assert!(app.focus_read.rsvp.active);
    assert_eq!(app.focus_read.rsvp.chapter, 1);
    assert_eq!(app.focus_read.rsvp.words.len(), 2);

    app.focus_read.rsvp.elapsed_reading_secs = 12.5;
    app.advance_rsvp_word();
    assert_eq!(app.focus_read.rsvp.chapter, 1);
    assert_eq!(app.focus_read.rsvp.index, 1);
    assert_eq!(app.focus_read.rsvp.words_read_in_session, 1);

    app.advance_rsvp_word();
    assert_eq!(app.focus_read.rsvp.chapter, 3);
    assert_eq!(app.focus_read.rsvp.index, 0);
    assert_eq!(app.focus_read.rsvp.words_read_in_session, 2);
    assert_eq!(app.focus_read.rsvp.elapsed_reading_secs, 12.5);

    app.step_rsvp_word(false);
    assert_eq!(app.focus_read.rsvp.chapter, 1);
    assert_eq!(app.focus_read.rsvp.index, 1);
}

#[test]
fn test_focus_read_views_fit_within_compact_and_ultrawide_bounds() {
    use crate::app::QuietReaderApp;
    use crate::document::{BookChapter, BookMetadata, Document};
    use crate::settings::AppSettings;
    use eframe::egui;
    use std::path::PathBuf;
    use std::sync::Arc;

    struct LongWordDoc {
        chapters: Vec<BookChapter>,
        meta: BookMetadata,
    }

    impl Document for LongWordDoc {
        fn metadata(&self) -> &BookMetadata {
            &self.meta
        }

        fn chapters(&self) -> &[BookChapter] {
            &self.chapters
        }
    }

    let doc: Arc<dyn Document> = Arc::new(LongWordDoc {
        chapters: vec![BookChapter {
            id: None,
            title: "Long Chapter Title For Responsive Header Wrapping Verification".to_string(),
            paragraphs: vec![
                "Превысокомногорассмотрительствующий читатель открывает компактное окно."
                    .to_string(),
            ],
        }],
        meta: BookMetadata {
            title: "Focus Book".to_string(),
            author: "Tester".to_string(),
        },
    });

    for (w, h, ppp) in [
        (360.0_f32, 320.0_f32, 1.5_f32),
        (1366.0, 768.0, 1.25),
        (3440.0, 1440.0, 2.0),
    ] {
        let mut app = QuietReaderApp::from_settings(AppSettings {
            check_updates_on_startup: false,
            dismiss_association_prompt: true,
            text_paragraph_mode: true,
            ..Default::default()
        });
        app.document = Some(Arc::clone(&doc));
        app.current_file_path = Some(PathBuf::from("focus.epub"));

        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(w, h));

        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(screen);
        raw.viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(ppp);
        let mut para_min_rect = egui::Rect::NOTHING;
        let mut out = ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                assert!(app.render_focus_reader(ui));
                para_min_rect = ui.min_rect();
            });
        });
        out.textures_delta.clear();
        assert!(
            para_min_rect.width() <= w + 1.0,
            "focus para {w}x{h}@{ppp}: rect={para_min_rect:?}"
        );

        app.start_speed_reading();
        let mut raw_rsvp = egui::RawInput::default();
        raw_rsvp.screen_rect = Some(screen);
        raw_rsvp
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(ppp);
        let mut rsvp_min_rect = egui::Rect::NOTHING;
        let mut out_rsvp = ctx.run_ui(raw_rsvp, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                assert!(app.render_focus_reader(ui));
                rsvp_min_rect = ui.min_rect();
            });
        });
        out_rsvp.textures_delta.clear();
        assert!(
            rsvp_min_rect.width() <= w + 1.0,
            "focus rsvp {w}x{h}@{ppp}: rect={rsvp_min_rect:?}"
        );
        assert!(
            rsvp_min_rect.height() <= h + 1.0,
            "focus rsvp {w}x{h}@{ppp}: rect={rsvp_min_rect:?}"
        );
    }
}
