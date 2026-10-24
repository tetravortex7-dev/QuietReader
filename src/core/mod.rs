pub mod bookmarks_export;
pub mod library;
pub mod library_export;
pub mod license;

use crate::document::Document;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn normalize_search_text(text: &str) -> String {
    text.to_lowercase().replace('ё', "е")
}

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
    ToggleSearch,
    ToggleTTS,
    ToggleToc,
    ToggleSettings,
    ToggleReadingRuler,
    ToggleBookmark,
    ToggleAmbient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BookmarkColor {
    #[default]
    Yellow,
    Green,
    Blue,
    Red,
}

impl BookmarkColor {
    pub const ALL: [BookmarkColor; 4] = [
        BookmarkColor::Yellow,
        BookmarkColor::Green,
        BookmarkColor::Blue,
        BookmarkColor::Red,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BookmarkKind {
    #[default]
    Bookmark,
    Quote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Bookmark {
    pub id: String,
    pub book_path: PathBuf,
    pub chapter: usize,
    pub scroll_y: f32,
    pub label: String,
    pub text: String,
    pub note: String,
    pub color: BookmarkColor,
    pub kind: BookmarkKind,
    pub timestamp: u64,
    pub progress_percent: f32,
    pub page_number: Option<usize>,
}

impl Default for Bookmark {
    fn default() -> Self {
        Self {
            id: String::new(),
            book_path: PathBuf::new(),
            chapter: 0,
            scroll_y: 0.0,
            label: String::new(),
            text: String::new(),
            note: String::new(),
            color: BookmarkColor::Yellow,
            kind: BookmarkKind::Bookmark,
            timestamp: 0,
            progress_percent: 0.0,
            page_number: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub chapter_index: usize,
    pub paragraph_index: Option<usize>,
    pub page_index: Option<usize>,
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

    pub fn remove_bookmark_by_id(&mut self, id: &str) {
        self.bookmarks.retain(|b| b.id != id);
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

    pub fn make_safe_snippet(text: &str, byte_pos: usize, match_len: usize) -> String {
        let safe_pos = text.floor_char_boundary(byte_pos.min(text.len()));
        let start = text.floor_char_boundary(safe_pos.saturating_sub(30));
        let end = text.ceil_char_boundary((safe_pos + match_len + 30).min(text.len()));
        let slice = text[start..end].trim().replace('\n', " ");
        let mut snippet = String::new();
        if start > 0 {
            snippet.push_str("...");
        }
        snippet.push_str(&slice);
        if end < text.len() {
            snippet.push_str("...");
        }
        snippet
    }

    pub fn search_in_document(doc: &dyn Document, query: &str) -> Vec<SearchHit> {
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        Self::search_in_document_with_cancel(doc, query, &cancelled)
    }

    pub fn search_in_document_with_cancel(
        doc: &dyn Document,
        query: &str,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Vec<SearchHit> {
        use std::sync::atomic::Ordering;
        let q = normalize_search_text(query.trim());
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for (ch_idx, chapter) in doc.chapters().iter().enumerate() {
            if cancelled.load(Ordering::Relaxed) {
                return Vec::new();
            }
            let clean_title = crate::document::strip_formatting_tags(&chapter.title);
            let lower_title = normalize_search_text(&clean_title);
            if lower_title.contains(&q) {
                hits.push(SearchHit {
                    chapter_index: ch_idx,
                    paragraph_index: None,
                    page_index: None,
                    snippet: clean_title.clone(),
                });
                if hits.len() >= 200 {
                    return hits;
                }
            }

            for (p_idx, para) in chapter.paragraphs.iter().enumerate() {
                if cancelled.load(Ordering::Relaxed) {
                    return Vec::new();
                }
                let clean = crate::document::strip_formatting_tags(para);
                let lower_para = normalize_search_text(&clean);
                let mut search_from = 0;
                while let Some(found_idx) = lower_para[search_from..].find(&q) {
                    let abs_pos = search_from + found_idx;
                    let snippet = Self::make_safe_snippet(&clean, abs_pos, q.len());
                    hits.push(SearchHit {
                        chapter_index: ch_idx,
                        paragraph_index: Some(p_idx),
                        page_index: None,
                        snippet,
                    });
                    if hits.len() >= 200 {
                        return hits;
                    }
                    search_from = abs_pos + q.len().max(1);
                    if search_from >= lower_para.len() {
                        break;
                    }
                }
            }
        }
        hits
    }

    pub fn search_page_text_in_document(
        doc: &dyn Document,
        query: &str,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Vec<SearchHit> {
        use std::sync::atomic::Ordering;
        let Some(pages) = doc.page_content() else {
            return Vec::new();
        };
        let q = normalize_search_text(query.trim());
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits = Vec::new();
        for page_idx in 0..pages.page_count() {
            if cancelled.load(Ordering::Relaxed) {
                return Vec::new();
            }
            let Some(text) = pages.extract_page_text(page_idx) else {
                continue;
            };
            let lower_text = normalize_search_text(&text);
            let mut search_from = 0;
            while let Some(found_idx) = lower_text[search_from..].find(&q) {
                let abs_pos = search_from + found_idx;
                let snippet = Self::make_safe_snippet(&text, abs_pos, q.len());
                hits.push(SearchHit {
                    chapter_index: 0,
                    paragraph_index: None,
                    page_index: Some(page_idx),
                    snippet,
                });
                if hits.len() >= 200 {
                    return hits;
                }
                search_from = abs_pos + q.len().max(1);
                if search_from >= lower_text.len() {
                    break;
                }
            }
        }
        hits
    }

    pub fn estimate_paragraph_scroll_y(
        doc: &dyn Document,
        chapter_idx: usize,
        paragraph_idx: Option<usize>,
        font_size: f32,
    ) -> f32 {
        let Some(p_idx) = paragraph_idx else {
            return 0.0;
        };
        let y = Self::paragraph_start_y(doc, chapter_idx, p_idx, font_size, 1.45, 760.0);
        (y - 60.0).max(0.0)
    }

    pub fn paragraph_anchor_scroll_y(
        doc: &dyn Document,
        chapter_idx: usize,
        paragraph_idx: usize,
        font_size: f32,
        line_height_scale: f32,
        column_width: f32,
    ) -> f32 {
        let y = Self::paragraph_start_y(
            doc,
            chapter_idx,
            paragraph_idx,
            font_size,
            line_height_scale,
            column_width,
        );
        (y - font_size * 0.5).max(0.0)
    }

    pub fn estimate_visible_paragraph(
        doc: &dyn Document,
        chapter_idx: usize,
        scroll_y: f32,
        font_size: f32,
    ) -> usize {
        Self::estimate_visible_paragraph_with_layout(
            doc,
            chapter_idx,
            scroll_y,
            font_size,
            1.45,
            760.0,
        )
    }

    pub fn estimate_visible_paragraph_with_layout(
        doc: &dyn Document,
        chapter_idx: usize,
        scroll_y: f32,
        font_size: f32,
        line_height_scale: f32,
        column_width: f32,
    ) -> usize {
        let Some(chapter) = doc.chapters().get(chapter_idx) else {
            return 0;
        };
        let line_height = font_size * line_height_scale;
        let para_spacing = font_size * 0.4;
        let chars_per_line = (column_width / (font_size * 0.52)).max(10.0);

        let clean_title = crate::document::strip_formatting_tags(&chapter.title);
        let mut y = if clean_title.is_empty() {
            20.0
        } else {
            20.0 + font_size * 1.35 + 24.0
        };

        let target_y = scroll_y + 40.0;
        let mut last_non_empty = 0;

        for (idx, para) in chapter.paragraphs.iter().enumerate() {
            if para.is_empty() {
                y += font_size * 0.7;
            } else {
                let clean = crate::document::strip_formatting_tags(para);
                let char_count = clean.chars().count();
                let num_lines = (char_count as f32 / chars_per_line).ceil().max(1.0);
                let h = num_lines * line_height + para_spacing;
                if y + h >= target_y {
                    return idx;
                }
                last_non_empty = idx;
                y += h;
            }
        }

        last_non_empty
    }

    fn paragraph_start_y(
        doc: &dyn Document,
        chapter_idx: usize,
        paragraph_idx: usize,
        font_size: f32,
        line_height_scale: f32,
        column_width: f32,
    ) -> f32 {
        let Some(chapter) = doc.chapters().get(chapter_idx) else {
            return 0.0;
        };
        let line_height = font_size * line_height_scale;
        let para_spacing = font_size * 0.4;
        let chars_per_line = (column_width / (font_size * 0.52)).max(10.0);

        let clean_title = crate::document::strip_formatting_tags(&chapter.title);
        let mut y = if clean_title.is_empty() {
            20.0
        } else {
            20.0 + font_size * 1.35 + 24.0
        };

        for para in chapter.paragraphs.iter().take(paragraph_idx) {
            if para.is_empty() {
                y += font_size * 0.7;
            } else {
                let clean = crate::document::strip_formatting_tags(para);
                let char_count = clean.chars().count();
                let num_lines = (char_count as f32 / chars_per_line).ceil().max(1.0);
                y += num_lines * line_height + para_spacing;
            }
        }
        y
    }

    pub fn detect_document_language(doc: &dyn Document) -> &'static str {
        if let Some(tc) = doc.text_content() {
            Self::detect_chapters_language(tc.chapters())
        } else {
            "ru"
        }
    }

    pub fn detect_chapters_language(chapters: &[crate::document::BookChapter]) -> &'static str {
        let mut sample = String::new();
        for ch in chapters {
            for p in &ch.paragraphs {
                let clean = crate::document::strip_formatting_tags(p);
                let trimmed = clean.trim();
                if !trimmed.is_empty() {
                    sample.push_str(trimmed);
                    sample.push(' ');
                    if sample.len() >= 3000 {
                        break;
                    }
                }
            }
            if sample.len() >= 3000 {
                break;
            }
        }
        Self::detect_language_from_text(&sample)
    }

    pub fn detect_language_from_text(text: &str) -> &'static str {
        let mut cyrillic = 0usize;
        let mut latin = 0usize;
        let mut ukrainian_specific = 0usize;
        let mut arabic = 0usize;
        let mut cjk = 0usize;
        let mut kana = 0usize;

        for c in text.chars() {
            match c {
                '\u{0400}'..='\u{04FF}' => {
                    cyrillic += 1;
                    if matches!(c, 'і' | 'І' | 'ї' | 'Ї' | 'є' | 'Є' | 'ґ' | 'Ґ') {
                        ukrainian_specific += 1;
                    }
                }
                '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' | '\u{08A0}'..='\u{08FF}' => {
                    arabic += 1;
                }
                '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => {
                    kana += 1;
                    cjk += 1;
                }
                '\u{4E00}'..='\u{9FFF}' => {
                    cjk += 1;
                }
                'a'..='z' | 'A'..='Z' => {
                    latin += 1;
                }
                _ => {}
            }
        }

        if arabic > 5 && arabic > cyrillic && arabic > latin && arabic > cjk {
            return "ar";
        }
        if cjk > 5 && cjk > cyrillic && cjk > latin && cjk > arabic {
            if kana > 0 {
                return "ja";
            }
            return "zh";
        }

        if cyrillic > latin {
            if ukrainian_specific >= 2 { "uk" } else { "ru" }
        } else if latin > 0 {
            let lower = text.to_lowercase();
            let mut de_score = 0usize;
            let mut fr_score = 0usize;
            let mut es_score = 0usize;
            let mut it_score = 0usize;
            let mut pt_score = 0usize;
            let mut tr_score = 0usize;
            let mut id_score = 0usize;
            let mut en_score = 0usize;

            for word in lower.split(|c: char| !c.is_alphabetic()) {
                if word.is_empty() {
                    continue;
                }
                match word {
                    "the" | "and" | "that" | "with" | "for" | "this" | "have" | "from" | "you" => {
                        en_score += 2
                    }
                    "der" | "die" | "das" | "und" | "nicht" | "ist" | "ein" | "eine" | "mit" => {
                        de_score += 2
                    }
                    "les" | "des" | "pour" | "dans" | "une" | "est" | "pas" | "sur" | "avec" => {
                        fr_score += 2
                    }
                    "para" | "como" | "los" | "las" | "una" | "por" | "con" | "del" => {
                        es_score += 2
                    }
                    "della" | "dello" | "degli" | "delle" | "sono" | "anche" | "questo"
                    | "questa" | "dopo" => it_score += 2,
                    "não" | "mais" | "mas" | "foi" | "ao" | "dos" | "pelo" | "pela" | "muito" => {
                        pt_score += 2
                    }
                    "bir" | "bu" | "ile" | "için" | "olan" | "olarak" | "daha" | "kadar"
                    | "sonra" => tr_score += 2,
                    "yang" | "dan" | "di" | "dari" | "ini" | "itu" | "dengan" | "untuk"
                    | "pada" | "adalah" => id_score += 2,
                    _ => {}
                }
            }

            for c in lower.chars() {
                match c {
                    'ä' | 'ö' | 'ü' | 'ß' => de_score += 2,
                    'é' | 'è' | 'ê' | 'ë' | 'ç' | 'œ' => fr_score += 2,
                    'ñ' | 'á' | 'í' | 'ó' | 'ú' => es_score += 1,
                    'ã' | 'õ' => pt_score += 2,
                    'ğ' | 'ı' | 'ş' => tr_score += 2,
                    _ => {}
                }
            }

            let max_score = en_score
                .max(de_score)
                .max(fr_score)
                .max(es_score)
                .max(it_score)
                .max(pt_score)
                .max(tr_score)
                .max(id_score);

            if max_score > 0 {
                if de_score == max_score {
                    "de"
                } else if fr_score == max_score {
                    "fr"
                } else if es_score == max_score {
                    "es"
                } else if it_score == max_score {
                    "it"
                } else if pt_score == max_score {
                    "pt"
                } else if tr_score == max_score {
                    "tr"
                } else if id_score == max_score {
                    "id"
                } else {
                    "en"
                }
            } else {
                "en"
            }
        } else {
            "ru"
        }
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
        is_webtoon: bool,
        is_double: bool,
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
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
