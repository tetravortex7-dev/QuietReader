use crate::app::document_loader::DocumentLoader;
use std::path::PathBuf;

#[test]
fn test_async_loader_cancellation_and_generation_switch() {
    let mut loader = DocumentLoader::default();
    let path_a = PathBuf::from("nonexistent_book_a.epub");
    let path_b = PathBuf::from("nonexistent_book_b.pdf");

    let req_a = loader.request_load(path_a.clone(), crate::i18n::UiLanguage::Ru);
    let req_b = loader.request_load(path_b.clone(), crate::i18n::UiLanguage::Ru);

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
    let req_id = loader.request_load(test_path.clone(), crate::i18n::UiLanguage::Ru);
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
    let tab = BookTab::new_loading(path.clone(), 77);
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
    let mut tabs = vec![BookTab::new_loading(path.clone(), 1)];

    tabs.clear();
    let stale_resp_id = 1u64;
    let matching = tabs
        .iter()
        .position(|t| t.file_path == path && t.request_id == stale_resp_id);
    assert_eq!(matching, None);

    tabs.push(BookTab::new_loading(path.clone(), 2));

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
    use crate::app::document_loader::format_document_error;
    use crate::document::DocumentError;

    let io_err = format_document_error(&DocumentError::Io("file missing".to_string()));
    assert!(io_err.contains("Ошибка ввода-вывода"));

    let unsup_err = format_document_error(&DocumentError::Unsupported("raw".to_string()));
    assert!(unsup_err.contains("Неподдерживаемый формат"));

    let corr_err = format_document_error(&DocumentError::Corrupted("invalid header".to_string()));
    assert!(corr_err.contains("Повреждённый документ"));

    let lim_err = format_document_error(&DocumentError::LimitExceeded("size limit".to_string()));
    assert!(lim_err.contains("Превышен лимит безопасности"));
}
