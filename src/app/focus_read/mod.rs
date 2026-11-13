mod views;

use super::QuietReaderApp;
use crate::core::{Command, ReaderCore};
use crate::document::{DocumentViewKind, strip_formatting_tags};
use eframe::egui;
use std::path::PathBuf;

#[derive(Default)]
pub struct FocusReadState {
    pub paragraph_chapter: usize,
    pub paragraph_index: usize,
    pub paragraph_initialized: bool,
    pub paragraph_file: Option<PathBuf>,
    pub rsvp: RsvpSession,
}

#[derive(Default)]
pub struct RsvpSession {
    pub active: bool,
    pub playing: bool,
    pub finished: bool,
    pub chapter: usize,
    pub words: Vec<RsvpWord>,
    pub index: usize,
    pub accumulator: f32,
    pub file_path: Option<PathBuf>,
    pub words_read_in_session: usize,
    pub elapsed_reading_secs: f32,
}

impl RsvpSession {
    pub(super) fn current_word(&self) -> Option<&RsvpWord> {
        self.words.get(self.index)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsvpWord {
    pub text: String,
    pub paragraph: usize,
}

pub(crate) fn collect_words(paragraphs: &[String]) -> Vec<RsvpWord> {
    let mut words = Vec::new();
    for (paragraph_index, paragraph) in paragraphs.iter().enumerate() {
        let clean = strip_formatting_tags(paragraph);
        for word in clean.split_whitespace() {
            words.push(RsvpWord {
                text: word.to_string(),
                paragraph: paragraph_index,
            });
        }
    }
    words
}

pub(crate) fn pivot_char_index(word: &str) -> usize {
    let count = word.chars().count();
    if count <= 2 {
        return 0;
    }
    (((count as f32) * 0.35).round() as usize).min(count - 1)
}

pub(crate) fn is_text_paragraph(paragraph: &str) -> bool {
    !paragraph.trim().is_empty()
        && crate::document::image_token_id(paragraph).is_none()
        && !strip_formatting_tags(paragraph).trim().is_empty()
}

pub(crate) fn delay_factor(word: &str) -> f32 {
    let trimmed = word.trim_end_matches(|ch: char| matches!(ch, '»' | '"' | ')' | ']' | '\''));
    if trimmed.ends_with(|ch: char| matches!(ch, '.' | '!' | '?' | '…')) {
        2.0
    } else if trimmed.ends_with(|ch: char| matches!(ch, ',' | ';' | ':' | '—')) {
        1.35
    } else {
        1.0
    }
}

pub(crate) fn step_paragraph(paragraphs: &[String], from: usize, forward: bool) -> Option<usize> {
    if forward {
        ((from + 1)..paragraphs.len()).find(|&index| is_text_paragraph(&paragraphs[index]))
    } else {
        (0..from)
            .rev()
            .find(|&index| is_text_paragraph(&paragraphs[index]))
    }
}

pub(crate) fn nearest_text_paragraph(
    paragraphs: &[String],
    start: usize,
    forward: bool,
) -> Option<usize> {
    if paragraphs.is_empty() {
        return None;
    }
    if forward {
        (start.min(paragraphs.len() - 1)..paragraphs.len())
            .find(|&index| is_text_paragraph(&paragraphs[index]))
    } else {
        (0..=start.min(paragraphs.len() - 1))
            .rev()
            .find(|&index| is_text_paragraph(&paragraphs[index]))
    }
}

pub(crate) fn text_paragraph_position(paragraphs: &[String], index: usize) -> (usize, usize) {
    let mut position = 0;
    let mut total = 0;
    for (current, paragraph) in paragraphs.iter().enumerate() {
        if is_text_paragraph(paragraph) {
            total += 1;
            if current == index {
                position = total;
            }
        }
    }
    (position, total)
}

impl QuietReaderApp {
    pub(crate) fn toggle_paragraph_mode(&mut self) {
        if self.document.is_none() || self.view_kind() != DocumentViewKind::Text {
            return;
        }
        let enable = !self.settings.text_paragraph_mode;
        self.settings.text_paragraph_mode = enable;
        if enable {
            self.close_speed_reading();
            self.autoscroll.stop();
            self.tts.stop();
            self.focus_read.paragraph_initialized = false;
        } else {
            self.pending_scroll_y = Some(self.current_scroll_y);
        }
        self.mark_settings_dirty();
    }

    pub(crate) fn start_speed_reading(&mut self) {
        if self.document.is_none() || self.view_kind() != DocumentViewKind::Text {
            return;
        }
        let start_paragraph =
            if self.settings.text_paragraph_mode && self.focus_read.paragraph_initialized {
                self.focus_read.paragraph_index
            } else {
                self.focus_paragraph_from_scroll()
            };
        self.autoscroll.stop();
        self.tts.stop();
        self.focus_read.rsvp = RsvpSession {
            active: true,
            playing: true,
            chapter: self.current_chapter,
            file_path: self.current_file_path.clone(),
            ..Default::default()
        };
        if !self.open_rsvp_from_chapter(self.current_chapter, start_paragraph) {
            self.focus_read.rsvp.active = false;
        }
    }

    pub(crate) fn close_speed_reading(&mut self) {
        self.focus_read.rsvp.active = false;
        self.focus_read.rsvp.playing = false;
        self.pending_scroll_y = Some(self.current_scroll_y);
    }

    pub(crate) fn close_focus_reading(&mut self) {
        if self.focus_read.rsvp.active {
            self.close_speed_reading();
        }
        if self.settings.text_paragraph_mode {
            self.settings.text_paragraph_mode = false;
            self.pending_scroll_y = Some(self.current_scroll_y);
            self.mark_settings_dirty();
        }
    }

    pub(crate) fn focus_nav_command(&mut self, cmd: &Command) -> bool {
        let rsvp_active = self.focus_read.rsvp.active;
        let paragraph_active = self.settings.text_paragraph_mode;
        if !rsvp_active && !paragraph_active {
            return false;
        }
        match cmd {
            Command::NextChapter => {
                if rsvp_active {
                    self.jump_rsvp_chapter(true);
                } else {
                    self.jump_paragraph_chapter(true);
                }
                true
            }
            Command::PrevChapter => {
                if rsvp_active {
                    self.jump_rsvp_chapter(false);
                } else {
                    self.jump_paragraph_chapter(false);
                }
                true
            }
            Command::PageDown
            | Command::PageUp
            | Command::LineDown
            | Command::LineUp
            | Command::NextPage
            | Command::PrevPage
            | Command::Home
            | Command::End => true,
            _ => false,
        }
    }

    pub(crate) fn render_focus_reader(&mut self, ui: &mut egui::Ui) -> bool {
        if self.document.is_none() || self.view_kind() != DocumentViewKind::Text {
            return false;
        }
        if self.focus_read.rsvp.active {
            if self.focus_read.rsvp.file_path != self.current_file_path {
                self.close_speed_reading();
                return false;
            }
            if self.focus_read.rsvp.chapter != self.current_chapter
                && !self.open_rsvp_from_chapter(self.current_chapter, 0)
            {
                self.focus_read.rsvp.active = false;
                self.focus_read.rsvp.playing = false;
                return false;
            }
            self.render_rsvp(ui);
            return true;
        }
        if self.settings.text_paragraph_mode {
            self.render_paragraph(ui);
            return true;
        }
        false
    }

    pub(super) fn focus_paragraph_from_scroll(&self) -> usize {
        let Some(doc) = &self.document else {
            return 0;
        };
        ReaderCore::estimate_visible_paragraph_with_layout(
            doc.as_ref(),
            self.current_chapter,
            self.current_scroll_y,
            self.font_size,
            self.settings.effective_line_height_scale(),
            self.settings.effective_max_column_width(),
        )
    }

    pub(super) fn apply_paragraph_progress(&mut self) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let scroll_y = ReaderCore::paragraph_anchor_scroll_y(
            doc.as_ref(),
            self.current_chapter,
            self.focus_read.paragraph_index,
            self.font_size,
            self.settings.effective_line_height_scale(),
            self.settings.effective_max_column_width(),
        );
        self.current_scroll_y = scroll_y;
        self.pending_scroll_y = Some(scroll_y);
        self.focus_read.paragraph_chapter = self.current_chapter;
        self.focus_read.paragraph_initialized = true;
        self.mark_progress_dirty();
    }

    pub(super) fn move_paragraph(&mut self, forward: bool) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let chapter_index = self.current_chapter;
        let Some(chapter) = doc.chapters().get(chapter_index) else {
            return;
        };
        if let Some(target) = step_paragraph(
            &chapter.paragraphs,
            self.focus_read.paragraph_index,
            forward,
        ) {
            self.focus_read.paragraph_index = target;
            self.apply_paragraph_progress();
            return;
        }
        let chapter_count = doc.chapters().len();
        let indices: Vec<usize> = if forward {
            ((chapter_index + 1)..chapter_count).collect()
        } else {
            (0..chapter_index).rev().collect()
        };
        for index in indices {
            let paragraphs = &doc.chapters()[index].paragraphs;
            let start = if forward {
                0
            } else {
                paragraphs.len().saturating_sub(1)
            };
            if let Some(target) = nearest_text_paragraph(paragraphs, start, forward) {
                self.current_chapter = index;
                self.last_rendered_chapter = index;
                self.focus_read.paragraph_chapter = index;
                self.focus_read.paragraph_index = target;
                self.apply_paragraph_progress();
                return;
            }
        }
    }

    fn jump_paragraph_chapter(&mut self, forward: bool) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let chapter_count = doc.chapters().len();
        let indices: Vec<usize> = if forward {
            ((self.current_chapter + 1)..chapter_count).collect()
        } else {
            (0..self.current_chapter).rev().collect()
        };
        for index in indices {
            let paragraphs = &doc.chapters()[index].paragraphs;
            let start = if forward {
                0
            } else {
                paragraphs.len().saturating_sub(1)
            };
            if let Some(target) = nearest_text_paragraph(paragraphs, start, forward) {
                self.current_chapter = index;
                self.last_rendered_chapter = index;
                self.focus_read.paragraph_chapter = index;
                self.focus_read.paragraph_index = target;
                self.apply_paragraph_progress();
                return;
            }
        }
    }

    fn open_rsvp_chapter(&mut self, chapter_index: usize, paragraph_hint: usize) -> bool {
        let Some(doc) = self.document.clone() else {
            return false;
        };
        let Some(chapter) = doc.chapters().get(chapter_index) else {
            return false;
        };
        let words = collect_words(&chapter.paragraphs);
        if words.is_empty() {
            return false;
        }
        let start = words
            .iter()
            .position(|word| word.paragraph >= paragraph_hint)
            .unwrap_or(0);
        self.focus_read.rsvp.chapter = chapter_index;
        self.focus_read.rsvp.words = words;
        self.focus_read.rsvp.index = start;
        self.focus_read.rsvp.accumulator = 0.0;
        self.focus_read.rsvp.finished = false;
        self.current_chapter = chapter_index;
        self.last_rendered_chapter = chapter_index;
        self.pending_scroll_y = Some(0.0);
        self.current_scroll_y = 0.0;
        self.apply_rsvp_progress();
        true
    }

    pub(crate) fn open_rsvp_from_chapter(
        &mut self,
        start_chapter: usize,
        paragraph_hint: usize,
    ) -> bool {
        if self.open_rsvp_chapter(start_chapter, paragraph_hint) {
            return true;
        }
        let total = self
            .document
            .as_ref()
            .map(|doc| doc.chapters().len())
            .unwrap_or(0);
        ((start_chapter + 1)..total).any(|next| self.open_rsvp_chapter(next, 0))
    }

    fn apply_rsvp_progress(&mut self) {
        let (chapter, paragraph) = {
            let rsvp = &self.focus_read.rsvp;
            (
                rsvp.chapter,
                rsvp.current_word().map(|word| word.paragraph).unwrap_or(0),
            )
        };
        self.focus_read.paragraph_chapter = chapter;
        self.focus_read.paragraph_index = paragraph;
        self.focus_read.paragraph_initialized = true;
        self.apply_paragraph_progress();
    }

    pub(crate) fn advance_rsvp_word(&mut self) {
        if self.focus_read.rsvp.playing {
            self.focus_read.rsvp.words_read_in_session += 1;
        }
        if self.focus_read.rsvp.index + 1 < self.focus_read.rsvp.words.len() {
            self.focus_read.rsvp.index += 1;
            self.apply_rsvp_progress();
            return;
        }
        if self.open_rsvp_from_chapter(self.focus_read.rsvp.chapter + 1, 0) {
            return;
        }
        self.focus_read.rsvp.playing = false;
        self.focus_read.rsvp.finished = true;
    }

    pub(crate) fn step_rsvp_word(&mut self, forward: bool) {
        if forward {
            self.advance_rsvp_word();
            return;
        }
        if self.focus_read.rsvp.index > 0 {
            self.focus_read.rsvp.index -= 1;
            self.apply_rsvp_progress();
            return;
        }
        self.jump_rsvp_chapter(false);
    }

    pub(super) fn jump_rsvp_paragraph(&mut self, forward: bool) {
        let current_paragraph = self
            .focus_read
            .rsvp
            .current_word()
            .map(|word| word.paragraph)
            .unwrap_or(0);
        let target = if forward {
            self.focus_read
                .rsvp
                .words
                .iter()
                .position(|word| word.paragraph > current_paragraph)
        } else {
            self.focus_read
                .rsvp
                .words
                .iter()
                .rposition(|word| word.paragraph < current_paragraph)
        };
        if let Some(index) = target {
            self.focus_read.rsvp.index = index;
            self.apply_rsvp_progress();
            return;
        }
        if forward {
            self.advance_rsvp_word();
        } else {
            self.step_rsvp_word(false);
        }
    }

    fn jump_rsvp_chapter(&mut self, forward: bool) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let total = doc.chapters().len();
        let chapter = self.focus_read.rsvp.chapter;
        if forward {
            for next in (chapter + 1)..total {
                if self.open_rsvp_chapter(next, 0) {
                    return;
                }
            }
        } else {
            for previous in (0..chapter).rev() {
                if self.open_rsvp_chapter(previous, 0) {
                    let last = self.focus_read.rsvp.words.len().saturating_sub(1);
                    self.focus_read.rsvp.index = last;
                    self.apply_rsvp_progress();
                    return;
                }
            }
        }
    }

    pub(super) fn restart_rsvp(&mut self) {
        let chapter = self.focus_read.rsvp.chapter;
        self.focus_read.rsvp.playing = false;
        self.focus_read.rsvp.words_read_in_session = 0;
        self.focus_read.rsvp.elapsed_reading_secs = 0.0;
        self.open_rsvp_chapter(chapter, 0);
    }

    pub(super) fn cycle_rsvp_speed(&mut self, faster: bool) {
        let step = 25;
        let current = self.settings.rsvp_wpm.clamp(100, 1000);
        self.settings.rsvp_wpm = if faster {
            (current + step).min(1000)
        } else {
            current.saturating_sub(step).max(100)
        };
        self.mark_settings_dirty();
    }
}
