use super::license_codec::LicenseClaims;
use ed25519_dalek::{Signer, SigningKey};

pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

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
