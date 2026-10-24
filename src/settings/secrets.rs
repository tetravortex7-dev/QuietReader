use super::AppSettings;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

const DEEPL_SECRET_KEY: &str = "deepl_api_key";
const LICENSE_SECRET_KEY: &str = "license_key";
const TRIAL_SECRET_KEY: &str = "trial_started_at";

static LAST_SECRET_ERROR: Mutex<Option<String>> = Mutex::new(None);
static LAST_SECRET_SNAPSHOT: Mutex<Option<SecretSnapshot>> = Mutex::new(None);

#[derive(Clone, PartialEq, Eq, Default)]
struct SecretSnapshot {
    deepl_api_key: Option<String>,
    opds_passwords: BTreeMap<String, String>,
    license_key: Option<String>,
    trial_started_at: Option<String>,
}

impl AppSettings {
    pub(crate) fn last_secret_error() -> Option<String> {
        LAST_SECRET_ERROR
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub(super) fn set_secret_error(error: String) {
        if let Ok(mut guard) = LAST_SECRET_ERROR.lock() {
            *guard = Some(error);
        }
    }

    pub(super) fn clear_secret_error() {
        if let Ok(mut guard) = LAST_SECRET_ERROR.lock() {
            *guard = None;
        }
    }

    fn opds_secret_key(catalog_id: &str) -> String {
        format!("opds_password:{catalog_id}")
    }

    pub(super) fn extract_legacy_secret_entries(content: &str) -> Vec<(String, String)> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        if let Some(key) = value
            .get("translation")
            .and_then(|translation| translation.get("deepl_api_key"))
            .and_then(|key| key.as_str())
            && !key.is_empty()
        {
            entries.push((DEEPL_SECRET_KEY.to_string(), key.to_string()));
        }
        if let Some(catalogs) = value
            .get("opds_catalogs")
            .and_then(|catalogs| catalogs.as_array())
        {
            for catalog in catalogs {
                let Some(id) = catalog.get("id").and_then(|id| id.as_str()) else {
                    continue;
                };
                let Some(password) = catalog
                    .get("password")
                    .and_then(|password| password.as_str())
                else {
                    continue;
                };
                if !id.is_empty() && !password.is_empty() {
                    entries.push((Self::opds_secret_key(id), password.to_string()));
                }
            }
        }
        if let Some(key) = value.get("license_key").and_then(|key| key.as_str())
            && !key.is_empty()
        {
            entries.push((LICENSE_SECRET_KEY.to_string(), key.to_string()));
        }
        entries
    }

    pub(super) fn sanitize_secret_fields(content: &str) -> Option<String> {
        let mut value = match serde_json::from_str::<serde_json::Value>(content) {
            Ok(value) => value,
            Err(_) => return redact_secret_text(content),
        };
        let mut changed = false;
        if let Some(translation) = value
            .get_mut("translation")
            .and_then(|translation| translation.as_object_mut())
            && translation.remove("deepl_api_key").is_some()
        {
            changed = true;
        }
        if let Some(catalogs) = value
            .get_mut("opds_catalogs")
            .and_then(|catalogs| catalogs.as_array_mut())
        {
            for catalog in catalogs {
                if let Some(object) = catalog.as_object_mut()
                    && object.remove("password").is_some()
                {
                    changed = true;
                }
            }
        }
        if let Some(object) = value.as_object_mut()
            && object.remove("license_key").is_some()
        {
            changed = true;
        }
        if !changed {
            return None;
        }
        serde_json::to_string_pretty(&value).ok()
    }

    fn secret_snapshot(&self) -> SecretSnapshot {
        SecretSnapshot {
            deepl_api_key: if self.translation.deepl_api_key.is_empty() {
                None
            } else {
                Some(self.translation.deepl_api_key.clone())
            },
            opds_passwords: self
                .opds_catalogs
                .iter()
                .filter_map(|catalog| {
                    catalog
                        .password
                        .as_ref()
                        .filter(|password| !password.is_empty())
                        .map(|password| (catalog.id.clone(), password.clone()))
                })
                .collect(),
            license_key: if self.license_key.is_empty() {
                None
            } else {
                Some(self.license_key.clone())
            },
            trial_started_at: self.trial_started_at_unix.map(|value| value.to_string()),
        }
    }

    pub(super) fn store_secrets(&self) -> Result<(), String> {
        let snapshot = self.secret_snapshot();
        let previous = LAST_SECRET_SNAPSHOT
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if previous.as_ref() == Some(&snapshot) {
            return Ok(());
        }
        let previous = previous.unwrap_or_default();

        if snapshot.deepl_api_key != previous.deepl_api_key {
            match snapshot.deepl_api_key.as_deref() {
                Some(value) => crate::platform::secrets::store(DEEPL_SECRET_KEY, value)?,
                None => crate::platform::secrets::delete(DEEPL_SECRET_KEY)?,
            }
        }

        for (catalog_id, password) in &snapshot.opds_passwords {
            if previous.opds_passwords.get(catalog_id) != Some(password) {
                crate::platform::secrets::store(&Self::opds_secret_key(catalog_id), password)?;
            }
        }

        for catalog_id in previous.opds_passwords.keys() {
            if !snapshot.opds_passwords.contains_key(catalog_id) {
                crate::platform::secrets::delete(&Self::opds_secret_key(catalog_id))?;
            }
        }

        if snapshot.license_key != previous.license_key {
            match snapshot.license_key.as_deref() {
                Some(value) => crate::platform::secrets::store(LICENSE_SECRET_KEY, value)?,
                None => crate::platform::secrets::delete(LICENSE_SECRET_KEY)?,
            }
        }

        if snapshot.trial_started_at != previous.trial_started_at {
            match snapshot.trial_started_at.as_deref() {
                Some(value) => crate::platform::secrets::store(TRIAL_SECRET_KEY, value)?,
                None => crate::platform::secrets::delete(TRIAL_SECRET_KEY)?,
            }
        }

        if let Ok(mut guard) = LAST_SECRET_SNAPSHOT.lock() {
            *guard = Some(snapshot);
        }
        Ok(())
    }

    fn apply_secrets(&mut self, legacy: &[(String, String)]) {
        let mut first_error = None;
        let legacy_deepl = legacy
            .iter()
            .find(|(key, _)| key == DEEPL_SECRET_KEY)
            .map(|(_, value)| value.clone());
        match crate::platform::secrets::load(DEEPL_SECRET_KEY) {
            Ok(Some(value)) => self.translation.deepl_api_key = value,
            Ok(None) => {
                if let Some(value) = legacy_deepl {
                    self.translation.deepl_api_key = value;
                }
            }
            Err(error) => {
                first_error = Some(error);
                if let Some(value) = legacy_deepl {
                    self.translation.deepl_api_key = value;
                }
            }
        }

        for catalog in &mut self.opds_catalogs {
            let key = Self::opds_secret_key(&catalog.id);
            let legacy_password = legacy
                .iter()
                .find(|(entry_key, _)| entry_key == &key)
                .map(|(_, value)| value.clone());
            match crate::platform::secrets::load(&key) {
                Ok(Some(value)) => catalog.password = Some(value),
                Ok(None) => {
                    if let Some(value) = legacy_password {
                        catalog.password = Some(value);
                    }
                }
                Err(error) => {
                    first_error = Some(error);
                    if let Some(value) = legacy_password {
                        catalog.password = Some(value);
                    }
                }
            }
        }

        let legacy_license = legacy
            .iter()
            .find(|(key, _)| key == LICENSE_SECRET_KEY)
            .map(|(_, value)| value.clone());
        match crate::platform::secrets::load(LICENSE_SECRET_KEY) {
            Ok(Some(value)) => self.license_key = value,
            Ok(None) => {
                if let Some(value) = legacy_license {
                    self.license_key = value;
                }
            }
            Err(error) => {
                first_error = Some(error);
                if let Some(value) = legacy_license {
                    self.license_key = value;
                }
            }
        }

        let legacy_trial = legacy
            .iter()
            .find(|(key, _)| key == TRIAL_SECRET_KEY)
            .and_then(|(_, value)| value.parse::<u64>().ok());
        match crate::platform::secrets::load(TRIAL_SECRET_KEY) {
            Ok(Some(value)) => {
                if let Ok(started_at) = value.parse::<u64>() {
                    self.trial_started_at_unix = Some(
                        self.trial_started_at_unix
                            .map_or(started_at, |current| current.min(started_at)),
                    );
                }
            }
            Ok(None) => {
                if let Some(started_at) = legacy_trial {
                    self.trial_started_at_unix = Some(
                        self.trial_started_at_unix
                            .map_or(started_at, |current| current.min(started_at)),
                    );
                }
            }
            Err(error) => {
                first_error = Some(error);
                if let Some(started_at) = legacy_trial {
                    self.trial_started_at_unix = Some(
                        self.trial_started_at_unix
                            .map_or(started_at, |current| current.min(started_at)),
                    );
                }
            }
        }

        if let Some(error) = first_error {
            Self::set_secret_error(error);
        }
    }

    pub(super) fn save_with_secret_result(
        &self,
        path: &Path,
        secret_result: Result<(), String>,
    ) -> Result<(), String> {
        if let Err(error) = secret_result {
            Self::set_secret_error(error.clone());
            return Err(error);
        }
        Self::clear_secret_error();
        let save_result = self.save_to_path(path);
        sanitize_auxiliary_files(path);
        save_result
    }
}

fn redact_secret_text(content: &str) -> Option<String> {
    let mut result = String::with_capacity(content.len());
    let mut cursor = 0;
    let mut changed = false;

    while let Some(relative) = content[cursor..].find("\"deepl_api_key\"") {
        let key_start = cursor + relative;
        let key_end = key_start + "\"deepl_api_key\"".len();
        let Some((value_start, value_end)) = json_string_value_bounds(content, key_end) else {
            cursor = key_end;
            continue;
        };
        result.push_str(&content[cursor..value_start + 1]);
        result.push('"');
        cursor = value_end;
        changed = true;
    }
    result.push_str(&content[cursor..]);

    let mut second = String::with_capacity(result.len());
    let mut cursor = 0;
    while let Some(relative) = result[cursor..].find("\"password\"") {
        let key_start = cursor + relative;
        let key_end = key_start + "\"password\"".len();
        let Some((value_start, value_end)) = json_string_value_bounds(&result, key_end) else {
            cursor = key_end;
            continue;
        };
        second.push_str(&result[cursor..value_start + 1]);
        second.push('"');
        cursor = value_end;
        changed = true;
    }
    second.push_str(&result[cursor..]);

    let mut third = String::with_capacity(second.len());
    let mut cursor = 0;
    while let Some(relative) = second[cursor..].find("\"license_key\"") {
        let key_start = cursor + relative;
        let key_end = key_start + "\"license_key\"".len();
        let Some((value_start, value_end)) = json_string_value_bounds(&second, key_end) else {
            cursor = key_end;
            continue;
        };
        third.push_str(&second[cursor..value_start + 1]);
        third.push('"');
        cursor = value_end;
        changed = true;
    }
    third.push_str(&second[cursor..]);

    if changed { Some(third) } else { None }
}

fn json_string_value_bounds(content: &str, key_end: usize) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut index = key_end;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if bytes.get(index) != Some(&b':') {
        return None;
    }
    index += 1;
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let value_start = index;
    index += 1;
    let mut escaped = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if !escaped => escaped = true,
            b'"' if !escaped => return Some((value_start, index + 1)),
            _ => escaped = false,
        }
        index += 1;
    }
    None
}

fn write_settings_file(path: &Path, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)?;
    crate::platform::permissions::restrict_file(path);
    Ok(())
}

pub(super) fn write_secret_safe_copy(source: &Path, destination: &Path) -> std::io::Result<()> {
    let content = std::fs::read_to_string(source)?;
    if let Some(sanitized) = AppSettings::sanitize_secret_fields(&content) {
        write_settings_file(destination, &sanitized)
    } else {
        std::fs::copy(source, destination)?;
        crate::platform::permissions::restrict_file(destination);
        Ok(())
    }
}

fn sanitize_existing_file(path: &Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let Some(sanitized) = AppSettings::sanitize_secret_fields(&content) else {
        return;
    };
    let tmp_path = path.with_extension(format!("sanitize.{}", std::process::id()));
    if write_settings_file(&tmp_path, &sanitized).is_ok() {
        let _ = std::fs::rename(&tmp_path, path);
    } else {
        let _ = std::fs::remove_file(&tmp_path);
    }
    crate::platform::permissions::restrict_file(path);
}

pub(super) fn sanitize_auxiliary_files(path: &Path) {
    sanitize_existing_file(&AppSettings::make_backup_path(path));
    sanitize_existing_file(&path.with_extension("corrupted"));
}

pub(super) fn read_legacy_secret_entries(path: &Path) -> Option<Vec<(String, String)>> {
    let content = std::fs::read_to_string(path).ok()?;
    let entries = AppSettings::extract_legacy_secret_entries(&content);
    if entries.is_empty() {
        None
    } else {
        Some(entries)
    }
}

pub(super) fn initialize(settings: &mut AppSettings, path: &Path, legacy: Vec<(String, String)>) {
    settings.apply_secrets(&legacy);
    let secrets_saved = if legacy.is_empty() {
        if let Ok(mut guard) = LAST_SECRET_SNAPSHOT.lock() {
            *guard = Some(settings.secret_snapshot());
        }
        true
    } else {
        match settings.store_secrets() {
            Ok(()) => {
                AppSettings::clear_secret_error();
                let _ = settings.save_to_path(path);
                true
            }
            Err(error) => {
                AppSettings::set_secret_error(error);
                false
            }
        }
    };
    if secrets_saved {
        sanitize_auxiliary_files(path);
    }
}
