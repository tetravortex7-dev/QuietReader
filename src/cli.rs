use std::ffi::OsString;
use std::path::PathBuf;

pub const MAX_FILES: usize = 16;

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Run(Vec<PathBuf>),
    Help,
    Version,
    Error(String),
}

pub fn parse_args(args: impl Iterator<Item = OsString>) -> CliAction {
    let mut files = Vec::new();
    for arg in args {
        let text = arg.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => return CliAction::Help,
            "-V" | "--version" => return CliAction::Version,
            _ if text.starts_with('-') && text.len() > 1 => {
                return CliAction::Error(format!("unknown option: {text}"));
            }
            _ => {
                let path = PathBuf::from(&arg);
                if path.as_os_str().is_empty() {
                    continue;
                }
                if !path.exists() {
                    return CliAction::Error(format!("file not found: {}", path.display()));
                }
                if files.len() >= MAX_FILES {
                    return CliAction::Error(format!("too many files (limit {MAX_FILES})"));
                }
                files.push(path);
            }
        }
    }
    CliAction::Run(files)
}

pub fn print_version() {
    write_console(&format!("QuietReader {}\n", env!("CARGO_PKG_VERSION")));
}

pub fn print_help() {
    let t = crate::i18n::UiLanguage::Auto.resolve().t();
    write_console(&format!("{}\n", t.cli_usage));
}

pub fn print_error(message: &str) {
    let t = crate::i18n::UiLanguage::Auto.resolve().t();
    write_console(&format!("{}\n", t.cli_error.replace("{}", message)));
}

fn write_console(text: &str) {
    #[cfg(windows)]
    {
        use std::io::Write;
        if stdout_is_redirected() {
            let mut out = std::io::stdout();
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
            return;
        }
        attach_parent_console();
        if let Ok(mut console) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
            let _ = console.write_all(text.as_bytes());
            return;
        }
        let _ = write!(std::io::stdout(), "{text}");
    }
    #[cfg(not(windows))]
    {
        print!("{text}");
    }
}

#[cfg(windows)]
fn stdout_is_redirected() -> bool {
    const STD_OUTPUT_HANDLE: i32 = -11;
    const FILE_TYPE_CHAR: u32 = 0x0002;
    unsafe extern "system" {
        fn GetStdHandle(which: i32) -> isize;
        fn GetFileType(handle: isize) -> u32;
    }
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle == 0 || handle == -1 {
            return false;
        }
        GetFileType(handle) != FILE_TYPE_CHAR
    }
}

#[cfg(windows)]
fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
