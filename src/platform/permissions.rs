use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static RESTRICTED_DIRS: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

pub fn create_private_dir(path: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            let _ = std::fs::remove_file(path);
        } else if !metadata.is_dir() {
            return;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let _ = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path);
    }
    #[cfg(not(unix))]
    {
        let _ = std::fs::create_dir_all(path);
    }

    let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let should_restrict = {
        let mut guard = match RESTRICTED_DIRS.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.get_or_insert_with(HashSet::new).insert(key)
    };
    if should_restrict {
        restrict_dir(path);
    }
}

#[cfg(unix)]
pub fn restrict_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        let _ = std::fs::set_permissions(path, permissions);
    }
}

#[cfg(unix)]
pub fn restrict_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o700);
        let _ = std::fs::set_permissions(path, permissions);
    }
}

#[cfg(windows)]
fn restrict_windows_path(path: &Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let Some(system_root) = std::env::var_os("SystemRoot") else {
        return;
    };
    let Some(user) = std::env::var_os("USERNAME") else {
        return;
    };
    let icacls = Path::new(&system_root).join("System32").join("icacls.exe");
    let grant = format!("{}:F", user.to_string_lossy());
    let _ = std::process::Command::new(icacls)
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(grant)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(windows)]
pub fn restrict_file(path: &Path) {
    restrict_windows_path(path);
}

#[cfg(windows)]
pub fn restrict_dir(path: &Path) {
    restrict_windows_path(path);
}

#[cfg(not(any(unix, windows)))]
pub fn restrict_file(_path: &Path) {}

#[cfg(not(any(unix, windows)))]
pub fn restrict_dir(_path: &Path) {}
