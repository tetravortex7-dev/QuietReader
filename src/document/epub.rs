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
    ) -> Result<String, DocumentError> {
        let normalized = entry_name.replace('\\', "/");
        if !crate::document::is_safe_archive_path(&normalized) {
            return Err(DocumentError::Corrupted(format!(
                "Недопустимый или небезопасный путь в EPUB: '{entry_name}'"
            )));
        }
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

        let index = found_index.ok_or_else(|| {
            DocumentError::Corrupted(format!("Файл '{entry_name}' не найден в архиве"))
        })?;

        let file = archive.by_index(index).map_err(|e| {
            DocumentError::Corrupted(format!("Ошибка открытия '{entry_name}': {e}"))
        })?;

        if file.size() > Self::MAX_ENTRY_BYTES {
            return Err(DocumentError::LimitExceeded(format!(
                "Файл '{entry_name}' превышает допустимый размер (50 МБ)"
            )));
        }

        let mut bytes = Vec::new();
        file.take(Self::MAX_ENTRY_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Ошибка чтения файла '{entry_name}': {e}")))?;

        match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(err) => {
                let bytes = err.into_bytes();
                let (cow, _, _) = encoding_rs::WINDOWS_1251.decode(&bytes);
                Ok(cow.into_owned())
            }
        }
    }

    fn find_opf_path<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
    ) -> Result<String, DocumentError> {
        let container_xml = match Self::read_zip_entry(archive, "META-INF/container.xml") {
            Ok(c) => c,
            Err(_) => {
                for i in 0..archive.len() {
                    if let Ok(file) = archive.by_index(i)
                        && file.name().ends_with(".opf")
                    {
                        return Ok(file.name().to_string());
                    }
                }
                return Err(DocumentError::Corrupted(
                    "META-INF/container.xml не найден в EPUB архиве".to_string(),
                ));
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
                Err(e) => {
                    return Err(DocumentError::Corrupted(format!(
                        "Ошибка парсинга container.xml: {e}"
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i)
                && file.name().ends_with(".opf")
            {
                return Ok(file.name().to_string());
            }
        }

        Err(DocumentError::Corrupted(
            "Не найден rootfile (OPF) в container.xml".to_string(),
        ))
    }

    fn from_archive<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
    ) -> Result<Self, DocumentError> {
        let mut total_unpacked: u64 = 0;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index_raw(i) {
                if !crate::document::is_safe_archive_path(entry.name()) {
                    return Err(DocumentError::Corrupted(format!(
                        "Обнаружен небезопасный путь в архиве EPUB: {}",
                        entry.name()
                    )));
                }
                total_unpacked = total_unpacked.saturating_add(entry.size());
                if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
                    return Err(DocumentError::LimitExceeded(
                        "Превышен общий лимит распаковки EPUB архива (500 МБ)".to_string(),
                    ));
                }
            }
        }

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
                spine_files.push((idref.clone(), resolve_path(&opf_dir, &item.href)));
            }
        }
        if spine_files.is_empty() {
            return Err(DocumentError::Corrupted(
                "EPUB spine не содержит разделов для чтения".to_string(),
            ));
        }

        let mut chapters = Vec::new();
        let mut chapter_path_to_index: HashMap<String, usize> = HashMap::new();
        for (ch_idx, (idref, path)) in spine_files.iter().enumerate() {
            let html_content = Self::read_zip_entry(archive, path)
                .or_else(|_| Self::read_zip_entry(archive, &percent_decode_str(path)))
                .unwrap_or_default();

            let (parsed_title, paragraphs) = parse_html_chapter(&html_content);
            let final_title = if !parsed_title.is_empty() {
                parsed_title
            } else {
                format!("Раздел {}", ch_idx + 1)
            };

            chapter_path_to_index.insert(normalize_zip_path(path), ch_idx);
            if let Some(file_name) = Path::new(path).file_name().and_then(|n| n.to_str()) {
                chapter_path_to_index
                    .entry(file_name.to_string())
                    .or_insert(ch_idx);
            }
            chapters.push(BookChapter {
                id: Some(idref.clone()),
                title: final_title,
                paragraphs,
            });
        }

        let mut toc = Vec::new();
        if let Some(ncx_id) = ncx_id
            && let Some(ncx_item) = manifest_map.get(&ncx_id)
        {
            let ncx_full_path = resolve_path(&opf_dir, &ncx_item.href);
            if let Ok(ncx_content) = Self::read_zip_entry(archive, &ncx_full_path) {
                toc = parse_ncx_toc(&ncx_content, &opf_dir, &chapter_path_to_index);
            }
        }
        if toc.is_empty() {
            toc = chapters
                .iter()
                .enumerate()
                .map(|(idx, ch)| TocItem {
                    id: ch.id.clone(),
                    title: ch.title.clone(),
                    chapter_index: idx,
                    children: Vec::new(),
                })
                .collect();
        }

        Ok(Self {
            metadata,
            chapters,
            toc,
        })
    }

    fn parse_opf(
        opf: &str,
    ) -> Result<(BookMetadata, Vec<ManifestItem>, Vec<String>, Option<String>), DocumentError> {
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
                                _ => {}
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
                    let text = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());

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
                Err(e) => {
                    return Err(DocumentError::Corrupted(format!(
                        "Ошибка парсинга OPF XML: {e}"
                    )));
                }
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
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte_val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            result.push(byte_val);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

fn flush_p(current_p: &mut String, paragraphs: &mut Vec<String>) {
    if !current_p.trim().is_empty() {
        paragraphs.push(current_p.trim().to_string());
        current_p.clear();
    }
}

fn parse_html_chapter(html: &str) -> (String, Vec<String>) {
    let mut reader = Reader::from_str(html);
    reader.config_mut().trim_text(true);
    let mut title = String::new();
    let mut paragraphs = Vec::new();
    let mut current_p = String::new();
    let (mut in_title_tag, mut in_heading_tag, mut skip_content) = (false, false, false);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "script" | "style" | "head" => skip_content = true,
                    "title" => {
                        skip_content = false;
                        in_title_tag = true;
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_p(&mut current_p, &mut paragraphs);
                        in_heading_tag = true;
                    }
                    "p" | "div" | "blockquote" | "li" => flush_p(&mut current_p, &mut paragraphs),
                    "br" => current_p.push('\n'),
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                match local_name(e.name().as_ref()).to_ascii_lowercase().as_str() {
                    "br" => current_p.push('\n'),
                    "hr" => {
                        flush_p(&mut current_p, &mut paragraphs);
                        paragraphs.push("* * *".to_string());
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if !skip_content => {
                let raw_str = e.as_ref();
                let text = quick_xml::escape::unescape(raw_str)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| raw_str.to_string());
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if in_title_tag && title.is_empty() {
                        title = trimmed.to_string();
                    } else {
                        if !current_p.is_empty()
                            && !current_p.ends_with(' ')
                            && !current_p.ends_with('\n')
                        {
                            current_p.push(' ');
                        }
                        current_p.push_str(trimmed);
                    }
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
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    flush_p(&mut current_p, &mut paragraphs);
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
            ext @ (Ok(Event::Start(_)) | Ok(Event::Empty(_))) => {
                let e = match &ext {
                    Ok(Event::Start(e)) | Ok(Event::Empty(e)) => e,
                    _ => unreachable!(),
                };
                let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                if tag == "navpoint" {
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

    fn convert_nav(nav: &NavPoint, opf_dir: &str, chapter_map: &HashMap<String, usize>) -> TocItem {
        let clean_src = nav
            .content_src
            .split('#')
            .next()
            .unwrap_or(&nav.content_src);
        let normalized = normalize_zip_path(&resolve_path(opf_dir, clean_src));
        let ch_index = chapter_map
            .get(&normalized)
            .or_else(|| {
                Path::new(&normalized)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|fname| chapter_map.get(fname))
            })
            .copied()
            .unwrap_or(0);
        let children = nav
            .children
            .iter()
            .map(|c| convert_nav(c, opf_dir, chapter_map))
            .collect();
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
        .map(|nav| convert_nav(nav, opf_dir, chapter_map))
        .collect()
}

fn local_name(name: &str) -> &str {
    name.rfind(':').map(|pos| &name[pos + 1..]).unwrap_or(name)
}
