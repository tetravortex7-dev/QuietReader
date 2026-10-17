use crate::document::Document;
use crate::settings::{AppSettings, ComicSettings};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    NextPage,
    PrevPage,
    PageDown,
    PageUp,
    LineDown,
    LineUp,
    NextChapter,
    PrevChapter,
    Home,
    End,
    ScrollBy(f32),
    GoToChapter(usize),
    GoToPage(usize),
    ToggleFullscreen,
    ExitFullscreen,
    ToggleAutoscroll,
    CopyChapterText,
    ZoomIn,
    ZoomOut,
    ResetZoom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bookmark {
    pub book_path: PathBuf,
    pub chapter: usize,
    pub scroll_y: f32,
    pub label: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub chapter_index: usize,
    pub paragraph_index: Option<usize>,
    pub snippet: String,
}

#[derive(Default)]
pub struct ReaderCore {
    pub bookmarks: Vec<Bookmark>,
}

impl ReaderCore {
    pub fn add_bookmark(&mut self, bookmark: Bookmark) {
        if !self.bookmarks.iter().any(|b| {
            b.book_path == bookmark.book_path
                && b.chapter == bookmark.chapter
                && (b.scroll_y - bookmark.scroll_y).abs() < 1.0
        }) {
            self.bookmarks.push(bookmark);
        }
    }

    pub fn remove_bookmark(&mut self, book_path: &Path, chapter: usize) {
        self.bookmarks
            .retain(|b| !(b.book_path == book_path && b.chapter == chapter));
    }

    pub fn bookmarks_for_path<'a>(
        &'a self,
        book_path: &'a Path,
    ) -> impl Iterator<Item = &'a Bookmark> {
        self.bookmarks
            .iter()
            .filter(move |b| b.book_path == book_path)
    }

    pub fn clear_bookmarks_for_path(&mut self, book_path: &Path) {
        self.bookmarks.retain(|b| b.book_path != book_path);
    }

    pub fn search_in_document(doc: &dyn Document, query: &str) -> Vec<SearchHit> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for (ch_idx, chapter) in doc.chapters().iter().enumerate() {
            for (p_idx, para) in chapter.paragraphs.iter().enumerate() {
                let clean = crate::document::strip_formatting_tags(para);
                if let Some(pos) = clean.to_lowercase().find(&q) {
                    let start = pos.saturating_sub(30);
                    let end = (pos + q.len() + 30).min(clean.len());
                    let snippet = clean[start..end].trim().to_string();
                    hits.push(SearchHit {
                        chapter_index: ch_idx,
                        paragraph_index: Some(p_idx),
                        snippet,
                    });
                    if hits.len() >= 100 {
                        return hits;
                    }
                }
            }
        }
        hits
    }

    pub fn find_next_non_empty_chapter(
        doc: &dyn Document,
        current_chapter: usize,
    ) -> Option<usize> {
        let total = doc.chapters().len();
        ((current_chapter + 1)..total).find(|&idx| {
            doc.chapters()
                .get(idx)
                .map(|c| !c.is_empty())
                .unwrap_or(false)
        })
    }

    pub fn find_prev_non_empty_chapter(
        doc: &dyn Document,
        current_chapter: usize,
    ) -> Option<usize> {
        (0..current_chapter).rev().find(|&idx| {
            doc.chapters()
                .get(idx)
                .map(|c| !c.is_empty())
                .unwrap_or(false)
        })
    }

    pub fn extract_chapter_text(doc: &dyn Document, current_chapter: usize) -> Option<String> {
        if let Some(page_content) = doc.page_content()
            && let Some(text) = page_content.extract_page_text(current_chapter)
        {
            return Some(text);
        }
        let ch = doc.chapters().get(current_chapter)?;
        let mut s = String::new();
        if !ch.title.is_empty() {
            s.push_str(&crate::document::strip_formatting_tags(&ch.title));
            s.push_str("\n\n");
        }
        for (i, p) in ch.paragraphs.iter().enumerate() {
            if i > 0 {
                s.push_str("\n\n");
            }
            s.push_str(&crate::document::strip_formatting_tags(p));
        }
        Some(s)
    }

    pub fn apply_text_command(
        cmd: &Command,
        doc: &dyn Document,
        current_chapter: &mut usize,
        last_rendered_chapter: &mut usize,
        current_scroll_y: f32,
        max_scroll_y: f32,
        viewport_height: f32,
        pending_scroll_y: &mut Option<f32>,
    ) -> bool {
        let page_step = (viewport_height * 0.85).max(200.0);
        let line_step = 60.0;
        let next_non_empty = Self::find_next_non_empty_chapter(doc, *current_chapter);
        let prev_non_empty = Self::find_prev_non_empty_chapter(doc, *current_chapter);

        match cmd {
            Command::NextChapter => {
                if let Some(next_idx) = next_non_empty {
                    *current_chapter = next_idx;
                    *pending_scroll_y = Some(0.0);
                    *last_rendered_chapter = *current_chapter;
                    return true;
                }
            }
            Command::PrevChapter => {
                if let Some(prev_idx) = prev_non_empty {
                    *current_chapter = prev_idx;
                    *pending_scroll_y = Some(0.0);
                    *last_rendered_chapter = *current_chapter;
                    return true;
                }
            }
            Command::PageDown => {
                if current_scroll_y + 15.0 >= max_scroll_y {
                    if let Some(next_idx) = next_non_empty {
                        *current_chapter = next_idx;
                        *pending_scroll_y = Some(0.0);
                        *last_rendered_chapter = *current_chapter;
                        return true;
                    }
                } else {
                    *pending_scroll_y = Some((current_scroll_y + page_step).min(max_scroll_y));
                    return true;
                }
            }
            Command::PageUp => {
                if current_scroll_y <= 5.0 {
                    if let Some(prev_idx) = prev_non_empty {
                        *current_chapter = prev_idx;
                        *pending_scroll_y = Some(999_999.0);
                        *last_rendered_chapter = *current_chapter;
                        return true;
                    }
                } else {
                    *pending_scroll_y = Some((current_scroll_y - page_step).max(0.0));
                    return true;
                }
            }
            Command::LineDown => {
                *pending_scroll_y = Some((current_scroll_y + line_step).min(max_scroll_y));
                return true;
            }
            Command::LineUp => {
                *pending_scroll_y = Some((current_scroll_y - line_step).max(0.0));
                return true;
            }
            Command::Home => {
                *pending_scroll_y = Some(0.0);
                return true;
            }
            Command::End => {
                *pending_scroll_y = Some(max_scroll_y);
                return true;
            }
            Command::ScrollBy(delta) => {
                *pending_scroll_y = Some((current_scroll_y + delta).clamp(0.0, max_scroll_y));
                return true;
            }
            Command::GoToChapter(ch_idx) if *ch_idx < doc.chapters().len() => {
                *current_chapter = *ch_idx;
                *pending_scroll_y = Some(0.0);
                *last_rendered_chapter = *current_chapter;
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn apply_paged_command(
        cmd: &Command,
        doc: &Option<Arc<dyn Document>>,
        comic_settings: &ComicSettings,
        current_page: &mut usize,
        current_scroll_y: f32,
        max_scroll_y: f32,
        viewport_height: f32,
        pending_scroll_y: &mut Option<f32>,
    ) -> bool {
        let page_count = doc
            .as_ref()
            .and_then(|d| d.page_content())
            .map(|c| c.page_count())
            .unwrap_or(0);
        if page_count == 0 {
            return false;
        }

        let is_webtoon = comic_settings.reading_mode == "Webtoon";
        let page_step = (viewport_height * 0.85).max(200.0);
        let line_step = 60.0;

        if is_webtoon {
            match cmd {
                Command::PageDown => {
                    *pending_scroll_y = Some((current_scroll_y + page_step).min(max_scroll_y));
                    true
                }
                Command::PageUp => {
                    *pending_scroll_y = Some((current_scroll_y - page_step).max(0.0));
                    true
                }
                Command::LineDown => {
                    *pending_scroll_y = Some((current_scroll_y + line_step).min(max_scroll_y));
                    true
                }
                Command::LineUp => {
                    *pending_scroll_y = Some((current_scroll_y - line_step).max(0.0));
                    true
                }
                Command::Home => {
                    *pending_scroll_y = Some(0.0);
                    true
                }
                Command::End => {
                    *pending_scroll_y = Some(max_scroll_y);
                    true
                }
                _ => false,
            }
        } else {
            let is_double = comic_settings.page_layout == "Double";
            let step = if is_double { 2 } else { 1 };
            match cmd {
                Command::NextPage | Command::PageDown => {
                    *current_page = (*current_page + step).min(page_count.saturating_sub(1));
                    true
                }
                Command::PrevPage | Command::PageUp => {
                    *current_page = current_page.saturating_sub(step);
                    true
                }
                Command::Home => {
                    *current_page = 0;
                    true
                }
                Command::End => {
                    *current_page = page_count.saturating_sub(1);
                    true
                }
                Command::GoToPage(p) => {
                    *current_page = (*p).min(page_count.saturating_sub(1));
                    true
                }
                _ => false,
            }
        }
    }

    pub fn record_progress(
        settings: &mut AppSettings,
        document: &Option<Arc<dyn Document>>,
        current_file_path: &Option<PathBuf>,
        current_chapter: usize,
        current_page: usize,
        current_scroll_y: f32,
    ) {
        if let (Some(doc), Some(path)) = (document, current_file_path) {
            let meta = doc.metadata();
            let saved_ch = if doc.page_content().is_some() {
                current_page
            } else {
                current_chapter
            };
            settings.record_book_progress(
                path,
                &meta.title,
                &meta.author,
                saved_ch,
                current_scroll_y,
            );
        }
    }
}
