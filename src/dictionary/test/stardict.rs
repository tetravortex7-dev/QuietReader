use crate::dictionary::stardict::{discover, load_index, lookup, parse_definition};
use std::fs;

fn push_index_entry(index: &mut Vec<u8>, word: &str, offset: u32, size: u32) {
    index.extend_from_slice(word.as_bytes());
    index.push(0);
    index.extend_from_slice(&offset.to_be_bytes());
    index.extend_from_slice(&size.to_be_bytes());
}

#[test]
fn looks_up_word_from_stardict_pack() {
    let root = crate::test::util::unique_test_dir("stardict");
    let pack_dir = root.join("demo-pack");
    fs::create_dir_all(&pack_dir).unwrap();

    let ifo = "StarDict's dict ifo file\nversion=3.0.0\nbookname=Demo Dictionary\nwordcount=2\nsametypesequence=h\n";
    fs::write(pack_dir.join("demo.ifo"), ifo).unwrap();

    let apple = b"<b>Apple</b><br>fruit\0";
    let pear = b"Pear<BR>tree fruit\0";
    let mut dictionary = Vec::new();
    dictionary.extend_from_slice(apple);
    dictionary.extend_from_slice(pear);
    fs::write(pack_dir.join("demo.dict"), &dictionary).unwrap();

    let mut index = Vec::new();
    push_index_entry(&mut index, "apple", 0, apple.len() as u32);
    push_index_entry(&mut index, "pear", apple.len() as u32, pear.len() as u32);
    fs::write(pack_dir.join("demo.idx"), index).unwrap();

    let packs = discover(&root);
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].name, "Demo Dictionary");
    let entries = load_index(&packs[0]).unwrap();
    assert_eq!(entries.len(), 2);

    let found = lookup(&packs[0], &entries, "Apple");
    assert_eq!(found.definitions.len(), 1);
    assert_eq!(found.definitions[0].0, "apple");
    assert_eq!(found.definitions[0].1, "Apple\nfruit");

    let suggestions = lookup(&packs[0], &entries, "pe");
    assert!(suggestions.definitions.is_empty());
    assert_eq!(suggestions.suggestions, vec!["pear".to_string()]);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn parses_plain_and_typed_definitions() {
    assert_eq!(parse_definition(b"plain text\0", "m"), "plain text");
    assert_eq!(
        parse_definition(b"mplain text\0h<b>bold</b>\0", ""),
        "plain text\nbold"
    );
    assert_eq!(parse_definition(b"first\0second\0", "mm"), "first\nsecond");
}
