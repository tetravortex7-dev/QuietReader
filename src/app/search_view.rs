use crate::app::icons::{Icon, IconButton, IconLabel};
use crate::app::theme::ReaderTheme;
use crate::core::{ReaderCore, SearchHit};
use crate::document::{Document, DocumentViewKind};
use eframe::egui::{self, Color32, Frame, Id, Key, Order, RichText, ScrollArea, Stroke, Vec2};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(250);

pub struct SearchState {
    pub is_open: bool,
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub current_hit_index: usize,
    pub should_focus: bool,
    pub last_searched_query: String,
    pub show_results_dropdown: bool,
    pub is_searching: bool,
    search_rx: Option<Receiver<(u64, Vec<SearchHit>)>>,
    search_cancel: Option<Arc<AtomicBool>>,
    search_generation: u64,
    search_due: Option<(String, Instant)>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            hits: Vec::new(),
            current_hit_index: 0,
            should_focus: false,
            last_searched_query: String::new(),
            show_results_dropdown: false,
            is_searching: false,
            search_rx: None,
            search_cancel: None,
            search_generation: 0,
            search_due: None,
        }
    }
}

impl SearchState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.should_focus = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.show_results_dropdown = false;
        self.cancel_search();
    }

    pub fn reset_for_document(&mut self) {
        self.cancel_search();
        self.hits.clear();
        self.current_hit_index = 0;
        self.last_searched_query.clear();
    }

    fn cancel_search(&mut self) {
        if let Some(cancel) = self.search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.search_rx = None;
        self.is_searching = false;
        self.search_due = None;
    }

    pub(crate) fn schedule_search(&mut self, ctx: &egui::Context) {
        let query = self.query.clone();
        self.cancel_search();
        self.hits.clear();
        self.current_hit_index = 0;
        if query.trim().is_empty() {
            self.last_searched_query = query;
            return;
        }
        self.is_searching = true;
        self.search_due = Some((query, Instant::now() + SEARCH_DEBOUNCE));
        ctx.request_repaint_after(SEARCH_DEBOUNCE);
    }

    fn start_search(&mut self, ctx: &egui::Context, document: Option<&Arc<dyn Document>>) {
        self.cancel_search();
        self.last_searched_query = self.query.clone();
        self.hits.clear();
        self.current_hit_index = 0;
        let Some(doc) = document else {
            return;
        };
        if self.query.trim().is_empty() {
            return;
        }

        self.search_generation = self.search_generation.wrapping_add(1);
        let generation = self.search_generation;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let query = self.query.clone();
        let doc = Arc::clone(doc);
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        let spawn = std::thread::Builder::new()
            .name("document-search".to_string())
            .spawn(move || {
                let hits = if doc.view_kind() == DocumentViewKind::Pdf {
                    ReaderCore::search_page_text_in_document(
                        doc.as_ref(),
                        &query,
                        worker_cancel.as_ref(),
                    )
                } else {
                    ReaderCore::search_in_document_with_cancel(
                        doc.as_ref(),
                        &query,
                        worker_cancel.as_ref(),
                    )
                };
                let _ = tx.send((generation, hits));
                ctx.request_repaint();
            });

        if spawn.is_ok() {
            self.search_rx = Some(rx);
            self.search_cancel = Some(cancel);
            self.is_searching = true;
        }
    }

    pub(crate) fn poll_debounce(
        &mut self,
        ctx: &egui::Context,
        document: Option<&Arc<dyn Document>>,
    ) {
        let Some((query, due)) = self.search_due.clone() else {
            return;
        };
        let now = Instant::now();
        if now < due {
            ctx.request_repaint_after(due.saturating_duration_since(now));
            return;
        }
        if query == self.query {
            self.search_due = None;
            self.start_search(ctx, document);
        } else {
            self.schedule_search(ctx);
        }
    }

    fn poll_search(&mut self) -> Option<Vec<SearchHit>> {
        let rx = self.search_rx.as_ref()?;
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => (self.search_generation, Vec::new()),
        };
        self.search_rx = None;
        self.search_cancel = None;
        self.is_searching = false;
        if result.0 == self.search_generation {
            Some(result.1)
        } else {
            None
        }
    }

    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn current_hit(&self) -> Option<&SearchHit> {
        self.hits.get(self.current_hit_index)
    }

    pub fn next_hit(&mut self) -> Option<&SearchHit> {
        if self.hits.is_empty() {
            return None;
        }
        self.current_hit_index = (self.current_hit_index + 1) % self.hits.len();
        self.hits.get(self.current_hit_index)
    }

    pub fn prev_hit(&mut self) -> Option<&SearchHit> {
        if self.hits.is_empty() {
            return None;
        }
        if self.current_hit_index == 0 {
            self.current_hit_index = self.hits.len() - 1;
        } else {
            self.current_hit_index -= 1;
        }
        self.hits.get(self.current_hit_index)
    }
}

fn select_hit(
    hit: &SearchHit,
    jump_target: &mut Option<(usize, Option<usize>)>,
    page_target: &mut Option<usize>,
) {
    if let Some(page) = hit.page_index {
        *page_target = Some(page);
    } else {
        *jump_target = Some((hit.chapter_index, hit.paragraph_index));
    }
}

pub struct SearchView;

impl SearchView {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        ctx: &egui::Context,
        state: &mut SearchState,
        document: Option<&Arc<dyn Document>>,
        current_chapter: &mut usize,
        last_rendered_chapter: &mut usize,
        pending_scroll_y: &mut Option<f32>,
        page_target: &mut Option<usize>,
        font_size: f32,
        theme: &ReaderTheme,
        lang: crate::i18n::UiLanguage,
    ) {
        if !state.is_open {
            return;
        }

        let t = lang.t();
        let mut jump_target = None;
        let mut local_page_target = None;

        if let Some(hits) = state.poll_search() {
            state.hits = hits;
            state.current_hit_index = 0;
            if let Some(first) = state.hits.first() {
                select_hit(first, &mut jump_target, &mut local_page_target);
            }
        }

        egui::Area::new(Id::unique("floating_search_panel"))
            .order(Order::Foreground)
            .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-20.0, 52.0))
            .show(ctx, |ui| {
                let frame = Frame::popup(ui.style())
                    .fill(theme.panel_bg_color())
                    .stroke(Stroke::new(1.0, theme.accent_color().gamma_multiply(0.45)))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(10, 8));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            IconLabel::new(Icon::Search, "")
                                .size(14.0)
                                .color(theme.accent_color()),
                        );

                        let text_edit = egui::TextEdit::singleline(&mut state.query)
                            .hint_text(t.search_placeholder)
                            .desired_width(190.0);
                        let resp = ui.add(text_edit);

                        if state.should_focus {
                            resp.request_focus();
                            state.should_focus = false;
                        }

                        if state.query != state.last_searched_query {
                            let already_scheduled = state
                                .search_due
                                .as_ref()
                                .is_some_and(|(pending, _)| pending == &state.query);
                            if !already_scheduled {
                                state.schedule_search(ctx);
                            }
                        }
                        state.poll_debounce(ctx, document);

                        let total = state.hits.len();
                        if state.query.trim().is_empty() {
                            ui.label(
                                RichText::new("Ctrl+F")
                                    .size(11.5)
                                    .color(theme.secondary_text_color()),
                            );
                        } else if state.is_searching {
                            ui.label(
                                RichText::new(t.search_searching)
                                    .size(12.0)
                                    .color(theme.secondary_text_color()),
                            );
                        } else if total == 0 {
                            ui.label(
                                RichText::new(format!("0 {} 0", t.page_of))
                                    .size(12.0)
                                    .color(Color32::from_rgb(220, 80, 80)),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "{} {} {total}",
                                    state.current_hit_index + 1,
                                    t.page_of
                                ))
                                .size(12.5)
                                .strong()
                                .color(theme.text_color()),
                            );
                        }

                        let prev_btn = ui
                            .add(
                                IconButton::new(Icon::ChevronUp, "")
                                    .min_size(Vec2::new(24.0, 22.0)),
                            )
                            .on_hover_text(t.search_prev);

                        let next_btn = ui
                            .add(
                                IconButton::new(Icon::ChevronDown, "")
                                    .min_size(Vec2::new(24.0, 22.0)),
                            )
                            .on_hover_text(t.search_next);

                        let list_btn = ui.add(
                            IconButton::new(Icon::Toc, "")
                                .min_size(Vec2::new(24.0, 22.0))
                                .selected(state.show_results_dropdown),
                        );

                        let close_btn = ui
                            .add(IconButton::new(Icon::Close, "").min_size(Vec2::new(24.0, 22.0)));

                        if close_btn.clicked() {
                            state.close();
                        }

                        if list_btn.clicked() {
                            state.show_results_dropdown = !state.show_results_dropdown;
                        }

                        let enter_hit = ui.input(|i| i.key_pressed(Key::Enter));
                        let shift = ui.input(|i| i.modifiers.shift);
                        let esc_hit = ui.input(|i| i.key_pressed(Key::Escape));

                        if esc_hit {
                            state.close();
                        } else if (enter_hit && resp.has_focus()) || next_btn.clicked() {
                            if shift {
                                if let Some(hit) = state.prev_hit() {
                                    select_hit(hit, &mut jump_target, &mut local_page_target);
                                }
                            } else if let Some(hit) = state.next_hit() {
                                select_hit(hit, &mut jump_target, &mut local_page_target);
                            }
                        } else if prev_btn.clicked() {
                            if let Some(hit) = state.prev_hit() {
                                select_hit(hit, &mut jump_target, &mut local_page_target);
                            }
                        }
                    });

                    if state.show_results_dropdown && !state.hits.is_empty() {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ScrollArea::vertical()
                            .id_salt("search_results_scroll")
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for (idx, hit) in state.hits.iter().enumerate() {
                                    let is_active = idx == state.current_hit_index;
                                    let chapter_title = if let Some(page) = hit.page_index {
                                        format!("{} {}", t.page_prefix, page + 1)
                                    } else {
                                        document
                                            .and_then(|d| d.chapters().get(hit.chapter_index))
                                            .map(|c| {
                                                crate::document::strip_formatting_tags(&c.title)
                                            })
                                            .filter(|t| !t.is_empty())
                                            .unwrap_or_else(|| {
                                                t.section_num.replace(
                                                    "{}",
                                                    &(hit.chapter_index + 1).to_string(),
                                                )
                                            })
                                    };

                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            let mut title_rt =
                                                RichText::new(&chapter_title).strong().size(12.5);
                                            if is_active {
                                                title_rt = title_rt.color(theme.accent_color());
                                            }
                                            ui.label(title_rt);
                                        });
                                        let snippet_btn = ui.add(
                                            egui::Button::new(
                                                RichText::new(&hit.snippet).size(11.5).color(
                                                    if is_active {
                                                        theme.text_color()
                                                    } else {
                                                        theme.secondary_text_color()
                                                    },
                                                ),
                                            )
                                            .frame(false)
                                            .wrap(),
                                        );
                                        if snippet_btn.clicked() {
                                            state.current_hit_index = idx;
                                            select_hit(
                                                hit,
                                                &mut jump_target,
                                                &mut local_page_target,
                                            );
                                        }
                                    });
                                    ui.add_space(2.0);
                                }
                            });
                    }
                });
            });

        let had_navigation = jump_target.is_some() || local_page_target.is_some();

        if let Some(page) = local_page_target {
            *page_target = Some(page);
        }

        if let (Some((target_ch, target_para)), Some(doc)) = (jump_target, document) {
            *current_chapter = target_ch;
            *last_rendered_chapter = target_ch;
            *pending_scroll_y = Some(ReaderCore::estimate_paragraph_scroll_y(
                doc.as_ref(),
                target_ch,
                target_para,
                font_size,
            ));
        }

        if had_navigation {
            ctx.request_repaint();
        }
    }
}
