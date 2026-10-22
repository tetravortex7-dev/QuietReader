use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_IPC_BYTES: u64 = 64 * 1024;
const MAX_IPC_PATHS: usize = 32;
const MAX_IPC_PATH_LEN: usize = 4096;

fn get_ipc_dir() -> PathBuf {
    let base = crate::settings::AppSettings::get_settings_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("ipc");
    crate::platform::permissions::create_private_dir(&dir);
    dir
}

pub(crate) fn write_private_file(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if data.len() as u64 > MAX_IPC_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "IPC payload is too large",
        ));
    }
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("ipc");
    let Some(dir) = path.parent() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "IPC path has no parent directory",
        ));
    };
    let tmp = dir.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        crate::platform::permissions::restrict_file(&tmp);
        let _ = std::fs::remove_file(path);
        std::fs::rename(&tmp, path)?;
        crate::platform::permissions::restrict_file(path);
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub(crate) fn read_private_file(path: &Path, remove: bool) -> Option<String> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_IPC_BYTES {
        let _ = std::fs::remove_file(path);
        return None;
    }
    match std::fs::read_to_string(path) {
        Ok(content) => {
            if remove {
                let _ = std::fs::remove_file(path);
            }
            Some(content)
        }
        Err(_) => {
            let _ = std::fs::remove_file(path);
            None
        }
    }
}

pub(crate) fn validated_ipc_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_IPC_PATH_LEN
        || trimmed.contains('\n')
        || trimmed.contains('\r')
    {
        return None;
    }
    let path = PathBuf::from(trimmed);
    let metadata = std::fs::symlink_metadata(&path).ok()?;
    if !metadata.file_type().is_file() {
        return None;
    }
    let canonical = std::fs::canonicalize(&path).ok()?;
    let canonical_str = canonical.to_str()?;
    if canonical_str.len() > MAX_IPC_PATH_LEN
        || canonical_str.contains('\n')
        || canonical_str.contains('\r')
    {
        return None;
    }
    let metadata = std::fs::metadata(&canonical).ok()?;
    metadata.is_file().then_some(canonical)
}

pub(crate) fn parse_ipc_payload(content: &str) -> Vec<PathBuf> {
    let mut lines = content.lines();
    if !matches!(lines.next().map(str::trim), Some("ACTIVATE")) {
        return Vec::new();
    }
    let mut files = Vec::new();
    for line in lines {
        if files.len() >= MAX_IPC_PATHS {
            break;
        }
        if let Some(path) = validated_ipc_path(line) {
            files.push(path);
        }
    }
    files
}

fn build_ipc_payload(initial_file: &Option<PathBuf>) -> Vec<u8> {
    let mut payload = String::from("ACTIVATE\n");
    if let Some(path) = initial_file
        .as_deref()
        .and_then(Path::to_str)
        .and_then(validated_ipc_path)
    {
        payload.push_str(&path.to_string_lossy());
        payload.push('\n');
    }
    payload.into_bytes()
}

fn get_ipc_request_path() -> PathBuf {
    get_ipc_dir().join("quietreader_ipc.req")
}

fn get_hwnd_file_path() -> PathBuf {
    get_ipc_dir().join("quietreader_hwnd.id")
}

pub struct SingleInstanceGuard {
    #[cfg(windows)]
    mutex_handle: isize,
    #[cfg(not(windows))]
    lock_file: PathBuf,
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(get_hwnd_file_path());
        let _ = std::fs::remove_file(get_ipc_request_path());
        #[cfg(windows)]
        unsafe {
            if self.mutex_handle != 0 {
                win32_ipc::CloseHandle(self.mutex_handle);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = std::fs::remove_file(&self.lock_file);
        }
    }
}

#[cfg(windows)]
mod win32_ipc {
    use super::{SingleInstanceGuard, get_hwnd_file_path, get_ipc_request_path};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicIsize, Ordering};

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SW_RESTORE: i32 = 9;
    const SW_SHOW: i32 = 5;
    const ASFW_ANY: u32 = 0xFFFF_FFFF;
    const WM_NULL: u32 = 0x0000;

    static REGISTERED_HWND: AtomicIsize = AtomicIsize::new(0);

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *const core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> isize;
        fn GetLastError() -> u32;
        fn GetCurrentProcessId() -> u32;
        pub fn CloseHandle(h_object: isize) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetActiveWindow() -> isize;
        fn GetForegroundWindow() -> isize;
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
        fn IsWindow(hwnd: isize) -> i32;
        fn IsIconic(hwnd: isize) -> i32;
        fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn BringWindowToTop(hwnd: isize) -> i32;
        fn AllowSetForegroundWindow(dw_process_id: u32) -> i32;
        fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn is_own_process_window(hwnd: isize) -> bool {
        if hwnd == 0 {
            return false;
        }
        let mut proc_id = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut proc_id);
            proc_id != 0 && proc_id == GetCurrentProcessId()
        }
    }

    fn find_primary_hwnd() -> isize {
        if let Some(text) = super::read_private_file(&get_hwnd_file_path(), false)
            && let Ok(val) = text.trim().parse::<isize>()
            && val != 0
            && unsafe { IsWindow(val) != 0 }
        {
            return val;
        }
        let title = to_wide("QuietReader");
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if is_own_process_window(hwnd) { hwnd } else { 0 }
    }

    pub fn focus_hwnd(hwnd: isize) {
        if hwnd == 0 {
            return;
        }
        unsafe {
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            } else {
                ShowWindow(hwnd, SW_SHOW);
            }
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            PostMessageW(hwnd, WM_NULL, 0, 0);
        }
    }

    pub fn acquire_single_instance(initial_file: &Option<PathBuf>) -> Option<SingleInstanceGuard> {
        let mutex_name = to_wide("Local\\QuietReader_SingleInstance_Mutex_v1");
        unsafe {
            let handle = CreateMutexW(std::ptr::null(), 0, mutex_name.as_ptr());
            if handle == 0 {
                return Some(SingleInstanceGuard { mutex_handle: 0 });
            }

            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);

                let payload = super::build_ipc_payload(initial_file);
                let _ = super::write_private_file(&get_ipc_request_path(), &payload);

                AllowSetForegroundWindow(ASFW_ANY);
                let hwnd = find_primary_hwnd();
                focus_hwnd(hwnd);

                None
            } else {
                let _ = std::fs::remove_file(get_ipc_request_path());
                Some(SingleInstanceGuard {
                    mutex_handle: handle,
                })
            }
        }
    }

    pub fn register_primary_window_hwnd() {
        if REGISTERED_HWND.load(Ordering::Relaxed) != 0 {
            return;
        }
        unsafe {
            let mut hwnd = GetActiveWindow();
            if !is_own_process_window(hwnd) {
                hwnd = GetForegroundWindow();
            }
            if !is_own_process_window(hwnd) {
                let title = to_wide("QuietReader");
                hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            }
            if is_own_process_window(hwnd) {
                REGISTERED_HWND.store(hwnd, Ordering::Relaxed);
                let _ =
                    super::write_private_file(&get_hwnd_file_path(), hwnd.to_string().as_bytes());
                crate::platform::optimize_window_for_rendering(hwnd);
            }
        }
    }

    pub fn get_primary_window_hwnd() -> isize {
        let hwnd = REGISTERED_HWND.load(Ordering::Relaxed);
        if hwnd != 0 {
            hwnd
        } else {
            register_primary_window_hwnd();
            REGISTERED_HWND.load(Ordering::Relaxed)
        }
    }

    pub fn focus_current_process_window() {
        let mut hwnd = REGISTERED_HWND.load(Ordering::Relaxed);
        if hwnd == 0 {
            hwnd = find_primary_hwnd();
        }
        focus_hwnd(hwnd);
    }
}

pub fn acquire_single_instance(initial_file: &Option<PathBuf>) -> Option<SingleInstanceGuard> {
    #[cfg(windows)]
    {
        win32_ipc::acquire_single_instance(initial_file)
    }
    #[cfg(not(windows))]
    {
        let lock_file = get_ipc_dir().join("quietreader_instance.lock");
        if let Some(pid_str) = read_private_file(&lock_file, false)
            && let Ok(pid) = pid_str.trim().parse::<u32>()
        {
            let proc_alive = std::path::Path::new(&format!("/proc/{pid}")).exists()
                || std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
            if proc_alive {
                let payload = build_ipc_payload(initial_file);
                let _ = write_private_file(&get_ipc_request_path(), &payload);
                return None;
            }
        }
        let _ = write_private_file(&lock_file, std::process::id().to_string().as_bytes());
        let _ = std::fs::remove_file(get_ipc_request_path());
        Some(SingleInstanceGuard { lock_file })
    }
}

pub fn register_primary_window_hwnd() {
    #[cfg(windows)]
    win32_ipc::register_primary_window_hwnd();
}

pub fn get_primary_window_hwnd() -> isize {
    #[cfg(windows)]
    {
        win32_ipc::get_primary_window_hwnd()
    }
    #[cfg(not(windows))]
    {
        0
    }
}

static LAST_IPC_POLL: Mutex<Option<Instant>> = Mutex::new(None);

pub fn poll_single_instance_requests() -> (bool, Vec<PathBuf>) {
    if let Ok(mut guard) = LAST_IPC_POLL.lock() {
        let now = Instant::now();
        if let Some(last) = *guard
            && now.duration_since(last) < Duration::from_millis(200)
        {
            return (false, Vec::new());
        }
        *guard = Some(now);
    }

    let req_path = get_ipc_request_path();
    let Some(content) = read_private_file(&req_path, true) else {
        return (false, Vec::new());
    };
    let files = parse_ipc_payload(&content);

    #[cfg(windows)]
    win32_ipc::focus_current_process_window();

    (true, files)
}
