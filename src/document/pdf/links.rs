use crate::document::PageLink;
use hayro::hayro_syntax::Pdf;
use hayro::hayro_syntax::object::{Array, Dict, Name, Object, ObjectIdentifier, Rect};
use std::collections::HashMap;

use super::outline::decode_pdf_string;

pub fn extract_pdf_links(pdf: &Pdf, page_count: usize) -> HashMap<usize, Vec<PageLink>> {
    let mut links: HashMap<usize, Vec<PageLink>> = HashMap::new();
    if page_count == 0 {
        return links;
    }

    let xref = pdf.xref();
    let Some(root) = xref.get::<Dict<'_>>(xref.root_id()) else {
        return links;
    };

    let mut page_ids: HashMap<ObjectIdentifier, usize> = HashMap::new();
    for (index, page) in pdf.pages().iter().enumerate() {
        if let Some(id) = page.raw().obj_id() {
            page_ids.insert(id, index);
        }
    }

    let mut named_dests: HashMap<String, usize> = HashMap::new();
    if let Some(dests) = root.get::<Dict<'_>>("Dests") {
        for (name, _) in dests.entries() {
            let key = decode_pdf_string(name.as_ref());
            if let Some(array) = dests.get::<Array<'_>>(name.as_ref()) {
                if let Some(page) =
                    super::outline::resolve_dest_array(&array, &page_ids, page_count)
                {
                    named_dests.insert(key, page);
                }
            } else if let Some(dict) = dests.get::<Dict<'_>>(name.as_ref())
                && let Some(array) = dict.get::<Array<'_>>("D")
                && let Some(page) =
                    super::outline::resolve_dest_array(&array, &page_ids, page_count)
            {
                named_dests.insert(key, page);
            }
        }
    }

    for (page_index, page) in pdf.pages().iter().enumerate() {
        let Some(annots) = page.raw().get::<Array<'_>>("Annots") else {
            continue;
        };
        let (width, height) = page.render_dimensions();
        if width <= 0.0 || height <= 0.0 {
            continue;
        }
        let transform = page.initial_transform(true).as_coeffs();
        for object in annots.iter::<Object<'_>>() {
            let Object::Dict(dict) = object else {
                continue;
            };
            let Some(subtype) = dict.get::<Name<'_>>("Subtype") else {
                continue;
            };
            if !subtype.as_str().eq_ignore_ascii_case("Link") {
                continue;
            }
            let Some(rect) = dict.get::<Rect>("Rect") else {
                continue;
            };
            let Some(target_page) =
                super::outline::resolve_destination(&dict, &page_ids, &named_dests, page_count)
            else {
                continue;
            };
            let Some(bbox) = normalize_rect(rect, transform, width, height) else {
                continue;
            };
            links
                .entry(page_index)
                .or_default()
                .push(PageLink { bbox, target_page });
        }
    }

    links
}

fn normalize_rect(rect: Rect, transform: [f64; 6], width: f32, height: f32) -> Option<[f32; 4]> {
    let map = |x: f64, y: f64| {
        (
            transform[0] * x + transform[2] * y + transform[4],
            transform[1] * x + transform[3] * y + transform[5],
        )
    };
    let (x0, y0) = map(rect.x0, rect.y0);
    let (x1, y1) = map(rect.x1, rect.y1);
    let min_x = x0.min(x1) as f32 / width;
    let max_x = x0.max(x1) as f32 / width;
    let min_y = y0.min(y1) as f32 / height;
    let max_y = y0.max(y1) as f32 / height;
    let bbox = [
        min_x.clamp(0.0, 1.0),
        min_y.clamp(0.0, 1.0),
        max_x.clamp(0.0, 1.0),
        max_y.clamp(0.0, 1.0),
    ];
    if bbox[2] > bbox[0] && bbox[3] > bbox[1] {
        Some(bbox)
    } else {
        None
    }
}
