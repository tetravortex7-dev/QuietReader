use crate::app::dictionary::words_to_csv;
use crate::settings::SavedWord;

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
