use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

fn get_ipc_dir() -> PathBuf {
    if let Some(parent) = crate::settings::AppSettings::get_settings_path().parent() {
        let _ = std::fs::create_dir_all(parent);
        return parent.to_path_buf();
    }
    std::env::temp_dir()
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
        if let Ok(text) = std::fs::read_to_string(get_hwnd_file_path())
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

                let mut payload = String::from("ACTIVATE\n");
                if let Some(path) = initial_file {
                    if path.is_file() {
                        let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                        payload.push_str(&abs.to_string_lossy());
                        payload.push('\n');
                    }
                }
                let _ = std::fs::write(get_ipc_request_path(), payload);

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
                let _ = std::fs::write(get_hwnd_file_path(), hwnd.to_string());
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
        if lock_file.exists()
            && let Ok(pid_str) = std::fs::read_to_string(&lock_file)
            && let Ok(pid) = pid_str.trim().parse::<u32>()
        {
            let proc_alive = std::path::Path::new(&format!("/proc/{pid}")).exists()
                || std::process::Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
            if proc_alive {
                let mut payload = String::from("ACTIVATE\n");
                if let Some(path) = initial_file {
                    if path.is_file() {
                        let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                        payload.push_str(&abs.to_string_lossy());
                        payload.push('\n');
                    }
                }
                let _ = std::fs::write(get_ipc_request_path(), payload);
                return None;
            }
        }
        let _ = std::fs::write(&lock_file, std::process::id().to_string());
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
    if !req_path.exists() {
        return (false, Vec::new());
    }

    let content = std::fs::read_to_string(&req_path).unwrap_or_default();
    let _ = std::fs::remove_file(&req_path);

    let mut files = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed != "ACTIVATE" {
            let p = PathBuf::from(trimmed);
            if p.is_file() {
                files.push(p);
            }
        }
    }

    #[cfg(windows)]
    win32_ipc::focus_current_process_window();

    (true, files)
}
