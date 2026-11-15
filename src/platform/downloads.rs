use std::path::{Path, PathBuf};

pub fn reveal_downloaded_file(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if path.exists() {
            let clean = path.display().to_string().replace('"', "");
            let _ = std::process::Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{clean}\""))
                .spawn();
        } else if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("explorer.exe").arg(parent).spawn();
        }
    }
    #[cfg(target_os = "macos")]
    {
        if path.exists() {
            let _ = std::process::Command::new("open")
                .args(["-R"])
                .arg(path)
                .spawn();
        } else if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("open").arg(parent).spawn();
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}

pub fn downloads_dir() -> PathBuf {
    let dir = resolve_downloads_dir();
    if std::fs::create_dir_all(&dir).is_ok() {
        return dir;
    }
    let fallback = std::env::temp_dir().join("QuietReader").join("Downloads");
    super::permissions::create_private_dir(&fallback);
    fallback
}

pub(crate) fn resolve_downloads_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(profile) = std::env::var_os("USERPROFILE")
            && !profile.is_empty()
        {
            return PathBuf::from(profile).join("Downloads");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME")
            && !home.is_empty()
        {
            return PathBuf::from(home).join("Downloads");
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(dir) = linux_downloads_dir() {
            return dir;
        }
    }
    std::env::temp_dir().join("QuietReader").join("Downloads")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_downloads_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DOWNLOAD_DIR") {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            return Some(path);
        }
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    let text = std::fs::read_to_string(config.join("user-dirs.dirs")).ok()?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    parse_xdg_download_dir(&text, home.as_deref())
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn parse_xdg_download_dir(
    text: &str,
    home: Option<&std::path::Path>,
) -> Option<PathBuf> {
    for line in text.lines() {
        let line = line.trim();
        let Some(value) = line.strip_prefix("XDG_DOWNLOAD_DIR=") else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if value.is_empty() {
            return None;
        }
        if let Some(relative) = value.strip_prefix("$HOME") {
            let home = home?;
            return Some(home.join(relative.trim_start_matches('/')));
        }
        if value.starts_with('/') {
            return Some(PathBuf::from(value));
        }
    }
    None
}
