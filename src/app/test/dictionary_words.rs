use crate::app::dictionary::{DictionaryState, words_to_csv};
use crate::settings::SavedWord;
use crate::test::util::unique_test_dir;

#[test]
fn exports_saved_words_as_anki_friendly_csv() {
    let words = vec![
        SavedWord {
            word: "apple".to_string(),
            definition: "fruit, round".to_string(),
            pack: "eng-rus".to_string(),
            language: "en".to_string(),
            book: "Book \"One\"".to_string(),
            created_at: 1,
        },
        SavedWord {
            word: "pear".to_string(),
            definition: "fruit".to_string(),
            pack: String::new(),
            language: "en".to_string(),
            book: String::new(),
            created_at: 2,
        },
    ];

    let csv = words_to_csv(&words);
    assert!(csv.starts_with("Word,Definition,Pack,Language,Book\n"));
    assert!(csv.contains("\"fruit, round\""));
    assert!(csv.contains("\"Book \"\"One\"\"\""));
    assert!(csv.contains("pear,fruit,,en,\n"));
}

#[test]
fn delete_pack_rejects_path_traversal_and_removes_only_valid_pack_dir() {
    let dir = unique_test_dir("dict_delete_traversal");
    let packs_dir = dir.join("packs");
    let outside_dir = dir.join("outside_secret");
    let valid_pack = packs_dir.join("eng-rus");
    std::fs::create_dir_all(&outside_dir).expect("create outside dir");
    std::fs::write(outside_dir.join("keep.txt"), b"keep").expect("write outside file");
    std::fs::create_dir_all(&valid_pack).expect("create valid pack");
    std::fs::write(valid_pack.join("dummy.ifo"), b"ifo").expect("write pack file");

    let mut state = DictionaryState::from_packs_dir(packs_dir.clone());
    state.delete_pack("../outside_secret");
    state.delete_pack("..\\outside_secret");
    state.delete_pack("..");
    state.delete_pack("");
    assert!(outside_dir.join("keep.txt").exists());
    assert!(valid_pack.exists());

    state.delete_pack("eng-rus");
    assert!(!valid_pack.exists());
    assert!(outside_dir.join("keep.txt").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn words_to_csv_neutralizes_formula_injection_and_surfaces_write_errors() {
    let words = vec![SavedWord {
        word: "=1+1".to_string(),
        definition: "@macro, danger".to_string(),
        pack: "+pack".to_string(),
        language: "-lang".to_string(),
        book: "  =Book".to_string(),
        created_at: 1,
    }];
    let csv = words_to_csv(&words);
    assert!(csv.contains("'=1+1,\"'@macro, danger\",'+pack,'-lang,'  =Book\n"));

    let dir = unique_test_dir("dict_export_err");
    let ok_file = dir.join("words.csv");
    let ok_msg =
        crate::app::bookmarks_view::save_export_text(&ok_file, &csv, "Exported", "Failed: {}");
    assert_eq!(ok_msg, "Exported");
    assert_eq!(std::fs::read_to_string(&ok_file).unwrap(), csv);

    let err_msg =
        crate::app::bookmarks_view::save_export_text(&dir, &csv, "Exported", "Failed: {}");
    assert!(err_msg.starts_with("Failed: "));

    let _ = std::fs::remove_dir_all(&dir);
}
