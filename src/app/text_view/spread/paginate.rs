use super::SpreadRenderParams;
use crate::app::reader_chrome::is_blockquote_paragraph;
use crate::app::text_images::TextImageState;
use crate::app::text_view::footnotes::{FootnoteMarker, ParagraphStyle};
use crate::app::text_view::layout::{append_with_highlight, create_paragraph_layout_job};
use crate::core::library::UiDesignMode;
use crate::document::BookChapter;
use eframe::egui::{self, Color32, Vec2};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

const SPREAD_CACHE_KEY: &str = "quietreader_spread_pagination_cache";

#[derive(Clone, PartialEq, Eq, Hash)]
struct PaginationKey {
    file_path: Option<PathBuf>,
    chapter_idx: usize,
    col_w_tenths: i32,
    page_h_tenths: i32,
    font_size_tenths: i32,
    line_height_tenths: i32,
    font_family: String,
    is_cozy: bool,
    is_rtl: bool,
    bionic: bool,
    query: String,
    active_para: Option<usize>,
    quotes_hash: u64,
    images_hash: u64,
    ink_rgba: [u8; 4],
    accent_rgba: [u8; 4],
    has_header: bool,
    has_subtitle: bool,
    has_footer: bool,
}

#[derive(Clone)]
pub(super) enum PageEntry {
    ChapterHeader {
        overline: Option<String>,
        title_job: egui::text::LayoutJob,
        title_text: String,
        show_rule: bool,
        subtitle: Option<String>,
        trailing_space: f32,
    },
    Space(f32),
    Image {
        id: String,
        size: Vec2,
        trailing_space: f32,
    },
    TextSlice {
        para_idx: usize,
        raw_paragraph: String,
        clean_paragraph: String,
        job: egui::text::LayoutJob,
        byte_start: usize,
        byte_end: usize,
        char_offset: usize,
        markers: Vec<FootnoteMarker>,
        is_opening: bool,
        is_blockquote: bool,
        pad_top: bool,
        pad_bot: bool,
        trailing_space: f32,
    },
    FooterNote {
        text: String,
    },
}

#[derive(Clone)]
pub(super) struct SpreadPage {
    pub entries: Vec<PageEntry>,
    pub para_indices: HashSet<usize>,
    pub includes_header: bool,
}

#[derive(Clone)]
struct CachedPagination {
    key: PaginationKey,
    pages: Arc<Vec<SpreadPage>>,
}

fn hash_quotes(quotes: &[(&str, Color32)]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    quotes.len().hash(&mut hasher);
    for (text, color) in quotes {
        text.hash(&mut hasher);
        color.to_srgba_unmultiplied().hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_images(chapter: &BookChapter, text_images: &TextImageState) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for p in &chapter.paragraphs {
        if let Some(id) = crate::document::image_token_id(p) {
            id.hash(&mut hasher);
            if let Some(sz) = text_images.image_size(id) {
                ((sz.x * 10.0).round() as i32).hash(&mut hasher);
                ((sz.y * 10.0).round() as i32).hash(&mut hasher);
            } else {
                0i32.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

pub(super) fn get_or_build_pages(
    ui: &mut egui::Ui,
    params: &SpreadRenderParams<'_>,
    page_w: f32,
    page_h: f32,
) -> Arc<Vec<SpreadPage>> {
    let line_height_scale = params.settings.effective_line_height_scale();
    let (query, active_para) = match params.search_highlight {
        Some((q, p)) => (q.to_string(), p),
        None => (String::new(), Some(usize::MAX)),
    };
    let footer_text = if *params.current_chapter + 1 >= params.total_chapters {
        params.doc.footer_text()
    } else {
        None
    };
    let subtitle_text = if *params.current_chapter == 0 {
        params.doc.header_text()
    } else {
        None
    };
    let key = PaginationKey {
        file_path: params.current_file_path.clone(),
        chapter_idx: *params.current_chapter,
        col_w_tenths: (page_w * 10.0).round() as i32,
        page_h_tenths: (page_h * 10.0).round() as i32,
        font_size_tenths: (params.font_size * 10.0).round() as i32,
        line_height_tenths: (line_height_scale * 100.0).round() as i32,
        font_family: params.settings.effective_font_family().to_string(),
        is_cozy: params.design == UiDesignMode::Cozy,
        is_rtl: params.is_rtl,
        bionic: params.settings.effective_bionic_reading(),
        query: query.clone(),
        active_para,
        quotes_hash: hash_quotes(params.chapter_quotes),
        images_hash: hash_images(params.chapter, params.text_images),
        ink_rgba: params.ink_color.to_srgba_unmultiplied(),
        accent_rgba: params.accent_color.to_srgba_unmultiplied(),
        has_header: !crate::document::strip_formatting_tags(&params.chapter.title).is_empty(),
        has_subtitle: subtitle_text.is_some(),
        has_footer: footer_text.is_some(),
    };

    let cache_id = egui::Id::unique(SPREAD_CACHE_KEY);
    if let Some(cached) = ui.ctx().data(|d| d.get_temp::<CachedPagination>(cache_id))
        && cached.key == key
    {
        return cached.pages;
    }

    let pages = Arc::new(build_chapter_pages(
        ui,
        params,
        page_w,
        page_h,
        &query,
        active_para,
        subtitle_text,
        footer_text,
    ));
    ui.ctx().data_mut(|d| {
        d.insert_temp(
            cache_id,
            CachedPagination {
                key,
                pages: Arc::clone(&pages),
            },
        );
    });
    pages
}

#[allow(clippy::too_many_arguments)]
fn build_chapter_pages(
    ui: &mut egui::Ui,
    params: &SpreadRenderParams<'_>,
    page_w: f32,
    page_h: f32,
    query: &str,
    active_para: Option<usize>,
    subtitle_text: Option<&str>,
    footer_text: Option<&str>,
) -> Vec<SpreadPage> {
    let mut pages: Vec<SpreadPage> = Vec::new();
    let mut cur_entries: Vec<PageEntry> = Vec::new();
    let mut cur_paras: HashSet<usize> = HashSet::new();
    let mut cur_has_header = false;
    let mut used_h = 0.0f32;

    let flush_page = |pages: &mut Vec<SpreadPage>,
                      entries: &mut Vec<PageEntry>,
                      paras: &mut HashSet<usize>,
                      has_hdr: &mut bool,
                      used: &mut f32| {
        if !entries.is_empty() {
            pages.push(SpreadPage {
                entries: std::mem::take(entries),
                para_indices: std::mem::take(paras),
                includes_header: *has_hdr,
            });
            *has_hdr = false;
            *used = 0.0;
        }
    };

    let is_cozy = params.design == UiDesignMode::Cozy;
    let line_height = params.font_size * params.settings.effective_line_height_scale();
    let para_spacing = params.font_size * 0.4;
    let clean_title = crate::document::strip_formatting_tags(&params.chapter.title);

    if !clean_title.is_empty() || subtitle_text.is_some() {
        let mut hdr_h = 0.0f32;
        let overline = if !clean_title.is_empty() && is_cozy && params.total_chapters > 1 {
            let s = params
                .t
                .extra
                .chapter_short
                .replace("{n}", &(*params.current_chapter + 1).to_string())
                .to_uppercase();
            let g = ui.painter().layout_no_wrap(
                s.clone(),
                egui::FontId::proportional(10.5),
                params.accent_color,
            );
            hdr_h += g.size().y + 4.0;
            Some(s)
        } else {
            None
        };

        let mut title_job = egui::text::LayoutJob::default();
        if !clean_title.is_empty() {
            if params.is_rtl {
                title_job.halign = egui::Align::RIGHT;
            }
            let base_title_fmt = egui::TextFormat {
                font_id: egui::FontId::new(
                    params.font_size * if is_cozy { 1.38 } else { 1.35 },
                    egui::FontFamily::Name(crate::app::fonts::FONT_FAMILY_BOLD.into()),
                ),
                color: params.ink_color,
                ..Default::default()
            };
            let is_active_title = active_para.is_none() && !query.is_empty();
            append_with_highlight(
                &mut title_job,
                &clean_title,
                &base_title_fmt,
                query,
                is_active_title,
                params.chapter_quotes,
            );
            title_job.wrap.max_width = page_w;
            let title_galley = ui.fonts_mut(|f| f.layout_job(title_job.clone()));
            hdr_h += title_galley.size().y + if is_cozy { 10.0 } else { 20.0 };
            if is_cozy {
                hdr_h += 16.0;
            }
        }

        let subtitle = subtitle_text.map(str::to_string);
        if let Some(sub) = &subtitle {
            let mut sub_job = egui::text::LayoutJob::simple(
                sub.clone(),
                egui::FontId::proportional(params.font_size * 0.78),
                params.ink_color.gamma_multiply(0.6),
                page_w,
            );
            sub_job.wrap.max_width = page_w;
            let sub_g = ui.fonts_mut(|f| f.layout_job(sub_job));
            hdr_h += sub_g.size().y + 10.0;
        }

        let trailing = 6.0;
        cur_entries.push(PageEntry::ChapterHeader {
            overline,
            title_job,
            title_text: clean_title,
            show_rule: is_cozy && !params.chapter.title.is_empty(),
            subtitle,
            trailing_space: trailing,
        });
        cur_has_header = true;
        used_h += hdr_h + trailing;
    }

    let img_max_w = page_w.min(params.column_width).max(1.0);
    let img_max_h = (page_h * 0.72).max(140.0);

    for (p_idx, paragraph) in params.chapter.paragraphs.iter().enumerate() {
        if paragraph.is_empty() {
            let sp = params.font_size * 0.7;
            if used_h > 0.0 && used_h + sp <= page_h {
                cur_entries.push(PageEntry::Space(sp));
                used_h += sp;
            }
            continue;
        }

        if let Some(image_id) = crate::document::image_token_id(paragraph) {
            let size = params
                .text_images
                .image_size(image_id)
                .map(|raw| {
                    if raw.x > 0.0 && raw.y > 0.0 {
                        let scale = (img_max_w / raw.x).min(img_max_h / raw.y).min(1.0);
                        raw * scale
                    } else {
                        Vec2::new(img_max_w, img_max_h.min(params.font_size * 14.0))
                    }
                })
                .unwrap_or_else(|| Vec2::new(img_max_w, img_max_h.min(params.font_size * 14.0)));
            let total_h = size.y + para_spacing;
            if used_h > 0.0 && used_h + size.y > page_h {
                flush_page(
                    &mut pages,
                    &mut cur_entries,
                    &mut cur_paras,
                    &mut cur_has_header,
                    &mut used_h,
                );
            }
            cur_entries.push(PageEntry::Image {
                id: image_id.to_string(),
                size,
                trailing_space: para_spacing,
            });
            cur_paras.insert(p_idx);
            used_h += total_h.min(page_h);
            continue;
        }

        let clean_paragraph = crate::document::strip_formatting_tags(paragraph);
        let is_active_para = active_para == Some(p_idx);
        let is_opening = params.first_text_p_idx == Some(p_idx);
        let is_blockquote = is_cozy && is_blockquote_paragraph(paragraph);
        let quote_h_pad = if is_blockquote { 26.0 } else { 0.0 };
        let inner_w = (page_w - quote_h_pad).max(80.0);

        let mut markers = Vec::new();
        let mut job = create_paragraph_layout_job(
            paragraph,
            &ParagraphStyle {
                font_size: params.font_size,
                text_color: params.ink_color,
                accent_color: params.accent_color,
                line_height,
                query,
                is_active_hit: is_active_para,
                is_rtl: params.is_rtl,
                bionic_reading: params.settings.effective_bionic_reading(),
                quotes: params.chapter_quotes,
                footnote_numbers: params.footnote_numbers,
                note_available: params.available_notes,
                tts_word_range: None,
                is_opening,
                is_blockquote,
            },
            &mut markers,
        );
        job.wrap.max_width = inner_w;
        let galley = ui.fonts_mut(|f| f.layout_job(job.clone()));
        let row_count = galley.rows.len();
        let total_chars = job.text.chars().count();

        if row_count == 0 {
            continue;
        }

        let mut row_starts = Vec::with_capacity(row_count + 1);
        let mut acc_chars = 0usize;
        for row in &galley.rows {
            row_starts.push(acc_chars);
            acc_chars += row.char_count_including_newline().0;
        }
        row_starts.push(total_chars);

        let mut start_row = 0usize;
        while start_row < row_count {
            let mut rem_h = page_h - used_h;
            let pad_top = is_blockquote && start_row == 0;
            let top_margin = if pad_top { 8.0 } else { 0.0 };
            let min_needed = (galley.rows[start_row].rect().height() + top_margin).max(line_height);
            if used_h > 0.0 && rem_h < min_needed {
                flush_page(
                    &mut pages,
                    &mut cur_entries,
                    &mut cur_paras,
                    &mut cur_has_header,
                    &mut used_h,
                );
                rem_h = page_h;
            }

            let top_y = galley.rows[start_row].rect().min.y;
            let mut end_row = start_row;
            while end_row < row_count {
                let bot_y = galley.rows[end_row].rect().max.y;
                let pad_bot = if is_blockquote && end_row + 1 == row_count {
                    8.0
                } else {
                    0.0
                };
                let slice_h = (bot_y - top_y) + top_margin + pad_bot;
                if slice_h <= rem_h || end_row == start_row {
                    end_row += 1;
                } else {
                    break;
                }
            }

            if used_h > 0.0
                && end_row < row_count
                && (end_row - start_row) < 2
                && (row_count - start_row) >= 2
            {
                flush_page(
                    &mut pages,
                    &mut cur_entries,
                    &mut cur_paras,
                    &mut cur_has_header,
                    &mut used_h,
                );
                continue;
            }

            let is_first_slice = start_row == 0;
            let is_last_slice = end_row >= row_count;
            let pad_bot = is_blockquote && is_last_slice;
            let slice_raw_h = (galley.rows[end_row - 1].rect().max.y - top_y)
                + if is_blockquote && is_first_slice {
                    8.0
                } else {
                    0.0
                }
                + if pad_bot { 8.0 } else { 0.0 };

            let c_start = row_starts[start_row].min(total_chars);
            let c_end = if is_last_slice {
                total_chars
            } else {
                row_starts[end_row].max(c_start + 1).min(total_chars)
            };

            let b_start = char_idx_to_byte_idx(&job.text, c_start);
            let mut b_end = char_idx_to_byte_idx(&job.text, c_end);
            if !is_last_slice && b_end > b_start {
                let trimmed = job.text[b_start..b_end].trim_end_matches(' ').len();
                if trimmed > 0 {
                    b_end = b_start + trimmed;
                }
            }

            let slice_job = slice_layout_job(&job, b_start, b_end, is_first_slice);
            let slice_markers = rebase_markers(&markers, c_start, c_end);
            let trailing = if is_last_slice { para_spacing } else { 0.0 };

            cur_entries.push(PageEntry::TextSlice {
                para_idx: p_idx,
                raw_paragraph: paragraph.clone(),
                clean_paragraph: clean_paragraph.clone(),
                job: slice_job,
                byte_start: b_start,
                byte_end: b_end,
                char_offset: c_start,
                markers: slice_markers,
                is_opening,
                is_blockquote,
                pad_top: is_blockquote && is_first_slice,
                pad_bot,
                trailing_space: trailing,
            });
            cur_paras.insert(p_idx);
            used_h += slice_raw_h + trailing;

            start_row = end_row;
            if start_row < row_count {
                flush_page(
                    &mut pages,
                    &mut cur_entries,
                    &mut cur_paras,
                    &mut cur_has_header,
                    &mut used_h,
                );
            }
        }
    }

    if let Some(footer) = footer_text {
        let mut f_job = egui::text::LayoutJob::simple(
            footer.to_string(),
            egui::FontId::proportional(params.font_size * 0.78),
            params.ink_color.gamma_multiply(0.6),
            page_w,
        );
        f_job.wrap.max_width = page_w;
        let f_g = ui.fonts_mut(|f| f.layout_job(f_job));
        let fh = f_g.size().y + 12.0;
        if used_h > 0.0 && used_h + fh > page_h {
            flush_page(
                &mut pages,
                &mut cur_entries,
                &mut cur_paras,
                &mut cur_has_header,
                &mut used_h,
            );
        }
        cur_entries.push(PageEntry::FooterNote {
            text: footer.to_string(),
        });
    }

    flush_page(
        &mut pages,
        &mut cur_entries,
        &mut cur_paras,
        &mut cur_has_header,
        &mut used_h,
    );

    if pages.is_empty() {
        pages.push(SpreadPage {
            entries: Vec::new(),
            para_indices: HashSet::new(),
            includes_header: true,
        });
    }
    pages
}

fn char_idx_to_byte_idx(text: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(byte_idx, _)| byte_idx)
}

pub(crate) fn slice_layout_job(
    source: &egui::text::LayoutJob,
    byte_start: usize,
    byte_end: usize,
    is_first_slice: bool,
) -> egui::text::LayoutJob {
    let end = byte_end.min(source.text.len());
    let start = byte_start.min(end);
    let mut out = source.clone();
    out.text = source.text[start..end].to_string();
    if !is_first_slice {
        out.first_row_min_height = 0.0;
    }
    let start_bi = egui::text::ByteIndex(start);
    let end_bi = egui::text::ByteIndex(end);
    out.sections.clear();
    for sec in &source.sections {
        let s = sec.byte_range.start.max(start_bi);
        let e = sec.byte_range.end.min(end_bi);
        if s < e {
            let mut sliced = sec.clone();
            sliced.byte_range =
                egui::text::ByteIndex(s.0 - start)..egui::text::ByteIndex(e.0 - start);
            if !is_first_slice {
                sliced.leading_space = 0.0;
            }
            out.sections.push(sliced);
        }
    }
    out
}

pub(crate) fn rebase_markers(
    markers: &[FootnoteMarker],
    c_start: usize,
    c_end: usize,
) -> Vec<FootnoteMarker> {
    markers
        .iter()
        .filter(|m| m.end_char > c_start && m.start_char < c_end)
        .map(|m| FootnoteMarker {
            id: m.id.clone(),
            number: m.number,
            start_char: m.start_char.max(c_start) - c_start,
            end_char: m.end_char.min(c_end) - c_start,
        })
        .collect()
}
