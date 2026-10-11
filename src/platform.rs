#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AssociationStatus {
    Default,
    Registered,
    NotRegistered,
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
            Err(format!("Не удалось создать ключ реестра '{sub_key}': код {res}"))
        }
    }

    fn reg_set_string(key: &RegKey, name: Option<&str>, val: &str) -> Result<(), String> {
        let wide_name = name.map(to_wide);
        let name_ptr = wide_name.as_ref().map(|w| w.as_ptr()).unwrap_or(std::ptr::null());
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
        let name_ptr = wide_name.as_ref().map(|w| w.as_ptr()).unwrap_or(std::ptr::null());
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
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, std::ptr::null(), std::ptr::null());
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
            if let Some(progid) = reg_get_string(&user_choice_key, Some("ProgId")) {
                if progid.eq_ignore_ascii_case("QuietReader.fb2") {
                    return AssociationStatus::Default;
                }
            }
            return AssociationStatus::Registered;
        }

        // 3. If UserChoice is absent, check HKCU\Software\Classes\.fb2 (Default)
        if let Some(dot_fb2_key) = reg_open_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\.fb2",
            KEY_READ,
        ) {
            if let Some(progid) = reg_get_string(&dot_fb2_key, None) {
                if progid.eq_ignore_ascii_case("QuietReader.fb2") {
                    return AssociationStatus::Default;
                }
            }
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
        let _ = reg_set_string(&dot_fb2, Some("Content Type"), "application/x-fictionbook+xml");

        let open_with = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\.fb2\OpenWithProgids")?;
        let _ = reg_set_string(&open_with, Some("QuietReader.fb2"), "");

        // 2. HKCU\Software\Classes\QuietReader.fb2
        let progid_key = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\QuietReader.fb2")?;
        let _ = reg_set_string(&progid_key, None, "Книга FictionBook (FB2)");
        let _ = reg_set_string(&progid_key, Some("FriendlyTypeName"), "Книга FictionBook (FB2)");

        let icon_key = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\QuietReader.fb2\DefaultIcon")?;
        let _ = reg_set_string(&icon_key, None, &doc_icon_cmd);

        let cmd_key = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\QuietReader.fb2\shell\open\command")?;
        let _ = reg_set_string(&cmd_key, None, &open_cmd);

        // 3. HKCU\Software\Classes\Applications\QuietReader.exe
        let app_key = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\Applications\QuietReader.exe")?;
        let _ = reg_set_string(&app_key, Some("FriendlyAppName"), "QuietReader");

        let app_icon_key = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\Applications\QuietReader.exe\DefaultIcon")?;
        let _ = reg_set_string(&app_icon_key, None, &app_icon_cmd);

        let supported = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\Applications\QuietReader.exe\SupportedTypes")?;
        let _ = reg_set_string(&supported, Some(".fb2"), "");

        let app_cmd = reg_create_key(HKEY_CURRENT_USER, r"Software\Classes\Applications\QuietReader.exe\shell\open\command")?;
        let _ = reg_set_string(&app_cmd, None, &open_cmd);

        // 4. HKCU\Software\QuietReader\Capabilities
        let caps = reg_create_key(HKEY_CURRENT_USER, r"Software\QuietReader\Capabilities")?;
        let _ = reg_set_string(&caps, Some("ApplicationName"), "QuietReader");
        let _ = reg_set_string(&caps, Some("ApplicationDescription"), "Минималистичная читалка книг FB2");

        let file_assoc = reg_create_key(HKEY_CURRENT_USER, r"Software\QuietReader\Capabilities\FileAssociations")?;
        let _ = reg_set_string(&file_assoc, Some(".fb2"), "QuietReader.fb2");

        // 5. HKCU\Software\RegisteredApplications
        let reg_apps = reg_create_key(HKEY_CURRENT_USER, r"Software\RegisteredApplications")?;
        let _ = reg_set_string(&reg_apps, Some("QuietReader"), r"Software\QuietReader\Capabilities");

        notify_shell_assoc_changed();

        Ok("QuietReader успешно зарегистрирован в реестре Windows (HKCU) для формата .fb2.".to_string())
    }

    pub fn is_association_prompt_dismissed() -> bool {
        reg_open_key(HKEY_CURRENT_USER, r"Software\QuietReader\Settings", KEY_READ)
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
