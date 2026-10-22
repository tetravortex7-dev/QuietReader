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

pub mod html;
pub mod toc;

#[allow(unused_imports)]
pub use html::{extract_html_lenient, parse_html_chapter, sanitize_xml_entities};
#[allow(unused_imports)]
pub use toc::{NavPoint, parse_epub3_nav_toc, parse_ncx_toc, resolve_toc_destination};

#[derive(Default, Debug)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
    properties: String,
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
            return Err(DocumentError::Corrupted(format!(
                "Небезопасный путь в EPUB: '{entry_name}'"
            )));
        }
        let decoded = percent_decode_str(&normalized);
        let mut matched = None;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file
                    .name()
                    .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения ZIP: {e}")))?
                    .replace('\\', "/");
                if name == normalized || name == decoded {
                    matched = Some(i);
                    break;
                } else if matched.is_none()
                    && (name.eq_ignore_ascii_case(&normalized)
                        || name.eq_ignore_ascii_case(&decoded))
                {
                    matched = Some(i);
                }
            }
        }

        let index = matched
            .ok_or_else(|| DocumentError::Corrupted(format!("Файл '{entry_name}' не найден")))?;
        let file = archive
            .by_index(index)
            .map_err(|e| DocumentError::Corrupted(format!("Ошибка '{entry_name}': {e}")))?;
        if file.size() > Self::MAX_ENTRY_BYTES {
            return Err(DocumentError::LimitExceeded(format!(
                "Файл '{entry_name}' превышает лимит"
            )));
        }

        let mut bytes = Vec::new();
        file.take(Self::MAX_ENTRY_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Ошибка чтения '{entry_name}': {e}")))?;

        if bytes.len() as u64 > Self::MAX_ENTRY_BYTES {
            return Err(DocumentError::LimitExceeded(format!(
                "Файл '{entry_name}' превышает лимит"
            )));
        }

        *running_unpacked = running_unpacked.saturating_add(bytes.len() as u64);
        if *running_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Превышен общий лимит распаковки EPUB (500 МБ)".to_string(),
            ));
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
        let container_xml =
            match Self::read_zip_entry(archive, "META-INF/container.xml", running_unpacked) {
                Ok(c) => c,
                Err(_) => {
                    for i in 0..archive.len() {
                        if let Ok(f) = archive.by_index(i)
                            && let Ok(name) = f.name()
                            && name.ends_with(".opf")
                        {
                            return Ok(name.into_owned());
                        }
                    }
                    return Err(DocumentError::Corrupted(
                        "META-INF/container.xml не найден".to_string(),
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
                        "Ошибка container.xml: {e}"
                    )));
                }
                _ => {}
            }
            buf.clear();
        }

        for i in 0..archive.len() {
            if let Ok(f) = archive.by_index(i)
                && let Ok(name) = f.name()
                && name.ends_with(".opf")
            {
                return Ok(name.into_owned());
            }
        }
        Err(DocumentError::Corrupted(
            "Не найден rootfile (OPF)".to_string(),
        ))
    }

    fn from_archive<R: Read + std::io::Seek>(
        archive: &mut ZipArchive<R>,
    ) -> Result<Self, DocumentError> {
        let mut total_unpacked: u64 = 0;
        for i in 0..archive.len() {
            if let Ok(entry) = archive.by_index_raw(i) {
                let name = entry
                    .name()
                    .map_err(|e| DocumentError::Corrupted(format!("Ошибка чтения ZIP: {e}")))?;
                if !crate::document::is_safe_archive_path(&name) {
                    return Err(DocumentError::Corrupted(format!(
                        "Небезопасный путь: {name}"
                    )));
                }
                total_unpacked = total_unpacked.saturating_add(entry.size());
                if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
                    return Err(DocumentError::LimitExceeded(
                        "Превышен общий лимит архива EPUB".to_string(),
                    ));
                }
            }
        }

        let mut running_unpacked: u64 = 0;
        let opf_path = Self::find_opf_path(archive, &mut running_unpacked)?;
        let opf_dir = Path::new(&opf_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let opf_content = Self::read_zip_entry(archive, &opf_path, &mut running_unpacked)?;
        let (metadata, manifest, spine_idrefs, ncx_id, nav_id) = Self::parse_opf(&opf_content)?;

        let manifest_map: HashMap<String, ManifestItem> =
            manifest.into_iter().map(|it| (it.id.clone(), it)).collect();
        let mut spine_files: Vec<(String, String)> = Vec::new();
        for idref in &spine_idrefs {
            if let Some(item) = manifest_map.get(idref) {
                spine_files.push((idref.clone(), resolve_path(&opf_dir, &item.href)));
            }
        }
        if spine_files.is_empty() {
            return Err(DocumentError::Corrupted(
                "EPUB spine не содержит разделов".to_string(),
            ));
        }

        let mut chapters = Vec::new();
        let mut chapter_path_to_index: HashMap<String, usize> = HashMap::new();
        let mut anchor_to_chapter: HashMap<String, usize> = HashMap::new();

        for (ch_idx, (idref, path)) in spine_files.iter().enumerate() {
            let html_content = Self::read_zip_entry(archive, path, &mut running_unpacked)
                .or_else(|_| {
                    Self::read_zip_entry(archive, &percent_decode_str(path), &mut running_unpacked)
                })
                .unwrap_or_default();

            let (parsed_title, paragraphs, anchors) = parse_html_chapter(&html_content);
            let final_title = if !parsed_title.is_empty() {
                parsed_title
            } else {
                format!("Раздел {}", ch_idx + 1)
            };

            let norm_path = normalize_zip_path(path);
            let dec_path = percent_decode_str(&norm_path);
            chapter_path_to_index.insert(norm_path.clone(), ch_idx);
            chapter_path_to_index.insert(dec_path.clone(), ch_idx);

            if let Some(fname) = Path::new(&norm_path).file_name().and_then(|n| n.to_str()) {
                chapter_path_to_index
                    .entry(fname.to_string())
                    .or_insert(ch_idx);
            }
            if let Some(fname) = Path::new(&dec_path).file_name().and_then(|n| n.to_str()) {
                chapter_path_to_index
                    .entry(fname.to_string())
                    .or_insert(ch_idx);
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
        if let Some(ncx_id) = ncx_id
            && let Some(ncx_item) = manifest_map.get(&ncx_id)
        {
            let ncx_full_path = resolve_path(&opf_dir, &ncx_item.href);
            if let Ok(ncx_content) =
                Self::read_zip_entry(archive, &ncx_full_path, &mut running_unpacked)
            {
                toc = parse_ncx_toc(
                    &ncx_content,
                    &opf_dir,
                    &chapter_path_to_index,
                    &anchor_to_chapter,
                );
            }
        }

        if toc.is_empty()
            && let Some(nav_id) = nav_id
            && let Some(nav_item) = manifest_map.get(&nav_id)
        {
            let nav_full_path = resolve_path(&opf_dir, &nav_item.href);
            if let Ok(nav_content) =
                Self::read_zip_entry(archive, &nav_full_path, &mut running_unpacked)
            {
                let nav_dir = Path::new(&nav_full_path)
                    .parent()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                toc = parse_epub3_nav_toc(
                    &nav_content,
                    &nav_dir,
                    &chapter_path_to_index,
                    &anchor_to_chapter,
                );
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
    ) -> Result<
        (
            BookMetadata,
            Vec<ManifestItem>,
            Vec<String>,
            Option<String>,
            Option<String>,
        ),
        DocumentError,
    > {
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
                            if item.properties.split_whitespace().any(|p| p == "nav")
                                && nav_id.is_none()
                            {
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
                        "Ошибка парсинга OPF: {e}"
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
        Ok((metadata, manifest, spine_idrefs, ncx_id, nav_id))
    }
}

pub fn resolve_path(base_dir: &str, href: &str) -> String {
    let clean_href = href.split('#').next().unwrap_or(href);
    if clean_href.contains(':') || clean_href.starts_with('/') || clean_href.starts_with('\\') {
        return clean_href.to_string();
    }
    let combined = if base_dir.is_empty() {
        clean_href.to_string()
    } else {
        format!("{base_dir}/{clean_href}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for seg in combined.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if parts.is_empty() {
                return String::new();
            }
            parts.pop();
        } else {
            parts.push(seg);
        }
    }
    parts.join("/")
}

pub(crate) fn normalize_zip_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

pub fn percent_decode_str(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b_val) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
        {
            result.push(b_val);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub(crate) fn local_name(name: &str) -> &str {
    name.rfind(':').map(|pos| &name[pos + 1..]).unwrap_or(name)
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
