pub mod fb2;

pub use fb2::Fb2Document;

#[derive(Debug, Clone)]
pub struct BookMetadata {
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone)]
pub struct BookChapter {
    pub id: Option<String>,
    pub title: String,
    pub paragraphs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocItem {
    pub id: Option<String>,
    pub title: String,
    pub chapter_index: usize,
    pub children: Vec<TocItem>,
}

pub trait Document: Send + Sync {
    fn metadata(&self) -> &BookMetadata;
    fn chapters(&self) -> &[BookChapter];
    fn toc(&self) -> &[TocItem] {
        &[]
    }
}
