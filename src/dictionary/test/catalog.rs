#[test]
fn parses_stardict_release_from_catalog() {
    let json = r#"[
      {
        "name": "eng-rus",
        "headwords": "120000",
        "edition": "2025.11.23",
        "releases": [
          {"URL": "https://example.org/eng-rus.src.tar.xz", "checksum": "aa", "platform": "src", "size": "10"},
          {"URL": "https://example.org/eng-rus.stardict.tar.xz", "checksum": "bb", "platform": "stardict", "size": "2048"}
        ]
      },
      {
        "name": "rus-eng",
        "headwords": 50000,
        "edition": "1.0",
        "releases": [
          {"URL": "https://example.org/rus-eng.stardict.tar.xz", "checksum": "cc", "platform": "stardict", "size": 1024}
        ]
      }
    ]"#;

    let entries = crate::dictionary::catalog::parse_catalog(json).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, "eng-rus");
    assert_eq!(entries[0].headwords, 120000);
    assert_eq!(entries[0].size, 2048);
    assert!(entries[0].url.ends_with("eng-rus.stardict.tar.xz"));
    assert_eq!(entries[1].headwords, 50000);
    assert_eq!(entries[1].size, 1024);
}

#[test]
fn skips_unsafe_or_incomplete_releases() {
    let json = r#"[
      {
        "name": "a-b",
        "releases": [
          {"URL": "http://example.org/a-b.stardict.tar.xz", "checksum": "aa", "platform": "stardict"}
        ]
      },
      {
        "name": "c-d",
        "releases": [
          {"URL": "https://example.org/c-d.stardict.tar.xz", "checksum": "", "platform": "stardict"}
        ]
      },
      {
        "name": "e-f",
        "releases": [
          {"URL": "https://example.org/e-f.dictd.tar.xz", "checksum": "bb", "platform": "dictd"}
        ]
      }
    ]"#;

    assert!(
        crate::dictionary::catalog::parse_catalog(json)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn skips_non_dictionary_entries_in_catalog() {
    let json = r#"[
      {
        "name": "deu-eng",
        "headwords": "90000",
        "edition": "1.9",
        "releases": [
          {"URL": "https://example.org/deu-eng.stardict.tar.xz", "checksum": "dd", "platform": "stardict", "size": "4096"}
        ]
      },
      {
        "software": {
          "tools": {
            "URL": "https://example.org/tools.tar.gz",
            "checksum": "ee",
            "version": "0.7.0"
          }
        }
      }
    ]"#;

    let entries = crate::dictionary::catalog::parse_catalog(json).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "deu-eng");
}

#[test]
fn skips_broken_dictionary_entries_in_catalog() {
    let json = r#"[
      {
        "name": "fra-eng",
        "edition": {"nested": true},
        "releases": [
          {"URL": "https://example.org/fra-eng.stardict.tar.xz", "checksum": "ff", "platform": "stardict"}
        ]
      },
      {
        "name": "eng-fra",
        "releases": [
          {"URL": "https://example.org/eng-fra.stardict.tar.xz", "checksum": "aa", "platform": "stardict"}
        ]
      }
    ]"#;

    let entries = crate::dictionary::catalog::parse_catalog(json).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "eng-fra");
}
