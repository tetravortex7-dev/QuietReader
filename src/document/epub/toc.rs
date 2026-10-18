use crate::document::TocItem;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::path::Path;

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

    if !anchor_part.is_empty() {
        if let Some(&ch) = anchor_map.get(anchor_part) {
            return ch;
        }
        let dec_anchor = percent_decode_str(anchor_part);
        if let Some(&ch) = anchor_map.get(&dec_anchor) {
            return ch;
        }
    }
    if file_part.is_empty() {
        return 0;
    }

    let resolved = resolve_path(opf_dir, file_part);
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
    0
}

pub fn parse_ncx_toc(
    ncx_xml: &str,
    opf_dir: &str,
    chapter_map: &HashMap<String, usize>,
    anchor_map: &HashMap<String, usize>,
) -> Vec<TocItem> {
    let sanitized = sanitize_xml_entities(ncx_xml);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = false;

    let mut root_nav_points: Vec<NavPoint> = Vec::new();
    let mut point_stack: Vec<NavPoint> = Vec::new();
    let mut in_text = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            ext @ (Ok(Event::Start(_)) | Ok(Event::Empty(_))) => {
                let e = match &ext {
                    Ok(Event::Start(e)) | Ok(Event::Empty(e)) => e,
                    _ => unreachable!(),
                };
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "navpoint" && point_stack.len() < 32 {
                    let mut id = None;
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("id") {
                            id = Some(attr.value.to_string());
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
                            last.content_src = attr.value.to_string();
                        }
                    }
                } else if tag == "text" {
                    in_text = true;
                }
            }
            Ok(Event::Text(ref e)) if in_text => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                let trimmed = text.trim();
                if !trimmed.is_empty()
                    && let Some(last) = point_stack.last_mut()
                    && last.label.is_empty()
                {
                    last.label = trimmed.to_string();
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "text" {
                    in_text = false;
                } else if tag == "navpoint"
                    && let Some(finished_point) = point_stack.pop()
                {
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

    fn convert_nav(
        nav: &NavPoint,
        opf_dir: &str,
        chapter_map: &HashMap<String, usize>,
        anchor_map: &HashMap<String, usize>,
        depth: usize,
    ) -> TocItem {
        let ch_index = resolve_toc_destination(&nav.content_src, opf_dir, chapter_map, anchor_map);
        let children = if depth < 32 {
            nav.children
                .iter()
                .map(|c| convert_nav(c, opf_dir, chapter_map, anchor_map, depth + 1))
                .collect()
        } else {
            Vec::new()
        };
        let title = if nav.label.is_empty() {
            "Без названия".to_string()
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
        .map(|nav| convert_nav(nav, opf_dir, chapter_map, anchor_map, 0))
        .collect()
}

pub fn parse_epub3_nav_toc(
    nav_html: &str,
    nav_dir: &str,
    chapter_map: &HashMap<String, usize>,
    anchor_map: &HashMap<String, usize>,
) -> Vec<TocItem> {
    let sanitized = sanitize_xml_entities(nav_html);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(true);
    reader.config_mut().check_end_names = false;

    let mut in_toc_nav = false;
    let mut current_href = String::new();
    let mut current_title = String::new();
    let mut in_anchor = false;
    let mut stack: Vec<Vec<TocItem>> = vec![Vec::new()];
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "nav" {
                    for attr in e.attributes().flatten() {
                        if attr.value.to_string().contains("toc") {
                            in_toc_nav = true;
                        }
                    }
                } else if in_toc_nav {
                    if (tag == "ol" || tag == "ul") && stack.len() < 32 {
                        stack.push(Vec::new());
                    } else if tag == "a" {
                        in_anchor = true;
                        current_title.clear();
                        current_href.clear();
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("href") {
                                current_href = attr.value.to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) if in_anchor => {
                let raw = e.as_ref();
                let text = quick_xml::escape::unescape(raw)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw.to_string());
                current_title.push_str(text.trim());
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "nav" {
                    in_toc_nav = false;
                } else if in_toc_nav {
                    if tag == "a" {
                        in_anchor = false;
                        if !current_href.is_empty() {
                            let ch = resolve_toc_destination(
                                &current_href,
                                nav_dir,
                                chapter_map,
                                anchor_map,
                            );
                            let title = if current_title.is_empty() {
                                "Раздел".to_string()
                            } else {
                                current_title.clone()
                            };
                            if let Some(cur_level) = stack.last_mut() {
                                cur_level.push(TocItem {
                                    id: None,
                                    title,
                                    chapter_index: ch,
                                    children: Vec::new(),
                                });
                            }
                        }
                    } else if (tag == "ol" || tag == "ul")
                        && stack.len() > 1
                        && let Some(children) = stack.pop()
                    {
                        if let Some(cur_level) = stack.last_mut()
                            && let Some(last_item) = cur_level.last_mut()
                        {
                            last_item.children = children;
                        }
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    stack.into_iter().next().unwrap_or_default()
}
