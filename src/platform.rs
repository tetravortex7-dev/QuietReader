#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AssociationStatus {
    Default,
    Registered,
    NotRegistered,
}

pub fn supports_file_association() -> bool {
    cfg!(windows)
}

#[cfg(windows)]
pub fn set_keep_screen_on(keep_on: bool) {
    const ES_CONTINUOUS: u32 = 0x80000000;
    const ES_DISPLAY_REQUIRED: u32 = 0x00000002;

    unsafe extern "system" {
        fn SetThreadExecutionState(es_flags: u32) -> u32;
    }

    unsafe {
        if keep_on {
            SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED);
        } else {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(not(windows))]
pub fn set_keep_screen_on(_keep_on: bool) {}

#[cfg(windows)]
mod windows_registry {
    use super::AssociationStatus;
    use std::ffi::c_void;

    const HKEY_CURRENT_USER: *mut c_void = 0x80000001u32 as usize as *mut c_void;
    const KEY_READ: u32 = 0x20019;
    const KEY_ALL_ACCESS: u32 = 0xF003F;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;
    const ERROR_SUCCESS: i32 = 0;
    const SHCNE_ASSOCCHANGED: i32 = 0x08000000;
    const SHCNF_IDLIST: u32 = 0x0000;

    #[link(name = "advapi32")]
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            hKey: *mut c_void,
            lpSubKey: *const u16,
            ulOptions: u32,
            samDesired: u32,
            phkResult: *mut *mut c_void,
        ) -> i32;

        fn RegCreateKeyExW(
            hKey: *mut c_void,
            lpSubKey: *const u16,
            Reserved: u32,
            lpClass: *mut u16,
            dwOptions: u32,
            samDesired: u32,
            lpSecurityAttributes: *mut c_void,
            phkResult: *mut *mut c_void,
            lpdwDisposition: *mut u32,
        ) -> i32;

        fn RegSetValueExW(
            hKey: *mut c_void,
            lpValueName: *const u16,
            Reserved: u32,
            dwType: u32,
            lpData: *const u8,
            cbData: u32,
        ) -> i32;

        fn RegQueryValueExW(
            hKey: *mut c_void,
            lpValueName: *const u16,
            lpReserved: *mut u32,
            lpType: *mut u32,
            lpData: *mut u8,
            lpcbData: *mut u32,
        ) -> i32;

        fn RegCloseKey(hKey: *mut c_void) -> i32;

        fn SHChangeNotify(
            wEventId: i32,
            uFlags: u32,
            dwItem1: *const c_void,
            dwItem2: *const c_void,
        );
    }

    struct RegKey(*mut c_void);

    impl Drop for RegKey {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { RegCloseKey(self.0) };
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn reg_open_key(root: *mut c_void, sub_key: &str, access: u32) -> Option<RegKey> {
        let wide = to_wide(sub_key);
        let mut hkey: *mut c_void = std::ptr::null_mut();
        let res = unsafe { RegOpenKeyExW(root, wide.as_ptr(), 0, access, &mut hkey) };
        if res == ERROR_SUCCESS && !hkey.is_null() {
            Some(RegKey(hkey))
        } else {
            None
        }
    }

    fn reg_create_key(root: *mut c_void, sub_key: &str) -> Result<RegKey, String> {
        let wide = to_wide(sub_key);
        let mut hkey: *mut c_void = std::ptr::null_mut();
        let mut disp: u32 = 0;
        let res = unsafe {
            RegCreateKeyExW(
                root,
                wide.as_ptr(),
                0,
                std::ptr::null_mut(),
                0,
                KEY_ALL_ACCESS,
                std::ptr::null_mut(),
                &mut hkey,
                &mut disp,
            )
        };
        if res == ERROR_SUCCESS && !hkey.is_null() {
            Ok(RegKey(hkey))
        } else {
            Err(format!(
                "Не удалось создать ключ реестра '{sub_key}': код {res}"
            ))
        }
    }

    fn reg_set_string(key: &RegKey, name: Option<&str>, val: &str) -> Result<(), String> {
        let wide_name = name.map(to_wide);
        let name_ptr = wide_name
            .as_ref()
            .map(|w| w.as_ptr())
            .unwrap_or(std::ptr::null());
        let wide_val = to_wide(val);
        let byte_len = (wide_val.len() * 2) as u32;
        let res = unsafe {
            RegSetValueExW(
                key.0,
                name_ptr,
                0,
                REG_SZ,
                wide_val.as_ptr() as *const u8,
                byte_len,
            )
        };
        if res == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("Не удалось записать значение в реестр: код {res}"))
        }
    }

    fn reg_get_string(key: &RegKey, name: Option<&str>) -> Option<String> {
        let wide_name = name.map(to_wide);
        let name_ptr = wide_name
            .as_ref()
            .map(|w| w.as_ptr())
            .unwrap_or(std::ptr::null());
        let mut data_type: u32 = 0;
        let mut data_len: u32 = 0;

        let res = unsafe {
            RegQueryValueExW(
                key.0,
                name_ptr,
                std::ptr::null_mut(),
                &mut data_type,
                std::ptr::null_mut(),
                &mut data_len,
            )
        };
        if res != ERROR_SUCCESS || data_len == 0 {
            return None;
        }

        let mut buf = vec![0u8; data_len as usize];
        let res = unsafe {
            RegQueryValueExW(
                key.0,
                name_ptr,
                std::ptr::null_mut(),
                &mut data_type,
                buf.as_mut_ptr(),
                &mut data_len,
            )
        };
        if res != ERROR_SUCCESS || (data_type != REG_SZ && data_type != REG_EXPAND_SZ) {
            return None;
        }

        let u16_len = (data_len as usize) / 2;
        if u16_len == 0 {
            return Some(String::new());
        }
        let u16_slice = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u16, u16_len) };
        let s = String::from_utf16_lossy(u16_slice);
        Some(s.trim_end_matches('\0').to_string())
    }

    fn notify_shell_assoc_changed() {
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED,
                SHCNF_IDLIST,
                std::ptr::null(),
                std::ptr::null(),
            );
        }
    }

    pub fn check_fb2_association_status() -> AssociationStatus {
        let current_exe = match std::env::current_exe() {
            Ok(p) => p.to_string_lossy().to_lowercase(),
            Err(_) => return AssociationStatus::NotRegistered,
        };

        // 1. Check if QuietReader.fb2 command is registered for the current exe
        let is_our_progid_registered = reg_open_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\QuietReader.fb2\shell\open\command",
            KEY_READ,
        )
        .and_then(|k| reg_get_string(&k, None))
        .map(|cmd| cmd.to_lowercase().contains(&current_exe))
        .unwrap_or(false);

        if !is_our_progid_registered {
            return AssociationStatus::NotRegistered;
        }

        // 2. Check Windows 10/11 UserChoice
        if let Some(user_choice_key) = reg_open_key(
            HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.fb2\UserChoice",
            KEY_READ,
        ) {
            if let Some(progid) = reg_get_string(&user_choice_key, Some("ProgId"))
                && progid.eq_ignore_ascii_case("QuietReader.fb2")
            {
                return AssociationStatus::Default;
            }
            return AssociationStatus::Registered;
        }

        // 3. If UserChoice is absent, check HKCU\Software\Classes\.fb2 (Default)
        if let Some(dot_fb2_key) =
            reg_open_key(HKEY_CURRENT_USER, r"Software\Classes\.fb2", KEY_READ)
            && let Some(progid) = reg_get_string(&dot_fb2_key, None)
            && progid.eq_ignore_ascii_case("QuietReader.fb2")
        {
            return AssociationStatus::Default;
        }

        AssociationStatus::Registered
    }

    pub fn register_fb2_association() -> Result<String, String> {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Не удалось определить путь к исполнимому файлу: {e}"))?;
        let exe_str = exe_path.to_string_lossy();
        let open_cmd = format!("\"{exe_str}\" \"%1\"");
        let doc_icon_cmd = format!("\"{exe_str}\",1");
        let app_icon_cmd = format!("\"{exe_str}\",0");

        // 1. HKCU\Software\Classes\.fb2
        let dot_fb2 = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\.fb2")?;
        let _ = reg_set_string(&dot_fb2, None, "QuietReader.fb2");
        let _ = reg_set_string(
            &dot_fb2,
            Some("Content Type"),
            "application/x-fictionbook+xml",
        );

        let open_with =
            reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\.fb2\OpenWithProgids")?;
        let _ = reg_set_string(&open_with, Some("QuietReader.fb2"), "");

        // 2. HKCU\Software\Classes\QuietReader.fb2
        let progid_key = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\QuietReader.fb2")?;
        let _ = reg_set_string(&progid_key, None, "Книга FictionBook (FB2)");
        let _ = reg_set_string(
            &progid_key,
            Some("FriendlyTypeName"),
            "Книга FictionBook (FB2)",
        );

        let icon_key = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\QuietReader.fb2\DefaultIcon",
        )?;
        let _ = reg_set_string(&icon_key, None, &doc_icon_cmd);

        let cmd_key = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\QuietReader.fb2\shell\open\command",
        )?;
        let _ = reg_set_string(&cmd_key, None, &open_cmd);

        // 3. HKCU\Software\Classes\Applications\QuietReader.exe
        let app_key = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\Applications\QuietReader.exe",
        )?;
        let _ = reg_set_string(&app_key, Some("FriendlyAppName"), "QuietReader");

        let app_icon_key = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\Applications\QuietReader.exe\DefaultIcon",
        )?;
        let _ = reg_set_string(&app_icon_key, None, &app_icon_cmd);

        let supported = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\Applications\QuietReader.exe\SupportedTypes",
        )?;
        let _ = reg_set_string(&supported, Some(".fb2"), "");

        let app_cmd = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\Applications\QuietReader.exe\shell\open\command",
        )?;
        let _ = reg_set_string(&app_cmd, None, &open_cmd);

        // 4. HKCU\Software\QuietReader\Capabilities
        let caps = reg_create_key(HKEY_CURRENT_USER, r"Software\QuietReader\Capabilities")?;
        let _ = reg_set_string(&caps, Some("ApplicationName"), "QuietReader");
        let _ = reg_set_string(
            &caps,
            Some("ApplicationDescription"),
            "Минималистичная читалка книг FB2",
        );

        let file_assoc = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\QuietReader\Capabilities\FileAssociations",
        )?;
        let _ = reg_set_string(&file_assoc, Some(".fb2"), "QuietReader.fb2");

        // 5. HKCU\Software\RegisteredApplications
        let reg_apps = reg_create_key(HKEY_CURRENT_USER, r"Software\RegisteredApplications")?;
        let _ = reg_set_string(
            &reg_apps,
            Some("QuietReader"),
            r"Software\QuietReader\Capabilities",
        );

        notify_shell_assoc_changed();

        Ok(
            "QuietReader успешно зарегистрирован в реестре Windows (HKCU) для формата .fb2."
                .to_string(),
        )
    }

    pub fn is_association_prompt_dismissed() -> bool {
        reg_open_key(
            HKEY_CURRENT_USER,
            r"Software\QuietReader\Settings",
            KEY_READ,
        )
        .and_then(|k| reg_get_string(&k, Some("DismissedDefaultPrompt")))
        .map(|v| v == "1")
        .unwrap_or(false)
    }

    pub fn set_association_prompt_dismissed(dismissed: bool) {
        if let Ok(key) = reg_create_key(HKEY_CURRENT_USER, r"Software\QuietReader\Settings") {
            let val = if dismissed { "1" } else { "0" };
            let _ = reg_set_string(&key, Some("DismissedDefaultPrompt"), val);
        }
    }
}

#[cfg(windows)]
pub fn check_fb2_association_status() -> AssociationStatus {
    windows_registry::check_fb2_association_status()
}

#[cfg(windows)]
pub fn register_fb2_association() -> Result<String, String> {
    windows_registry::register_fb2_association()
}

#[cfg(windows)]
pub fn open_default_apps_settings() {
    let _ = std::process::Command::new("explorer.exe")
        .arg("ms-settings:defaultapps")
        .spawn();
}

#[cfg(windows)]
pub fn is_association_prompt_dismissed() -> bool {
    windows_registry::is_association_prompt_dismissed()
}

#[cfg(windows)]
pub fn set_association_prompt_dismissed(dismissed: bool) {
    windows_registry::set_association_prompt_dismissed(dismissed);
}

#[cfg(not(windows))]
pub fn check_fb2_association_status() -> AssociationStatus {
    AssociationStatus::NotRegistered
}

#[cfg(not(windows))]
pub fn register_fb2_association() -> Result<String, String> {
    Ok("Регистрация ассоциаций поддерживается только в Windows.".to_string())
}

#[cfg(not(windows))]
pub fn open_default_apps_settings() {}

#[cfg(not(windows))]
pub fn is_association_prompt_dismissed() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_association_prompt_dismissed(_dismissed: bool) {}

/// Общий платформенный интерфейс загрузки байтов системного шрифта по имени гарнитуры
pub fn load_system_font_bytes(font_name: &str) -> Option<Vec<u8>> {
    platform_fonts::load_font(font_name)
}

#[cfg(windows)]
mod platform_fonts {
    use std::path::PathBuf;

    pub fn load_font(font_name: &str) -> Option<Vec<u8>> {
        let filename = match font_name {
            "Georgia" => "georgia.ttf",
            "Palatino Linotype" => "pala.ttf",
            "Calibri" => "calibri.ttf",
            "Segoe UI" => "segoeui.ttf",
            "Verdana" => "verdana.ttf",
            _ => "georgia.ttf",
        };

        if let Some(windir) = std::env::var_os("WINDIR") {
            let p = PathBuf::from(windir).join("Fonts").join(filename);
            if let Ok(bytes) = std::fs::read(&p) {
                return Some(bytes);
            }
        }
        let fallback = PathBuf::from(r"C:\Windows\Fonts").join(filename);
        std::fs::read(fallback).ok()
    }
}

#[cfg(target_os = "linux")]
mod platform_fonts {
    use std::path::{Path, PathBuf};

    fn find_font_in_dir(dir: &Path, candidates: &[&str], depth: usize) -> Option<Vec<u8>> {
        if depth > 4 || !dir.is_dir() {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(bytes) = find_font_in_dir(&path, candidates, depth + 1) {
                    return Some(bytes);
                }
            } else if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                for &cand in candidates {
                    if fname.eq_ignore_ascii_case(cand) {
                        if let Ok(bytes) = std::fs::read(&path) {
                            return Some(bytes);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn load_font(font_name: &str) -> Option<Vec<u8>> {
        let candidates: &[&str] = match font_name {
            "Georgia" => &[
                "georgia.ttf",
                "LiberationSerif-Regular.ttf",
                "DejaVuSerif.ttf",
            ],
            "Palatino Linotype" => &["pala.ttf", "P052-Roman.otf", "DejaVuSerif.ttf"],
            "Calibri" => &["calibri.ttf", "Carlito-Regular.ttf", "DejaVuSans.ttf"],
            "Segoe UI" => &["segoeui.ttf", "Cantarell-Regular.otf", "DejaVuSans.ttf"],
            "Verdana" => &["verdana.ttf", "DejaVuSans.ttf"],
            _ => &["georgia.ttf", "DejaVuSerif.ttf"],
        };

        let mut dirs = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
            dirs.push(PathBuf::from(home).join(".fonts"));
        }

        for dir in dirs {
            if let Some(bytes) = find_font_in_dir(&dir, candidates, 0) {
                return Some(bytes);
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
mod platform_fonts {
    use std::path::PathBuf;

    pub fn load_font(font_name: &str) -> Option<Vec<u8>> {
        let candidates: &[&str] = match font_name {
            "Georgia" => &["Georgia.ttf", "georgia.ttf"],
            "Palatino Linotype" => &["Palatino.ttc", "pala.ttf"],
            "Calibri" => &["Calibri.ttf", "calibri.ttf", "HelveticaNeue.ttc"],
            "Segoe UI" => &["SFNS.ttf", "Helvetica.ttc", "Arial.ttf"],
            "Verdana" => &["Verdana.ttf", "verdana.ttf"],
            _ => &["Georgia.ttf", "georgia.ttf"],
        };

        let mut dirs = vec![
            PathBuf::from("/Library/Fonts"),
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/System/Library/Fonts/Supplemental"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }

        for dir in dirs {
            for &cand in candidates {
                let p = dir.join(cand);
                if let Ok(bytes) = std::fs::read(&p) {
                    return Some(bytes);
                }
            }
        }
        None
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod platform_fonts {
    pub fn load_font(_font_name: &str) -> Option<Vec<u8>> {
        None
    }
}

pub fn is_window_accessible_in_work_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
) -> bool {
    if width <= 0 || height <= 0 || work_right <= work_left || work_bottom <= work_top {
        return false;
    }

    let grab_left = x + 16;
    let grab_right = (x + width - 16).max(grab_left + 64);
    let grab_top = y;
    let grab_bottom = y + 40;

    let grab_overlap_w = grab_right.min(work_right) - grab_left.max(work_left);
    let grab_overlap_h = grab_bottom.min(work_bottom) - grab_top.max(work_top);

    let body_overlap_w = (x + width).min(work_right) - x.max(work_left);
    let body_overlap_h = (y + height).min(work_bottom) - y.max(work_top);

    y >= work_top - 8
        && grab_overlap_w >= 80
        && grab_overlap_h >= 20
        && body_overlap_w >= 120
        && body_overlap_h >= 80
}

pub fn reposition_window_into_work_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
) -> (i32, i32, i32, i32) {
    let work_w = (work_right - work_left).max(400);
    let work_h = (work_bottom - work_top).max(300);

    let new_w = width.clamp(400.min(work_w), work_w);
    let new_h = height.clamp(300.min(work_h), work_h);

    let completely_outside =
        x + new_w <= work_left || x >= work_right || y + new_h <= work_top || y >= work_bottom;

    if completely_outside {
        let centered_x = work_left + (work_w - new_w) / 2;
        let centered_y = work_top + (work_h - new_h) / 2;
        (centered_x, centered_y, new_w, new_h)
    } else {
        let clamped_x = x.clamp(work_left, (work_right - new_w).max(work_left));
        let clamped_y = y.clamp(work_top, (work_bottom - new_h).max(work_top));
        (clamped_x, clamped_y, new_w, new_h)
    }
}

#[cfg(windows)]
mod windows_fullscreen {
    use super::{is_window_accessible_in_work_area, reposition_window_into_work_area};
    use std::sync::Mutex;

    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_CAPTION: i32 = 0x00C00000;
    const WS_THICKFRAME: i32 = 0x00040000;
    const WS_EX_DLGMODALFRAME: i32 = 0x00000001;
    const WS_EX_WINDOWEDGE: i32 = 0x00000100;
    const WS_EX_CLIENTEDGE: i32 = 0x00000200;
    const WS_EX_STATICEDGE: i32 = 0x00020000;
    const MONITOR_DEFAULTTONULL: u32 = 0;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const HWND_TOP: isize = 0;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_NOOWNERZORDER: u32 = 0x0200;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct WindowPlacement {
        length: u32,
        flags: u32,
        show_cmd: u32,
        pt_min_position: Point,
        pt_max_position: Point,
        rc_normal_position: Rect,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct MonitorInfo {
        cb_size: u32,
        rc_monitor: Rect,
        rc_work: Rect,
        dw_flags: u32,
    }

    #[derive(Clone, Copy)]
    struct SavedWindowState {
        hwnd: isize,
        style: i32,
        ex_style: i32,
        placement: WindowPlacement,
    }

    static SAVED_STATE: Mutex<Option<SavedWindowState>> = Mutex::new(None);

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetActiveWindow() -> isize;
        fn GetForegroundWindow() -> isize;
        fn IsIconic(hwnd: isize) -> i32;
        fn IsZoomed(hwnd: isize) -> i32;
        fn GetWindowRect(hwnd: isize, lprect: *mut Rect) -> i32;
        fn GetWindowLongW(hwnd: isize, n_index: i32) -> i32;
        fn SetWindowLongW(hwnd: isize, n_index: i32, dw_new_long: i32) -> i32;
        fn GetWindowPlacement(hwnd: isize, lpwndpl: *mut WindowPlacement) -> i32;
        fn SetWindowPlacement(hwnd: isize, lpwndpl: *const WindowPlacement) -> i32;
        fn MonitorFromWindow(hwnd: isize, dw_flags: u32) -> isize;
        fn MonitorFromRect(lprc: *const Rect, dw_flags: u32) -> isize;
        fn GetMonitorInfoW(hmonitor: isize, lpmi: *mut MonitorInfo) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            hwnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
    }

    pub fn sanitize_window_geometry(
        x: Option<f32>,
        y: Option<f32>,
        width: f32,
        height: f32,
    ) -> (Option<f32>, Option<f32>, f32, f32) {
        let w = if width.is_finite() {
            width.clamp(600.0, 7680.0)
        } else {
            1024.0
        };
        let h = if height.is_finite() {
            height.clamp(400.0, 4320.0)
        } else {
            768.0
        };

        let (Some(px), Some(py)) = (x, y) else {
            return (None, None, w, h);
        };
        if !px.is_finite() || !py.is_finite() || px <= -15000.0 || py <= -15000.0 {
            return (None, None, w, h);
        }

        let ix = px.round() as i32;
        let iy = py.round() as i32;
        let iw = w.round() as i32;
        let ih = h.round() as i32;

        unsafe {
            let grab_rect = Rect {
                left: ix + 16,
                top: iy,
                right: (ix + iw - 16).max(ix + 80),
                bottom: iy + 40,
            };
            let mon = MonitorFromRect(&grab_rect, MONITOR_DEFAULTTONULL);
            if mon != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(mon, &mut mi) != 0
                    && is_window_accessible_in_work_area(
                        ix,
                        iy,
                        iw,
                        ih,
                        mi.rc_work.left,
                        mi.rc_work.top,
                        mi.rc_work.right,
                        mi.rc_work.bottom,
                    )
                {
                    let work_w = (mi.rc_work.right - mi.rc_work.left).max(600) as f32;
                    let work_h = (mi.rc_work.bottom - mi.rc_work.top).max(400) as f32;
                    return (Some(px), Some(py), w.min(work_w), h.min(work_h));
                }
            }

            let win_rect = Rect {
                left: ix,
                top: iy,
                right: ix + iw,
                bottom: iy + ih,
            };
            let nearest = MonitorFromRect(&win_rect, MONITOR_DEFAULTTONEAREST);
            if nearest != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(nearest, &mut mi) != 0 {
                    let (nx, ny, nw, nh) = reposition_window_into_work_area(
                        ix,
                        iy,
                        iw,
                        ih,
                        mi.rc_work.left,
                        mi.rc_work.top,
                        mi.rc_work.right,
                        mi.rc_work.bottom,
                    );
                    return (
                        Some(nx as f32),
                        Some(ny as f32),
                        (nw as f32).max(600.0),
                        (nh as f32).max(400.0),
                    );
                }
            }
        }

        (None, None, w, h)
    }

    pub fn ensure_window_on_screen() -> bool {
        unsafe {
            let mut hwnd = GetActiveWindow();
            if hwnd == 0 {
                hwnd = GetForegroundWindow();
            }
            if hwnd == 0 || IsIconic(hwnd) != 0 || IsZoomed(hwnd) != 0 {
                return false;
            }

            let mut rect = Rect::default();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return false;
            }

            let ix = rect.left;
            let iy = rect.top;
            let iw = rect.right - rect.left;
            let ih = rect.bottom - rect.top;
            if iw <= 0 || ih <= 0 {
                return false;
            }

            let grab_rect = Rect {
                left: ix + 16,
                top: iy,
                right: (ix + iw - 16).max(ix + 80),
                bottom: iy + 40,
            };
            let mon = MonitorFromRect(&grab_rect, MONITOR_DEFAULTTONULL);
            if mon != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(mon, &mut mi) != 0
                    && is_window_accessible_in_work_area(
                        ix,
                        iy,
                        iw,
                        ih,
                        mi.rc_work.left,
                        mi.rc_work.top,
                        mi.rc_work.right,
                        mi.rc_work.bottom,
                    )
                {
                    return false;
                }
            }

            let nearest = MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST);
            if nearest != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(nearest, &mut mi) != 0 {
                    let (nx, ny, nw, nh) = reposition_window_into_work_area(
                        ix,
                        iy,
                        iw,
                        ih,
                        mi.rc_work.left,
                        mi.rc_work.top,
                        mi.rc_work.right,
                        mi.rc_work.bottom,
                    );
                    SetWindowPos(
                        hwnd,
                        0,
                        nx,
                        ny,
                        nw,
                        nh,
                        SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
                    );
                    return true;
                }
            }
            false
        }
    }

    pub fn set_window_fullscreen(fullscreen: bool) -> bool {
        unsafe {
            if fullscreen {
                let mut hwnd = GetActiveWindow();
                if hwnd == 0 {
                    hwnd = GetForegroundWindow();
                }
                if hwnd == 0 {
                    return false;
                }

                let style = GetWindowLongW(hwnd, GWL_STYLE);
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let mut placement = WindowPlacement {
                    length: std::mem::size_of::<WindowPlacement>() as u32,
                    ..Default::default()
                };
                GetWindowPlacement(hwnd, &mut placement);

                if let Ok(mut guard) = SAVED_STATE.lock() {
                    *guard = Some(SavedWindowState {
                        hwnd,
                        style,
                        ex_style,
                        placement,
                    });
                }

                let new_style = style & !(WS_CAPTION | WS_THICKFRAME);
                let new_ex_style = ex_style
                    & !(WS_EX_DLGMODALFRAME
                        | WS_EX_WINDOWEDGE
                        | WS_EX_CLIENTEDGE
                        | WS_EX_STATICEDGE);

                SetWindowLongW(hwnd, GWL_STYLE, new_style);
                SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style);

                let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if monitor != 0 && GetMonitorInfoW(monitor, &mut mi) != 0 {
                    let x = mi.rc_monitor.left;
                    let y = mi.rc_monitor.top;
                    let w = mi.rc_monitor.right - mi.rc_monitor.left;
                    // +1 физический пиксель по высоте: окно полностью перекрывает монитор
                    // (поэтому панель задач Windows «Пуск» скрывается под окном), но размер
                    // не совпадает с нативным видеорежимом 1-в-1, предотвращая захват
                    // OpenGL Exclusive Fullscreen / DirectFlip и мигание экрана.
                    let h = (mi.rc_monitor.bottom - mi.rc_monitor.top) + 1;
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        x,
                        y,
                        w,
                        h,
                        SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                    );
                    return true;
                }
                false
            } else {
                let saved = SAVED_STATE.lock().ok().and_then(|mut g| g.take());
                if let Some(saved) = saved {
                    SetWindowLongW(saved.hwnd, GWL_STYLE, saved.style);
                    SetWindowLongW(saved.hwnd, GWL_EXSTYLE, saved.ex_style);
                    SetWindowPlacement(saved.hwnd, &saved.placement);
                    SetWindowPos(
                        saved.hwnd,
                        0,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE
                            | SWP_NOSIZE
                            | SWP_NOZORDER
                            | SWP_NOOWNERZORDER
                            | SWP_FRAMECHANGED,
                    );
                    let _ = ensure_window_on_screen();
                    return true;
                }
                false
            }
        }
    }
}

#[cfg(windows)]
pub fn sanitize_window_geometry(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
) -> (Option<f32>, Option<f32>, f32, f32) {
    windows_fullscreen::sanitize_window_geometry(x, y, width, height)
}

#[cfg(not(windows))]
pub fn sanitize_window_geometry(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
) -> (Option<f32>, Option<f32>, f32, f32) {
    let w = if width.is_finite() {
        width.clamp(600.0, 7680.0)
    } else {
        1024.0
    };
    let h = if height.is_finite() {
        height.clamp(400.0, 4320.0)
    } else {
        768.0
    };
    if let (Some(px), Some(py)) = (x, y)
        && px.is_finite()
        && py.is_finite()
        && (-10000.0..=10000.0).contains(&px)
        && (-10000.0..=10000.0).contains(&py)
    {
        (Some(px), Some(py), w, h)
    } else {
        (None, None, w, h)
    }
}

#[cfg(windows)]
pub fn ensure_window_on_screen() -> bool {
    windows_fullscreen::ensure_window_on_screen()
}

#[cfg(not(windows))]
pub fn ensure_window_on_screen() -> bool {
    false
}

#[cfg(windows)]
pub fn set_window_fullscreen(fullscreen: bool) -> bool {
    windows_fullscreen::set_window_fullscreen(fullscreen)
}

#[cfg(not(windows))]
pub fn set_window_fullscreen(_fullscreen: bool) -> bool {
    false
}
