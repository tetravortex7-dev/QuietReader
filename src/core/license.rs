use ed25519_dalek::{Signature, VerifyingKey};
#[cfg(feature = "enable_tests")]
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PUBLIC_VERIFYING_KEY_BYTES: [u8; 32] = [
    0xc0, 0x50, 0xc5, 0x63, 0x7a, 0x44, 0xfa, 0x86, 0x29, 0xff, 0xf3, 0xcc, 0xcc, 0xe2, 0x30, 0x0c,
    0xb3, 0x62, 0xa6, 0x3d, 0x99, 0xd9, 0x5f, 0xc5, 0x41, 0x45, 0x26, 0x6f, 0x43, 0x32, 0x44, 0x5a,
];

pub const TRIAL_DAYS: u64 = 14;
pub const SECONDS_PER_DAY: u64 = 86_400;
pub const TRIAL_DURATION_SECONDS: u64 = TRIAL_DAYS * SECONDS_PER_DAY;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProFeature {
    NeuralTts,
    Translation,
    OpdsPro,
    FocusTools,
    AmbientAudio,
    SmartScanProcessing,
}

impl ProFeature {
    pub const ALL: [ProFeature; 6] = [
        ProFeature::NeuralTts,
        ProFeature::Translation,
        ProFeature::OpdsPro,
        ProFeature::FocusTools,
        ProFeature::AmbientAudio,
        ProFeature::SmartScanProcessing,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseStatus {
    Free,
    TrialActive {
        days_left: u32,
        expires_at_unix: u64,
    },
    TrialExpired,
    ProActivated {
        email: String,
        issued_at_unix: u64,
    },
}

impl LicenseStatus {
    pub fn from_stored(
        license_key: Option<&str>,
        trial_started_at_unix: Option<u64>,
        now_unix: u64,
    ) -> Self {
        if let Some(key) = license_key.filter(|key| !key.trim().is_empty())
            && let Ok(claims) = verify_license_key(key)
        {
            return Self::ProActivated {
                email: claims.email,
                issued_at_unix: claims.issued_at_unix,
            };
        }

        match trial_started_at_unix {
            Some(started_at) => {
                let expires_at_unix = started_at.saturating_add(TRIAL_DURATION_SECONDS);
                if now_unix < expires_at_unix {
                    let remaining = expires_at_unix.saturating_sub(now_unix).max(1);
                    let days_left =
                        remaining.div_ceil(SECONDS_PER_DAY).min(TRIAL_DAYS).max(1) as u32;
                    Self::TrialActive {
                        days_left,
                        expires_at_unix,
                    }
                } else {
                    Self::TrialExpired
                }
            }
            None => Self::Free,
        }
    }

    pub fn can_use(&self, _feature: ProFeature) -> bool {
        matches!(self, Self::TrialActive { .. } | Self::ProActivated { .. })
    }

    pub fn is_pro_active(&self) -> bool {
        matches!(self, Self::ProActivated { .. })
    }

    pub fn is_trial_active(&self) -> bool {
        matches!(self, Self::TrialActive { .. })
    }

    pub fn is_trial_expired(&self) -> bool {
        matches!(self, Self::TrialExpired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseClaims {
    pub email: String,
    pub issued_at_unix: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseError {
    InvalidFormat,
    InvalidSignature,
    InvalidPayload,
    InvalidPublicKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialError {
    AlreadyStarted,
}

#[cfg(feature = "enable_tests")]
pub fn sign_license_key(claims: &LicenseClaims, signing_key_bytes: &[u8; 32]) -> String {
    let payload = serde_json::to_vec(claims).expect("license claims serialize");
    let signing_key = SigningKey::from_bytes(signing_key_bytes);
    let signature = signing_key.sign(&payload);
    format!(
        "QR1.{}.{}",
        encode_hex(&payload),
        encode_hex(&signature.to_bytes())
    )
}

pub fn verify_license_key(key: &str) -> Result<LicenseClaims, LicenseError> {
    verify_license_key_with_verifying_key(key, &PUBLIC_VERIFYING_KEY_BYTES)
}

pub(crate) fn verify_license_key_with_verifying_key(
    key: &str,
    verifying_key_bytes: &[u8; 32],
) -> Result<LicenseClaims, LicenseError> {
    let (payload_bytes, signature_bytes) = split_license_key(key)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| LicenseError::InvalidSignature)?;
    let verifying_key = VerifyingKey::from_bytes(verifying_key_bytes)
        .map_err(|_| LicenseError::InvalidPublicKey)?;
    verifying_key
        .verify_strict(&payload_bytes, &signature)
        .map_err(|_| LicenseError::InvalidSignature)?;
    let claims: LicenseClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| LicenseError::InvalidPayload)?;
    if claims.email.trim().is_empty() {
        return Err(LicenseError::InvalidPayload);
    }
    Ok(claims)
}

pub fn start_trial(
    trial_started_at_unix: &mut Option<u64>,
    now_unix: u64,
) -> Result<(), TrialError> {
    if trial_started_at_unix.is_some() {
        return Err(TrialError::AlreadyStarted);
    }
    *trial_started_at_unix = Some(now_unix);
    Ok(())
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn split_license_key(key: &str) -> Result<(Vec<u8>, Vec<u8>), LicenseError> {
    let trimmed = key.trim();
    let rest = trimmed
        .strip_prefix("QR1.")
        .or_else(|| trimmed.strip_prefix("QR1-"))
        .ok_or(LicenseError::InvalidFormat)?;
    let (payload, signature) = rest.split_once('.').ok_or(LicenseError::InvalidFormat)?;
    if payload.is_empty() || signature.is_empty() {
        return Err(LicenseError::InvalidFormat);
    }
    let payload_bytes = decode_component(payload).ok_or(LicenseError::InvalidFormat)?;
    let signature_bytes = decode_component(signature).ok_or(LicenseError::InvalidFormat)?;
    if payload_bytes.is_empty() || signature_bytes.len() != 64 {
        return Err(LicenseError::InvalidFormat);
    }
    Ok((payload_bytes, signature_bytes))
}

fn decode_component(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 == 0 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return decode_hex(value);
    }
    decode_base64(value)
}

#[allow(dead_code)]
fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        output.push((high << 4) | low);
    }
    Some(output)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    let mut output = Vec::with_capacity(value.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for byte in value.bytes() {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(digit);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Some(output)
}
