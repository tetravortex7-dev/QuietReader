use serde::{Deserialize, Serialize};
use std::io::Read;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_RELEASES_URL: &str = "https://github.com/tetravortex7-dev/QuietReader/releases";
pub const GITHUB_REPO_URL: &str = "https://github.com/tetravortex7-dev/QuietReader";
pub const GITHUB_API_LATEST_RELEASE: &str =
    "https://api.github.com/repos/tetravortex7-dev/QuietReader/releases/latest";
pub const RAW_CARGO_TOML_URL: &str =
    "https://raw.githubusercontent.com/tetravortex7-dev/QuietReader/master/Cargo.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub release_url: String,
    pub download_url: Option<String>,
    pub is_newer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate { version: String },
    UpdateAvailable(UpdateInfo),
    Error(String),
}

impl Default for UpdateStatus {
    fn default() -> Self {
        UpdateStatus::Idle
    }
}

pub fn parse_version_tuple(v: &str) -> Option<(u64, u64, u64)> {
    let s = v.trim().trim_start_matches(|c: char| c == 'v' || c == 'V');
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let cur = parse_version_tuple(current);
    let lat = parse_version_tuple(latest);
    match (cur, lat) {
        (Some(c), Some(l)) => l > c,
        _ => latest.trim() != current.trim() && !latest.trim().is_empty(),
    }
}

pub fn extract_version_from_cargo_toml(toml: &str) -> Option<String> {
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version") {
            if let Some((_, val)) = trimmed.split_once('=') {
                let clean = val.trim().trim_matches('"').trim_matches('\'').trim();
                if !clean.is_empty() {
                    return Some(clean.to_string());
                }
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct GitHubReleaseResponse {
    tag_name: Option<String>,
    html_url: Option<String>,
    body: Option<String>,
    assets: Option<Vec<GitHubAsset>>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: Option<String>,
    browser_download_url: Option<String>,
}

pub fn check_for_updates_sync() -> Result<UpdateInfo, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(8)))
        .build()
        .new_agent();

    let release_res = agent
        .get(GITHUB_API_LATEST_RELEASE)
        .header("User-Agent", "QuietReader-Updater")
        .call();

    if let Ok(mut resp) = release_res {
        if resp.status().is_success() {
            let mut text = String::new();
            if resp.body_mut().as_reader().read_to_string(&mut text).is_ok() {
                if let Ok(gh) = serde_json::from_str::<GitHubReleaseResponse>(&text) {
                    if let Some(tag) = gh.tag_name {
                        let is_newer = is_newer_version(CURRENT_VERSION, &tag);
                        let release_url = gh.html_url.unwrap_or_else(|| GITHUB_RELEASES_URL.to_string());
                        let download_url = gh.assets.and_then(|assets| {
                            assets.into_iter().find_map(|a| {
                                let name = a.name?;
                                if name.ends_with(".exe") || name.ends_with(".zip") || name.ends_with(".tar.gz") {
                                    a.browser_download_url
                                } else {
                                    None
                                }
                            })
                        });
                        return Ok(UpdateInfo {
                            current_version: CURRENT_VERSION.to_string(),
                            latest_version: tag,
                            release_notes: gh.body,
                            release_url,
                            download_url,
                            is_newer,
                        });
                    }
                }
            }
        }
    }

    let cargo_res = agent
        .get(RAW_CARGO_TOML_URL)
        .header("User-Agent", "QuietReader-Updater")
        .call();

    match cargo_res {
        Ok(mut resp) => {
            if resp.status().is_success() {
                let mut text = String::new();
                resp.body_mut()
                    .as_reader()
                    .read_to_string(&mut text)
                    .map_err(|e| format!("Failed to read Cargo.toml: {e}"))?;
                if let Some(ver) = extract_version_from_cargo_toml(&text) {
                    let is_newer = is_newer_version(CURRENT_VERSION, &ver);
                    Ok(UpdateInfo {
                        current_version: CURRENT_VERSION.to_string(),
                        latest_version: ver,
                        release_notes: None,
                        release_url: GITHUB_REPO_URL.to_string(),
                        download_url: None,
                        is_newer,
                    })
                } else {
                    Err("Could not find version field in remote Cargo.toml".to_string())
                }
            } else {
                Err(format!("HTTP error {}", resp.status()))
            }
        }
        Err(e) => Err(format!("Network error: {e}")),
    }
}

pub struct UpdateChecker {
    status: Arc<Mutex<UpdateStatus>>,
    tx: Sender<()>,
    pub dismiss_banner: bool,
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateChecker {
    pub fn new() -> Self {
        let status = Arc::new(Mutex::new(UpdateStatus::Idle));
        let (tx, rx) = channel::<()>();

        let status_clone = Arc::clone(&status);
        thread::spawn(move || {
            while rx.recv().is_ok() {
                {
                    if let Ok(mut st) = status_clone.lock() {
                        *st = UpdateStatus::Checking;
                    }
                }
                let result = check_for_updates_sync();
                if let Ok(mut st) = status_clone.lock() {
                    match result {
                        Ok(info) => {
                            if info.is_newer {
                                *st = UpdateStatus::UpdateAvailable(info);
                            } else {
                                *st = UpdateStatus::UpToDate {
                                    version: info.latest_version,
                                };
                            }
                        }
                        Err(err) => {
                            *st = UpdateStatus::Error(err);
                        }
                    }
                }
            }
        });

        Self {
            status,
            tx,
            dismiss_banner: false,
        }
    }

    pub fn check_now(&self) {
        let _ = self.tx.send(());
    }

    pub fn status(&self) -> UpdateStatus {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing_and_comparison() {
        assert_eq!(parse_version_tuple("0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version_tuple("v0.2.1"), Some((0, 2, 1)));
        assert_eq!(parse_version_tuple("1.0"), Some((1, 0, 0)));

        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.0", "v0.1.1"));
        assert!(is_newer_version("0.1.0", "1.0.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.2.0", "0.1.9"));
        assert!(!is_newer_version("1.0.0", "0.9.9"));
    }

    #[test]
    fn test_extract_version_from_cargo_toml() {
        let sample = r#"
[package]
name = "QuietReader"
version = "0.2.5"
edition = "2024"
"#;
        assert_eq!(extract_version_from_cargo_toml(sample), Some("0.2.5".to_string()));
    }
}
