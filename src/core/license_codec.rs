use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

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

pub fn verify_license_key_with_verifying_keys(
    key: &str,
    verifying_keys: &[&[u8; 32]],
) -> Result<LicenseClaims, LicenseError> {
    let (payload_bytes, signature_bytes) = split_license_key(key)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| LicenseError::InvalidSignature)?;

    let mut matched = false;
    let mut had_valid_key = false;
    for key_bytes in verifying_keys {
        let Ok(verifying_key) = VerifyingKey::from_bytes(key_bytes) else {
            continue;
        };
        had_valid_key = true;
        if verifying_key
            .verify_strict(&payload_bytes, &signature)
            .is_ok()
        {
            matched = true;
            break;
        }
    }

    if !had_valid_key {
        return Err(LicenseError::InvalidPublicKey);
    }
    if !matched {
        return Err(LicenseError::InvalidSignature);
    }

    let claims: LicenseClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| LicenseError::InvalidPayload)?;
    if claims.email.trim().is_empty() {
        return Err(LicenseError::InvalidPayload);
    }
    Ok(claims)
}

pub fn split_license_key(key: &str) -> Result<(Vec<u8>, Vec<u8>), LicenseError> {
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

pub fn decode_hex(value: &str) -> Option<Vec<u8>> {
    let cleaned = value.trim();
    if cleaned.len() % 2 != 0 {
        return None;
    }
    let bytes = cleaned.as_bytes();
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
