use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static RESTRICTED_DIRS: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

fn is_reparse_or_symlink(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_type().is_symlink()
            || (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT) != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn remove_reparse_or_symlink(path: &Path, metadata: &std::fs::Metadata) {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
        if (metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY) != 0 {
            let _ = std::fs::remove_dir(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        if std::fs::remove_file(path).is_err() {
            let _ = std::fs::remove_dir(path);
        }
    }
}

pub fn create_private_dir(path: &Path) {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if is_reparse_or_symlink(&metadata) {
            remove_reparse_or_symlink(path, &metadata);
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
        if std::fs::create_dir_all(path).is_err() {
            return;
        }
    }

    if !path.is_dir() {
        return;
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

pub(crate) fn fallback_windows_principal(
    domain: Option<&str>,
    user: Option<&str>,
) -> Option<String> {
    let user = user.map(str::trim).filter(|s| !s.is_empty())?;
    if let Some(domain) = domain.map(str::trim).filter(|s| !s.is_empty()) {
        Some(format!("{domain}\\{user}"))
    } else {
        Some(user.to_string())
    }
}

pub(crate) fn build_icacls_args(principal: &str, is_dir: bool) -> Vec<String> {
    let perms = if is_dir { "(OI)(CI)F" } else { "F" };
    vec![
        "/inheritance:r".to_string(),
        "/grant:r".to_string(),
        format!("{principal}:{perms}"),
        format!("*S-1-5-18:{perms}"),
        format!("*S-1-5-32-544:{perms}"),
    ]
}

#[cfg(windows)]
mod win_acl {
    use core::ffi::c_void;

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_USER_CLASS: u32 = 1;

    #[repr(C)]
    struct SidAndAttributes {
        sid: *mut c_void,
        attributes: u32,
    }

    #[repr(C)]
    struct TokenUser {
        user: SidAndAttributes,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn CloseHandle(h_object: isize) -> i32;
        fn LocalFree(h_mem: *mut c_void) -> *mut c_void;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn OpenProcessToken(
            process_handle: isize,
            desired_access: u32,
            token_handle: *mut isize,
        ) -> i32;
        fn GetTokenInformation(
            token_handle: isize,
            token_information_class: u32,
            token_information: *mut c_void,
            token_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
        fn ConvertSidToStringSidW(sid: *mut c_void, string_sid: *mut *mut u16) -> i32;
    }

    pub fn current_user_sid() -> Option<String> {
        unsafe {
            let mut token: isize = 0;
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 || token == 0 {
                return None;
            }
            let mut len = 0u32;
            GetTokenInformation(token, TOKEN_USER_CLASS, core::ptr::null_mut(), 0, &mut len);
            if len == 0 {
                CloseHandle(token);
                return None;
            }
            let words = (len as usize).div_ceil(core::mem::size_of::<usize>());
            let mut buf = vec![0usize; words];
            let ok = GetTokenInformation(
                token,
                TOKEN_USER_CLASS,
                buf.as_mut_ptr().cast::<c_void>(),
                len,
                &mut len,
            );
            CloseHandle(token);
            if ok == 0 || (len as usize) < core::mem::size_of::<TokenUser>() {
                return None;
            }
            let token_user = &*buf.as_ptr().cast::<TokenUser>();
            let mut wide_ptr: *mut u16 = core::ptr::null_mut();
            if ConvertSidToStringSidW(token_user.user.sid, &mut wide_ptr) == 0 || wide_ptr.is_null()
            {
                return None;
            }
            let mut n = 0usize;
            while *wide_ptr.add(n) != 0 {
                n += 1;
            }
            let slice = core::slice::from_raw_parts(wide_ptr, n);
            let sid = String::from_utf16_lossy(slice);
            LocalFree(wide_ptr.cast::<c_void>());
            sid.starts_with("S-").then_some(sid)
        }
    }
}

#[cfg(windows)]
fn resolve_windows_principal() -> Option<String> {
    if let Some(sid) = win_acl::current_user_sid() {
        return Some(format!("*{sid}"));
    }
    let domain = std::env::var_os("USERDOMAIN").map(|v| v.to_string_lossy().into_owned());
    let user = std::env::var_os("USERNAME").map(|v| v.to_string_lossy().into_owned());
    fallback_windows_principal(domain.as_deref(), user.as_deref())
}

#[cfg(windows)]
fn restrict_windows_path(path: &Path, is_dir: bool) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let Some(system_root) = std::env::var_os("SystemRoot") else {
        return;
    };
    let Some(principal) = resolve_windows_principal() else {
        return;
    };
    let icacls = Path::new(&system_root).join("System32").join("icacls.exe");
    let args = build_icacls_args(&principal, is_dir);
    let _ = std::process::Command::new(icacls)
        .arg(path)
        .args(&args)
        .creation_flags(CREATE_NO_WINDOW)
        .output();
}

#[cfg(windows)]
pub fn restrict_file(path: &Path) {
    restrict_windows_path(path, false);
}

#[cfg(windows)]
pub fn restrict_dir(path: &Path) {
    restrict_windows_path(path, true);
}

#[cfg(not(any(unix, windows)))]
pub fn restrict_file(_path: &Path) {}

#[cfg(not(any(unix, windows)))]
pub fn restrict_dir(_path: &Path) {}
