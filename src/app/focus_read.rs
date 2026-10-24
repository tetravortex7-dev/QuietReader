use super::QuietReaderApp;
use super::icons::{Icon, IconButton};
use super::text_view::create_focus_layout_job;
use crate::core::{Command, ReaderCore};
use crate::document::{DocumentViewKind, strip_formatting_tags};
use eframe::egui::{self, Align, FontId, Layout, RichText, ScrollArea, Sense, TextFormat, Vec2};
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
}

impl RsvpSession {
    fn current_word(&self) -> Option<&RsvpWord> {
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

fn icon_button_width(ui: &egui::Ui, text: &str) -> f32 {
    let padding = ui.spacing().button_padding;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let width = ui
        .painter()
        .layout_no_wrap(text.to_string(), font, ui.visuals().text_color())
        .size()
        .x;
    padding.x * 2.0 + 21.0 + width
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
        if !self.open_rsvp_chapter(self.current_chapter, start_paragraph) {
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
            self.render_rsvp(ui);
            return true;
        }
        if self.settings.text_paragraph_mode {
            self.render_paragraph(ui);
            return true;
        }
        false
    }

    fn focus_paragraph_from_scroll(&self) -> usize {
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

    fn apply_paragraph_progress(&mut self) {
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

    fn move_paragraph(&mut self, forward: bool) {
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

    fn advance_rsvp_word(&mut self) {
        if self.focus_read.rsvp.index + 1 < self.focus_read.rsvp.words.len() {
            self.focus_read.rsvp.index += 1;
            self.apply_rsvp_progress();
            return;
        }
        let Some(doc) = self.document.clone() else {
            return;
        };
        let total = doc.chapters().len();
        for next in (self.focus_read.rsvp.chapter + 1)..total {
            if self.open_rsvp_chapter(next, 0) {
                return;
            }
        }
        self.focus_read.rsvp.playing = false;
        self.focus_read.rsvp.finished = true;
    }

    fn step_rsvp_word(&mut self, forward: bool) {
        if forward {
            self.advance_rsvp_word();
            return;
        }
        if self.focus_read.rsvp.index > 0 {
            self.focus_read.rsvp.index -= 1;
            self.apply_rsvp_progress();
            return;
        }
        if self.focus_read.rsvp.chapter == 0 {
            return;
        }
        let previous = self.focus_read.rsvp.chapter - 1;
        if self.open_rsvp_chapter(previous, 0) {
            let last = self.focus_read.rsvp.words.len().saturating_sub(1);
            self.focus_read.rsvp.index = last;
            self.apply_rsvp_progress();
        }
    }

    fn jump_rsvp_paragraph(&mut self, forward: bool) {
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

    fn restart_rsvp(&mut self) {
        let chapter = self.focus_read.rsvp.chapter;
        self.focus_read.rsvp.playing = false;
        self.open_rsvp_chapter(chapter, 0);
    }

    fn cycle_rsvp_speed(&mut self, faster: bool) {
        let step = 25;
        let current = self.settings.rsvp_wpm.clamp(100, 1000);
        self.settings.rsvp_wpm = if faster {
            (current + step).min(1000)
        } else {
            current.saturating_sub(step).max(100)
        };
        self.mark_settings_dirty();
    }

    fn render_paragraph(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = self.document.clone() else {
            return;
        };
        let chapter_count = doc.chapters().len();
        if chapter_count == 0 {
            return;
        }
        if self.current_chapter >= chapter_count {
            self.current_chapter = chapter_count - 1;
        }
        let Some(chapter) = doc.chapters().get(self.current_chapter) else {
            return;
        };
        if self.focus_read.paragraph_file != self.current_file_path {
            self.focus_read.paragraph_file = self.current_file_path.clone();
            self.focus_read.paragraph_initialized = false;
        }
        if !self.focus_read.paragraph_initialized
            || self.focus_read.paragraph_chapter != self.current_chapter
        {
            let estimated = self.focus_paragraph_from_scroll();
            self.focus_read.paragraph_chapter = self.current_chapter;
            self.focus_read.paragraph_index =
                nearest_text_paragraph(&chapter.paragraphs, estimated, true)
                    .or_else(|| nearest_text_paragraph(&chapter.paragraphs, estimated, false))
                    .unwrap_or(0);
            self.focus_read.paragraph_initialized = true;
            self.apply_paragraph_progress();
        }

        let t = self.settings.ui_language.t();
        let text_color = self.theme.text_color();
        let secondary = self.theme.secondary_text_color();
        let accent = self.theme.accent_color();
        let is_rtl = self.settings.ui_language.is_rtl();

        if !ui.ctx().egui_wants_keyboard_input() {
            let (space, shift, left, right, up, down, page_up, page_down, home, end) =
                ui.input(|input| {
                    (
                        input.key_pressed(egui::Key::Space),
                        input.modifiers.shift,
                        input.key_pressed(egui::Key::ArrowLeft),
                        input.key_pressed(egui::Key::ArrowRight),
                        input.key_pressed(egui::Key::ArrowUp),
                        input.key_pressed(egui::Key::ArrowDown),
                        input.key_pressed(egui::Key::PageUp),
                        input.key_pressed(egui::Key::PageDown),
                        input.key_pressed(egui::Key::Home),
                        input.key_pressed(egui::Key::End),
                    )
                });
            let forward = (space && !shift) || page_down || down || right;
            let backward = (space && shift) || page_up || up || left;
            if forward {
                self.move_paragraph(true);
            } else if backward {
                self.move_paragraph(false);
            } else if home {
                let target = nearest_text_paragraph(&chapter.paragraphs, 0, true);
                if let Some(target) = target {
                    self.focus_read.paragraph_index = target;
                    self.apply_paragraph_progress();
                }
            } else if end {
                let target = nearest_text_paragraph(
                    &chapter.paragraphs,
                    chapter.paragraphs.len().saturating_sub(1),
                    false,
                );
                if let Some(target) = target {
                    self.focus_read.paragraph_index = target;
                    self.apply_paragraph_progress();
                }
            }
        }

        let Some(chapter) = doc.chapters().get(self.current_chapter) else {
            return;
        };
        let paragraph_count = chapter.paragraphs.len();
        let index = self
            .focus_read
            .paragraph_index
            .min(paragraph_count.saturating_sub(1));
        let (position, text_total) = text_paragraph_position(&chapter.paragraphs, index);
        let progress = t
            .paragraph_progress
            .replace("{current}", &position.max(1).to_string())
            .replace("{total}", &text_total.max(1).to_string());

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(strip_formatting_tags(&chapter.title))
                    .size(13.0)
                    .color(secondary),
            );
            ui.with_layout(
                if is_rtl {
                    Layout::right_to_left(Align::Center)
                } else {
                    Layout::left_to_right(Align::Center)
                },
                |ui| {
                    ui.label(RichText::new(progress).size(12.0).color(secondary));
                },
            );
        });
        ui.separator();

        let max_width = self
            .settings
            .effective_max_column_width()
            .min(ui.available_width());
        let paragraph = chapter
            .paragraphs
            .get(index)
            .map(String::as_str)
            .unwrap_or("");
        let line_height = self.font_size * self.settings.effective_line_height_scale();
        let top_space = (ui.available_height() * 0.12).clamp(12.0, 140.0);

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(max_width);
                    ui.add_space(top_space);
                    let job = create_focus_layout_job(
                        paragraph,
                        self.font_size,
                        text_color,
                        accent,
                        line_height,
                        is_rtl,
                        self.settings.effective_bionic_reading(),
                        max_width,
                    );
                    ui.add(egui::Label::new(job).wrap().selectable(true));
                    ui.add_space(20.0);
                });
            });

        ui.add_space(4.0);
        ui.label(
            RichText::new(t.paragraph_mode_desc)
                .size(11.0)
                .color(secondary),
        );
    }

    fn render_rsvp(&mut self, ui: &mut egui::Ui) {
        if !self.focus_read.rsvp.active {
            return;
        }
        let t = self.settings.ui_language.t();
        let text_color = self.theme.text_color();
        let secondary = self.theme.secondary_text_color();
        let accent = self.theme.accent_color();

        if !ui.ctx().egui_wants_keyboard_input() {
            let (space, left, right, up, down, page_up, page_down, home) = ui.input(|input| {
                (
                    input.key_pressed(egui::Key::Space),
                    input.key_pressed(egui::Key::ArrowLeft),
                    input.key_pressed(egui::Key::ArrowRight),
                    input.key_pressed(egui::Key::ArrowUp),
                    input.key_pressed(egui::Key::ArrowDown),
                    input.key_pressed(egui::Key::PageUp),
                    input.key_pressed(egui::Key::PageDown),
                    input.key_pressed(egui::Key::Home),
                )
            });
            if space {
                if self.focus_read.rsvp.finished {
                    self.restart_rsvp();
                    self.focus_read.rsvp.playing = true;
                } else {
                    self.focus_read.rsvp.playing = !self.focus_read.rsvp.playing;
                }
                self.focus_read.rsvp.accumulator = 0.0;
            }
            if right || down {
                self.focus_read.rsvp.playing = false;
                self.step_rsvp_word(true);
            } else if left || up {
                self.focus_read.rsvp.playing = false;
                self.step_rsvp_word(false);
            } else if page_down {
                self.focus_read.rsvp.playing = false;
                self.jump_rsvp_paragraph(true);
            } else if page_up {
                self.focus_read.rsvp.playing = false;
                self.jump_rsvp_paragraph(false);
            } else if home {
                self.focus_read.rsvp.playing = false;
                self.restart_rsvp();
            }
        }

        if self.focus_read.rsvp.playing && !self.focus_read.rsvp.finished {
            let dt = ui.input(|input| input.stable_dt).min(0.2);
            let interval = 60.0 / self.settings.rsvp_wpm.clamp(100, 1000) as f32;
            let mut elapsed = self.focus_read.rsvp.accumulator + dt;
            let mut steps = 0;
            loop {
                let factor = self
                    .focus_read
                    .rsvp
                    .current_word()
                    .map(|word| delay_factor(&word.text))
                    .unwrap_or(1.0);
                let needed = interval * factor;
                if elapsed < needed || steps >= 8 {
                    break;
                }
                elapsed -= needed;
                self.advance_rsvp_word();
                if !self.focus_read.rsvp.playing || self.focus_read.rsvp.finished {
                    elapsed = 0.0;
                    break;
                }
                steps += 1;
            }
            self.focus_read.rsvp.accumulator = elapsed;
            ui.ctx().request_repaint();
        }

        let word = self
            .focus_read
            .rsvp
            .current_word()
            .map(|word| word.text.clone())
            .unwrap_or_default();
        let chapter = self.focus_read.rsvp.chapter;
        let total_paragraphs = self
            .document
            .as_ref()
            .and_then(|doc| doc.chapters().get(chapter))
            .map(|chapter| chapter.paragraphs.len())
            .unwrap_or(0);
        let paragraph_index = self
            .focus_read
            .rsvp
            .current_word()
            .map(|word| word.paragraph)
            .unwrap_or(0);
        let progress = t
            .paragraph_progress
            .replace("{current}", &(paragraph_index + 1).to_string())
            .replace("{total}", &total_paragraphs.max(1).to_string());

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(t.rsvp_title)
                    .size(15.0)
                    .strong()
                    .color(accent),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(IconButton::new(Icon::Cross, t.close_btn))
                    .on_hover_text(t.close_btn)
                    .clicked()
                {
                    self.close_speed_reading();
                }
            });
        });
        ui.separator();

        let font_size = (self.font_size * 2.4).clamp(28.0, 72.0);
        let font = FontId::proportional(font_size);
        let pivot = pivot_char_index(&word);
        let pivot_byte = word
            .char_indices()
            .nth(pivot)
            .map(|(byte, _)| byte)
            .unwrap_or(0);
        let pivot_end = word[pivot_byte..]
            .chars()
            .next()
            .map(|ch| pivot_byte + ch.len_utf8())
            .unwrap_or(pivot_byte);
        let normal = TextFormat {
            font_id: font.clone(),
            color: text_color,
            ..Default::default()
        };
        let highlighted = TextFormat {
            font_id: font.clone(),
            color: accent,
            ..Default::default()
        };
        let mut job = egui::text::LayoutJob::default();
        job.append(&word[..pivot_byte], 0.0, normal.clone());
        job.append(&word[pivot_byte..pivot_end], 0.0, highlighted.clone());
        job.append(&word[pivot_end..], 0.0, normal);
        job.wrap.max_width = f32::INFINITY;
        let galley = ui.painter().layout_job(job);
        let prefix_width = ui
            .painter()
            .layout_no_wrap(word[..pivot_byte].to_string(), font.clone(), text_color)
            .size()
            .x;
        let pivot_width = ui
            .painter()
            .layout_no_wrap(
                word[pivot_byte..pivot_end].to_string(),
                font.clone(),
                accent,
            )
            .size()
            .x;
        let pivot_x = prefix_width + pivot_width * 0.5;
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), font_size * 2.2),
            Sense::hover(),
        );
        let center = rect.center();
        let position = egui::pos2(center.x - pivot_x, center.y - galley.size().y * 0.5);
        ui.painter().galley(position, galley, text_color);
        ui.painter().line_segment(
            [
                egui::pos2(center.x, rect.top() + 4.0),
                egui::pos2(center.x, rect.top() + 14.0),
            ],
            egui::Stroke::new(1.0, accent.gamma_multiply(0.5)),
        );
        ui.painter().line_segment(
            [
                egui::pos2(center.x, rect.bottom() - 14.0),
                egui::pos2(center.x, rect.bottom() - 4.0),
            ],
            egui::Stroke::new(1.0, accent.gamma_multiply(0.5)),
        );

        let index = if self.focus_read.rsvp.words.is_empty() {
            0
        } else {
            self.focus_read.rsvp.index + 1
        };
        let total = self.focus_read.rsvp.words.len();
        ui.vertical_centered(|ui| {
            if self.focus_read.rsvp.finished {
                ui.label(RichText::new(t.rsvp_finished).size(15.0).color(accent));
            }
            ui.label(
                RichText::new(format!("{index} / {total}"))
                    .size(12.0)
                    .color(secondary),
            );
            ui.label(RichText::new(progress).size(12.0).color(secondary));
            ui.add_space(8.0);
            let play_text = if self.focus_read.rsvp.playing {
                t.rsvp_pause
            } else {
                t.rsvp_play
            };
            let speed_label = format!("{} {}", self.settings.rsvp_wpm, t.rsvp_speed);
            let row_width = icon_button_width(ui, play_text)
                + icon_button_width(ui, t.rsvp_restart)
                + icon_button_width(ui, "-")
                + icon_button_width(ui, "+")
                + ui.painter()
                    .layout_no_wrap(speed_label.clone(), FontId::proportional(12.0), secondary)
                    .size()
                    .x
                + ui.spacing().item_spacing.x * 4.0;
            let row_offset = ((ui.available_width() - row_width) * 0.5).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(row_offset);
                let play_icon = if self.focus_read.rsvp.playing {
                    Icon::Pause
                } else {
                    Icon::Play
                };
                if ui
                    .add(
                        IconButton::new(play_icon, play_text)
                            .selected(self.focus_read.rsvp.playing),
                    )
                    .clicked()
                {
                    if self.focus_read.rsvp.finished {
                        self.restart_rsvp();
                        self.focus_read.rsvp.playing = true;
                    } else {
                        self.focus_read.rsvp.playing = !self.focus_read.rsvp.playing;
                    }
                    self.focus_read.rsvp.accumulator = 0.0;
                }
                if ui
                    .add(IconButton::new(Icon::Refresh, t.rsvp_restart))
                    .clicked()
                {
                    self.restart_rsvp();
                }
                if ui.add(IconButton::new(Icon::Minus, "-")).clicked() {
                    self.cycle_rsvp_speed(false);
                }
                ui.label(RichText::new(speed_label).size(12.0).color(secondary));
                if ui.add(IconButton::new(Icon::Plus, "+")).clicked() {
                    self.cycle_rsvp_speed(true);
                }
            });
            ui.add_space(6.0);
            ui.label(RichText::new(t.rsvp_hint).size(11.0).color(secondary));
        });
    }
}
