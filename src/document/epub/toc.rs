use crate::document::TocItem;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::path::Path;

use super::entities::{decode_attr_value, decode_xml_ref};
use super::html::sanitize_xml_entities;
use super::{local_name, normalize_zip_path, percent_decode_str, resolve_path};

#[derive(Debug)]
pub struct NavPoint {
    pub id: Option<String>,
    pub label: String,
    pub content_src: String,
    pub children: Vec<NavPoint>,
}

pub fn resolve_toc_destination(
    src: &str,
    opf_dir: &str,
    chapter_map: &HashMap<String, usize>,
    anchor_map: &HashMap<String, usize>,
) -> usize {
    let clean = src.trim();
    if clean.is_empty() {
        return 0;
    }

    let (file_part, anchor_part) = if let Some(pos) = clean.find('#') {
        (&clean[..pos], &clean[pos + 1..])
    } else {
        (clean, "")
    };

    if !file_part.is_empty() {
        let resolved = resolve_path(opf_dir, file_part);
        if !resolved.is_empty() {
            let norm = normalize_zip_path(&resolved);
            let dec = percent_decode_str(&norm);

            if let Some(&ch) = chapter_map.get(&norm) {
                return ch;
            }
            if let Some(&ch) = chapter_map.get(&dec) {
                return ch;
            }
            if let Some(fname) = Path::new(&norm).file_name().and_then(|n| n.to_str())
                && let Some(&ch) = chapter_map.get(fname)
            {
                return ch;
            }
            if let Some(fname) = Path::new(&dec).file_name().and_then(|n| n.to_str())
                && let Some(&ch) = chapter_map.get(fname)
            {
                return ch;
            }
        }
    }

    if !anchor_part.is_empty() {
        if let Some(&ch) = anchor_map.get(anchor_part) {
            return ch;
        }
        let dec_anchor = percent_decode_str(anchor_part);
        if let Some(&ch) = anchor_map.get(&dec_anchor) {
            return ch;
        }
    }
    0
}

pub fn parse_ncx_toc(
    ncx_xml: &str,
    toc_dir: &str,
    chapter_map: &HashMap<String, usize>,
    anchor_map: &HashMap<String, usize>,
) -> Vec<TocItem> {
    let sanitized = sanitize_xml_entities(ncx_xml);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;

    let mut root_nav_points: Vec<NavPoint> = Vec::new();
    let mut point_stack: Vec<NavPoint> = Vec::new();
    let mut in_text = false;
    let mut label_buf = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            ext @ (Ok(Event::Start(_)) | Ok(Event::Empty(_))) => {
                let (e, is_start) = match &ext {
                    Ok(Event::Start(e)) => (e, true),
                    Ok(Event::Empty(e)) => (e, false),
                    _ => unreachable!(),
                };
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if is_start && tag == "navpoint" && point_stack.len() < 32 {
                    in_text = false;
                    label_buf.clear();
                    let mut id = None;
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("id") {
                            id = Some(decode_attr_value(&attr));
                        }
                    }
                    point_stack.push(NavPoint {
                        id,
                        label: String::new(),
                        content_src: String::new(),
                        children: Vec::new(),
                    });
                } else if tag == "content" {
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("src")
                            && let Some(last) = point_stack.last_mut()
                        {
                            last.content_src = decode_attr_value(&attr);
                        }
                    }
                } else if is_start && tag == "text" && !point_stack.is_empty() {
                    in_text = true;
                    label_buf.clear();
                }
            }
            Ok(Event::Text(ref e)) if in_text => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                label_buf.push_str(&text);
            }
            Ok(Event::CData(ref e)) if in_text => {
                label_buf.push_str(e.as_ref());
            }
            Ok(Event::GeneralRef(ref e)) if in_text => {
                if let Some(decoded) = decode_xml_ref(e.as_ref()) {
                    label_buf.push_str(&decoded);
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "text" {
                    if in_text {
                        let collapsed = label_buf.split_whitespace().collect::<Vec<_>>().join(" ");
                        if !collapsed.is_empty()
                            && let Some(last) = point_stack.last_mut()
                            && last.label.is_empty()
                        {
                            last.label = collapsed;
                        }
                        label_buf.clear();
                    }
                    in_text = false;
                } else if tag == "navpoint"
                    && let Some(finished_point) = point_stack.pop()
                {
                    in_text = false;
                    label_buf.clear();
                    if let Some(parent) = point_stack.last_mut() {
                        parent.children.push(finished_point);
                    } else {
                        root_nav_points.push(finished_point);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    while let Some(finished_point) = point_stack.pop() {
        if let Some(parent) = point_stack.last_mut() {
            parent.children.push(finished_point);
        } else {
            root_nav_points.push(finished_point);
        }
    }

    fn convert_nav(
        nav: &NavPoint,
        toc_dir: &str,
        chapter_map: &HashMap<String, usize>,
        anchor_map: &HashMap<String, usize>,
        depth: usize,
    ) -> TocItem {
        let ch_index = resolve_toc_destination(&nav.content_src, toc_dir, chapter_map, anchor_map);
        let children = if depth < 32 {
            nav.children
                .iter()
                .map(|c| convert_nav(c, toc_dir, chapter_map, anchor_map, depth + 1))
                .collect()
        } else {
            Vec::new()
        };
        let title = if nav.label.is_empty() {
            crate::document::errors::tokens::UNTITLED.to_string()
        } else {
            nav.label.clone()
        };
        TocItem {
            id: nav.id.clone(),
            title,
            chapter_index: ch_index,
            children,
        }
    }

    root_nav_points
        .iter()
        .map(|nav| convert_nav(nav, toc_dir, chapter_map, anchor_map, 0))
        .collect()
}

fn is_toc_nav_element(e: &quick_xml::events::BytesStart<'_>) -> bool {
    e.attributes().flatten().any(|attr| {
        let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
        if !matches!(key.as_str(), "type" | "role" | "class" | "id") {
            return false;
        }
        let val = decode_attr_value(&attr).to_ascii_lowercase();
        val.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|token| token == "toc")
    })
}

fn close_toc_list(
    stack: &mut Vec<Vec<TocItem>>,
    root_items: &mut Vec<TocItem>,
    li_has_anchor: &[bool],
    seen_li_in_toc: bool,
) {
    let Some(children) = stack.pop() else {
        return;
    };
    if let Some(parent_level) = stack.last_mut() {
        let attach_to_last = if seen_li_in_toc {
            li_has_anchor.last().copied() == Some(true)
        } else {
            true
        };
        if attach_to_last && let Some(last_item) = parent_level.last_mut() {
            last_item.children.extend(children);
        } else {
            parent_level.extend(children);
        }
    } else {
        root_items.extend(children);
    }
}

pub fn parse_epub3_nav_toc(
    nav_html: &str,
    nav_dir: &str,
    chapter_map: &HashMap<String, usize>,
    anchor_map: &HashMap<String, usize>,
) -> Vec<TocItem> {
    let sanitized = sanitize_xml_entities(nav_html);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;

    let mut nav_depth: usize = 0;
    let mut toc_nav_depth: Option<usize> = None;
    let mut current_href = String::new();
    let mut current_title = String::new();
    let mut in_anchor = false;
    let mut root_items: Vec<TocItem> = Vec::new();
    let mut stack: Vec<Vec<TocItem>> = Vec::new();
    let mut li_has_anchor: Vec<bool> = Vec::new();
    let mut seen_li_in_toc = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "nav" {
                    nav_depth = nav_depth.saturating_add(1);
                    if toc_nav_depth.is_none() && root_items.is_empty() && is_toc_nav_element(e) {
                        toc_nav_depth = Some(nav_depth);
                    }
                } else if toc_nav_depth.is_some() {
                    if (tag == "ol" || tag == "ul") && stack.len() < 32 {
                        stack.push(Vec::new());
                    } else if tag == "li" {
                        seen_li_in_toc = true;
                        li_has_anchor.push(false);
                    } else if tag == "a" {
                        in_anchor = true;
                        current_title.clear();
                        current_href.clear();
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("href") {
                                current_href = decode_attr_value(&attr);
                            }
                        }
                    } else if tag == "br" && in_anchor {
                        current_title.push(' ');
                    }
                }
            }
            Ok(Event::Empty(ref e)) if toc_nav_depth.is_some() && in_anchor => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "br" {
                    current_title.push(' ');
                }
            }
            Ok(Event::Text(ref e)) if toc_nav_depth.is_some() && in_anchor => {
                let raw = e.as_ref();
                let text = quick_xml::escape::unescape(raw)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                current_title.push_str(&text);
            }
            Ok(Event::CData(ref e)) if toc_nav_depth.is_some() && in_anchor => {
                current_title.push_str(e.as_ref());
            }
            Ok(Event::GeneralRef(ref e)) if toc_nav_depth.is_some() && in_anchor => {
                if let Some(decoded) = decode_xml_ref(e.as_ref()) {
                    current_title.push_str(&decoded);
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "nav" {
                    if toc_nav_depth == Some(nav_depth) {
                        while !stack.is_empty() {
                            close_toc_list(
                                &mut stack,
                                &mut root_items,
                                &li_has_anchor,
                                seen_li_in_toc,
                            );
                        }
                        toc_nav_depth = None;
                        in_anchor = false;
                        li_has_anchor.clear();
                    }
                    nav_depth = nav_depth.saturating_sub(1);
                } else if toc_nav_depth.is_some() {
                    if tag == "a" {
                        in_anchor = false;
                        if !current_href.is_empty() {
                            let ch = resolve_toc_destination(
                                &current_href,
                                nav_dir,
                                chapter_map,
                                anchor_map,
                            );
                            let collapsed = current_title
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ");
                            let title = if collapsed.is_empty() {
                                crate::document::errors::tokens::SECTION_PLAIN.to_string()
                            } else {
                                collapsed
                            };
                            let item = TocItem {
                                id: None,
                                title,
                                chapter_index: ch,
                                children: Vec::new(),
                            };
                            if let Some(cur_level) = stack.last_mut() {
                                cur_level.push(item);
                            } else {
                                root_items.push(item);
                            }
                            if let Some(last) = li_has_anchor.last_mut() {
                                *last = true;
                            }
                        }
                        current_title.clear();
                        current_href.clear();
                    } else if tag == "li" {
                        li_has_anchor.pop();
                    } else if tag == "ol" || tag == "ul" {
                        close_toc_list(&mut stack, &mut root_items, &li_has_anchor, seen_li_in_toc);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    while !stack.is_empty() {
        close_toc_list(&mut stack, &mut root_items, &li_has_anchor, seen_li_in_toc);
    }
    root_items
}
