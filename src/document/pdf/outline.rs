use crate::document::TocItem;
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::object::{
    Array, Dict, Name, Object, ObjectIdentifier, String as PdfString,
};
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

    let named_dests = collect_named_dests(&root, &page_id_to_index, page_count);

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

pub(crate) fn collect_named_dests(
    root: &Dict<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    page_count: usize,
) -> HashMap<String, usize> {
    let mut named_dests = HashMap::new();
    if let Some(dests) = root.get::<Dict<'_>>("Dests") {
        for (name, _) in dests.entries() {
            let key = decode_pdf_string(name.as_ref());
            let page = dests
                .get::<Array<'_>>(name.as_ref())
                .and_then(|arr| resolve_dest_array(&arr, page_id_to_index, page_count))
                .or_else(|| {
                    dests
                        .get::<Dict<'_>>(name.as_ref())
                        .and_then(|dict| dict.get::<Array<'_>>("D"))
                        .and_then(|arr| resolve_dest_array(&arr, page_id_to_index, page_count))
                });
            if let Some(page) = page {
                named_dests.insert(key, page);
            }
        }
    }
    if let Some(names) = root.get::<Dict<'_>>("Names")
        && let Some(dests_tree) = names.get::<Dict<'_>>("Dests")
    {
        let mut visited = HashSet::new();
        collect_dests_name_tree(
            &dests_tree,
            page_id_to_index,
            page_count,
            &mut named_dests,
            &mut visited,
            0,
        );
    }
    named_dests
}

fn resolve_dest_object(
    obj: Object<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    page_count: usize,
) -> Option<usize> {
    match obj {
        Object::Array(arr) => resolve_dest_array(&arr, page_id_to_index, page_count),
        Object::Dict(dict) => dict
            .get::<Array<'_>>("D")
            .and_then(|arr| resolve_dest_array(&arr, page_id_to_index, page_count)),
        _ => None,
    }
}

fn collect_dests_name_tree(
    node: &Dict<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    page_count: usize,
    named_dests: &mut HashMap<String, usize>,
    visited: &mut HashSet<ObjectIdentifier>,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    if let Some(id) = node.obj_id()
        && !visited.insert(id)
    {
        return;
    }
    if let Some(names) = node.get::<Array<'_>>("Names") {
        let mut iter = names.iter::<Object<'_>>();
        while let Some(key_obj) = iter.next() {
            let Some(val_obj) = iter.next() else {
                break;
            };
            let key = match key_obj {
                Object::String(s) => decode_pdf_string(s.as_bytes()),
                Object::Name(n) => decode_pdf_string(n.as_ref()),
                _ => continue,
            };
            if let Some(page) = resolve_dest_object(val_obj, page_id_to_index, page_count) {
                named_dests.insert(key, page);
            }
        }
    }
    if let Some(kids) = node.get::<Array<'_>>("Kids") {
        for kid in kids.iter::<Dict<'_>>() {
            collect_dests_name_tree(
                &kid,
                page_id_to_index,
                page_count,
                named_dests,
                visited,
                depth + 1,
            );
        }
    }
}

fn resolve_dest_entry(
    dict: &Dict<'_>,
    key: &str,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    named_dests: &HashMap<String, usize>,
    page_count: usize,
) -> Option<usize> {
    if let Some(dest_arr) = dict.get::<Array<'_>>(key)
        && let Some(page) = resolve_dest_array(&dest_arr, page_id_to_index, page_count)
    {
        return Some(page);
    }
    if let Some(dest_str) = dict.get::<PdfString<'_>>(key) {
        let name_key = decode_pdf_string(dest_str.as_bytes());
        if let Some(&page) = named_dests.get(&name_key) {
            return Some(page);
        }
    }
    if let Some(dest_name) = dict.get::<Name<'_>>(key) {
        let name_key = decode_pdf_string(dest_name.as_ref());
        if let Some(&page) = named_dests.get(&name_key) {
            return Some(page);
        }
    }
    None
}

pub(crate) fn resolve_destination(
    item_dict: &Dict<'_>,
    page_id_to_index: &HashMap<ObjectIdentifier, usize>,
    named_dests: &HashMap<String, usize>,
    page_count: usize,
) -> Option<usize> {
    if let Some(page) =
        resolve_dest_entry(item_dict, "Dest", page_id_to_index, named_dests, page_count)
    {
        return Some(page);
    }

    if let Some(a_dict) = item_dict.get::<Dict<'_>>("A") {
        if let Some(action) = a_dict.get::<Name<'_>>("S")
            && action.as_str() != "GoTo"
        {
            return None;
        }
        return resolve_dest_entry(&a_dict, "D", page_id_to_index, named_dests, page_count);
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
            title: crate::document::errors::tokens::page(i + 1),
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
