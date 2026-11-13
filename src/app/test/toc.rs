use crate::app::toc_view::toc_matches;
use crate::document::TocItem;

fn item(title: &str, children: Vec<TocItem>) -> TocItem {
    TocItem {
        id: None,
        title: title.to_string(),
        chapter_index: 0,
        children,
    }
}

#[test]
fn matches_parent_title_and_keeps_its_subtree_visible() {
    let tree = item("Часть первая", vec![item("Глава первая", Vec::new())]);
    assert!(toc_matches(&tree, "часть"));
    assert!(!toc_matches(&tree, "эпилог"));
}

#[test]
fn matches_nested_child_and_normalizes_yo() {
    let tree = item("Основное", vec![item("Ещё одна глава", Vec::new())]);
    assert!(toc_matches(&tree, "еще"));
    assert!(toc_matches(&tree, "одна"));
}

#[test]
fn empty_filter_matches_everything() {
    let tree = item("Любой заголовок", Vec::new());
    assert!(toc_matches(&tree, ""));
}

#[test]
fn toc_node_ids_distinguish_duplicates_and_documents() {
    use crate::app::toc_view::toc_node_id;
    use eframe::egui::Id;

    let root_a = Id::unique(("toc_tree", Some("book_a.epub")));
    let root_b = Id::unique(("toc_tree", Some("book_b.epub")));

    let sibling_first = toc_node_id(root_a, 0, 1, "Chapter 1");
    let sibling_second = toc_node_id(root_a, 1, 1, "Chapter 1");
    assert_ne!(sibling_first, sibling_second);

    let child_of_first = toc_node_id(sibling_first, 0, 2, "Section");
    let child_of_second = toc_node_id(sibling_second, 0, 2, "Section");
    assert_ne!(child_of_first, child_of_second);

    let other_book = toc_node_id(root_b, 0, 1, "Chapter 1");
    assert_ne!(sibling_first, other_book);
}
