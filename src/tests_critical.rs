use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::app::document_loader::DocumentLoader;
use crate::document::{
    is_safe_archive_path, read_bounded, DjvuDocument, DocumentError, EpubDocument,
    Fb2Document, PdfDocument, MAX_ARCHIVE_ENTRIES, MAX_SINGLE_ENTRY_BYTES, MAX_TOTAL_UNPACK_BYTES,
};
use crate::settings::AppSettings;

fn unique_test_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("quietreader_crit_{prefix}_{pid}_{id}"));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[test]
fn test_read_bounded_limits_and_overflow_protection() {
    let data = vec![0xAAu8; 4096];
    let mut total_unpacked = 0u64;

    let ok_read = read_bounded(&data[..100], 500, &mut total_unpacked, 1000);
    assert!(ok_read.is_ok());
    assert_eq!(ok_read.unwrap().len(), 100);
    assert_eq!(total_unpacked, 100);

    let err_single = read_bounded(&data[..], 200, &mut total_unpacked, 10000);
    assert!(matches!(err_single, Err(DocumentError::LimitExceeded(_))));

    let mut total_unpacked = 900u64;
    let err_total = read_bounded(&data[..200], 1000, &mut total_unpacked, 1000);
    assert!(matches!(err_total, Err(DocumentError::LimitExceeded(_))));

    let mut total_unpacked = u64::MAX - 5;
    let err_overflow = read_bounded(&data[..10], 100, &mut total_unpacked, u64::MAX);
    assert!(matches!(err_overflow, Err(DocumentError::LimitExceeded(_))));
}

#[test]
fn test_is_safe_archive_path_comprehensive() {
    assert!(!is_safe_archive_path(""));
    assert!(!is_safe_archive_path(" "));
    assert!(!is_safe_archive_path(".."));
    assert!(!is_safe_archive_path("../test.txt"));
    assert!(!is_safe_archive_path("..\\test.txt"));
    assert!(!is_safe_archive_path("foo/../bar"));
    assert!(!is_safe_archive_path("foo/..\\bar"));
    assert!(!is_safe_archive_path("/etc/shadow"));
    assert!(!is_safe_archive_path("\\Windows\\System32"));
    assert!(!is_safe_archive_path("C:\\autoexec.bat"));
    assert!(!is_safe_archive_path("D:file.txt"));
    assert!(!is_safe_archive_path("\\\\server\\share\\file"));
    assert!(!is_safe_archive_path("//server/share/file"));
    assert!(!is_safe_archive_path("%2e%2e/evil.txt"));
    assert!(!is_safe_archive_path("..%2fevil.txt"));
    assert!(!is_safe_archive_path("%2e%2e%5cevil.txt"));
    assert!(!is_safe_archive_path("%2fetc/passwd"));
    assert!(!is_safe_archive_path("%5cWindows"));

    assert!(is_safe_archive_path("chapter1.xhtml"));
    assert!(is_safe_archive_path("images/cover.jpg"));
    assert!(is_safe_archive_path("OEBPS/text/part1.xhtml"));
    assert!(is_safe_archive_path("a/b/c/d/e.png"));
}

#[test]
fn test_corrupted_and_truncated_djvu_handling() {
    let dummy_path = Path::new("test.djvu");

    let res_empty = DjvuDocument::from_bytes(Vec::new(), dummy_path);
    assert!(res_empty.is_err());

    let res_short = DjvuDocument::from_bytes(vec![0x41, 0x54], dummy_path);
    assert!(res_short.is_err());

    let res_garbage = DjvuDocument::from_bytes(b"RANDOM_NON_DJVU_DATA_BYTES".to_vec(), dummy_path);
    assert!(res_garbage.is_err());

    let res_bad_form = DjvuDocument::from_bytes(b"AT&TXXXX\x00\x00\x00\x04DJVU".to_vec(), dummy_path);
    assert!(res_bad_form.is_err());

    let res_trunc = DjvuDocument::from_bytes(b"AT&TFORM".to_vec(), dummy_path);
    assert!(res_trunc.is_err());
}

#[test]
fn test_corrupted_and_truncated_pdf_handling() {
    let dummy_path = Path::new("test.pdf");

    let res_empty = PdfDocument::from_bytes(Vec::new(), dummy_path);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_short = PdfDocument::from_bytes(vec![b'%', b'P'], dummy_path);
    assert!(matches!(res_short, Err(DocumentError::Corrupted(_))));

    let res_garbage = PdfDocument::from_bytes(b"THIS_IS_DEFINITELY_NOT_A_PDF".to_vec(), dummy_path);
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let res_header_only = PdfDocument::from_bytes(b"%PDF-1.7\n%%EOF".to_vec(), dummy_path);
    assert!(matches!(res_header_only, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_corrupted_and_malformed_epub_handling() {
    let res_empty = EpubDocument::from_bytes(&[]);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_garbage = EpubDocument::from_bytes(b"PK\x03\x04\x00\x00BROKEN_ZIP_DATA");
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("unrelated.txt", options).unwrap();
        std::io::Write::write_all(&mut zip, b"no container xml").unwrap();
        zip.finish().unwrap();
    }
    let res_no_container = EpubDocument::from_bytes(&zip_buf.into_inner());
    assert!(matches!(res_no_container, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_corrupted_and_malformed_fb2_handling() {
    let res_empty = Fb2Document::from_bytes(&[]);
    assert!(matches!(res_empty, Err(DocumentError::Corrupted(_))));

    let res_garbage = Fb2Document::from_bytes(b"PLAIN TEXT WITHOUT ANY XML TAGS");
    assert!(matches!(res_garbage, Err(DocumentError::Corrupted(_))));

    let res_invalid_xml = Fb2Document::from_bytes(b"<FictionBook><body><section><title><p>Broken");
    assert!(matches!(res_invalid_xml, Err(DocumentError::Corrupted(_))));
}

#[test]
fn test_reading_progress_persistence_and_normalization() {
    let mut settings = AppSettings::default();
    let p1 = PathBuf::from("C:\\Books\\Library\\Novel.epub");
    let p1_lower = PathBuf::from("c:\\books\\library\\novel.epub");
    let p2 = PathBuf::from("D:\\Comics\\Manga_Vol1.cbz");

    settings.record_book_progress(&p1, "Novel Title", "Author Name", 4, 320.0);
    assert_eq!(settings.get_book_progress(&p1), Some((4, 320.0)));
    assert_eq!(settings.get_book_progress(&p1_lower), Some((4, 320.0)));

    settings.record_book_progress(&p2, "Manga Title", "Mangaka", 12, 0.0);
    assert_eq!(settings.get_book_progress(&p2), Some((12, 0.0)));

    assert_eq!(settings.reading_progress.len(), 2);
    assert_eq!(settings.recent_books.len(), 2);

    settings.recent_books.clear();
    assert_eq!(settings.recent_books.len(), 0);
    assert_eq!(settings.get_book_progress(&p1), Some((4, 320.0)));
    assert_eq!(settings.get_book_progress(&p2), Some((12, 0.0)));
}

#[test]
fn test_atomic_settings_save_and_backup_safety() {
    let temp_dir = unique_test_dir("save_safety");
    let settings_path = temp_dir.join("settings.json");
    let bak_path = temp_dir.join("settings.json.bak");
    let corrupted_path = temp_dir.join("settings.corrupted");

    let mut settings = AppSettings::default();
    settings.font_size = 28.0;
    settings.theme = "Sepia".to_string();

    let save_res = settings.save_to_path(&settings_path);
    assert!(save_res.is_ok());
    assert!(settings_path.exists());

    let loaded = AppSettings::load_from_path(&settings_path);
    assert_eq!(loaded.font_size, 28.0);
    assert_eq!(loaded.theme, "Sepia");

    std::fs::write(&bak_path, r#"{"schema_version":2,"font_size":25.0,"theme":"Dark"}"#).unwrap();
    std::fs::write(&settings_path, "{ broken invalid json...").unwrap();

    let recovered = AppSettings::load_from_path(&settings_path);
    assert_eq!(recovered.font_size, 25.0);
    assert_eq!(recovered.theme, "Dark");
    assert!(corrupted_path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_async_loader_cancellation_and_generation_switch() {
    let mut loader = DocumentLoader::default();
    let path_a = PathBuf::from("nonexistent_book_a.epub");
    let path_b = PathBuf::from("nonexistent_book_b.pdf");

    let req_a = loader.request_load(path_a.clone());
    let req_b = loader.request_load(path_b.clone());

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
fn test_resource_limits_constants() {
    assert!(MAX_ARCHIVE_ENTRIES >= 1000);
    assert!(MAX_SINGLE_ENTRY_BYTES >= 10 * 1024 * 1024);
    assert!(MAX_TOTAL_UNPACK_BYTES >= 50 * 1024 * 1024);
    assert!(MAX_TOTAL_UNPACK_BYTES > MAX_SINGLE_ENTRY_BYTES);
}
