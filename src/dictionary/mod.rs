pub mod catalog;
pub mod install;
pub mod stardict;
pub mod wikipedia;

use std::io::Read;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    let path = crate::settings::AppSettings::get_settings_path();
    let dir = path
        .parent()
        .map(|parent| parent.join("dictionaries"))
        .unwrap_or_else(|| PathBuf::from("dictionaries"));
    crate::platform::permissions::create_private_dir(&dir);
    dir
}

pub(crate) fn read_limited(reader: &mut impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut data)
        .map_err(|error| error.to_string())?;
    if data.len() as u64 > limit {
        return Err("response limit exceeded".to_string());
    }
    Ok(data)
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
