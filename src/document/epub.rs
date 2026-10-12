use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use zip::ZipArchive;
use crate::document::{BookChapter, BookMetadata, Document, TocItem};

pub struct EpubDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl Document for EpubDocument {
    fn metadata(&self) -> &BookMetadata {
        &self.metadata
    }

    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }

    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

#[derive(Default, Debug)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
}

#[derive(Debug)]
struct NavPoint {
    id: Option<String>,
    label: String,
    content_src: String,
    children: Vec<NavPoint>,
}

impl EpubDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| format!("Не удалось открыть файл EPUB: {e}"))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Неверный ZIP-архив EPUB: {e}"))?;
        Self::from_archive(&mut archive)
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| format!("Неверный ZIP-архив EPUB: {e}"))?;
        Self::from_archive(&mut archive)
    }

    fn read_zip_entry<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
        entry_name: &str,
    ) -> Result<String, String> {
        let normalized = entry_name.replace('\\', "/");
        let found_index = {
            let mut matched = None;
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name().replace('\\', "/");
                    if name == normalized {
                        matched = Some(i);
                        break;
                    } else if matched.is_none() && name.eq_ignore_ascii_case(&normalized) {
                        matched = Some(i);
                    }
                }
            }
            matched
        };

        let index = found_index
            .ok_or_else(|| format!("Файл '{entry_name}' не найден в архиве"))?;

        let mut file = archive
            .by_index(index)
            .map_err(|e| format!("Ошибка открытия '{entry_name}': {e}"))?;

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| format!("Ошибка чтения файла '{entry_name}': {e}"))?;

        match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(err) => {
                let bytes = err.into_bytes();
                let (cow, _, _) = encoding_rs::WINDOWS_1251.decode(&bytes);
                Ok(cow.into_owned())
            }
        }
    }

    fn find_opf_path<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<String, String> {
        let container_xml = match Self::read_zip_entry(archive, "META-INF/container.xml") {
            Ok(c) => c,
            Err(_) => {
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index(i) {
                        let name = file.name();
                        if name.ends_with(".opf") {
                            return Ok(name.to_string());
                        }
                    }
                }
                return Err("META-INF/container.xml не найден в EPUB архиве".to_string());
            }
        };

        let mut reader = Reader::from_str(&container_xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag == "rootfile" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "full-path" {
                                return Ok(attr.value.to_string());
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Ошибка парсинга container.xml: {e}")),
                _ => {}
            }
            buf.clear();
        }

        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                if name.ends_with(".opf") {
                    return Ok(name.to_string());
                }
            }
        }

        Err("Не найден rootfile (OPF) в container.xml".to_string())
    }

    fn from_archive<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<Self, String> {
        let opf_path = Self::find_opf_path(archive)?;
        let opf_dir = Path::new(&opf_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let opf_content = Self::read_zip_entry(archive, &opf_path)?;

        let (metadata, manifest, spine_idrefs, ncx_id) = Self::parse_opf(&opf_content)?;

        let manifest_map: HashMap<String, ManifestItem> = manifest
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect();

        let mut spine_files: Vec<(String, String)> = Vec::new();
        for idref in &spine_idrefs {
            if let Some(item) = manifest_map.get(idref) {
                let full_path = resolve_path(&opf_dir, &item.href);
                spine_files.push((idref.clone(), full_path));
            }
        }

        if spine_files.is_empty() {
            return Err("EPUB spine не содержит разделов для чтения".to_string());
        }

        let mut chapters = Vec::new();
        let mut chapter_path_to_index: HashMap<String, usize> = HashMap::new();

        for (ch_idx, (idref, path)) in spine_files.iter().enumerate() {
            let html_content = match Self::read_zip_entry(archive, path) {
                Ok(c) => c,
                Err(_) => {
                    let decoded_path = percent_decode_str(path);
                    Self::read_zip_entry(archive, &decoded_path).unwrap_or_default()
                }
            };

            let (parsed_title, paragraphs) = parse_html_chapter(&html_content);
            let final_title = if !parsed_title.is_empty() {
                parsed_title
            } else {
                format!("Раздел {}", ch_idx + 1)
            };

            chapter_path_to_index.insert(normalize_zip_path(path), ch_idx);
            if let Some(file_name) = Path::new(path).file_name().and_then(|n| n.to_str()) {
                chapter_path_to_index.entry(file_name.to_string()).or_insert(ch_idx);
            }

            chapters.push(BookChapter {
                id: Some(idref.clone()),
                title: final_title,
                paragraphs,
            });
        }

        let mut toc = Vec::new();
        if let Some(ncx_id) = ncx_id {
            if let Some(ncx_item) = manifest_map.get(&ncx_id) {
                let ncx_full_path = resolve_path(&opf_dir, &ncx_item.href);
                if let Ok(ncx_content) = Self::read_zip_entry(archive, &ncx_full_path) {
                    toc = parse_ncx_toc(&ncx_content, &opf_dir, &chapter_path_to_index);
                }
            }
        }

        if toc.is_empty() {
            for (idx, ch) in chapters.iter().enumerate() {
                toc.push(TocItem {
                    id: ch.id.clone(),
                    title: ch.title.clone(),
                    chapter_index: idx,
                    children: Vec::new(),
                });
            }
        }

        Ok(Self {
            metadata,
            chapters,
            toc,
        })
    }

    fn parse_opf(
        opf: &str,
    ) -> Result<(BookMetadata, Vec<ManifestItem>, Vec<String>, Option<String>), String> {
        let mut reader = Reader::from_str(opf);
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut author = String::new();
        let mut manifest = Vec::new();
        let mut spine_idrefs = Vec::new();
        let mut ncx_id = None;

        let mut buf = Vec::new();
        let mut current_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();

                    if tag_name == "spine" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "toc" {
                                ncx_id = Some(attr.value.to_string());
                            }
                        }
                    } else if tag_name == "item" {
                        let mut item = ManifestItem::default();
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "id" {
                                item.id = attr.value.to_string();
                            } else if key == "href" {
                                item.href = attr.value.to_string();
                            } else if key == "media-type" {
                                item.media_type = attr.value.to_string();
                            }
                        }
                        if !item.id.is_empty() {
                            if item.media_type == "application/x-dtbncx+xml" && ncx_id.is_none() {
                                ncx_id = Some(item.id.clone());
                            }
                            manifest.push(item);
                        }
                    } else if tag_name == "itemref" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "idref" {
                                spine_idrefs.push(attr.value.to_string());
                            }
                        }
                    }

                    current_tag = tag_name;
                }
                Ok(Event::Empty(ref e)) => {
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();

                    if tag_name == "item" {
                        let mut item = ManifestItem::default();
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "id" {
                                item.id = attr.value.to_string();
                            } else if key == "href" {
                                item.href = attr.value.to_string();
                            } else if key == "media-type" {
                                item.media_type = attr.value.to_string();
                            }
                        }
                        if !item.id.is_empty() {
                            if item.media_type == "application/x-dtbncx+xml" && ncx_id.is_none() {
                                ncx_id = Some(item.id.clone());
                            }
                            manifest.push(item);
                        }
                    } else if tag_name == "itemref" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "idref" {
                                spine_idrefs.push(attr.value.to_string());
                            }
                        }
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let raw_str = e.as_ref();
                    let text = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());

                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        match current_tag.as_str() {
                            "title" => {
                                if title.is_empty() {
                                    title = trimmed.to_string();
                                }
                            }
                            "creator" => {
                                if author.is_empty() {
                                    author = trimmed.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    current_tag.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Ошибка парсинга OPF XML: {e}")),
                _ => {}
            }
            buf.clear();
        }

        if title.is_empty() {
            title = "Без названия".to_string();
        }

        let metadata = BookMetadata { title, author };
        Ok((metadata, manifest, spine_idrefs, ncx_id))
    }
}

fn resolve_path(base_dir: &str, href: &str) -> String {
    let combined = if base_dir.is_empty() {
        href.to_string()
    } else {
        format!("{base_dir}/{href}")
    };

    let mut parts: Vec<&str> = Vec::new();
    for seg in combined.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            parts.pop();
        } else {
            parts.push(seg);
        }
    }
    parts.join("/")
}

fn normalize_zip_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

fn percent_decode_str(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte_val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                result.push(byte_val);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

fn parse_html_chapter(html: &str) -> (String, Vec<String>) {
    let mut reader = Reader::from_str(html);
    reader.config_mut().trim_text(true);

    let mut title = String::new();
    let mut paragraphs = Vec::new();
    let mut current_p = String::new();
    let mut in_title_tag = false;
    let mut in_heading_tag = false;
    let mut skip_content = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();

                match tag.as_str() {
                    "script" | "style" | "head" => {
                        skip_content = true;
                    }
                    "title" => {
                        skip_content = false;
                        in_title_tag = true;
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        if !current_p.trim().is_empty() {
                            paragraphs.push(current_p.trim().to_string());
                            current_p.clear();
                        }
                        in_heading_tag = true;
                    }
                    "p" | "div" | "blockquote" | "li" => {
                        if !current_p.trim().is_empty() {
                            paragraphs.push(current_p.trim().to_string());
                            current_p.clear();
                        }
                    }
                    "br" => {
                        current_p.push('\n');
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "br" {
                    current_p.push('\n');
                } else if tag == "hr" {
                    if !current_p.trim().is_empty() {
                        paragraphs.push(current_p.trim().to_string());
                        current_p.clear();
                    }
                    paragraphs.push("* * *".to_string());
                }
            }
            Ok(Event::Text(ref e)) => {
                if !skip_content {
                    let raw_str = e.as_ref();
                    let text = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());

                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        if in_title_tag && title.is_empty() {
                            title = trimmed.to_string();
                        } else {
                            if !current_p.is_empty() && !current_p.ends_with(' ') && !current_p.ends_with('\n') {
                                current_p.push(' ');
                            }
                            current_p.push_str(trimmed);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();

                match tag.as_str() {
                    "script" | "style" | "head" => {
                        skip_content = false;
                    }
                    "title" => {
                        in_title_tag = false;
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        if in_heading_tag && title.is_empty() && !current_p.trim().is_empty() {
                            title = current_p.trim().to_string();
                        }
                        if !current_p.trim().is_empty() {
                            paragraphs.push(current_p.trim().to_string());
                            current_p.clear();
                        }
                        in_heading_tag = false;
                    }
                    "p" | "div" | "blockquote" | "li" => {
                        if !current_p.trim().is_empty() {
                            paragraphs.push(current_p.trim().to_string());
                            current_p.clear();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if !current_p.trim().is_empty() {
        paragraphs.push(current_p.trim().to_string());
    }

    (title, paragraphs)
}

fn parse_ncx_toc(
    ncx_xml: &str,
    opf_dir: &str,
    chapter_map: &HashMap<String, usize>,
) -> Vec<TocItem> {
    let mut reader = Reader::from_str(ncx_xml);
    reader.config_mut().trim_text(true);

    let mut root_nav_points: Vec<NavPoint> = Vec::new();
    let mut point_stack: Vec<NavPoint> = Vec::new();
    let mut in_text = false;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();

                if tag == "navpoint" {
                    let mut id = None;
                    for attr in e.attributes().flatten() {
                        let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                        if key == "id" {
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
                        let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                        if key == "src" {
                            if let Some(last) = point_stack.last_mut() {
                                last.content_src = attr.value.to_string();
                            }
                        }
                    }
                } else if tag == "text" {
                    in_text = true;
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "content" {
                    for attr in e.attributes().flatten() {
                        let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                        if key == "src" {
                            if let Some(last) = point_stack.last_mut() {
                                last.content_src = attr.value.to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    let raw_str = e.as_ref();
                    let text = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());

                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        if let Some(last) = point_stack.last_mut() {
                            if last.label.is_empty() {
                                last.label = trimmed.to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();

                if tag == "text" {
                    in_text = false;
                } else if tag == "navpoint" {
                    if let Some(finished_point) = point_stack.pop() {
                        if let Some(parent) = point_stack.last_mut() {
                            parent.children.push(finished_point);
                        } else {
                            root_nav_points.push(finished_point);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    fn convert_nav(
        nav: &NavPoint,
        opf_dir: &str,
        chapter_map: &HashMap<String, usize>,
    ) -> TocItem {
        let clean_src = nav.content_src.split('#').next().unwrap_or(&nav.content_src);
        let resolved = resolve_path(opf_dir, clean_src);
        let normalized = normalize_zip_path(&resolved);

        let ch_index = chapter_map
            .get(&normalized)
            .or_else(|| {
                Path::new(&normalized)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|file_name| chapter_map.get(file_name))
            })
            .copied()
            .unwrap_or(0);

        let children = nav
            .children
            .iter()
            .map(|c| convert_nav(c, opf_dir, chapter_map))
            .collect();

        TocItem {
            id: nav.id.clone(),
            title: if nav.label.is_empty() { "Без названия".to_string() } else { nav.label.clone() },
            chapter_index: ch_index,
            children,
        }
    }

    root_nav_points
        .iter()
        .map(|nav| convert_nav(nav, opf_dir, chapter_map))
        .collect()
}

fn local_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
}
