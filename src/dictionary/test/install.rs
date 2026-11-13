use crate::dictionary::install::{decompress_xz_limited, sanitize_pack_id};
use std::io::Cursor;

#[test]
fn sanitize_pack_id_blocks_path_traversal() {
    assert_eq!(sanitize_pack_id("eng-rus"), Some("eng-rus".to_string()));
    assert_eq!(sanitize_pack_id("pack_01"), Some("pack_01".to_string()));
    assert_eq!(sanitize_pack_id("../secret"), None);
    assert_eq!(sanitize_pack_id("..\\secret"), None);
    assert_eq!(sanitize_pack_id("foo/bar"), None);
    assert_eq!(sanitize_pack_id("C:drive"), None);
    assert_eq!(sanitize_pack_id(".."), None);
    assert_eq!(sanitize_pack_id(""), None);
}

#[test]
fn decompress_xz_limited_enforces_output_budget() {
    let payload = vec![b'A'; 4096];
    let mut compressed = Vec::new();
    lzma_rs::xz_compress(&mut Cursor::new(&payload), &mut compressed).expect("compress xz");

    let mut out_ok = Vec::new();
    let written = decompress_xz_limited(
        &mut Cursor::new(&compressed),
        &mut out_ok,
        payload.len() as u64,
    )
    .expect("within exact limit");
    assert_eq!(written, payload.len() as u64);
    assert_eq!(out_ok, payload);

    let mut out_err = Vec::new();
    let err = decompress_xz_limited(
        &mut Cursor::new(&compressed),
        &mut out_err,
        (payload.len() - 1) as u64,
    );
    assert!(err.is_err());
    assert!(out_err.len() < payload.len());
}
