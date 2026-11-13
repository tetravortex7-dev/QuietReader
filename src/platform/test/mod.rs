use crate::document::{Document, Fb2Document};
use crate::settings::AppSettings;
use crate::test::util::unique_test_dir;

#[test]
fn test_platform_multi_extension_selection() {
    let exts: Vec<&str> = crate::platform::supported_extensions().to_vec();
    assert!(exts.contains(&"fb2"));
    assert!(exts.contains(&"txt"));
    assert!(exts.contains(&"doc"));
    assert!(exts.contains(&"docx"));
    assert!(exts.contains(&"epub"));
    assert!(exts.contains(&"mobi"));
    assert!(exts.contains(&"azw3"));
    assert!(exts.contains(&"rtf"));
    assert!(exts.contains(&"pdf"));
    assert!(exts.contains(&"djvu"));
    assert!(exts.contains(&"cbz"));
    assert!(exts.contains(&"cbr"));
    assert!(exts.contains(&"cb7"));
    assert!(exts.contains(&"cbt"));
    assert!(exts.contains(&"md"));

    let mut settings = AppSettings::default();
    assert_eq!(settings.associated_extensions, vec!["fb2", "epub"]);

    settings.associated_extensions = vec!["fb2".to_string(), "txt".to_string(), "doc".to_string()];
    let json = serde_json::to_string(&settings).expect("serialize settings");
    let loaded: AppSettings = serde_json::from_str(&json).expect("deserialize settings");
    assert_eq!(loaded.associated_extensions, vec!["fb2", "txt", "doc"]);
}

#[test]
fn test_platform_keep_screen_on() {
    crate::platform::set_keep_screen_on(true);
    crate::platform::set_keep_screen_on(false);
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn test_xdg_downloads_dir_parsing() {
    use std::path::{Path, PathBuf};
    let home = Path::new("/home/reader");
    assert_eq!(
        crate::platform::downloads::parse_xdg_download_dir(
            "XDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\n",
            Some(home)
        ),
        Some(PathBuf::from("/home/reader/Downloads"))
    );
    assert_eq!(
        crate::platform::downloads::parse_xdg_download_dir(
            "XDG_DOWNLOAD_DIR=\"/mnt/books/downloads\"\n",
            Some(home)
        ),
        Some(PathBuf::from("/mnt/books/downloads"))
    );
    assert_eq!(
        crate::platform::downloads::parse_xdg_download_dir("XDG_DOWNLOAD_DIR=\"\"\n", Some(home)),
        None
    );
}

#[test]
fn test_system_downloads_dir_is_absolute() {
    assert!(crate::platform::downloads::resolve_downloads_dir().is_absolute());
}

#[test]
fn test_platform_file_association_query_and_dismiss() {
    let status = crate::platform::check_fb2_association_status();
    assert!(matches!(
        status,
        crate::platform::AssociationStatus::Default
            | crate::platform::AssociationStatus::Registered
            | crate::platform::AssociationStatus::NotRegistered
    ));

    crate::platform::set_association_prompt_dismissed(true);
    assert!(crate::platform::is_association_prompt_dismissed());
    crate::platform::set_association_prompt_dismissed(false);
    assert!(!crate::platform::is_association_prompt_dismissed());
}

#[test]
#[ignore = "РР·РјРµРЅСЏРµС‚ СЂРµР°Р»СЊРЅС‹Рµ СЃРёСЃС‚РµРјРЅС‹Рµ Р°СЃСЃРѕС†РёР°С†РёРё С„Р°Р№Р»РѕРІ РІ СЂРµРµСЃС‚СЂРµ РћРЎ"]
fn test_platform_register_association() {
    let result = crate::platform::register_fb2_association();
    assert!(result.is_ok());

    let status = crate::platform::check_fb2_association_status();
    assert!(
        status == crate::platform::AssociationStatus::Registered
            || status == crate::platform::AssociationStatus::Default
    );
}

#[cfg(windows)]
#[test]
fn test_windows_secret_storage_roundtrip() {
    let key = format!(
        "test_secret_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let value = "quietreader-secret-test";

    crate::platform::secrets::store(&key, value).expect("store secret");
    assert_eq!(
        crate::platform::secrets::load(&key)
            .expect("load secret")
            .as_deref(),
        Some(value)
    );
    crate::platform::secrets::delete(&key).expect("delete secret");
    assert_eq!(
        crate::platform::secrets::load(&key).expect("load deleted secret"),
        None
    );
}

#[test]
fn test_command_line_path_handling() {
    let temp_file = std::env::temp_dir().join(format!(
        "test_spaces_{}_{}.fb2",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    let xml = r#"<FictionBook><body><section><title><p>Р“Р»Р°РІР° 1</p></title><p>РўРµРєСЃС‚</p></section></body></FictionBook>"#;
    std::fs::write(&temp_file, xml).unwrap();

    let doc = Fb2Document::from_file(&temp_file)
        .expect("Р”РѕР»Р¶РµРЅ РѕС‚РєСЂС‹С‚СЊ С„Р°Р№Р» РїРѕ РїСѓС‚Рё СЃ РїСЂРѕР±РµР»Р°РјРё");
    assert_eq!(doc.toc().len(), 1);
    assert_eq!(doc.toc()[0].title, "Р“Р»Р°РІР° 1");

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_ipc_payload_validation() {
    let dir = unique_test_dir("ipc_payload");
    let file = dir.join("book.fb2");
    std::fs::write(&file, b"test").unwrap();
    let canonical = std::fs::canonicalize(&file).unwrap();

    let payload = format!("ACTIVATE\n{}\n{}\n", file.display(), dir.display());
    let parsed = crate::platform::single_instance::parse_ipc_payload(&payload);
    assert_eq!(parsed, vec![canonical]);

    let invalid = format!("OPEN\n{}\n", file.display());
    assert!(crate::platform::single_instance::parse_ipc_payload(&invalid).is_empty());

    let repeated = format!("ACTIVATE\n{}", format!("{}\n", file.display()).repeat(40));
    assert_eq!(
        crate::platform::single_instance::parse_ipc_payload(&repeated).len(),
        32
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_ipc_private_file_limits() {
    let dir = unique_test_dir("ipc_file");
    let path = dir.join("request.req");

    crate::platform::single_instance::write_private_file(&path, b"ACTIVATE\n").unwrap();
    assert_eq!(
        crate::platform::single_instance::read_private_file(&path, false).as_deref(),
        Some("ACTIVATE\n")
    );
    assert!(crate::platform::single_instance::read_private_file(&path, true).is_some());
    assert!(!path.exists());

    let too_large = vec![b'x'; 70 * 1024];
    assert!(crate::platform::single_instance::write_private_file(&path, &too_large).is_err());
    assert!(!path.exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let target = dir.join("target.txt");
        std::fs::write(&target, b"secret").unwrap();
        let link = dir.join("link.req");
        symlink(&target, &link).unwrap();
        assert!(crate::platform::single_instance::read_private_file(&link, true).is_none());
        assert!(!link.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "secret");

        symlink(&target, &link).unwrap();
        crate::platform::single_instance::write_private_file(&link, b"ACTIVATE\n").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "secret");
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "ACTIVATE\n");
    }

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_sync_fullscreen_state_reads_platform_state() {
    let expected = crate::platform::is_window_fullscreen();
    let mut state = !expected;
    crate::platform::sync_fullscreen_state(&mut state);
    assert_eq!(state, expected);
}

#[test]
fn test_create_private_dir() {
    let dir = unique_test_dir("private_dir");
    let nested = dir.join("a").join("b");
    crate::platform::permissions::create_private_dir(&nested);
    assert!(nested.is_dir());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&nested).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_windows_acl_args_and_principal_fallback() {
    use crate::platform::permissions::{build_icacls_args, fallback_windows_principal};

    assert_eq!(
        fallback_windows_principal(Some("CORP"), Some("reader")),
        Some("CORP\\reader".to_string())
    );
    assert_eq!(
        fallback_windows_principal(None, Some("reader")),
        Some("reader".to_string())
    );
    assert_eq!(fallback_windows_principal(Some("CORP"), Some("   ")), None);

    let dir_args = build_icacls_args("*S-1-5-21-1234", true);
    assert_eq!(
        dir_args,
        vec![
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-21-1234:(OI)(CI)F",
            "*S-1-5-18:(OI)(CI)F",
            "*S-1-5-32-544:(OI)(CI)F",
        ]
    );

    let file_args = build_icacls_args("*S-1-5-21-1234", false);
    assert_eq!(
        file_args,
        vec![
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-21-1234:F",
            "*S-1-5-18:F",
            "*S-1-5-32-544:F",
        ]
    );
}

#[cfg(windows)]
#[test]
fn test_windows_private_dir_allows_child_io_and_replaces_junction_safely() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let dir = unique_test_dir("private_dir_win");
    let nested = dir.join("store");
    crate::platform::permissions::create_private_dir(&nested);
    let child_dir = nested.join("sub");
    std::fs::create_dir_all(&child_dir).expect("create child dir under restricted dir");
    std::fs::write(child_dir.join("note.txt"), b"ok").expect("write child file");

    let target = dir.join("outside_target");
    std::fs::create_dir_all(&target).expect("create junction target");
    std::fs::write(target.join("keep.txt"), b"preserved").expect("write target file");

    let junction = dir.join("junction_dir");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .expect("run mklink")
        .status;
    assert!(status.success());

    crate::platform::permissions::create_private_dir(&junction);
    assert!(junction.is_dir());
    let meta = std::fs::symlink_metadata(&junction).expect("junction replaced by dir");
    assert!(!meta.file_type().is_symlink());
    assert_eq!(
        std::fs::read_to_string(target.join("keep.txt")).expect("target preserved"),
        "preserved"
    );

    std::fs::remove_dir_all(&dir).expect("cleanup restricted dir");
}

#[test]
fn test_ipc_write_private_file_overwrites_without_gap_and_matches_windows() {
    use crate::platform::single_instance::{
        read_private_file, should_accept_window, window_title_matches, write_private_file,
    };

    let dir = unique_test_dir("ipc_overwrite");
    let req = dir.join("request.req");
    write_private_file(&req, b"ACTIVATE\nfirst\n").expect("first write");
    write_private_file(&req, b"ACTIVATE\nsecond\n").expect("atomic overwrite");
    assert_eq!(
        read_private_file(&req, true).as_deref(),
        Some("ACTIVATE\nsecond\n")
    );
    let _ = std::fs::remove_dir_all(&dir);

    assert!(window_title_matches("QuietReader"));
    assert!(window_title_matches("QuietReader — Book"));
    assert!(!window_title_matches("Notepad"));

    assert!(!should_accept_window(
        true,
        true,
        false,
        Some("QuietReader")
    ));
    assert!(should_accept_window(
        true,
        false,
        false,
        Some("QuietReader")
    ));
    assert!(should_accept_window(true, true, true, Some("QuietReader")));
    assert!(!should_accept_window(
        false,
        false,
        false,
        Some("QuietReader")
    ));
    assert!(!should_accept_window(true, false, false, Some("Other App")));
    assert!(should_accept_window(true, false, false, None));
}

#[test]
fn test_dpi_scale_factor_and_monitor_clamping() {
    use crate::platform::{
        MonitorWorkArea, normalize_scale_factor, sanitize_geometry_with_monitors,
        scale_factor_from_dpi,
    };

    assert_eq!(scale_factor_from_dpi(0), 1.0);
    assert_eq!(scale_factor_from_dpi(96), 1.0);
    assert_eq!(scale_factor_from_dpi(120), 1.25);
    assert_eq!(scale_factor_from_dpi(144), 1.5);
    assert_eq!(scale_factor_from_dpi(192), 2.0);
    assert_eq!(normalize_scale_factor(Some(f32::NAN)), None);
    assert_eq!(normalize_scale_factor(Some(0.2)), None);
    assert_eq!(normalize_scale_factor(Some(1.5)), Some(1.5));

    let mon_150 = MonitorWorkArea {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1032,
        scale: 1.5,
    };

    let (sx, sy, sw, sh) = sanitize_geometry_with_monitors(
        None,
        None,
        1050.0,
        750.0,
        None,
        Some(mon_150),
        |_, _, _, _, _| Some(mon_150),
    );
    assert_eq!((sx, sy), (None, None));
    assert_eq!(sw, 1050.0);
    assert!((sh - 688.0).abs() < 0.01);

    let (ax, ay, aw, ah) = sanitize_geometry_with_monitors(
        Some(1150.0),
        Some(600.0),
        900.0,
        650.0,
        Some(1.5),
        Some(mon_150),
        |_, _, _, _, _| Some(mon_150),
    );
    assert_eq!((ax, ay), (Some(1150.0), Some(600.0)));
    assert!((aw - 900.0).abs() < 0.01 && (ah - 650.0).abs() < 0.01);

    for (in_x, in_y) in [(1240.0, 665.0), (2500.0, 1500.0)] {
        let (rx, ry, rw, rh) = sanitize_geometry_with_monitors(
            Some(in_x),
            Some(in_y),
            900.0,
            650.0,
            Some(1.5),
            Some(mon_150),
            |_, _, _, _, _| Some(mon_150),
        );
        let rx = rx.expect("clamped x");
        let ry = ry.expect("clamped y");
        assert!(
            rx >= -0.5 && rx + rw <= 1280.5,
            "input=({in_x},{in_y}) rx={rx} rw={rw}"
        );
        assert!(
            ry >= -0.5 && ry + rh <= 688.5,
            "input=({in_x},{in_y}) ry={ry} rh={rh}"
        );
    }
}

#[test]
fn test_mixed_dpi_multimonitor_geometry_recovery() {
    use crate::platform::{MonitorWorkArea, sanitize_geometry_with_monitors};

    let primary = MonitorWorkArea {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
        scale: 1.0,
    };
    let secondary = MonitorWorkArea {
        left: 1920,
        top: 0,
        right: 5760,
        bottom: 2080,
        scale: 2.0,
    };

    let lookup_both = |px: i32, py: i32, pw: i32, ph: i32, _s: f32| {
        let cx = px + pw / 2;
        let cy = py + ph / 2;
        if cx >= secondary.left
            && cx < secondary.right
            && cy >= secondary.top
            && cy < secondary.bottom
        {
            Some(secondary)
        } else {
            Some(primary)
        }
    };

    let (sx, sy, sw, sh) = sanitize_geometry_with_monitors(
        Some(1100.0),
        Some(80.0),
        900.0,
        600.0,
        Some(2.0),
        Some(primary),
        lookup_both,
    );
    assert_eq!(sx, Some(2200.0));
    assert_eq!(sy, Some(160.0));
    assert_eq!((sw, sh), (900.0, 600.0));

    let (fx, fy, fw, fh) = sanitize_geometry_with_monitors(
        Some(1100.0),
        Some(80.0),
        900.0,
        600.0,
        Some(2.0),
        Some(primary),
        |_, _, _, _, _| Some(primary),
    );
    let fx = fx.expect("recovered x");
    let fy = fy.expect("recovered y");
    assert!(fx >= 0.0 && fx + fw <= 1920.0);
    assert!(fy >= 0.0 && fy + fh <= 1040.0);
}

#[test]
fn test_window_fullscreen_rejects_invalid_and_foreign_hwnd_safely() {
    use crate::platform::{
        is_current_thread_window_owner, is_window_fullscreen, set_window_fullscreen,
    };

    let before = is_window_fullscreen();
    assert!(!is_current_thread_window_owner(0));
    assert!(!is_current_thread_window_owner(0x7FFF_DEAD));
    assert!(!set_window_fullscreen(0, true));
    assert!(!set_window_fullscreen(0x7FFF_DEAD, true));
    assert_eq!(is_window_fullscreen(), before);

    let bg = std::thread::spawn(|| {
        assert!(!is_current_thread_window_owner(0));
        assert!(!set_window_fullscreen(0, !is_window_fullscreen()))
    });
    bg.join().expect("thread finished");
    assert_eq!(is_window_fullscreen(), before);
}
