use crate::app::QuietReaderApp;
use crate::app::commands::next_unique_bookmark_id;
use crate::app::tabs::BookTab;
use crate::core::{Bookmark, BookmarkColor, BookmarkKind, Command};
use crate::document::txt::TxtDocument;
use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, DocumentViewKind, PageContent,
};
use crate::settings::AppSettings;
use crate::test::util::unique_test_dir;
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct MockPagedDoc {
    kind: DocumentViewKind,
    pages: usize,
    meta: BookMetadata,
}

impl MockPagedDoc {
    fn new(kind: DocumentViewKind, pages: usize, title: &str) -> Arc<dyn Document> {
        Arc::new(Self {
            kind,
            pages,
            meta: BookMetadata {
                title: title.to_string(),
                author: "Tester".to_string(),
            },
        })
    }
}

impl PageContent for MockPagedDoc {
    fn page_count(&self) -> usize {
        self.pages
    }

    fn page_name(&self, index: usize) -> &str {
        let _ = index;
        ""
    }

    fn load_page_image(&self, _index: usize) -> Result<DecodedImage, DocumentError> {
        Ok(DecodedImage {
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        })
    }
}

impl Document for MockPagedDoc {
    fn metadata(&self) -> &BookMetadata {
        &self.meta
    }

    fn chapters(&self) -> &[BookChapter] {
        &[]
    }

    fn view_kind(&self) -> DocumentViewKind {
        self.kind
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }
}

fn test_app() -> QuietReaderApp {
    let settings = AppSettings {
        check_updates_on_startup: false,
        dismiss_association_prompt: true,
        ..Default::default()
    };
    QuietReaderApp::from_settings(settings)
}

#[test]
fn test_save_active_to_tab_isolates_page_by_view_kind() {
    let mut app = test_app();
    let text_doc: Arc<dyn Document> =
        Arc::new(TxtDocument::from_text("Chapter 1\nBody text", "TextBook"));
    let pdf_doc = MockPagedDoc::new(DocumentViewKind::Pdf, 10, "PdfBook");
    let comic_doc = MockPagedDoc::new(DocumentViewKind::Comic, 12, "ComicBook");

    app.tabs = vec![
        BookTab::new(PathBuf::from("a.txt"), text_doc, 0, 0.0, 0),
        BookTab::new(PathBuf::from("b.pdf"), pdf_doc, 0, 0.0, 0),
        BookTab::new(PathBuf::from("c.cbz"), comic_doc, 0, 0.0, 0),
    ];

    app.active_tab = 0;
    app.apply_tab_state(0);
    app.comic_view.current_page = 7;
    app.pdf_view.current_page = 5;
    app.save_active_to_tab();
    assert_eq!(app.tabs[0].comic_page, 0);

    app.activate_tab(1);
    assert_eq!(app.tabs[0].comic_page, 0);
    app.pdf_view.current_page = 4;
    app.comic_view.current_page = 9;
    app.save_active_to_tab();
    assert_eq!(app.tabs[1].comic_page, 4);

    app.activate_tab(2);
    app.comic_view.current_page = 8;
    app.pdf_view.current_page = 2;
    app.save_active_to_tab();
    assert_eq!(app.tabs[2].comic_page, 8);
}

#[test]
fn test_close_inactive_tab_preserves_active_tts_and_reader_state() {
    let mut app = test_app();
    let doc_a: Arc<dyn Document> =
        Arc::new(TxtDocument::from_text("Ch1\nHello\n\nCh2\nWorld", "A"));
    let doc_b: Arc<dyn Document> = Arc::new(TxtDocument::from_text("Other", "B"));

    app.tabs = vec![
        BookTab::new(PathBuf::from("a.txt"), doc_a, 0, 0.0, 0),
        BookTab::new(PathBuf::from("b.txt"), doc_b, 0, 0.0, 0),
    ];
    app.active_tab = 0;
    app.apply_tab_state(0);
    app.current_chapter = 1;
    app.current_scroll_y = 250.0;
    app.pdf_view.current_page = 3;
    app.comic_view.current_page = 4;
    app.tts.show_panel = true;

    app.close_tab(1);
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.active_tab, 0);
    assert!(app.document.is_some());
    assert_eq!(app.current_chapter, 1);
    assert_eq!(app.current_scroll_y, 250.0);
    assert!(app.tts.show_panel);

    app.close_tab(0);
    assert!(app.tabs.is_empty());
    assert!(app.document.is_none());
    assert!(app.current_file_path.is_none());
    assert_eq!(app.current_chapter, 0);
    assert_eq!(app.pdf_view.current_page, 0);
    assert_eq!(app.comic_view.current_page, 0);
    assert!(!app.tts.show_panel);
}

#[test]
fn test_activate_tab_same_index_and_switching_resets_stale_inputs() {
    let mut app = test_app();
    let doc_a: Arc<dyn Document> = Arc::new(TxtDocument::from_text("Alpha", "Alpha"));
    let tab_b = BookTab::new(
        PathBuf::from("b.cbz"),
        MockPagedDoc::new(DocumentViewKind::Comic, 20, "B"),
        0,
        0.0,
        6,
    );
    app.tabs = vec![
        BookTab::new(PathBuf::from("a.txt"), doc_a, 0, 0.0, 0),
        tab_b,
    ];
    app.active_tab = 0;
    app.apply_tab_state(0);
    app.comic_view.page_input = "99".to_string();
    app.focus_read.paragraph_initialized = true;
    app.focus_read.rsvp.active = true;

    app.activate_tab(0);
    assert_eq!(app.comic_view.page_input, "99");
    assert!(app.focus_read.paragraph_initialized);

    app.activate_tab(1);
    assert_eq!(app.active_tab, 1);
    assert_eq!(app.comic_view.page_input, "7");
    assert!(app.pdf_view.page_input.is_empty());
    assert!(!app.focus_read.paragraph_initialized);
    assert!(!app.focus_read.rsvp.active);
    assert_eq!(app.comic_view.current_page, 6);
    assert_eq!(app.comic_view.pending_target_page, Some(6));
}

#[test]
fn test_load_error_clears_active_document_and_isolates_tabs() {
    let dir = unique_test_dir("tab_load_error_isolation");
    let valid_path = dir.join("valid.txt");
    let missing_path = dir.join("missing.fb2");
    std::fs::write(&valid_path, "Chapter 1\nValid text content").unwrap();

    let mut app = test_app();
    let valid_doc: Arc<dyn Document> =
        Arc::new(TxtDocument::from_text("Chapter 1\nValid", "Valid"));
    app.tabs = vec![BookTab::new(valid_path.clone(), valid_doc, 0, 0.0, 0)];
    app.active_tab = 0;
    app.apply_tab_state(0);
    assert!(app.document.is_some());

    app.load_file(missing_path);
    assert_eq!(app.tabs.len(), 2);
    assert_eq!(app.active_tab, 1);

    let deadline = Instant::now() + Duration::from_secs(3);
    while app.tabs[1].is_loading && Instant::now() < deadline {
        app.poll_document_loader();
        std::thread::sleep(Duration::from_millis(15));
    }

    assert!(!app.tabs[1].is_loading);
    assert!(app.tabs[1].load_error.is_some());
    assert!(app.tabs[1].document.is_none());
    assert!(app.document.is_none());

    assert!(app.tabs[0].document.is_some());
    assert!(app.tabs[0].load_error.is_none());
    app.activate_tab(0);
    assert!(app.document.is_some());
    assert_eq!(app.current_file_path, Some(valid_path));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_commands_zoom_dispatch_pdf_home_end_and_bookmark_ids() {
    let mut app = test_app();
    let ctx = egui::Context::default();

    let base_font = app.font_size;
    let base_pdf_zoom = app.settings.pdf.zoom_percent;
    let base_comic_zoom = app.settings.comic.zoom_percent;

    app.document = Some(Arc::new(TxtDocument::from_text("Hello world", "Text")));
    app.execute_command(&ctx, &Command::ZoomIn, DocumentViewKind::Text);
    assert!(app.font_size > base_font);
    assert_eq!(app.settings.pdf.zoom_percent, base_pdf_zoom);
    assert_eq!(app.settings.comic.zoom_percent, base_comic_zoom);
    app.execute_command(&ctx, &Command::ResetZoom, DocumentViewKind::Text);
    assert_eq!(app.font_size, 19.0);

    app.document = Some(MockPagedDoc::new(DocumentViewKind::Comic, 5, "Comic"));
    app.execute_command(&ctx, &Command::ZoomIn, DocumentViewKind::Comic);
    assert!(app.settings.comic.zoom_percent > base_comic_zoom);
    assert_eq!(app.settings.pdf.zoom_percent, base_pdf_zoom);
    app.execute_command(&ctx, &Command::ResetZoom, DocumentViewKind::Comic);
    assert_eq!(app.settings.comic.zoom_percent, 100.0);

    app.document = Some(MockPagedDoc::new(DocumentViewKind::Pdf, 5, "Pdf"));
    app.execute_command(&ctx, &Command::ZoomIn, DocumentViewKind::Pdf);
    assert!(app.settings.pdf.zoom_percent > base_pdf_zoom);
    assert_eq!(app.settings.comic.zoom_percent, 100.0);
    app.execute_command(&ctx, &Command::End, DocumentViewKind::Pdf);
    assert_eq!(app.pdf_view.current_page, 4);
    assert_eq!(app.pdf_view.pending_target_page, Some(4));
    app.execute_command(&ctx, &Command::Home, DocumentViewKind::Pdf);
    assert_eq!(app.pdf_view.current_page, 0);
    assert_eq!(app.pdf_view.pending_target_page, Some(0));

    let existing = vec![
        Bookmark {
            id: "1000_2_0".to_string(),
            book_path: PathBuf::from("book.epub"),
            chapter: 2,
            scroll_y: 0.0,
            label: "A".to_string(),
            text: String::new(),
            note: String::new(),
            color: BookmarkColor::Yellow,
            kind: BookmarkKind::Bookmark,
            timestamp: 1000,
            progress_percent: 0.0,
            page_number: None,
        },
        Bookmark {
            id: "1000_2_1".to_string(),
            book_path: PathBuf::from("book.epub"),
            chapter: 2,
            scroll_y: 10.0,
            label: "B".to_string(),
            text: String::new(),
            note: String::new(),
            color: BookmarkColor::Yellow,
            kind: BookmarkKind::Bookmark,
            timestamp: 1000,
            progress_percent: 0.0,
            page_number: None,
        },
    ];
    let after_delete = vec![existing[1].clone()];
    let new_id = next_unique_bookmark_id(&after_delete, 1000, 2);
    assert!(after_delete.iter().all(|b| b.id != new_id));
}
