use crate::document::registry::{
    DEFAULT_BOOK_ASSOCIATIONS, DIALOG_FILTER_GROUPS, EXTENSION_SPECS, FORMAT_REGISTRY,
    FormatBackend, FormatFilterGroup, all_picker_extensions, associable_extensions,
    document_file_stem, filter_extensions, format_badge_for_path, is_supported_document_path,
    mime_type_for_extension, normalize_extension, open_with_registry_and_encoding_and_password,
    supports_text_encoding,
};
use crate::document::{BookChapter, BookMetadata, Document, TxtDocument};
use crate::i18n::UiLanguage;
use crate::i18n::format::format_extension_description;
use crate::test::util::unique_test_dir;
use std::collections::HashSet;
use std::path::Path;

#[test]
fn test_extension_specs_are_unique_normalized_and_complete() {
    let mut seen_exts = HashSet::new();
    for spec in EXTENSION_SPECS {
        assert!(!spec.extension.is_empty());
        assert_eq!(spec.extension, normalize_extension(spec.extension));
        assert!(
            seen_exts.insert(spec.extension),
            "duplicate extension spec: {}",
            spec.extension
        );
        assert!(!spec.backends.is_empty());
        assert!(!spec.badge.is_empty());
        assert!(
            spec.badge
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()),
            "invalid badge '{}' for '{}'",
            spec.badge,
            spec.extension
        );
        assert!(spec.mime_type.contains('/'));
        assert_ne!(
            spec.mime_type, "application/octet-stream",
            "missing specific MIME type for '{}'",
            spec.extension
        );
    }

    assert_eq!(
        mime_type_for_extension("  .EPUB "),
        mime_type_for_extension("epub")
    );
    assert_eq!(mime_type_for_extension("azw3"), "application/x-mobi8-ebook");
    assert_eq!(
        mime_type_for_extension("unknown_xyz"),
        "application/octet-stream"
    );
}

#[test]
fn test_format_registry_and_extension_specs_stay_synchronized() {
    let dir = unique_test_dir("registry_sync");

    for fmt in FORMAT_REGISTRY {
        let exts = fmt.extensions();
        assert!(!exts.is_empty(), "empty extensions for {}", fmt.name());
        for &ext in exts {
            let spec = EXTENSION_SPECS
                .iter()
                .find(|s| s.extension == ext)
                .unwrap_or_else(|| {
                    panic!(
                        "format {} declares extension '{}' absent from EXTENSION_SPECS",
                        fmt.name(),
                        ext
                    )
                });
            assert!(
                spec.open_by_extension,
                "format {} declares extension '{}' marked open_by_extension=false",
                fmt.name(),
                ext
            );
        }
    }

    for spec in EXTENSION_SPECS {
        let sample_lower = dir.join(format!("sample.{}", spec.extension));
        let sample_upper = dir.join(format!("SAMPLE.{}", spec.extension.to_ascii_uppercase()));
        std::fs::write(&sample_lower, b"dummy").unwrap();
        std::fs::write(&sample_upper, b"dummy").unwrap();

        if spec.open_by_extension {
            assert!(
                FORMAT_REGISTRY
                    .iter()
                    .any(|fmt| fmt.extensions().contains(&spec.extension)),
                "open_by_extension spec '{}' missing from FORMAT_REGISTRY",
                spec.extension
            );
            assert!(
                is_supported_document_path(&sample_lower),
                "is_supported_document_path should accept {}",
                spec.extension
            );
            assert!(
                is_supported_document_path(&sample_upper),
                "is_supported_document_path should accept uppercase {}",
                spec.extension
            );
        } else {
            assert!(
                !is_supported_document_path(&sample_lower),
                "probe-only extension '{}' must not bypass probe",
                spec.extension
            );
        }
    }

    let covered_backends: Vec<FormatBackend> = EXTENSION_SPECS
        .iter()
        .flat_map(|spec| spec.backends.iter().copied())
        .collect();
    for expected in [
        FormatBackend::Pdf,
        FormatBackend::Djvu,
        FormatBackend::Fb2Zip,
        FormatBackend::Comic,
        FormatBackend::Epub,
        FormatBackend::Fb2,
        FormatBackend::Docx,
        FormatBackend::Rtf,
        FormatBackend::Doc,
        FormatBackend::Txt,
        FormatBackend::Html,
        FormatBackend::Mobi,
    ] {
        assert!(covered_backends.contains(&expected));
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_dialog_filter_groups_partition_all_picker_extensions() {
    let mut grouped = Vec::new();
    for &group in DIALOG_FILTER_GROUPS {
        let exts = filter_extensions(group);
        assert!(!exts.is_empty(), "empty filter group {group:?}");
        grouped.extend_from_slice(exts);
    }

    let grouped_set: HashSet<&str> = grouped.iter().copied().collect();
    assert_eq!(grouped_set.len(), grouped.len());

    let picker = all_picker_extensions();
    let picker_set: HashSet<&str> = picker.iter().copied().collect();
    assert_eq!(picker_set.len(), picker.len());
    assert_eq!(grouped_set, picker_set);
    assert!(picker.contains(&"dot"));
    assert!(picker.contains(&"xml"));
    assert!(picker.contains(&"fb2.zip"));
    assert!(filter_extensions(FormatFilterGroup::Fb2).contains(&"xml"));
    assert!(filter_extensions(FormatFilterGroup::Word).contains(&"dot"));
}

#[test]
fn test_associable_extensions_descriptions_and_defaults_consistent() {
    let assoc = associable_extensions();
    assert_eq!(crate::platform::supported_extensions(), assoc);
    assert!(!DEFAULT_BOOK_ASSOCIATIONS.is_empty());
    for &default_ext in DEFAULT_BOOK_ASSOCIATIONS {
        assert!(assoc.contains(&default_ext));
    }

    for lang in UiLanguage::CONCRETE {
        for spec in EXTENSION_SPECS {
            let desc = format_extension_description(spec.extension, lang);
            assert!(
                !desc.is_empty(),
                "missing description for '{}' in {:?}",
                spec.extension,
                lang
            );
        }

        let mut seen_labels = HashSet::new();
        for &ext in assoc {
            let label = format_extension_description(ext, lang);
            assert!(
                seen_labels.insert(label),
                "duplicate association checkbox label '{}' for '{}' in {:?}",
                label,
                ext,
                lang
            );
        }
    }
}

#[test]
fn test_badges_stems_and_text_encoding_dispatch() {
    let dir = unique_test_dir("registry_badges_encoding");
    let sub_dir = dir.join("manga_folder");
    std::fs::create_dir_all(&sub_dir).unwrap();

    assert_eq!(format_badge_for_path(&sub_dir), "DIR");
    assert_eq!(format_badge_for_path(Path::new("book.fb2.zip")), "FB2");
    assert_eq!(format_badge_for_path(Path::new("BOOK.FB2.ZIP")), "FB2");
    assert_eq!(format_badge_for_path(Path::new("book.xml")), "FB2");
    assert_eq!(format_badge_for_path(Path::new("scan.djv")), "DJVU");
    assert_eq!(format_badge_for_path(Path::new("notes.markdown")), "MD");
    assert_eq!(format_badge_for_path(Path::new("comic.zip")), "CBZ");
    assert_eq!(format_badge_for_path(Path::new("comic.rar")), "CBR");
    assert_eq!(format_badge_for_path(Path::new("comic.7z")), "CB7");
    assert_eq!(format_badge_for_path(Path::new("comic.tar")), "CBT");
    assert_eq!(format_badge_for_path(Path::new("no_extension")), "DOC");

    assert_eq!(
        document_file_stem(Path::new("Author - Title.fb2.zip")),
        "Author - Title"
    );
    assert_eq!(document_file_stem(Path::new("Novel.epub")), "Novel");

    for ext in ["txt", "md", "markdown", "log", "html", "htm", "xhtml"] {
        assert!(supports_text_encoding(&dir.join(format!("sample.{ext}"))));
        assert!(supports_text_encoding(
            &dir.join(format!("SAMPLE.{}", ext.to_ascii_uppercase()))
        ));
    }
    for ext in ["pdf", "epub", "fb2", "fb2.zip", "djvu", "cbz"] {
        assert!(!supports_text_encoding(&dir.join(format!("sample.{ext}"))));
    }

    let sample_text = "Привет, реестр кодировок!";
    let (cp1251, _, _) = encoding_rs::WINDOWS_1251.encode(sample_text);
    let txt_path = dir.join("encoded.txt");
    std::fs::write(&txt_path, &cp1251).unwrap();
    let txt_doc =
        open_with_registry_and_encoding_and_password(&txt_path, Some("windows-1251"), None)
            .unwrap();
    assert_eq!(txt_doc.chapters()[0].paragraphs[0], sample_text);

    let html_str = format!("<html><body><p>{sample_text}</p></body></html>");
    let (html_cp1251, _, _) = encoding_rs::WINDOWS_1251.encode(&html_str);
    let html_path = dir.join("encoded.html");
    std::fs::write(&html_path, &html_cp1251).unwrap();
    let html_doc =
        open_with_registry_and_encoding_and_password(&html_path, Some("windows-1251"), None)
            .unwrap();
    assert_eq!(html_doc.chapters()[0].paragraphs[0], sample_text);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_document_search_highlight_capability_default() {
    struct HighlightCapableDoc {
        meta: BookMetadata,
        chapters: Vec<BookChapter>,
    }

    impl Document for HighlightCapableDoc {
        fn metadata(&self) -> &BookMetadata {
            &self.meta
        }
        fn chapters(&self) -> &[BookChapter] {
            &self.chapters
        }
        fn supports_search_highlight(&self) -> bool {
            true
        }
    }

    let txt = TxtDocument::from_text("Plain body", "Title");
    assert!(!txt.supports_search_highlight());

    let doc = HighlightCapableDoc {
        meta: BookMetadata {
            title: "Scan".to_string(),
            author: String::new(),
        },
        chapters: Vec::new(),
    };
    assert!(doc.supports_search_highlight());
}
