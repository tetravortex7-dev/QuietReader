use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::read_limited;

const CATALOG_URL: &str = "https://freedict.org/freedict-database.json";
const CATALOG_LIMIT: u64 = 4 * 1024 * 1024;
const CACHE_FILE: &str = "catalog.json";
const CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub headwords: u64,
    pub edition: String,
    pub url: String,
    pub checksum: String,
    pub size: u64,
}

impl CatalogEntry {
    pub fn size_mb(&self) -> f32 {
        self.size as f32 / (1024.0 * 1024.0)
    }
}

#[derive(Deserialize)]
struct RawDictionary {
    name: String,
    #[serde(default)]
    headwords: Option<serde_json::Value>,
    #[serde(default)]
    edition: String,
    #[serde(default)]
    releases: Vec<RawRelease>,
}

#[derive(Deserialize)]
struct RawRelease {
    #[serde(rename = "URL", default)]
    url: String,
    #[serde(default)]
    checksum: String,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    size: Option<serde_json::Value>,
}

fn value_u64(value: Option<&serde_json::Value>) -> u64 {
    match value {
        Some(serde_json::Value::Number(number)) => number.as_u64().unwrap_or(0),
        Some(serde_json::Value::String(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

pub fn parse_catalog(json: &str) -> Result<Vec<CatalogEntry>, String> {
    let raw: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|error| format!("catalog parse: {error}"))?;
    let mut entries = Vec::new();
    for value in raw {
        let Ok(dictionary) = serde_json::from_value::<RawDictionary>(value) else {
            continue;
        };
        if dictionary.name.is_empty() {
            continue;
        }
        let Some(release) = dictionary.releases.iter().find(|release| {
            release.platform.eq_ignore_ascii_case("stardict") && !release.url.is_empty()
        }) else {
            continue;
        };
        if !release.url.starts_with("https://") || release.checksum.is_empty() {
            continue;
        }
        entries.push(CatalogEntry {
            id: dictionary.name,
            headwords: value_u64(dictionary.headwords.as_ref()),
            edition: dictionary.edition,
            url: release.url.clone(),
            checksum: release.checksum.clone(),
            size: value_u64(release.size.as_ref()),
        });
    }
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(entries)
}

pub fn cache_path(dir: &Path) -> PathBuf {
    dir.join(CACHE_FILE)
}

pub fn load_cached(dir: &Path) -> Option<Vec<CatalogEntry>> {
    let text = std::fs::read_to_string(cache_path(dir)).ok()?;
    parse_catalog(&text).ok()
}

fn cache_is_fresh(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|age| age.as_secs() < CACHE_MAX_AGE_SECS)
        .unwrap_or(false)
}

fn save_cache(dir: &Path, text: &str) {
    let path = cache_path(dir);
    let temp = path.with_extension("json.tmp");
    if std::fs::write(&temp, text).is_ok() {
        let _ = std::fs::rename(&temp, &path);
    }
}

pub fn fetch_catalog() -> Result<(String, Vec<CatalogEntry>), String> {
    let agent = crate::net::agent(Duration::from_secs(20));
    let mut response = agent
        .get(CATALOG_URL)
        .header("User-Agent", "QuietReader/0.13")
        .call()
        .map_err(|error| error.to_string())?;
    let data = read_limited(&mut response.body_mut().as_reader(), CATALOG_LIMIT)?;
    let text = String::from_utf8_lossy(&data).to_string();
    let entries = parse_catalog(&text)?;
    Ok((text, entries))
}

pub fn load_or_fetch(dir: &Path, force: bool) -> Result<Vec<CatalogEntry>, String> {
    let cache = cache_path(dir);
    if !force && cache_is_fresh(&cache) {
        if let Some(entries) = load_cached(dir) {
            return Ok(entries);
        }
    }
    match fetch_catalog() {
        Ok((text, entries)) => {
            save_cache(dir, &text);
            Ok(entries)
        }
        Err(error) => load_cached(dir).ok_or(error),
    }
}
