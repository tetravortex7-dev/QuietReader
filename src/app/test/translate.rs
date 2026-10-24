use crate::app::translate::read_provider_body;
use std::io::Cursor;

#[test]
fn test_read_provider_body_accepts_exact_limit() {
    let mut data = Cursor::new(b"hello".to_vec());
    assert_eq!(read_provider_body(&mut data, 5).unwrap(), "hello");
}

#[test]
fn test_read_provider_body_rejects_oversized_response() {
    let mut data = Cursor::new(b"hello!".to_vec());
    assert!(read_provider_body(&mut data, 5).is_err());
}

#[test]
fn test_read_provider_body_rejects_invalid_utf8() {
    let mut data = Cursor::new(vec![0xff, 0xfe]);
    assert!(read_provider_body(&mut data, 10).is_err());
}
