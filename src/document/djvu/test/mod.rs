use crate::document::DjvuDocument;
use crate::*;
use std::path::Path;

#[test]
fn test_corrupted_and_truncated_djvu_handling() {
    let dummy_path = Path::new("test.djvu");

    let res_empty = DjvuDocument::from_bytes(Vec::new(), dummy_path);
    assert!(res_empty.is_err());

    let res_short = DjvuDocument::from_bytes(vec![0x41, 0x54], dummy_path);
    assert!(res_short.is_err());

    let res_garbage = DjvuDocument::from_bytes(b"RANDOM_NON_DJVU_DATA_BYTES".to_vec(), dummy_path);
    assert!(res_garbage.is_err());

    let res_bad_form =
        DjvuDocument::from_bytes(b"AT&TXXXX\x00\x00\x00\x04DJVU".to_vec(), dummy_path);
    assert!(res_bad_form.is_err());

    let res_trunc = DjvuDocument::from_bytes(b"AT&TFORM".to_vec(), dummy_path);
    assert!(res_trunc.is_err());
}

#[test]
fn test_djvu_corrupted_header_and_short_bytes() {
    use document::DjvuDocument;
    use std::path::Path;

    let res_empty = DjvuDocument::from_bytes(vec![], Path::new("dummy.djvu"));
    assert!(res_empty.is_err());

    let res_short = DjvuDocument::from_bytes(vec![1, 2, 3], Path::new("dummy.djvu"));
    assert!(res_short.is_err());

    let res_garbage =
        DjvuDocument::from_bytes(b"NOT_A_DJVU_FILE_AT_ALL".to_vec(), Path::new("dummy.djvu"));
    assert!(res_garbage.is_err());

    let res_trunc = DjvuDocument::from_bytes(
        b"AT&TFORM\x00\x00\x00\x04DJVU".to_vec(),
        Path::new("dummy.djvu"),
    );
    assert!(res_trunc.is_err());
}

#[test]
fn test_djvu_bookmark_page_resolution() {
    use document::djvu::resolve_djvu_bookmark_page;
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert("intro.djvu".to_string(), 0);
    map.insert("ch1.djvu".to_string(), 1);
    map.insert("ch2.djvu".to_string(), 2);
    map.insert("глава.djvu".to_string(), 3);

    assert_eq!(resolve_djvu_bookmark_page("#intro.djvu", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("#ch1.djvu", &map, 10), 1);
    assert_eq!(resolve_djvu_bookmark_page("#ch2", &map, 10), 2);
    assert_eq!(
        resolve_djvu_bookmark_page("#%D0%B3%D0%BB%D0%B0%D0%B2%D0%B0.djvu", &map, 10),
        3
    );
    assert_eq!(resolve_djvu_bookmark_page("#1", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("#+5", &map, 10), 4);
    assert_eq!(resolve_djvu_bookmark_page("#page0006", &map, 10), 5);
    assert_eq!(resolve_djvu_bookmark_page("#pg_8", &map, 10), 7);
    assert_eq!(resolve_djvu_bookmark_page("#p3", &map, 10), 2);
    assert_eq!(resolve_djvu_bookmark_page("#9999", &map, 10), 9);
    assert_eq!(resolve_djvu_bookmark_page("#0", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("", &map, 10), 0);
    assert_eq!(resolve_djvu_bookmark_page("#unknown_target", &map, 10), 0);
}

#[test]
fn test_djvu_percent_decode() {
    use document::djvu::percent_decode_djvu;
    assert_eq!(percent_decode_djvu("hello%20world"), "hello world");
    assert_eq!(percent_decode_djvu("%2B5"), "+5");
    assert_eq!(percent_decode_djvu("%D1%82%D0%B5%D1%81%D1%82"), "тест");
    assert_eq!(percent_decode_djvu("clean"), "clean");
}

#[test]
fn test_djvu_thread_safety_send_sync() {
    use document::DjvuDocument;
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DjvuDocument>();
}
