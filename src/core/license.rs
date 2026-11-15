pub use super::license_codec::{
    LicenseClaims, LicenseError, verify_license_key_with_verifying_keys,
};
#[cfg(all(test, feature = "enable_tests"))]
pub use super::license_sign::sign_license_key;
use std::time::{SystemTime, UNIX_EPOCH};

pub const PUBLIC_VERIFYING_KEY_BYTES: [u8; 32] = [
    0xc0, 0x50, 0xc5, 0x63, 0x7a, 0x44, 0xfa, 0x86, 0x29, 0xff, 0xf3, 0xcc, 0xcc, 0xe2, 0x30, 0x0c,
    0xb3, 0x62, 0xa6, 0x3d, 0x99, 0xd9, 0x5f, 0xc5, 0x41, 0x45, 0x26, 0x6f, 0x43, 0x32, 0x44, 0x5a,
];

pub const DEVELOPER_VERIFYING_KEY_BYTES: [u8; 32] = [
    0x47, 0xe4, 0x9e, 0x52, 0xda, 0xfe, 0x00, 0xdd, 0x02, 0xf7, 0xe2, 0x0c, 0x50, 0xa6, 0xb4, 0xc7,
    0x46, 0xfb, 0x6b, 0x4b, 0xe0, 0x37, 0x01, 0x7c, 0x6f, 0x4d, 0x79, 0x34, 0x52, 0xe0, 0xbf, 0x2e,
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

    #[cfg(all(test, feature = "enable_tests"))]
    pub fn is_trial_active(&self) -> bool {
        matches!(self, Self::TrialActive { .. })
    }

    pub fn is_trial_expired(&self) -> bool {
        matches!(self, Self::TrialExpired)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialError {
    AlreadyStarted,
}

pub fn verify_license_key(key: &str) -> Result<LicenseClaims, LicenseError> {
    verify_license_key_with_verifying_keys(
        key,
        &[&PUBLIC_VERIFYING_KEY_BYTES, &DEVELOPER_VERIFYING_KEY_BYTES],
    )
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
