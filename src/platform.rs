#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AssociationStatus {
    Default,
    Registered,
    NotRegistered,
}

pub fn supports_file_association() -> bool {
    cfg!(windows)
}

pub const SUPPORTED_EXTENSIONS_INFO: &[(&str, &str)] = &[
    ("fb2", "FB2 (.fb2) — Книги FictionBook"),
    ("epub", "EPUB (.epub) — Электронные книги"),
    ("mobi", "MOBI (.mobi, .prc) — Книги Mobipocket"),
    ("txt", "TXT (.txt) — Текстовые документы"),
    ("doc", "DOC (.doc) — Документы Word 97-2004"),
    ("docx", "DOCX (.docx) — Документы Word"),
    ("rtf", "RTF (.rtf) — Форматированный текст"),
    ("pdf", "PDF (.pdf) — Документы PDF"),
    ("djvu", "DJVU (.djvu, .djv) — Документы DjVu"),
    ("cbz", "CBZ (.cbz) — Комиксы CBZ (ZIP)"),
    ("cbr", "CBR (.cbr) — Комиксы CBR (RAR)"),
    ("cb7", "CB7 (.cb7) — Комиксы CB7 (7-Zip)"),
    ("md", "MD (.md) — Документы Markdown"),
];

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

        fn RegDeleteKeyW(hKey: *mut c_void, lpSubKey: *const u16) -> i32;

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

    pub fn check_extension_association_status(ext: &str) -> AssociationStatus {
        let ext_clean = ext.trim_start_matches('.').to_lowercase();
        let ext_dot = format!(".{ext_clean}");
        let progid_expected = format!("QuietReader.{ext_clean}");

        let current_exe = match std::env::current_exe() {
            Ok(p) => p.to_string_lossy().to_lowercase(),
            Err(_) => return AssociationStatus::NotRegistered,
        };

        let cmd_key_path = format!(r"Software\Classes\{progid_expected}\shell\open\command");
        let is_our_progid_registered = reg_open_key(HKEY_CURRENT_USER, &cmd_key_path, KEY_READ)
            .and_then(|k| reg_get_string(&k, None))
            .map(|cmd| cmd.to_lowercase().contains(&current_exe))
            .unwrap_or(false);

        if !is_our_progid_registered {
            return AssociationStatus::NotRegistered;
        }

        let user_choice_path = format!(
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext_dot}\UserChoice"
        );
        if let Some(user_choice_key) = reg_open_key(HKEY_CURRENT_USER, &user_choice_path, KEY_READ)
        {
            if let Some(progid) = reg_get_string(&user_choice_key, Some("ProgId"))
                && progid.eq_ignore_ascii_case(&progid_expected)
            {
                return AssociationStatus::Default;
            }
            return AssociationStatus::Registered;
        }

        let dot_key_path = format!(r"Software\Classes\{ext_dot}");
        if let Some(dot_key) = reg_open_key(HKEY_CURRENT_USER, &dot_key_path, KEY_READ)
            && let Some(progid) = reg_get_string(&dot_key, None)
            && progid.eq_ignore_ascii_case(&progid_expected)
        {
            return AssociationStatus::Default;
        }

        AssociationStatus::Registered
    }

    pub fn check_fb2_association_status() -> AssociationStatus {
        check_extension_association_status("fb2")
    }

    pub fn register_file_associations(extensions: &[&str]) -> Result<String, String> {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Не удалось определить путь к исполнимому файлу: {e}"))?;
        let exe_str = exe_path.to_string_lossy();
        let open_cmd = format!("\"{exe_str}\" \"%1\"");
        let doc_icon_cmd = format!("\"{exe_str}\",1");
        let app_icon_cmd = format!("\"{exe_str}\",0");

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

        let app_cmd = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\Applications\QuietReader.exe\shell\open\command",
        )?;
        let _ = reg_set_string(&app_cmd, None, &open_cmd);

        let supported = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\Classes\Applications\QuietReader.exe\SupportedTypes",
        )?;

        let caps = reg_create_key(HKEY_CURRENT_USER, r"Software\QuietReader\Capabilities")?;
        let _ = reg_set_string(&caps, Some("ApplicationName"), "QuietReader");
        let _ = reg_set_string(
            &caps,
            Some("ApplicationDescription"),
            "Модульная читалка электронных книг",
        );

        let file_assoc = reg_create_key(
            HKEY_CURRENT_USER,
            r"Software\QuietReader\Capabilities\FileAssociations",
        )?;

        let reg_apps = reg_create_key(HKEY_CURRENT_USER, r"Software\RegisteredApplications")?;
        let _ = reg_set_string(
            &reg_apps,
            Some("QuietReader"),
            r"Software\QuietReader\Capabilities",
        );

        let mut count = 0;
        for &raw_ext in extensions {
            let ext = raw_ext.trim_start_matches('.').to_lowercase();
            if ext.is_empty() {
                continue;
            }
            let ext_dot = format!(".{ext}");
            let progid = format!("QuietReader.{ext}");
            let (friendly_name, content_type) = match ext.as_str() {
                "fb2" => ("Книга FictionBook (FB2)", "application/x-fictionbook+xml"),
                "epub" => ("Книга EPUB", "application/epub+zip"),
                "txt" => ("Текстовый документ", "text/plain"),
                "md" => ("Документ Markdown", "text/markdown"),
                "log" => ("Журнал логов", "text/plain"),
                "pdf" => ("Документ PDF", "application/pdf"),
                "djvu" | "djv" => ("Документ DjVu", "image/vnd.djvu"),
                "cbz" => ("Комикс CBZ", "application/vnd.comicbook+zip"),
                "cbr" => ("Комикс CBR", "application/vnd.comicbook-rar"),
                "cb7" => ("Комикс CB7", "application/x-cb7"),
                "docx" => (
                    "Документ Microsoft Word (DOCX)",
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                ),
                "doc" => ("Документ Microsoft Word (DOC)", "application/msword"),
                "rtf" => ("Документ RTF", "application/rtf"),
                _ => ("Документ", "application/octet-stream"),
            };

            if let Ok(dot_key) =
                reg_create_key(HKEY_CURRENT_USER, &format!(r"Software\Classes\{ext_dot}"))
            {
                let _ = reg_set_string(&dot_key, None, &progid);
                let _ = reg_set_string(&dot_key, Some("Content Type"), content_type);
            }

            if let Ok(open_with) = reg_create_key(
                HKEY_CURRENT_USER,
                &format!(r"Software\Classes\{ext_dot}\OpenWithProgids"),
            ) {
                let _ = reg_set_string(&open_with, Some(&progid), "");
            }

            if let Ok(progid_key) =
                reg_create_key(HKEY_CURRENT_USER, &format!(r"Software\Classes\{progid}"))
            {
                let _ = reg_set_string(&progid_key, None, friendly_name);
                let _ = reg_set_string(&progid_key, Some("FriendlyTypeName"), friendly_name);
            }

            if let Ok(icon_key) = reg_create_key(
                HKEY_CURRENT_USER,
                &format!(r"Software\Classes\{progid}\DefaultIcon"),
            ) {
                let _ = reg_set_string(&icon_key, None, &doc_icon_cmd);
            }

            if let Ok(cmd_key) = reg_create_key(
                HKEY_CURRENT_USER,
                &format!(r"Software\Classes\{progid}\shell\open\command"),
            ) {
                let _ = reg_set_string(&cmd_key, None, &open_cmd);
            }

            let file_exts_open_with = format!(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext_dot}\OpenWithProgids"
            );
            if let Ok(file_exts_key) = reg_create_key(HKEY_CURRENT_USER, &file_exts_open_with) {
                let _ = reg_set_string(&file_exts_key, Some(&progid), "");
            }

            let file_exts_list = format!(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext_dot}\OpenWithList"
            );
            if let Ok(list_key) = reg_create_key(HKEY_CURRENT_USER, &file_exts_list) {
                let _ = reg_set_string(&list_key, Some("a"), "QuietReader.exe");
                let _ = reg_set_string(&list_key, Some("MRUList"), "a");
            }

            let user_choice_path = format!(
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\{ext_dot}\UserChoice"
            );
            let wide_uc = to_wide(&user_choice_path);
            unsafe {
                RegDeleteKeyW(HKEY_CURRENT_USER, wide_uc.as_ptr());
            }

            let _ = reg_set_string(&supported, Some(&ext_dot), "");
            let _ = reg_set_string(&file_assoc, Some(&ext_dot), &progid);
            count += 1;
        }

        notify_shell_assoc_changed();

        Ok(format!(
            "QuietReader успешно зарегистрирован в реестре Windows (HKCU) для {count} формат(ов)."
        ))
    }

    #[allow(dead_code)]
    pub fn register_fb2_association() -> Result<String, String> {
        register_file_associations(&["fb2"])
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
pub fn check_extension_association_status(ext: &str) -> AssociationStatus {
    windows_registry::check_extension_association_status(ext)
}

#[cfg(windows)]
pub fn check_fb2_association_status() -> AssociationStatus {
    windows_registry::check_fb2_association_status()
}

#[cfg(windows)]
pub fn register_file_associations(extensions: &[&str]) -> Result<String, String> {
    windows_registry::register_file_associations(extensions)
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn register_fb2_association() -> Result<String, String> {
    windows_registry::register_fb2_association()
}

#[cfg(windows)]
#[allow(dead_code)]
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
pub fn check_extension_association_status(_ext: &str) -> AssociationStatus {
    AssociationStatus::NotRegistered
}

#[cfg(not(windows))]
pub fn check_fb2_association_status() -> AssociationStatus {
    AssociationStatus::NotRegistered
}

#[cfg(not(windows))]
pub fn register_file_associations(_extensions: &[&str]) -> Result<String, String> {
    Ok("Регистрация ассоциаций поддерживается только в Windows.".to_string())
}

#[cfg(not(windows))]
pub fn register_fb2_association() -> Result<String, String> {
    Ok("Регистрация ассоциаций поддерживается только в Windows.".to_string())
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn open_default_apps_settings() {}

#[cfg(not(windows))]
pub fn is_association_prompt_dismissed() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_association_prompt_dismissed(_dismissed: bool) {}

#[allow(dead_code)]
pub fn load_system_font_bytes(font_name: &str) -> Option<Vec<u8>> {
    platform_fonts::load_font_variant(font_name, false, false)
}

pub fn load_system_font_variant_bytes(
    font_name: &str,
    bold: bool,
    italic: bool,
) -> Option<Vec<u8>> {
    platform_fonts::load_font_variant(font_name, bold, italic)
}

pub fn load_system_cjk_font_bytes() -> Option<Vec<u8>> {
    platform_fonts::load_cjk_font()
}

pub fn load_system_arabic_font_bytes() -> Option<Vec<u8>> {
    platform_fonts::load_arabic_font()
}

#[cfg(windows)]
mod platform_fonts {
    use std::path::PathBuf;

    pub fn load_font_variant(font_name: &str, bold: bool, italic: bool) -> Option<Vec<u8>> {
        let filename = match (font_name, bold, italic) {
            ("Georgia", false, false) => "georgia.ttf",
            ("Georgia", true, false) => "georgiab.ttf",
            ("Georgia", false, true) => "georgiai.ttf",
            ("Georgia", true, true) => "georgiaz.ttf",

            ("Palatino Linotype", false, false) => "pala.ttf",
            ("Palatino Linotype", true, false) => "palab.ttf",
            ("Palatino Linotype", false, true) => "palai.ttf",
            ("Palatino Linotype", true, true) => "palabi.ttf",

            ("Calibri", false, false) => "calibri.ttf",
            ("Calibri", true, false) => "calibrib.ttf",
            ("Calibri", false, true) => "calibrii.ttf",
            ("Calibri", true, true) => "calibriz.ttf",

            ("Segoe UI", false, false) => "segoeui.ttf",
            ("Segoe UI", true, false) => "segoeuib.ttf",
            ("Segoe UI", false, true) => "segoeuii.ttf",
            ("Segoe UI", true, true) => "segoeuiz.ttf",

            ("Verdana", false, false) => "verdana.ttf",
            ("Verdana", true, false) => "verdanab.ttf",
            ("Verdana", false, true) => "verdanai.ttf",
            ("Verdana", true, true) => "verdanaz.ttf",

            ("Comic Sans MS", false, false) => "comic.ttf",
            ("Comic Sans MS", true, false) => "comicbd.ttf",
            ("Comic Sans MS", false, true) => "comici.ttf",
            ("Comic Sans MS", true, true) => "comicz.ttf",

            ("Sitka", false, false) => "SitkaVF.ttf",
            ("Sitka", true, false) => "SitkaVF.ttf",
            ("Sitka", false, true) => "SitkaVF-Italic.ttf",
            ("Sitka", true, true) => "SitkaVF-Italic.ttf",

            ("OpenDyslexic", false, false) => "OpenDyslexic-Regular.otf",
            ("OpenDyslexic", true, false) => "OpenDyslexic-Bold.otf",
            ("OpenDyslexic", false, true) => "OpenDyslexic-Italic.otf",
            ("OpenDyslexic", true, true) => "OpenDyslexic-BoldItalic.otf",

            _ => {
                if bold && italic {
                    "georgiaz.ttf"
                } else if bold {
                    "georgiab.ttf"
                } else if italic {
                    "georgiai.ttf"
                } else {
                    "georgia.ttf"
                }
            }
        };

        if let Some(windir) = std::env::var_os("WINDIR") {
            let p = PathBuf::from(windir).join("Fonts").join(filename);
            if let Ok(bytes) = std::fs::read(&p) {
                return Some(bytes);
            }
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let p = PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts")
                .join(filename);
            if let Ok(bytes) = std::fs::read(&p) {
                return Some(bytes);
            }
        }
        let fallback = PathBuf::from(r"C:\Windows\Fonts").join(filename);
        if let Ok(bytes) = std::fs::read(&fallback) {
            return Some(bytes);
        }
        if font_name == "OpenDyslexic" {
            let comic_fn = if bold && italic {
                "comicz.ttf"
            } else if bold {
                "comicbd.ttf"
            } else if italic {
                "comici.ttf"
            } else {
                "comic.ttf"
            };
            return std::fs::read(PathBuf::from(r"C:\Windows\Fonts").join(comic_fn)).ok();
        }
        None
    }

    pub fn load_cjk_font() -> Option<Vec<u8>> {
        let candidates = ["msyh.ttc", "msyh.ttf", "simsun.ttc", "YuGothR.ttc", "msgothic.ttc"];
        if let Some(windir) = std::env::var_os("WINDIR") {
            let fonts_dir = PathBuf::from(windir).join("Fonts");
            for cand in candidates {
                let p = fonts_dir.join(cand);
                if let Ok(bytes) = std::fs::read(&p) {
                    return Some(bytes);
                }
            }
        }
        for cand in candidates {
            let fallback = PathBuf::from(r"C:\Windows\Fonts").join(cand);
            if let Ok(bytes) = std::fs::read(fallback) {
                return Some(bytes);
            }
        }
        None
    }

    pub fn load_arabic_font() -> Option<Vec<u8>> {
        let candidates = ["tahoma.ttf", "segoeui.ttf", "arial.ttf", "times.ttf"];
        if let Some(windir) = std::env::var_os("WINDIR") {
            let fonts_dir = PathBuf::from(windir).join("Fonts");
            for cand in candidates {
                let p = fonts_dir.join(cand);
                if let Ok(bytes) = std::fs::read(&p) {
                    return Some(bytes);
                }
            }
        }
        for cand in candidates {
            let fallback = PathBuf::from(r"C:\Windows\Fonts").join(cand);
            if let Ok(bytes) = std::fs::read(fallback) {
                return Some(bytes);
            }
        }
        None
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

    pub fn load_font_variant(font_name: &str, _bold: bool, _italic: bool) -> Option<Vec<u8>> {
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

    pub fn load_cjk_font() -> Option<Vec<u8>> {
        let candidates = [
            "NotoSansCJK-Regular.ttc",
            "NotoSansSC-Regular.otf",
            "NotoSansCJK.ttc",
            "wqy-microhei.ttc",
            "wqy-zenhei.ttc",
            "DroidSansFallbackFull.ttf",
        ];
        let mut dirs = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
            dirs.push(PathBuf::from(home).join(".fonts"));
        }
        for dir in dirs {
            if let Some(bytes) = find_font_in_dir(&dir, &candidates, 0) {
                return Some(bytes);
            }
        }
        None
    }

    pub fn load_arabic_font() -> Option<Vec<u8>> {
        let candidates = [
            "NotoSansArabic-Regular.ttf",
            "NotoSansArabic-Regular.otf",
            "DejaVuSans.ttf",
            "amiri-regular.ttf",
            "Scheherazade-Regular.ttf",
        ];
        let mut dirs = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
            dirs.push(PathBuf::from(home).join(".fonts"));
        }
        for dir in dirs {
            if let Some(bytes) = find_font_in_dir(&dir, &candidates, 0) {
                return Some(bytes);
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
mod platform_fonts {
    use std::path::PathBuf;

    pub fn load_font_variant(font_name: &str, _bold: bool, _italic: bool) -> Option<Vec<u8>> {
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

    pub fn load_cjk_font() -> Option<Vec<u8>> {
        let candidates = [
            "PingFang.ttc",
            "Hiragino Sans GB.ttc",
            "Arial Unicode.ttf",
            "STHeiti Light.ttc",
        ];
        let mut dirs = vec![
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/System/Library/Fonts/Supplemental"),
            PathBuf::from("/Library/Fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }

        for dir in dirs {
            for &cand in &candidates {
                let p = dir.join(cand);
                if let Ok(bytes) = std::fs::read(&p) {
                    return Some(bytes);
                }
            }
        }
        None
    }

    pub fn load_arabic_font() -> Option<Vec<u8>> {
        let candidates = [
            "GeezaPro.ttc",
            "Geeza Pro.ttf",
            "Arial.ttf",
            "Times.ttc",
            "SFNS.ttf",
        ];
        let mut dirs = vec![
            PathBuf::from("/System/Library/Fonts"),
            PathBuf::from("/System/Library/Fonts/Supplemental"),
            PathBuf::from("/Library/Fonts"),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }

        for dir in dirs {
            for &cand in &candidates {
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
    pub fn load_font_variant(_font_name: &str, _bold: bool, _italic: bool) -> Option<Vec<u8>> {
        None
    }

    pub fn load_cjk_font() -> Option<Vec<u8>> {
        None
    }

    pub fn load_arabic_font() -> Option<Vec<u8>> {
        None
    }
}

pub fn detect_system_ui_language() -> Option<String> {
    #[cfg(windows)]
    {
        unsafe extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
            fn GetUserDefaultLocaleName(lpLocaleName: *mut u16, cchLocaleName: i32) -> i32;
        }
        let langid = unsafe { GetUserDefaultUILanguage() };
        let primary = langid & 0x3ff;
        match primary {
            0x19 | 0x23 | 0x3f => return Some("ru".to_string()),
            0x22 => return Some("uk".to_string()),
            0x10 => return Some("it".to_string()),
            0x1f => return Some("tr".to_string()),
            0x01 => return Some("ar".to_string()),
            0x21 => return Some("id".to_string()),
            0x09 => return Some("en".to_string()),
            0x0c => return Some("fr".to_string()),
            0x0a => return Some("es".to_string()),
            0x04 => return Some("zh".to_string()),
            0x11 => return Some("ja".to_string()),
            0x16 => return Some("pt".to_string()),
            _ => {}
        }
        let mut buffer = [0u16; 85];
        let len = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
        if len > 1 {
            let name = String::from_utf16_lossy(&buffer[..len as usize - 1]);
            return Some(name);
        }
    }

    for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let trimmed = val.trim();
            if !trimmed.is_empty() && trimmed != "C" && trimmed != "POSIX" {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

pub mod single_instance;
pub mod window;

pub use single_instance::*;
pub use window::*;

