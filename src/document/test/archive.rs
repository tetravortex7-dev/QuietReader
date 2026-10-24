use crate::document::{
    DocumentError, MAX_ARCHIVE_ENTRIES, MAX_SINGLE_ENTRY_BYTES, MAX_TOTAL_UNPACK_BYTES,
    is_safe_archive_path, read_bounded,
};
use crate::test::util::unique_test_dir;
use crate::*;

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
fn test_resource_limits_constants() {
    assert!(MAX_ARCHIVE_ENTRIES >= 1000);
    assert!(MAX_SINGLE_ENTRY_BYTES >= 10 * 1024 * 1024);
    assert!(MAX_TOTAL_UNPACK_BYTES >= 50 * 1024 * 1024);
    assert!(MAX_TOTAL_UNPACK_BYTES > MAX_SINGLE_ENTRY_BYTES);
}

#[test]
fn test_archive_path_traversal_protection() {
    use crate::document::{ComicDocument, DocumentError, is_safe_archive_path};
    use std::io::Write;

    assert!(!is_safe_archive_path("../evil.png"));
    assert!(!is_safe_archive_path("ch1/../../evil.png"));
    assert!(!is_safe_archive_path("/etc/passwd"));
    assert!(!is_safe_archive_path(r"\Windows\System32\cmd.exe"));
    assert!(!is_safe_archive_path("C:/evil.png"));
    assert!(is_safe_archive_path("ch01/page_01.png"));

    let test_dir = unique_test_dir("traversal");
    let cbz_path = test_dir.join("traversal.cbz");

    {
        let file = std::fs::File::create(&cbz_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("../evil.png", options).unwrap();
        zip.write_all(b"fake_image").unwrap();
        zip.finish().unwrap();
    }

    let res = ComicDocument::from_path(&cbz_path);
    assert!(matches!(res, Err(DocumentError::Corrupted(_))));
    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_cbt_path_traversal_protection() {
    use crate::document::{ComicDocument, DocumentError};

    let test_dir = unique_test_dir("cbt_traversal");
    let cbt_path = test_dir.join("traversal.cbt");
    let payload = b"fake_image";

    {
        let file = std::fs::File::create(&cbt_path).unwrap();
        let mut builder = tar::Builder::new(file);
        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.as_old_mut().name[..11].copy_from_slice(b"../evil.png");
        header.set_cksum();
        builder.append(&header, payload.as_slice()).unwrap();
        builder.finish().unwrap();
    }

    let res = ComicDocument::from_path(&cbt_path);
    assert!(matches!(res, Err(DocumentError::Corrupted(_))));
    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn test_tar_magic_probe_detects_cbt() {
    use crate::document::registry::{ComicFormat, DocumentFormat};

    let dir = unique_test_dir("tar_probe");
    let path = dir.join("book.bin");
    {
        let file = std::fs::File::create(&path).unwrap();
        let mut builder = tar::Builder::new(file);
        let mut header = tar::Header::new_gnu();
        header.set_size(3);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "a.png", &b"abc"[..])
            .unwrap();
        builder.finish().unwrap();
    }
    assert!(ComicFormat.probe(&path));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_zip_bomb_and_bounded_read_protection() {
    use crate::document::{DocumentError, read_bounded};

    let data = vec![0u8; 2048];
    let mut total_unpacked = 0u64;

    let res_single = read_bounded(&data[..], 1024, &mut total_unpacked, 10_000);
    assert!(matches!(res_single, Err(DocumentError::LimitExceeded(_))));

    let mut total_unpacked = 9500u64;
    let res_total = read_bounded(&data[..1000], 2048, &mut total_unpacked, 10_000);
    assert!(matches!(res_total, Err(DocumentError::LimitExceeded(_))));
}

#[test]
fn test_archive_excessive_entries_protection() {
    use crate::document::{DocumentError, EpubDocument, MAX_ARCHIVE_ENTRIES};

    let mut zip_buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut zip_buf);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for i in 0..=MAX_ARCHIVE_ENTRIES {
            zip.start_file(format!("f_{i}.txt"), options).unwrap();
        }
        zip.finish().unwrap();
    }

    let res = EpubDocument::from_bytes(&zip_buf.into_inner());
    assert!(matches!(res, Err(DocumentError::LimitExceeded(_))));
}

#[test]
fn test_safe_archive_path_checks() {
    use crate::document::is_safe_archive_path;
    assert!(!is_safe_archive_path(""));
    assert!(!is_safe_archive_path("../secret"));
    assert!(!is_safe_archive_path("foo/../../secret"));
    assert!(!is_safe_archive_path("foo/../secret"));
    assert!(!is_safe_archive_path("/etc/passwd"));
    assert!(!is_safe_archive_path("C:\\windows\\system32"));
    assert!(!is_safe_archive_path("C:foo"));
    assert!(is_safe_archive_path("page01.png"));
    assert!(is_safe_archive_path("sub/folder/page01.png"));
}

#[test]
fn test_rar5_magic_and_fb2_zip_not_comic() {
    use crate::document::registry::{ComicFormat, DocumentFormat, Fb2ZipFormat};
    use std::io::Write;

    let dir = unique_test_dir("rar5_fb2zip");
    let rar4 = dir.join("book4.bin");
    let rar5 = dir.join("book5.bin");
    std::fs::write(&rar4, b"Rar!\x1a\x07\x00payload").unwrap();
    std::fs::write(&rar5, b"Rar!\x1a\x07\x01\x00payload").unwrap();
    assert!(ComicFormat.probe(&rar4));
    assert!(ComicFormat.probe(&rar5));
    let rar5_err = match document::ComicDocument::from_path(&rar5) {
        Ok(_) => panic!("RAR5 test file is not a valid archive"),
        Err(e) => e.to_string(),
    };
    assert!(rar5_err.contains("RAR"));

    let zip_path = dir.join("book.zip");
    let file = std::fs::File::create(&zip_path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("book.fb2", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(
        br#"<FictionBook><body><section><title><p>Title</p></title><p>Text</p></section></body></FictionBook>"#,
    )
    .unwrap();
    zip.finish().unwrap();

    assert!(Fb2ZipFormat.probe(&zip_path));
    assert!(!ComicFormat.probe(&zip_path));
    let doc = document::open_document(&zip_path).expect("ZIP with FB2 must open as a book");
    assert_eq!(doc.format_name(), "FB2");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_is_safe_archive_path_percent_encoding_traversal() {
    use document::is_safe_archive_path;
    assert!(is_safe_archive_path("OEBPS/ch1.xhtml"));
    assert!(is_safe_archive_path("images/cover.jpg"));
    assert!(!is_safe_archive_path("../escape.xhtml"));
    assert!(!is_safe_archive_path("%2e%2e/escape.xhtml"));
    assert!(!is_safe_archive_path("..%2fescape.xhtml"));
    assert!(!is_safe_archive_path("/absolute/path.xhtml"));
    assert!(!is_safe_archive_path("C:/Windows/system.ini"));
    assert!(!is_safe_archive_path("%2fetc/passwd"));
}
