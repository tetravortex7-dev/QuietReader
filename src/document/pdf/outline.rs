use crate::document::TocItem;
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::object::{Array, Dict, Object, ObjectIdentifier, String as PdfString};
use std::collections::{HashMap, HashSet};

pub fn extract_pdf_toc(pdf: &Pdf, page_count: usize) -> Vec<TocItem> {
    if page_count == 0 {
        return Vec::new();
    }

    let xref = pdf.xref();
    let root = match xref.get::<Dict<'_>>(xref.root_id()) {
        Some(r) => r,
        None => return build_fallback_toc(page_count),
    };

    let outlines = match root.get::<Dict<'_>>("Outlines") {
        Some(o) => o,
        None => return build_fallback_toc(page_count),
    };

    let mut page_id_to_index: HashMap<ObjectIdentifier, usize> = HashMap::with_capacity(page_count);
    for (i, p) in pdf.pages().iter().enumerate() {
        if let Some(id) = p.raw().obj_id() {
            page_id_to_index.insert(id, i);
        }
    }

    let mut named_dests: HashMap<String, usize> = HashMap::new();
    if let Some(dests) = root.get::<Dict<'_>>("Dests") {
        for (name, _) in dests.entries() {
            let key = decode_pdf_string(name.as_ref());
            if let Some(arr) = dests.get::<Array<'_>>(name.as_ref()) {
                if let Some(page) = resolve_dest_array(&arr, &page_id_to_index, page_count) {
                    named_dests.insert(key, page);
                }
            } else if let Some(dict) = dests.get::<Dict<'_>>(name.as_ref()) {
                if let Some(arr) = dict.get::<Array<'_>>("D") {
                    if let Some(page) = resolve_dest_array(&arr, &page_id_to_index, page_count) {
                        named_dests.insert(key, page);
                    }
                }
            }
        }
    }

    let mut items = Vec::new();
    let mut cur = outlines.get::<Dict<'_>>("First");
    let mut visited = HashSet::new();

    while let Some(item_dict) = cur {
        if let Some(id) = item_dict.obj_id()
            && !visited.insert(id)
        {
            break;
        }

        if let Some(toc_item) = parse_outline_item(
            &item_dict,
            &page_id_to_index,
            &named_dests,
            page_count,
            &mut visited,
            0,
        ) {
            items.push(toc_item);
        }

        cur = item_dict.get::<Dict<'_>>("Next");
    }

    if items.is_empty() {
        build_fallback_toc(page_count)
    } else {
        items
    }
}

fn parse_outline_item(
    item_dict: &Dict<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    named_dests: &HashMap<String, usize>,
    page_count: usize,
    visited: &mut HashSet<ObjectIdentifier>,
    depth: usize,
) -> Option<TocItem> {
    if depth > 25 {
        return None;
    }

    let title_bytes = item_dict.get::<PdfString<'_>>("Title")?;
    let title = decode_pdf_string(title_bytes.as_bytes());
    if title.trim().is_empty() {
        return None;
    }

    let explicit_target = resolve_destination(item_dict, page_id_to_index, named_dests, page_count);

    let mut children = Vec::new();
    let mut cur_child = item_dict.get::<Dict<'_>>("First");
    while let Some(child_dict) = cur_child {
        if let Some(id) = child_dict.obj_id()
            && !visited.insert(id)
        {
            break;
        }
        if let Some(child_item) = parse_outline_item(
            &child_dict,
            page_id_to_index,
            named_dests,
            page_count,
            visited,
            depth + 1,
        ) {
            children.push(child_item);
        }
        cur_child = child_dict.get::<Dict<'_>>("Next");
    }

    let target_page = explicit_target
        .or_else(|| children.first().map(|c| c.chapter_index))
        .unwrap_or(0)
        .min(page_count.saturating_sub(1));

    Some(TocItem {
        id: Some(format!("page_{target_page}")),
        title,
        chapter_index: target_page,
        children,
    })
}

pub(crate) fn resolve_destination(
    item_dict: &Dict<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    named_dests: &HashMap<String, usize>,
    page_count: usize,
) -> Option<usize> {
    if let Some(dest_arr) = item_dict.get::<Array<'_>>("Dest")
        && let Some(page) = resolve_dest_array(&dest_arr, page_id_to_index, page_count)
    {
        return Some(page);
    }

    if let Some(dest_str) = item_dict.get::<PdfString<'_>>("Dest") {
        let key = decode_pdf_string(dest_str.as_bytes());
        if let Some(&page) = named_dests.get(&key) {
            return Some(page);
        }
    }

    if let Some(a_dict) = item_dict.get::<Dict<'_>>("A") {
        if let Some(d_arr) = a_dict.get::<Array<'_>>("D")
            && let Some(page) = resolve_dest_array(&d_arr, page_id_to_index, page_count)
        {
            return Some(page);
        }
        if let Some(d_str) = a_dict.get::<PdfString<'_>>("D") {
            let key = decode_pdf_string(d_str.as_bytes());
            if let Some(&page) = named_dests.get(&key) {
                return Some(page);
            }
        }
    }

    None
}

pub fn resolve_dest_array(
    arr: &Array<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    page_count: usize,
) -> Option<usize> {
    let first = arr.iter::<Object<'_>>().next()?;
    match first {
        Object::Dict(d) => {
            if let Some(id) = d.obj_id()
                && let Some(&page_idx) = page_id_to_index.get(&id)
            {
                return Some(page_idx.min(page_count.saturating_sub(1)));
            }
        }
        Object::Number(n) => {
            let i = n.as_i64();
            if i >= 0 {
                return Some((i as usize).min(page_count.saturating_sub(1)));
            }
        }
        _ => {}
    }
    None
}

pub fn build_fallback_toc(page_count: usize) -> Vec<TocItem> {
    (0..page_count)
        .map(|i| TocItem {
            id: Some(format!("page_{}", i + 1)),
            title: format!("Страница {}", i + 1),
            chapter_index: i,
            children: Vec::new(),
        })
        .collect()
}

pub fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16_chars: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&u16_chars) {
            return s;
        }
    }

    if let Ok(s) = std::str::from_utf8(bytes)
        && s.chars().any(|c| c as u32 > 127)
    {
        return s.to_string();
    }

    encoding_rs::WINDOWS_1252
        .decode_without_bom_handling(bytes)
        .0
        .to_string()
}
