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
