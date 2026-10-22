const SERVICE: &str = "QuietReader";

#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
mod imp {
    use super::SERVICE;

    fn entry(key: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())
    }

    pub fn store(key: &str, value: &str) -> Result<(), String> {
        entry(key)?.set_password(value).map_err(|e| e.to_string())
    }

    pub fn load(key: &str) -> Result<Option<String>, String> {
        match entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn delete(key: &str) -> Result<(), String> {
        match entry(key)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn store(_key: &str, _value: &str) -> Result<(), String> {
        Err("Secure storage is unavailable on this platform".to_string())
    }

    pub fn load(_key: &str) -> Result<Option<String>, String> {
        Err("Secure storage is unavailable on this platform".to_string())
    }

    pub fn delete(_key: &str) -> Result<(), String> {
        Err("Secure storage is unavailable on this platform".to_string())
    }
}

pub use imp::{delete, load, store};
