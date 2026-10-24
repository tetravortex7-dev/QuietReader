use crate::cli::{CliAction, MAX_FILES, parse_args};
use std::ffi::OsString;

fn args(items: &[&str]) -> impl Iterator<Item = OsString> {
    items
        .iter()
        .map(OsString::from)
        .collect::<Vec<_>>()
        .into_iter()
}

#[test]
fn parses_help_and_version_flags() {
    assert_eq!(parse_args(args(&["--help"])), CliAction::Help);
    assert_eq!(parse_args(args(&["-h"])), CliAction::Help);
    assert_eq!(parse_args(args(&["--version"])), CliAction::Version);
    assert_eq!(parse_args(args(&["-V"])), CliAction::Version);
}

#[test]
fn rejects_unknown_flags() {
    match parse_args(args(&["--unknown"])) {
        CliAction::Error(message) => assert!(message.contains("--unknown")),
        other => panic!("expected error, got {other:?}"),
    }
}

#[test]
fn collects_existing_files_and_rejects_missing() {
    let dir = crate::test::util::unique_test_dir("cli_args");
    let first = dir.join("first.epub");
    let second = dir.join("second.fb2");
    std::fs::write(&first, b"a").unwrap();
    std::fs::write(&second, b"b").unwrap();

    match parse_args(args(&[first.to_str().unwrap(), second.to_str().unwrap()])) {
        CliAction::Run(files) => {
            assert_eq!(files, vec![first.clone(), second.clone()]);
        }
        other => panic!("expected run, got {other:?}"),
    }

    match parse_args(args(&[dir.join("missing.epub").to_str().unwrap()])) {
        CliAction::Error(message) => assert!(message.contains("file not found")),
        other => panic!("expected error, got {other:?}"),
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn limits_number_of_files() {
    let dir = crate::test::util::unique_test_dir("cli_limit");
    let files: Vec<String> = (0..=MAX_FILES)
        .map(|index| {
            let path = dir.join(format!("book_{index}.txt"));
            std::fs::write(&path, b"x").unwrap();
            path.to_string_lossy().to_string()
        })
        .collect();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    match parse_args(args(&refs)) {
        CliAction::Error(message) => assert!(message.contains("too many files")),
        other => panic!("expected error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}
