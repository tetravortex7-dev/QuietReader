use crate::core::license::{
    LicenseClaims, LicenseError, LicenseStatus, ProFeature, SECONDS_PER_DAY, TrialError,
    sign_license_key, start_trial, verify_license_key, verify_license_key_with_verifying_key,
};
use ed25519_dalek::SigningKey;

const VALID_KEY: &str = "QR1.7b22656d61696c223a227265616465724071756965747265616465722e617070222c226973737565645f61745f756e6978223a313730303030303030307d.066c69a2d8f01300708695e73c1d9942bd66108aa40a3df49b0eaa77151992b7beaa7c459120a61f8f38ba40ef0cde812599a6df90bcb5979c8ea4dbc7a10309";
const VALID_PAYLOAD_BASE64: &str =
    "eyJlbWFpbCI6InJlYWRlckBxdWlldHJlYWRlci5hcHAiLCJpc3N1ZWRfYXRfdW5peCI6MTcwMDAwMDAwMH0=";
const VALID_SIGNATURE_BASE64: &str =
    "BmxpotjwEwBwhpXnPB2ZQr1mEIqkCj30mw6qdxUZkre+qnxFkSCmH484ukDvDN6BJZmm35C8tZecjqTbx6EDCQ==";

#[test]
fn valid_ed25519_key_is_accepted() {
    let claims = verify_license_key(VALID_KEY).expect("valid key");
    assert_eq!(claims.email, "reader@quietreader.app");
    assert_eq!(claims.issued_at_unix, 1_700_000_000);
}

#[test]
fn base64_encoded_key_is_accepted() {
    let key = format!("QR1.{VALID_PAYLOAD_BASE64}.{VALID_SIGNATURE_BASE64}");
    let claims = verify_license_key(&key).expect("valid base64 key");
    assert_eq!(claims.email, "reader@quietreader.app");
}

#[test]
fn sign_license_key_roundtrip() {
    let claims = LicenseClaims {
        email: "owner@quietreader.app".to_string(),
        issued_at_unix: 1_700_000_000,
    };
    let signing_key_bytes = [7u8; 32];
    let key = sign_license_key(&claims, &signing_key_bytes);

    let parts = key.split('.').collect::<Vec<_>>();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "QR1");
    assert!(parts[1].bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(parts[2].len(), 128);
    assert!(parts[2].bytes().all(|byte| byte.is_ascii_hexdigit()));

    let verifying_key_bytes = SigningKey::from_bytes(&signing_key_bytes)
        .verifying_key()
        .to_bytes();
    let verified = verify_license_key_with_verifying_key(&key, &verifying_key_bytes)
        .expect("signed key verifies");
    assert_eq!(verified, claims);
}

#[test]
fn corrupted_signature_is_rejected() {
    let (prefix, _) = VALID_KEY.rsplit_once('.').expect("key parts");
    let corrupted = format!("{prefix}.{}", "00".repeat(64));
    assert_eq!(
        verify_license_key(&corrupted),
        Err(LicenseError::InvalidSignature)
    );
}

#[test]
fn malformed_key_is_rejected() {
    assert_eq!(
        verify_license_key("QR1.bad"),
        Err(LicenseError::InvalidFormat)
    );
    assert_eq!(
        verify_license_key("not-a-license"),
        Err(LicenseError::InvalidFormat)
    );
}

#[test]
fn trial_start_is_one_time() {
    let mut started_at = None;
    start_trial(&mut started_at, 1_000).expect("first trial start");
    assert_eq!(started_at, Some(1_000));
    assert_eq!(
        start_trial(&mut started_at, 2_000),
        Err(TrialError::AlreadyStarted)
    );
    assert_eq!(started_at, Some(1_000));
}

#[test]
fn trial_is_active_on_day_thirteen() {
    let started_at = 1_000u64;
    let status =
        LicenseStatus::from_stored(None, Some(started_at), started_at + 13 * SECONDS_PER_DAY);
    match status {
        LicenseStatus::TrialActive {
            days_left,
            expires_at_unix,
        } => {
            assert_eq!(days_left, 1);
            assert_eq!(expires_at_unix, started_at + 14 * SECONDS_PER_DAY);
        }
        other => panic!("unexpected status: {other:?}"),
    }
}

#[test]
fn trial_expires_on_day_fifteen() {
    let started_at = 1_000u64;
    let status =
        LicenseStatus::from_stored(None, Some(started_at), started_at + 15 * SECONDS_PER_DAY);
    assert_eq!(status, LicenseStatus::TrialExpired);
}

#[test]
fn trial_expires_at_the_boundary() {
    let started_at = 1_000u64;
    let status =
        LicenseStatus::from_stored(None, Some(started_at), started_at + 14 * SECONDS_PER_DAY);
    assert_eq!(status, LicenseStatus::TrialExpired);
}

#[test]
fn activated_key_unlocks_all_pro_features() {
    let status = LicenseStatus::from_stored(Some(VALID_KEY), None, 0);
    match &status {
        LicenseStatus::ProActivated {
            email,
            issued_at_unix,
        } => {
            assert_eq!(email, "reader@quietreader.app");
            assert_eq!(*issued_at_unix, 1_700_000_000);
        }
        other => panic!("unexpected status: {other:?}"),
    }
    for feature in ProFeature::ALL {
        assert!(status.can_use(feature));
    }
}

#[test]
fn free_and_expired_statuses_lock_pro_features() {
    let free = LicenseStatus::Free;
    let expired = LicenseStatus::TrialExpired;
    for feature in ProFeature::ALL {
        assert!(!free.can_use(feature));
        assert!(!expired.can_use(feature));
    }
}

#[test]
fn trial_status_unlocks_all_pro_features() {
    let status = LicenseStatus::TrialActive {
        days_left: 14,
        expires_at_unix: 14 * SECONDS_PER_DAY,
    };
    for feature in ProFeature::ALL {
        assert!(status.can_use(feature));
    }
}

#[test]
fn invalid_stored_key_falls_back_to_trial() {
    let status = LicenseStatus::from_stored(Some("QR1.invalid"), Some(1_000), 1_000);
    assert!(status.is_trial_active());
}
