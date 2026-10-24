use super::footnotes;
use super::{BodyNode, SectionNode};
use crate::document::{BookChapter, TocItem};
use std::collections::HashMap;

struct FlattenContext {
    chapters: Vec<BookChapter>,
    section_id_to_chapter: HashMap<String, usize>,
}

pub(super) fn build_chapters_and_toc(
    bodies: Vec<BodyNode>,
) -> (
    Vec<BookChapter>,
    Vec<TocItem>,
    HashMap<String, usize>,
    HashMap<String, String>,
    HashMap<String, crate::document::NoteLocation>,
) {
    let mut ctx = FlattenContext {
        chapters: Vec::new(),
        section_id_to_chapter: HashMap::new(),
    };
    let mut root_toc: Vec<TocItem> = Vec::new();
    let mut footnotes = HashMap::new();
    let mut notes_order: Vec<(String, String)> = Vec::new();
    let mut notes_title: Option<String> = None;

    for body in bodies {
        if footnotes::is_notes_body(body.name.as_deref()) {
            notes_order.extend(footnotes::collect_body(&body, &mut footnotes));
            if notes_title.is_none()
                && let Some(title) = body.title.clone().filter(|title| !title.trim().is_empty())
            {
                notes_title = Some(title);
            }
            continue;
        }
        if body
            .name
            .as_deref()
            .unwrap_or("")
            .eq_ignore_ascii_case("toc")
        {
            continue;
        }

        let body_has_direct_text = body.direct_paragraphs.iter().any(|p| !p.trim().is_empty());
        let body_has_title = body
            .title
            .as_ref()
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false);

        let mut body_start_chapter: Option<usize> = None;

        if body_has_direct_text || (body.sections.is_empty() && body_has_title) {
            let idx = ctx.chapters.len();
            body_start_chapter = Some(idx);
            ctx.chapters.push(BookChapter {
                id: None,
                title: body.title.clone().unwrap_or_default(),
                paragraphs: body.direct_paragraphs,
            });
        }

        let mut body_sections_toc: Vec<TocItem> = Vec::new();

        for section in body.sections {
            let (sec_first_ch, sec_tocs) = flatten_section(section, &mut ctx, 0);
            if body_start_chapter.is_none() {
                body_start_chapter = sec_first_ch;
            }
            body_sections_toc.extend(sec_tocs);
        }

        if body_has_title {
            let title = body.title.unwrap_or_default();
            let chapter_index = body_start_chapter.unwrap_or(0);
            root_toc.push(TocItem {
                id: None,
                title,
                chapter_index,
                children: body_sections_toc,
            });
        } else {
            root_toc.extend(body_sections_toc);
        }
    }

    let note_locations = crate::document::push_notes_chapter(
        &mut ctx.chapters,
        notes_title.unwrap_or_else(|| crate::document::NOTES_CHAPTER_TITLE.to_string()),
        &notes_order,
    );

    (
        ctx.chapters,
        root_toc,
        ctx.section_id_to_chapter,
        footnotes,
        note_locations,
    )
}

fn flatten_section(
    mut section: SectionNode,
    ctx: &mut FlattenContext,
    depth: usize,
) -> (Option<usize>, Vec<TocItem>) {
    let has_own_title = section
        .title
        .as_ref()
        .map(|t| !t.trim().is_empty())
        .unwrap_or(false);

    if has_own_title && section.paragraphs.is_empty() && !section.subsections.is_empty() {
        while !section.subsections.is_empty() {
            let is_untitled_leaf = section.subsections[0]
                .title
                .as_ref()
                .map(|t| t.trim().is_empty())
                .unwrap_or(true)
                && section.subsections[0].subsections.is_empty();

            if is_untitled_leaf {
                let first_sub = section.subsections.remove(0);
                if section.id.is_none() {
                    section.id = first_sub.id;
                }
                section.paragraphs.extend(first_sub.paragraphs);
            } else {
                break;
            }
        }
    }

    if depth >= 32 {
        let mut overflow_paragraphs = Vec::new();
        for sub in section.subsections.drain(..) {
            overflow_paragraphs.extend(sub.paragraphs);
            for s in sub.subsections {
                overflow_paragraphs.extend(s.paragraphs);
            }
        }
        section.paragraphs.extend(overflow_paragraphs);
    }

    let has_own_paragraphs = section.paragraphs.iter().any(|p| !p.trim().is_empty());
    let mut own_chapter_idx: Option<usize> = None;

    let should_create_chapter = if has_own_paragraphs {
        true
    } else if section.subsections.is_empty() {
        has_own_title
    } else {
        false
    };

    if should_create_chapter {
        let idx = ctx.chapters.len();
        own_chapter_idx = Some(idx);
        ctx.chapters.push(BookChapter {
            id: section.id.clone(),
            title: section.title.clone().unwrap_or_default(),
            paragraphs: section.paragraphs,
        });
    }

    let mut child_tocs: Vec<TocItem> = Vec::new();
    let mut first_child_chapter: Option<usize> = None;

    for sub in section.subsections {
        let (sub_first_ch, sub_tocs) = flatten_section(sub, ctx, depth + 1);
        if first_child_chapter.is_none() {
            first_child_chapter = sub_first_ch;
        }
        child_tocs.extend(sub_tocs);
    }

    let effective_chapter_idx = own_chapter_idx.or(first_child_chapter);

    if let (Some(id), Some(ch_idx)) = (&section.id, effective_chapter_idx) {
        ctx.section_id_to_chapter
            .entry(id.clone())
            .or_insert(ch_idx);
        ctx.section_id_to_chapter
            .entry(crate::document::epub::percent_decode_str(id))
            .or_insert(ch_idx);
    }

    if has_own_title {
        let title = section.title.unwrap_or_default();
        let chapter_index = effective_chapter_idx.unwrap_or(0);
        let toc_item = TocItem {
            id: section.id,
            title,
            chapter_index,
            children: child_tocs,
        };
        (effective_chapter_idx, vec![toc_item])
    } else {
        (effective_chapter_idx, child_tocs)
    }
}
