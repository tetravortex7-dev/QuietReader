use crate::core::library::{
    LibraryStore, MAX_LIBRARY_SNAPSHOTS, latest_snapshot, restore_latest_snapshot, snapshot_dir,
};
use crate::test::util::unique_test_dir;

fn store_with_title(title: &str) -> LibraryStore {
    let mut store = LibraryStore::default();
    store.entries.clear();
    let mut entry = crate::core::library::LibraryEntry::default();
    entry.path = std::path::PathBuf::from("book.epub");
    entry.title = title.to_string();
    store.entries.push(entry);
    store
}

#[test]
fn snapshots_are_written_and_restore_reverts_library() {
    let dir = unique_test_dir("library_snapshots_restore");
    let path = dir.join("library.json");

    store_with_title("First").save_to_path(&path).unwrap();
    assert!(latest_snapshot(&path).is_none());

    store_with_title("Second").save_to_path(&path).unwrap();
    let snapshot = latest_snapshot(&path).expect("snapshot after second save");
    assert!(snapshot.starts_with(snapshot_dir(&path)));
    assert_eq!(
        LibraryStore::load_from_path(&path).entries[0].title,
        "Second"
    );

    store_with_title("Third").save_to_path(&path).unwrap();
    restore_latest_snapshot(&path).unwrap();
    assert_eq!(
        LibraryStore::load_from_path(&path).entries[0].title,
        "Second"
    );
    assert!(path.with_extension("before_restore.json").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snapshots_are_pruned_to_limit() {
    let dir = unique_test_dir("library_snapshots_prune");
    let path = dir.join("library.json");

    store_with_title("Start").save_to_path(&path).unwrap();
    for index in 0..(MAX_LIBRARY_SNAPSHOTS + 5) {
        store_with_title(&format!("Book {index}"))
            .save_to_path(&path)
            .unwrap();
    }

    let count = std::fs::read_dir(snapshot_dir(&path))
        .unwrap()
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("library_") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(count, MAX_LIBRARY_SNAPSHOTS);

    let _ = std::fs::remove_dir_all(&dir);
}
