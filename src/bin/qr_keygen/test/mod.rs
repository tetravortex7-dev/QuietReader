use super::{
    Command, format_rust_key_array, issue_lifetime_token, parse_args, parse_key_32,
    write_signing_seed,
};
use crate::license_codec::{LicenseError, verify_license_key_with_verifying_keys};
use ed25519_dalek::SigningKey;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_test_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("quietreader_keygen_{name}_{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create test temp dir");
    dir
}

#[test]
fn issue_lifetime_token_creates_verifiable_qr1_key() {
    let seed = [11u8; 32];
    let token =
        issue_lifetime_token(&seed, "author@quietreader.app", 1_750_000_000).expect("issue token");
    assert!(token.starts_with("QR1."));

    let verifying = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    let claims =
        verify_license_key_with_verifying_keys(&token, &[&verifying]).expect("verify token");
    assert_eq!(claims.email, "author@quietreader.app");
    assert_eq!(claims.issued_at_unix, 1_750_000_000);

    let wrong_pub = SigningKey::from_bytes(&[99u8; 32])
        .verifying_key()
        .to_bytes();
    assert_eq!(
        verify_license_key_with_verifying_keys(&token, &[&wrong_pub]),
        Err(LicenseError::InvalidSignature)
    );
}

#[test]
fn issue_rejects_empty_email() {
    let seed = [5u8; 32];
    assert!(issue_lifetime_token(&seed, "   ", 100).is_err());
}

#[test]
fn parse_key_32_validates_length_and_hex() {
    let hex_ok = "01".repeat(32);
    assert_eq!(
        parse_key_32(&format!("  {hex_ok}\n"), "k").unwrap(),
        [1u8; 32]
    );
    assert!(parse_key_32("0102", "k").is_err());
    assert!(parse_key_32(&"zz".repeat(32), "k").is_err());
}

#[test]
fn write_signing_seed_refuses_overwrite_without_force() {
    let dir = temp_test_dir("no_overwrite");
    let path = dir.join("dev.key");
    let seed = [42u8; 32];
    write_signing_seed(&path, &seed, false).expect("first write");
    assert!(write_signing_seed(&path, &[7u8; 32], false).is_err());
    write_signing_seed(&path, &[7u8; 32], true).expect("overwrite with force");
    let content = std::fs::read_to_string(&path).expect("read back");
    assert_eq!(parse_key_32(&content, "k").unwrap(), [7u8; 32]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn format_rust_key_array_matches_literal_syntax() {
    let bytes = [0xabu8; 32];
    let formatted = format_rust_key_array(&bytes);
    assert!(formatted.starts_with('['));
    assert!(formatted.ends_with(']'));
    assert_eq!(formatted.matches("0xab").count(), 32);
}

#[test]
fn parse_args_handles_issue_and_verify() {
    let issue_args: Vec<String> = [
        "issue",
        "--email",
        "dev@example.com",
        "--issued-at",
        "12345",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    match parse_args(&issue_args).expect("parse issue") {
        Command::Issue {
            email,
            issued_at_unix,
            ..
        } => {
            assert_eq!(email, "dev@example.com");
            assert_eq!(issued_at_unix, Some(12345));
        }
        _ => panic!("expected Issue command"),
    }
}
