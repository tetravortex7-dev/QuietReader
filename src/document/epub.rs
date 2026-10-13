use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

pub struct EpubDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for EpubDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for EpubDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
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

    fn format_name(&self) -> &'static str {
        "EPUB"
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn text_content(&self) -> Option<&dyn TextContent> {
        Some(self)
    }

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        Some(self)
    }
}

#[derive(Default, Debug)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
    properties: String,
}

#[derive(Debug)]
struct NavPoint {
    id: Option<String>,
    label: String,
    content_src: String,
    children: Vec<NavPoint>,
}

impl EpubDocument {
    pub const MAX_EPUB_SIZE: u64 = 200 * 1024 * 1024;
    pub const MAX_ENTRIES: usize = 10_000;
    pub const MAX_ENTRY_BYTES: u64 = 50 * 1024 * 1024;

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let meta = std::fs::metadata(path_ref).map_err(|e| {
            DocumentError::Io(format!("Не удалось получить метаданные EPUB файла: {e}"))
        })?;
        if meta.len() > Self::MAX_EPUB_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер файла EPUB превышает допустимый лимит (200 МБ)".to_string(),
            ));
        }

        let file = std::fs::File::open(path_ref)
            .map_err(|e| DocumentError::Io(format!("Не удалось открыть файл EPUB: {e}")))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| DocumentError::Corrupted(format!("Неверный ZIP-архив EPUB: {e}")))?;
        if archive.len() > Self::MAX_ENTRIES {
            return Err(DocumentError::LimitExceeded(
                "Архив EPUB содержит слишком много файлов (превышен лимит 10 000)".to_string(),
            ));
        }
        Self::from_archive(&mut archive)
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > Self::MAX_EPUB_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер данных EPUB превышает допустимый лимит (200 МБ)".to_string(),
            ));
        }
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| DocumentError::Corrupted(format!("Неверный ZIP-архив EPUB: {e}")))?;
        if archive.len() > Self::MAX_ENTRIES {
            return Err(DocumentError::LimitExceeded(
                "Архив EPUB содержит слишком много файлов (превышен лимит 10 000)".to_string(),
            ));
        }
        Self::from_archive(&mut archive)
    }

    fn read_zip_entry<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
        entry_name: &str,
        running_unpacked: &mut u64,
    ) -> Result<String, DocumentError> {
        let normalized = entry_name.replace('\\', "/");
        if !crate::document::is_safe_archive_path(&normalized) {
            return Err(DocumentError::Corrupted(format!("Небезопасный путь в EPUB: '{entry_name}'")));
        }
        let decoded = percent_decode_str(&normalized);
        let mut matched = None;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().replace('\\', "/");
                if name == normalized || name == decoded {
                    matched = Some(i);
                    break;
                } else if matched.is_none() && (name.eq_ignore_ascii_case(&normalized) || name.eq_ignore_ascii_case(&decoded)) {
                    matched = Some(i);
                }
            }
        }

        let index = matched.ok_or_else(|| DocumentError::Corrupted(format!("Файл '{entry_name}' не найден")))?;
        let file = archive.by_index(index).map_err(|e| DocumentError::Corrupted(format!("Ошибка '{entry_name}': {e}")))?;
        if file.size() > Self::MAX_ENTRY_BYTES {
            return Err(DocumentError::LimitExceeded(format!("Файл '{entry_name}' превышает лимит")));
        }

        let mut bytes = Vec::new();
        file.take(Self::MAX_ENTRY_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Ошибка чтения '{entry_name}': {e}")))?;

        if bytes.len() as u64 > Self::MAX_ENTRY_BYTES {
            return Err(DocumentError::LimitExceeded(format!("Файл '{entry_name}' превышает лимит")));
        }

        *running_unpacked = running_unpacked.saturating_add(bytes.len() as u64);
        if *running_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded("Превышен общий лимит распаковки EPUB (500 МБ)".to_string()));
        }

        match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(err) => {
                let raw = err.into_bytes();
                let (cow, _, _) = encoding_rs::WINDOWS_1251.decode(&raw);
                Ok(cow.into_owned())
            }
        }
    }

    fn find_opf_path<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
        running_unpacked: &mut u64,
    ) -> Result<String, DocumentError> {
        let container_xml = match Self::read_zip_entry(archive, "META-INF/container.xml", running_unpacked) {
            Ok(c) => c,
            Err(_) => {
                for i in 0..archive.len() {
                    if let Ok(f) = archive.by_index(i) && f.name().ends_with(".opf") {
                        return Ok(f.name().to_string());
                    }
                }
                return Err(DocumentError::Corrupted("META-INF/container.xml не найден".to_string()));
            }
        };

        let mut reader = Reader::from_str(&container_xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if local_name(e.name().as_ref()).eq_ignore_ascii_case("rootfile") {
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("full-path") {
                                return Ok(attr.value.to_string());
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(DocumentError::Corrupted(format!("Ошибка container.xml: {e}"))),
                _ => {}
            }
            buf.clear();
        }

        for i in 0..archive.len() {
            if let Ok(f) = archive.by_index(i) && f.name().ends_with(".opf") {
                return Ok(f.name().to_string());
            }
        }
        Err(DocumentError::Corrupted("Не найден rootfile (OPF)".to_string()))
    }

    fn from_archive<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
    ) -> Result<Self, DocumentError> {
        let mut total_unpacked: u64 = 0;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index_raw(i) {
                if !crate::document::is_safe_archive_path(entry.name()) {
                    return Err(DocumentError::Corrupted(format!("Небезопасный путь: {}", entry.name())));
                }
                total_unpacked = total_unpacked.saturating_add(entry.size());
                if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
                    return Err(DocumentError::LimitExceeded("Превышен общий лимит архива EPUB".to_string()));
                }
            }
        }

        let mut running_unpacked: u64 = 0;
        let opf_path = Self::find_opf_path(archive, &mut running_unpacked)?;
        let opf_dir = Path::new(&opf_path).parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        let opf_content = Self::read_zip_entry(archive, &opf_path, &mut running_unpacked)?;
        let (metadata, manifest, spine_idrefs, ncx_id, nav_id) = Self::parse_opf(&opf_content)?;

        let manifest_map: HashMap<String, ManifestItem> = manifest.into_iter().map(|it| (it.id.clone(), it)).collect();
        let mut spine_files: Vec<(String, String)> = Vec::new();
        for idref in &spine_idrefs {
            if let Some(item) = manifest_map.get(idref) {
                spine_files.push((idref.clone(), resolve_path(&opf_dir, &item.href)));
            }
        }
        if spine_files.is_empty() {
            return Err(DocumentError::Corrupted("EPUB spine не содержит разделов".to_string()));
        }

        let mut chapters = Vec::new();
        let mut chapter_path_to_index: HashMap<String, usize> = HashMap::new();
        let mut anchor_to_chapter: HashMap<String, usize> = HashMap::new();

        for (ch_idx, (idref, path)) in spine_files.iter().enumerate() {
            let html_content = Self::read_zip_entry(archive, path, &mut running_unpacked)
                .or_else(|_| Self::read_zip_entry(archive, &percent_decode_str(path), &mut running_unpacked))
                .unwrap_or_default();

            let (parsed_title, paragraphs, anchors) = parse_html_chapter(&html_content);
            let final_title = if !parsed_title.is_empty() { parsed_title } else { format!("Раздел {}", ch_idx + 1) };

            let norm_path = normalize_zip_path(path);
            let dec_path = percent_decode_str(&norm_path);
            chapter_path_to_index.insert(norm_path.clone(), ch_idx);
            chapter_path_to_index.insert(dec_path.clone(), ch_idx);

            if let Some(fname) = Path::new(&norm_path).file_name().and_then(|n| n.to_str()) {
                chapter_path_to_index.entry(fname.to_string()).or_insert(ch_idx);
            }
            if let Some(fname) = Path::new(&dec_path).file_name().and_then(|n| n.to_str()) {
                chapter_path_to_index.entry(fname.to_string()).or_insert(ch_idx);
            }

            for anchor in anchors {
                let dec_anchor = percent_decode_str(&anchor);
                anchor_to_chapter.entry(anchor).or_insert(ch_idx);
                anchor_to_chapter.entry(dec_anchor).or_insert(ch_idx);
            }

            chapters.push(BookChapter {
                id: Some(idref.clone()),
                title: final_title,
                paragraphs,
            });
        }

        let mut toc = Vec::new();
        if let Some(ncx_id) = ncx_id && let Some(ncx_item) = manifest_map.get(&ncx_id) {
            let ncx_full_path = resolve_path(&opf_dir, &ncx_item.href);
            if let Ok(ncx_content) = Self::read_zip_entry(archive, &ncx_full_path, &mut running_unpacked) {
                toc = parse_ncx_toc(&ncx_content, &opf_dir, &chapter_path_to_index, &anchor_to_chapter);
            }
        }

        if toc.is_empty() && let Some(nav_id) = nav_id && let Some(nav_item) = manifest_map.get(&nav_id) {
            let nav_full_path = resolve_path(&opf_dir, &nav_item.href);
            if let Ok(nav_content) = Self::read_zip_entry(archive, &nav_full_path, &mut running_unpacked) {
                let nav_dir = Path::new(&nav_full_path).parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
                toc = parse_epub3_nav_toc(&nav_content, &nav_dir, &chapter_path_to_index, &anchor_to_chapter);
            }
        }

        if toc.is_empty() {
            toc = chapters.iter().enumerate().map(|(idx, ch)| TocItem {
                id: ch.id.clone(),
                title: ch.title.clone(),
                chapter_index: idx,
                children: Vec::new(),
            }).collect();
        }

        Ok(Self { metadata, chapters, toc })
    }

    fn parse_opf(
        opf: &str,
    ) -> Result<(BookMetadata, Vec<ManifestItem>, Vec<String>, Option<String>, Option<String>), DocumentError> {
        let mut reader = Reader::from_str(opf);
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut author = String::new();
        let mut manifest = Vec::new();
        let mut spine_idrefs = Vec::new();
        let mut ncx_id = None;
        let mut nav_id = None;
        let mut buf = Vec::new();
        let mut current_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                ext @ (Ok(Event::Start(_)) | Ok(Event::Empty(_))) => {
                    let (e, is_start) = match &ext {
                        Ok(Event::Start(e)) => (e, true),
                        Ok(Event::Empty(e)) => (e, false),
                        _ => unreachable!(),
                    };
                    let tag_name = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag_name == "spine" {
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("toc") {
                                ncx_id = Some(attr.value.to_string());
                            }
                        }
                    } else if tag_name == "item" {
                        let mut item = ManifestItem::default();
                        for attr in e.attributes().flatten() {
                            match local_name(attr.key.as_ref()).to_ascii_lowercase().as_str() {
                                "id" => item.id = attr.value.to_string(),
                                "href" => item.href = attr.value.to_string(),
                                "media-type" => item.media_type = attr.value.to_string(),
                                "properties" => item.properties = attr.value.to_string(),
                                _ => {}
                            }
                        }
                        if !item.id.is_empty() {
                            if item.media_type == "application/x-dtbncx+xml" && ncx_id.is_none() {
                                ncx_id = Some(item.id.clone());
                            }
                            if item.properties.split_whitespace().any(|p| p == "nav") && nav_id.is_none() {
                                nav_id = Some(item.id.clone());
                            }
                            manifest.push(item);
                        }
                    } else if tag_name == "itemref" {
                        for attr in e.attributes().flatten() {
                            if local_name(attr.key.as_ref()).eq_ignore_ascii_case("idref") {
                                spine_idrefs.push(attr.value.to_string());
                            }
                        }
                    }
                    if is_start {
                        current_tag = tag_name;
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let raw_str = e.as_ref();
                    let text = quick_xml::escape::unescape(raw_str).map(|c| c.into_owned()).unwrap_or_else(|_| raw_str.to_string());
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        match current_tag.as_str() {
                            "title" if title.is_empty() => title = trimmed.to_string(),
                            "creator" if author.is_empty() => author = trimmed.to_string(),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(_)) => current_tag.clear(),
                Ok(Event::Eof) => break,
                Err(e) => return Err(DocumentError::Corrupted(format!("Ошибка парсинга OPF: {e}"))),
                _ => {}
            }
            buf.clear();
        }

        if title.is_empty() {
            title = "Без названия".to_string();
        }
        let metadata = BookMetadata { title, author };
        Ok((metadata, manifest, spine_idrefs, ncx_id, nav_id))
    }
}

pub fn resolve_path(base_dir: &str, href: &str) -> String {
    let clean_href = href.split('#').next().unwrap_or(href);
    if clean_href.contains(':') || clean_href.starts_with('/') || clean_href.starts_with('\\') {
        return clean_href.to_string();
    }
    let combined = if base_dir.is_empty() { clean_href.to_string() } else { format!("{base_dir}/{clean_href}") };
    let mut parts: Vec<&str> = Vec::new();
    for seg in combined.split('/') {
        if seg.is_empty() || seg == "." { continue; }
        if seg == ".." {
            if parts.is_empty() { return String::new(); }
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

pub fn percent_decode_str(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() && let Ok(b_val) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
            result.push(b_val);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub fn sanitize_xml_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    while let Some((idx, c)) = chars.next() {
        if c == '&' {
            let rest = &input[idx + 1..];
            if let Some(semi_pos) = rest.find(';') && semi_pos <= 10 {
                let entity_name = &rest[..semi_pos];
                if matches!(entity_name, "amp" | "lt" | "gt" | "quot" | "apos") {
                    out.push('&');
                    out.push_str(entity_name);
                    out.push(';');
                    for _ in 0..=semi_pos { chars.next(); }
                    continue;
                }
                if entity_name.starts_with('#')
                    && (entity_name[1..].chars().all(|ch| ch.is_ascii_digit())
                        || (entity_name.starts_with("#x") || entity_name.starts_with("#X")) && entity_name[2..].chars().all(|ch| ch.is_ascii_hexdigit()))
                {
                    out.push('&');
                    out.push_str(entity_name);
                    out.push(';');
                    for _ in 0..=semi_pos { chars.next(); }
                    continue;
                }
                let repl = match entity_name {
                    "nbsp" => " ", "mdash" => "—", "ndash" => "–", "hellip" => "…",
                    "laquo" => "«", "raquo" => "»", "ldquo" => "“", "rdquo" => "”",
                    "lsquo" => "‘", "rsquo" => "’", "copy" => "©", "trade" => "™",
                    "reg" => "®", "bull" => "•", "times" => "×", "divide" => "÷",
                    "minus" => "−", "deg" => "°", "euro" => "€", "pound" => "£",
                    "yen" => "¥", "cent" => "¢", "sect" => "§", "para" => "¶",
                    _ => "",
                };
                if !repl.is_empty() {
                    out.push_str(repl);
                    for _ in 0..=semi_pos { chars.next(); }
                    continue;
                }
            }
            out.push_str("&amp;");
        } else {
            out.push(c);
        }
    }
    out
}

fn flush_p(current_p: &mut String, paragraphs: &mut Vec<String>) {
    if !current_p.trim().is_empty() {
        paragraphs.push(current_p.trim().to_string());
        current_p.clear();
    }
}

pub fn extract_html_lenient(html: &str) -> (String, Vec<String>) {
    let mut title = String::new();
    let mut paragraphs = Vec::new();
    let mut current_p = String::new();
    let mut in_title = false;
    let mut skip = false;
    let mut i = 0;
    let bytes = html.as_bytes();

    while i < bytes.len() {
        if bytes[i] == b'<' {
            let rest = &html[i + 1..];
            if let Some(end_bracket) = rest.find('>') {
                let tag_raw = rest[..end_bracket].trim();
                let is_closing = tag_raw.starts_with('/');
                let tag_name = tag_raw.split_whitespace().next().unwrap_or("").trim_start_matches('/').to_ascii_lowercase();

                match tag_name.as_str() {
                    "script" | "style" | "head" => skip = !is_closing,
                    "title" => { in_title = !is_closing; skip = false; }
                    "p" | "div" | "blockquote" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => flush_p(&mut current_p, &mut paragraphs),
                    "br" => current_p.push('\n'),
                    "hr" => { flush_p(&mut current_p, &mut paragraphs); paragraphs.push("* * *".to_string()); }
                    _ => {}
                }
                i += 1 + end_bracket + 1;
                continue;
            }
        }

        if !skip {
            let start = i;
            while i < bytes.len() && bytes[i] != b'<' { i += 1; }
            if let Ok(text_slice) = std::str::from_utf8(&bytes[start..i]) {
                let unescaped = quick_xml::escape::unescape(text_slice).map(|c| c.into_owned()).unwrap_or_else(|_| text_slice.to_string());
                let trimmed = unescaped.trim();
                if !trimmed.is_empty() {
                    if in_title && title.is_empty() {
                        title = trimmed.to_string();
                    } else {
                        if !current_p.is_empty() && !current_p.ends_with(' ') && !current_p.ends_with('\n') {
                            current_p.push(' ');
                        }
                        current_p.push_str(trimmed);
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    flush_p(&mut current_p, &mut paragraphs);
    (title, paragraphs)
}

pub fn parse_html_chapter(html: &str) -> (String, Vec<String>, Vec<String>) {
    let sanitized = sanitize_xml_entities(html);
    let mut reader = Reader::from_str(&sanitized);
    reader.config_mut().trim_text(false);
    reader.config_mut().check_end_names = false;

    let mut title = String::new();
    let mut paragraphs = Vec::new();
    let mut anchors = Vec::new();
    let mut current_p = String::new();
    let (mut in_title_tag, mut in_heading_tag, mut skip_content) = (false, false, false);
    let mut buf = Vec::new();
    let mut had_error = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                for attr in e.attributes().flatten() {
                    let k = local_name(attr.key.as_ref()).to_ascii_lowercase();
                    if k == "id" || k == "name" { anchors.push(attr.value.to_string()); }
                }
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "script" | "style" | "head" => skip_content = true,
                    "title" => { skip_content = false; in_title_tag = true; }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => { flush_p(&mut current_p, &mut paragraphs); in_heading_tag = true; }
                    "p" | "div" | "blockquote" | "li" => flush_p(&mut current_p, &mut paragraphs),
                    "br" => current_p.push('\n'),
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                for attr in e.attributes().flatten() {
                    let k = local_name(attr.key.as_ref()).to_ascii_lowercase();
                    if k == "id" || k == "name" { anchors.push(attr.value.to_string()); }
                }
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "br" => current_p.push('\n'),
                    "hr" => { flush_p(&mut current_p, &mut paragraphs); paragraphs.push("* * *".to_string()); }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if !skip_content => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str).map(|c| c.into_owned()).unwrap_or_else(|_| raw_str.to_string());
                if !text.is_empty() {
                    if in_title_tag && title.is_empty() {
                        title.push_str(text.trim());
                    } else {
                        current_p.push_str(&text);
                    }
                }
            }
            Ok(Event::Text(ref e)) if !skip_content => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str).map(|c| c.into_owned()).unwrap_or_else(|_| raw_str.to_string());
                if !text.is_empty() {
                    if in_title_tag && title.is_empty() {
                        title.push_str(text.trim());
                    } else {
                        current_p.push_str(&text);
                    }
                }
            }
            Ok(Event::GeneralRef(ref e)) if !skip_content => {
                let repl = match e.as_ref() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "apos" => "'",
                    _ => "",
                };
                if !repl.is_empty() {
                    current_p.push_str(repl);
                }
            }
            Ok(Event::End(ref e)) => {
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "script" | "style" | "head" => skip_content = false,
                    "title" => in_title_tag = false,
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        if in_heading_tag && title.is_empty() && !current_p.trim().is_empty() {
                            title = current_p.trim().to_string();
                        }
                        flush_p(&mut current_p, &mut paragraphs);
                        in_heading_tag = false;
                    }
                    "p" | "div" | "blockquote" | "li" => flush_p(&mut current_p, &mut paragraphs),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => { had_error = true; break; }
            _ => {}
        }
        buf.clear();
    }
    flush_p(&mut current_p, &mut paragraphs);

    if had_error || paragraphs.is_empty() {
        let (fb_title, fb_paras) = extract_html_lenient(html);
        if !fb_paras.is_empty() {
            if paragraphs.is_empty() || paragraphs.len() < fb_paras.len() / 2 {
                paragraphs = fb_paras;
            }
            if title.is_empty() { title = fb_title; }
        }
    }
    (title, paragraphs, anchors)
}

fn resolve_toc_destination(
    src: &str,
    opf_dir: &str,
    chapter_map: &HashMap<String, usize>,
    anchor_map: &HashMap<String, usize>,
) -> usize {
    let clean = src.trim();
    if clean.is_empty() { return 0; }

    let (file_part, anchor_part) = if let Some(pos) = clean.find('#') { (&clean[..pos], &clean[pos + 1..]) } else { (clean, "") };

    if !anchor_part.is_empty() {
        if let Some(&ch) = anchor_map.get(anchor_part) { return ch; }
        let dec_anchor = percent_decode_str(anchor_part);
        if let Some(&ch) = anchor_map.get(&dec_anchor) { return ch; }
    }
    if file_part.is_empty() { return 0; }

    let resolved = resolve_path(opf_dir, file_part);
    let norm = normalize_zip_path(&resolved);
    let dec = percent_decode_str(&norm);

    if let Some(&ch) = chapter_map.get(&norm) { return ch; }
    if let Some(&ch) = chapter_map.get(&dec) { return ch; }
    if let Some(fname) = Path::new(&norm).file_name().and_then(|n| n.to_str()) && let Some(&ch) = chapter_map.get(fname) { return ch; }
    if let Some(fname) = Path::new(&dec).file_name().and_then(|n| n.to_str()) && let Some(&ch) = chapter_map.get(fname) { return ch; }
    0
}

fn parse_ncx_toc(
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
                let e = match &ext { Ok(Event::Start(e)) | Ok(Event::Empty(e)) => e, _ => unreachable!() };
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "navpoint" && point_stack.len() < 32 {
                    let mut id = None;
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("id") { id = Some(attr.value.to_string()); }
                    }
                    point_stack.push(NavPoint { id, label: String::new(), content_src: String::new(), children: Vec::new() });
                } else if tag == "content" {
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()).eq_ignore_ascii_case("src") && let Some(last) = point_stack.last_mut() {
                            last.content_src = attr.value.to_string();
                        }
                    }
                } else if tag == "text" {
                    in_text = true;
                }
            }
            Ok(Event::Text(ref e)) if in_text => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str).map(|c| c.into_owned()).unwrap_or_else(|_| raw_str.to_string());
                let trimmed = text.trim();
                if !trimmed.is_empty() && let Some(last) = point_stack.last_mut() && last.label.is_empty() {
                    last.label = trimmed.to_string();
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "text" {
                    in_text = false;
                } else if tag == "navpoint" && let Some(finished_point) = point_stack.pop() {
                    if let Some(parent) = point_stack.last_mut() { parent.children.push(finished_point); } else { root_nav_points.push(finished_point); }
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
            nav.children.iter().map(|c| convert_nav(c, opf_dir, chapter_map, anchor_map, depth + 1)).collect()
        } else {
            Vec::new()
        };
        let title = if nav.label.is_empty() { "Без названия".to_string() } else { nav.label.clone() };
        TocItem { id: nav.id.clone(), title, chapter_index: ch_index, children }
    }

    root_nav_points.iter().map(|nav| convert_nav(nav, opf_dir, chapter_map, anchor_map, 0)).collect()
}

fn parse_epub3_nav_toc(
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
                        if attr.value.to_string().contains("toc") { in_toc_nav = true; }
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
                let text = quick_xml::escape::unescape(raw).map(|c| c.into_owned()).unwrap_or_else(|_| raw.to_string());
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
                            let ch = resolve_toc_destination(&current_href, nav_dir, chapter_map, anchor_map);
                            let title = if current_title.is_empty() { "Раздел".to_string() } else { current_title.clone() };
                            if let Some(cur_level) = stack.last_mut() {
                                cur_level.push(TocItem { id: None, title, chapter_index: ch, children: Vec::new() });
                            }
                        }
                    } else if (tag == "ol" || tag == "ul") && stack.len() > 1 && let Some(children) = stack.pop() {
                        if let Some(cur_level) = stack.last_mut() && let Some(last_item) = cur_level.last_mut() {
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

fn local_name(name: &str) -> &str {
    name.rfind(':').map(|pos| &name[pos + 1..]).unwrap_or(name)
}
