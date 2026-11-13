use crate::app::localized_document::LocalizedDocument;
use crate::document::{
    BookChapter, BookMetadata, Document, DocumentViewKind, PageContent, TextContent, TocItem,
};
use crate::i18n::UiLanguage;
use std::sync::Arc;

struct MockDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    view_kind: DocumentViewKind,
    custom_toc: bool,
    can_copy: bool,
}

impl Document for MockDocument {
    fn metadata(&self) -> &BookMetadata {
        &self.metadata
    }

    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }

    fn toc(&self) -> &[TocItem] {
        &self.toc
    }

    fn view_kind(&self) -> DocumentViewKind {
        self.view_kind
    }

    fn can_copy(&self) -> bool {
        self.can_copy
    }

    fn has_custom_toc(&self) -> bool {
        self.custom_toc
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        None
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        None
    }

    fn footnote(&self, id: &str) -> Option<&str> {
        match id {
            "note_1" => Some("Footnote body"),
            _ => None,
        }
    }
}

fn mock_doc(view_kind: DocumentViewKind, paragraph: &str) -> Arc<dyn Document> {
    Arc::new(MockDocument {
        metadata: BookMetadata {
            title: "Без названия".to_string(),
            author: "Неизвестный автор".to_string(),
        },
        chapters: vec![BookChapter {
            id: Some("chap_1".to_string()),
            title: "Раздел 2".to_string(),
            paragraphs: vec![paragraph.to_string()],
        }],
        toc: vec![TocItem {
            id: Some("toc_1".to_string()),
            title: "Раздел 2 (страница 5)".to_string(),
            chapter_index: 0,
            children: vec![TocItem {
                id: Some("toc_2".to_string()),
                title: "Страница 9".to_string(),
                chapter_index: 1,
                children: Vec::new(),
            }],
        }],
        view_kind,
        custom_toc: true,
        can_copy: true,
    })
}

#[test]
fn localizes_generated_text_document_titles() {
    let doc = LocalizedDocument::wrap(
        mock_doc(DocumentViewKind::Text, "Документ не содержит текста."),
        UiLanguage::En,
    );
    assert_eq!(doc.metadata().title, "Untitled");
    assert_eq!(doc.metadata().author, "Unknown author");
    assert_eq!(doc.chapters()[0].title, "Section 2");
    assert_eq!(
        doc.chapters()[0].paragraphs[0],
        "The document contains no text."
    );
    assert_eq!(doc.toc()[0].title, "Section 2 (page 5)");
    assert_eq!(doc.toc()[0].children[0].title, "Page 9");
    assert!(doc.has_custom_toc());
}

#[test]
fn localizes_generated_page_document_titles() {
    let doc = LocalizedDocument::wrap(
        mock_doc(DocumentViewKind::Comic, "Всего страниц: 7"),
        UiLanguage::En,
    );
    assert_eq!(doc.chapters()[0].paragraphs[0], "Total pages: 7");
}

#[test]
fn keeps_authored_text_unchanged() {
    let doc = LocalizedDocument::wrap(
        mock_doc(DocumentViewKind::Text, "Страница 9"),
        UiLanguage::En,
    );
    assert_eq!(doc.chapters()[0].paragraphs[0], "Страница 9");
}

#[test]
fn localizes_mobi_and_txt_generated_titles() {
    let inner: Arc<dyn Document> = Arc::new(MockDocument {
        metadata: BookMetadata {
            title: "Документ MOBI".to_string(),
            author: "Неизвестный автор".to_string(),
        },
        chapters: vec![
            BookChapter {
                id: None,
                title: "Глава 3 (часть 2)".to_string(),
                paragraphs: Vec::new(),
            },
            BookChapter {
                id: None,
                title: "Текстовый документ (строки 10–20)".to_string(),
                paragraphs: Vec::new(),
            },
        ],
        toc: Vec::new(),
        view_kind: DocumentViewKind::Text,
        custom_toc: false,
        can_copy: true,
    });
    let doc = LocalizedDocument::wrap(inner, UiLanguage::En);
    assert_eq!(doc.metadata().title, "MOBI document");
    assert!(!doc.has_custom_toc());
    assert_eq!(doc.chapters()[0].title, "Chapter 3 (part 2)");
    assert_eq!(doc.chapters()[1].title, "Text document (lines 10–20)");
}

#[test]
fn localizes_part_suffix_for_authored_titles() {
    let inner: Arc<dyn Document> = Arc::new(MockDocument {
        metadata: BookMetadata {
            title: "Документ RTF".to_string(),
            author: "Неизвестный автор".to_string(),
        },
        chapters: vec![BookChapter {
            id: None,
            title: "Chapter One (часть 2)".to_string(),
            paragraphs: Vec::new(),
        }],
        toc: Vec::new(),
        view_kind: DocumentViewKind::Text,
        custom_toc: false,
        can_copy: true,
    });
    let doc = LocalizedDocument::wrap(inner, UiLanguage::En);
    assert_eq!(doc.metadata().title, "RTF Document");
    assert_eq!(doc.chapters()[0].title, "Chapter One (part 2)");
}

#[test]
fn forwards_footnotes_to_the_wrapped_document() {
    let doc = LocalizedDocument::wrap(
        mock_doc(DocumentViewKind::Text, "Text with a note."),
        UiLanguage::En,
    );
    assert_eq!(doc.footnote("note_1"), Some("Footnote body"));
    assert_eq!(doc.footnote("missing_note"), None);
}

#[test]
fn localizes_notes_chapter_and_toc_title() {
    let inner: Arc<dyn Document> = Arc::new(MockDocument {
        metadata: BookMetadata {
            title: "Book".to_string(),
            author: "Author".to_string(),
        },
        chapters: vec![
            BookChapter {
                id: None,
                title: "Chapter".to_string(),
                paragraphs: Vec::new(),
            },
            BookChapter {
                id: Some(crate::document::NOTES_CHAPTER_ID.to_string()),
                title: crate::document::NOTES_CHAPTER_TITLE.to_string(),
                paragraphs: vec!["[1] Note body".to_string()],
            },
        ],
        toc: vec![TocItem {
            id: None,
            title: crate::document::NOTES_CHAPTER_TITLE.to_string(),
            chapter_index: 1,
            children: Vec::new(),
        }],
        view_kind: DocumentViewKind::Text,
        custom_toc: true,
        can_copy: true,
    });
    let doc = LocalizedDocument::wrap(inner, UiLanguage::En);
    assert_eq!(doc.chapters()[1].title, "Notes");
    assert_eq!(doc.toc()[0].title, "Notes");
    assert_eq!(doc.chapters()[1].paragraphs[0], "[1] Note body");
}

#[test]
fn forwards_can_copy_to_the_wrapped_document() {
    let restricted_inner: Arc<dyn Document> = Arc::new(MockDocument {
        metadata: BookMetadata {
            title: "Protected".to_string(),
            author: "Author".to_string(),
        },
        chapters: Vec::new(),
        toc: Vec::new(),
        view_kind: DocumentViewKind::Pdf,
        custom_toc: false,
        can_copy: false,
    });
    let restricted = LocalizedDocument::wrap(restricted_inner, UiLanguage::En);
    assert!(!restricted.can_copy());

    let allowed =
        LocalizedDocument::wrap(mock_doc(DocumentViewKind::Pdf, "Sample"), UiLanguage::En);
    assert!(allowed.can_copy());
}

#[test]
fn forwards_supports_search_highlight_to_wrapped_document() {
    struct HighlightMock {
        meta: BookMetadata,
        highlight: bool,
    }

    impl Document for HighlightMock {
        fn metadata(&self) -> &BookMetadata {
            &self.meta
        }
        fn chapters(&self) -> &[BookChapter] {
            &[]
        }
        fn supports_search_highlight(&self) -> bool {
            self.highlight
        }
    }

    let enabled = LocalizedDocument::wrap(
        Arc::new(HighlightMock {
            meta: BookMetadata {
                title: "Scan".to_string(),
                author: String::new(),
            },
            highlight: true,
        }),
        UiLanguage::En,
    );
    assert!(enabled.supports_search_highlight());

    let disabled =
        LocalizedDocument::wrap(mock_doc(DocumentViewKind::Pdf, "Sample"), UiLanguage::En);
    assert!(!disabled.supports_search_highlight());
}
