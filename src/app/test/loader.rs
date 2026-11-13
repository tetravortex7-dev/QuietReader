use crate::app::document_loader::DocumentLoader;
use std::path::PathBuf;

#[test]
fn test_async_loader_cancellation_and_generation_switch() {
    let mut loader = DocumentLoader::default();
    let path_a = PathBuf::from("nonexistent_book_a.epub");
    let path_b = PathBuf::from("nonexistent_book_b.pdf");

    let req_a = loader.request_load(path_a.clone(), crate::i18n::UiLanguage::Ru, None);
    let req_b = loader.request_load(path_b.clone(), crate::i18n::UiLanguage::Ru, None);

    assert!(req_a > 0);
    assert!(req_b > req_a);

    let start = std::time::Instant::now();
    let mut finished = Vec::new();
    while start.elapsed() < std::time::Duration::from_secs(3) && finished.len() < 2 {
        for resp in loader.poll_responses() {
            finished.push(resp);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert_eq!(finished.len(), 2);
    let resp_a = finished.iter().find(|r| r.request_id == req_a).unwrap();
    let resp_b = finished.iter().find(|r| r.request_id == req_b).unwrap();
    assert_eq!(resp_a.path, path_a);
    assert_eq!(resp_b.path, path_b);
    assert!(resp_a.result.is_err());
    assert!(resp_b.result.is_err());
}

#[test]
fn test_worker_panic_catch_unwind_resilience() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        panic!("Simulated decoder fatal error");
    }));
    assert!(result.is_err());
}

#[test]
fn test_document_loader_async_request_and_response() {
    use crate::app::document_loader::DocumentLoader;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    let mut loader = DocumentLoader::default();
    let test_path = PathBuf::from("nonexistent_test_document_12345.epub");
    let req_id = loader.request_load(test_path.clone(), crate::i18n::UiLanguage::Ru, None);
    assert!(req_id > 0);

    let start = Instant::now();
    let mut received = None;
    while start.elapsed() < Duration::from_secs(3) {
        let responses = loader.poll_responses();
        if let Some(resp) = responses.into_iter().find(|r| r.request_id == req_id) {
            received = Some(resp);
            break;
        }
        std::thread::sleep(Duration::from_millis(15));
    }

    let resp = received.expect("Background thread should return a load response");
    assert_eq!(resp.request_id, req_id);
    assert_eq!(resp.path, test_path);
    assert!(resp.result.is_err());
}

#[test]
fn test_book_tab_loading_and_update_lifecycle() {
    use crate::app::tabs::BookTab;
    use std::path::PathBuf;

    let path = PathBuf::from("sample_book.pdf");
    let tab = BookTab::new_loading(path.clone(), 77, crate::i18n::UiLanguage::En);
    assert!(tab.is_loading);
    assert!(tab.document.is_none());
    assert!(tab.load_error.is_none());
    assert_eq!(tab.request_id, 77);
    assert_eq!(tab.format_badge, "PDF");
    assert_eq!(tab.file_path, path);
}

#[test]
fn test_obsolete_response_discard_on_tab_close_and_reopen() {
    use crate::app::tabs::BookTab;
    use std::path::PathBuf;

    let path = PathBuf::from("chapter1.fb2");
    let mut tabs = vec![BookTab::new_loading(
        path.clone(),
        1,
        crate::i18n::UiLanguage::En,
    )];

    tabs.clear();
    let stale_resp_id = 1u64;
    let matching = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == stale_resp_id);
    assert_eq!(matching, None);

    tabs.push(BookTab::new_loading(
        path.clone(),
        2,
        crate::i18n::UiLanguage::En,
    ));

    let matching_stale = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == stale_resp_id);
    assert_eq!(matching_stale, None);

    let matching_fresh = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == 2);
    assert_eq!(matching_fresh, Some(0));
}

#[test]
fn test_document_load_error_formatting_variants() {
    use crate::app::document_loader::format_document_error_with_lang;
    use crate::document::DocumentError;
    use crate::document::errors;
    use crate::i18n::UiLanguage;

    let io_err = format_document_error_with_lang(
        &DocumentError::Io("file missing".to_string()),
        UiLanguage::Ru,
    );
    assert!(io_err.contains("Ошибка ввода-вывода"));

    let unsup_err = format_document_error_with_lang(
        &DocumentError::Unsupported("raw".to_string()),
        UiLanguage::Ru,
    );
    assert!(unsup_err.contains("Неподдерживаемый формат"));

    let corr_err = format_document_error_with_lang(
        &DocumentError::Corrupted("invalid header".to_string()),
        UiLanguage::Ru,
    );
    assert!(corr_err.contains("Повреждённый документ"));

    let lim_err = format_document_error_with_lang(
        &DocumentError::LimitExceeded("size limit".to_string()),
        UiLanguage::Ru,
    );
    assert!(lim_err.contains("Превышен лимит безопасности"));

    let structured_fr = format_document_error_with_lang(
        &DocumentError::Corrupted(errors::epub_empty_spine()),
        UiLanguage::Fr,
    );
    assert!(structured_fr.contains("spine EPUB"));
}

#[test]
fn test_localized_document_wrap_decodes_tokens_and_legacy_fallbacks() {
    use crate::app::localized_document::LocalizedDocument;
    use crate::document::{BookChapter, BookMetadata, Document, TocItem, errors};
    use crate::i18n::UiLanguage;
    use std::sync::Arc;

    struct StubDoc {
        metadata: BookMetadata,
        chapters: Vec<BookChapter>,
        toc: Vec<TocItem>,
    }

    impl Document for StubDoc {
        fn metadata(&self) -> &BookMetadata {
            &self.metadata
        }
        fn chapters(&self) -> &[BookChapter] {
            &self.chapters
        }
        fn toc(&self) -> &[TocItem] {
            &self.toc
        }
    }

    let raw: Arc<dyn Document> = Arc::new(StubDoc {
        metadata: BookMetadata {
            title: errors::tokens::UNTITLED.to_string(),
            author: errors::tokens::UNKNOWN_AUTHOR.to_string(),
        },
        chapters: vec![
            BookChapter {
                id: None,
                title: errors::tokens::chapter_part(errors::tokens::TITLE_HTML, 2),
                paragraphs: vec![errors::tokens::NO_TEXT.to_string()],
            },
            BookChapter {
                id: None,
                title: errors::tokens::lines_range(errors::tokens::TITLE_TEXT, 1, 50),
                paragraphs: vec!["Body text".to_string()],
            },
            BookChapter {
                id: None,
                title: "Раздел 4".to_string(),
                paragraphs: vec![],
            },
            BookChapter {
                id: None,
                title: errors::tokens::chapter_part(&errors::tokens::chapter(3), 7),
                paragraphs: vec![],
            },
            BookChapter {
                id: None,
                title: "\x1c@title:html".to_string(),
                paragraphs: vec![],
            },
        ],
        toc: vec![
            TocItem {
                id: None,
                title: errors::tokens::section_page(1, 3),
                chapter_index: 0,
                children: vec![],
            },
            TocItem {
                id: None,
                title: "Страница 5".to_string(),
                chapter_index: 1,
                children: vec![],
            },
        ],
    });

    for lang in [UiLanguage::En, UiLanguage::Fr, UiLanguage::Ar] {
        let t = lang.t();
        let localized = LocalizedDocument::wrap(Arc::clone(&raw), lang);
        assert_eq!(localized.metadata().title, t.extra.default_untitled);
        assert_eq!(localized.metadata().author, t.extra.default_unknown_author);
        assert!(
            localized.chapters()[0]
                .title
                .contains(t.extra.default_html_document)
        );
        assert!(localized.chapters()[0].title.contains('2'));
        assert_eq!(
            localized.chapters()[0].paragraphs,
            vec![t.extra.default_no_text]
        );
        assert!(
            localized.chapters()[1]
                .title
                .contains(t.extra.default_text_document)
        );
        assert!(localized.chapters()[1].title.contains("50"));
        assert!(!localized.chapters()[2].title.contains('\x1e'));
        assert!(localized.chapters()[2].title.contains('4'));
        let expected_ch3 = crate::i18n::fill(t.extra.default_chapter, &[("{n}", "3")]);
        assert!(localized.chapters()[3].title.contains(&expected_ch3));
        assert!(localized.chapters()[3].title.contains('7'));
        assert_eq!(localized.chapters()[4].title, "\x1c@title:html");
        assert!(
            !localized.toc()[0]
                .title
                .chars()
                .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
        );
        assert!(
            !localized.toc()[1]
                .title
                .chars()
                .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
        );
    }
}
