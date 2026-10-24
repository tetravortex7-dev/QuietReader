use crate::document::comic::{ComicDocument, is_supported_image, natural_cmp};
use crate::document::{DecodedImage, PageContent, is_safe_archive_path, read_bounded};
use base64::Engine;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use zip::ZipArchive;

pub const MAX_COVER_SOURCE_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_COVER_THUMBNAIL_WIDTH: u32 = 240;
pub const MAX_COVER_THUMBNAIL_HEIGHT: u32 = 360;

const MAX_CACHE_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SOURCE_IMAGE_WIDTH: u32 = 8192;
const MAX_SOURCE_IMAGE_HEIGHT: u32 = 16384;
const MAX_SOURCE_IMAGE_ALLOC: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 10_000;
const MAX_EPUB_XML_BYTES: u64 = 4 * 1024 * 1024;
const MAX_FB2_XML_BYTES: u64 = 150 * 1024 * 1024;
const MAX_BASE64_CHARS: usize = 14_000_000;

pub fn load_or_create_cover_thumbnail(cache_dir: &Path, book_path: &Path) -> Option<DecodedImage> {
    let cache_path = cover_cache_path(cache_dir, book_path);
    if let Some(image) = load_cached_cover(&cache_path) {
        return Some(image);
    }
    let image = load_cover_thumbnail(book_path)?;
    let _ = save_cached_cover(&cache_path, &image);
    Some(image)
}

pub fn load_cover_thumbnail(book_path: &Path) -> Option<DecodedImage> {
    for cover_path in sibling_cover_paths(book_path) {
        if let Some(image) = decode_file_thumbnail(&cover_path) {
            return Some(image);
        }
    }

    if book_path.is_dir() {
        return comic_folder_cover(book_path);
    }

    let name = book_path
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();
    if name.ends_with(".epub") {
        return epub_cover(book_path);
    }
    if name.ends_with(".fb2") || name.ends_with(".fb2.zip") || name.ends_with(".xml") {
        return fb2_cover(book_path);
    }
    if name.ends_with(".cbz") {
        return cbz_cover(book_path);
    }
    if name.ends_with(".zip") {
        if crate::document::fb2::zip_contains_fb2(book_path) {
            return fb2_cover(book_path);
        }
        return cbz_cover(book_path);
    }
    if name.ends_with(".cbr")
        || name.ends_with(".rar")
        || name.ends_with(".cb7")
        || name.ends_with(".7z")
        || name.ends_with(".cbt")
        || name.ends_with(".tar")
    {
        return comic_archive_cover(book_path);
    }
    if name.ends_with(".mobi")
        || name.ends_with(".azw")
        || name.ends_with(".azw3")
        || name.ends_with(".prc")
    {
        return mobi_cover(book_path);
    }
    None
}

fn thumbnail_from_decoded(image: DecodedImage) -> Option<DecodedImage> {
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.rgba)?;
    let dynamic = image::DynamicImage::ImageRgba8(rgba);
    let thumbnail = if dynamic.width() > MAX_COVER_THUMBNAIL_WIDTH
        || dynamic.height() > MAX_COVER_THUMBNAIL_HEIGHT
    {
        dynamic.thumbnail(MAX_COVER_THUMBNAIL_WIDTH, MAX_COVER_THUMBNAIL_HEIGHT)
    } else {
        dynamic
    };
    let rgba = thumbnail.to_rgba8();
    Some(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn comic_archive_cover(book_path: &Path) -> Option<DecodedImage> {
    let document = ComicDocument::from_path(book_path).ok()?;
    let page = document.load_page_image(0).ok()?;
    thumbnail_from_decoded(page)
}

fn mobi_cover(book_path: &Path) -> Option<DecodedImage> {
    let book = mobi::Mobi::from_path(book_path).ok()?;
    let records = book.raw_records();
    let first_image = book.metadata.mobi.first_image_index as usize;
    let offset = book
        .metadata
        .exth_record(mobi::headers::ExthRecord::CoverOffset)
        .and_then(|values| values.first())
        .and_then(|value| {
            let mut result = 0u32;
            for byte in value {
                result = result.checked_shl(8)? | u32::from(*byte);
            }
            Some(result)
        });
    if let Some(offset) = offset
        && let Some(record) = records
            .records()
            .get(first_image.saturating_add(offset as usize))
        && let Some(thumbnail) = decode_thumbnail(record.content)
    {
        return Some(thumbnail);
    }
    for record in book.image_records() {
        if let Some(thumbnail) = decode_thumbnail(record.content) {
            return Some(thumbnail);
        }
    }
    None
}

pub fn cover_cache_path(cache_dir: &Path, book_path: &Path) -> PathBuf {
    let mut key = book_path.to_string_lossy().replace('\\', "/");
    if let Ok(metadata) = fs::metadata(book_path) {
        key.push('|');
        key.push_str(&metadata.len().to_string());
        if let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(UNIX_EPOCH)
        {
            key.push('|');
            key.push_str(&duration.as_nanos().to_string());
        }
    }
    cache_dir.join(format!("{:016x}.png", stable_hash(key.as_bytes())))
}

fn sibling_cover_paths(book_path: &Path) -> Vec<PathBuf> {
    let directory = if book_path.is_dir() {
        book_path
    } else {
        match book_path.parent() {
            Some(parent) => parent,
            None => return Vec::new(),
        }
    };
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut covers = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let rank = match name.as_str() {
            "cover.jpg" => 0,
            "cover.jpeg" => 1,
            "cover.png" => 2,
            _ => continue,
        };
        covers.push((rank, entry.path()));
    }
    covers.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    covers.into_iter().map(|(_, path)| path).collect()
}

fn decode_file_thumbnail(path: &Path) -> Option<DecodedImage> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_COVER_SOURCE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    decode_thumbnail(&bytes)
}

fn decode_thumbnail(bytes: &[u8]) -> Option<DecodedImage> {
    if bytes.is_empty() {
        return None;
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_IMAGE_WIDTH);
    limits.max_image_height = Some(MAX_SOURCE_IMAGE_HEIGHT);
    limits.max_alloc = Some(MAX_SOURCE_IMAGE_ALLOC);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    if image.width() == 0 || image.height() == 0 {
        return None;
    }
    let thumbnail = if image.width() > MAX_COVER_THUMBNAIL_WIDTH
        || image.height() > MAX_COVER_THUMBNAIL_HEIGHT
    {
        image.thumbnail(MAX_COVER_THUMBNAIL_WIDTH, MAX_COVER_THUMBNAIL_HEIGHT)
    } else {
        image
    };
    let rgba = thumbnail.to_rgba8();
    Some(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn load_cached_cover(path: &Path) -> Option<DecodedImage> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_CACHE_FILE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_COVER_THUMBNAIL_WIDTH);
    limits.max_image_height = Some(MAX_COVER_THUMBNAIL_HEIGHT);
    limits.max_alloc = Some(MAX_CACHE_FILE_BYTES);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    if image.width() == 0
        || image.height() == 0
        || image.width() > MAX_COVER_THUMBNAIL_WIDTH
        || image.height() > MAX_COVER_THUMBNAIL_HEIGHT
    {
        return None;
    }
    let rgba = image.to_rgba8();
    Some(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn save_cached_cover(path: &Path, image: &DecodedImage) -> Option<()> {
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let expected = (image.width as usize)
        .checked_mul(image.height as usize)?
        .checked_mul(4)?;
    if image.rgba.len() != expected {
        return None;
    }
    let parent = path.parent()?;
    crate::platform::permissions::create_private_dir(parent);
    let rgba = image::RgbaImage::from_raw(image.width, image.height, image.rgba.clone())?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    rgba.save_with_format(&temporary, image::ImageFormat::Png)
        .ok()?;
    crate::platform::permissions::restrict_file(&temporary);
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&temporary, path).ok()?;
    crate::platform::permissions::restrict_file(path);
    Some(())
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn epub_cover(book_path: &Path) -> Option<DecodedImage> {
    let file = File::open(book_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return None;
    }
    let opf_path = find_epub_opf(&mut archive)?;
    let opf = read_zip_entry_by_name(&mut archive, &opf_path, MAX_EPUB_XML_BYTES)?;
    let opf = String::from_utf8_lossy(&opf);
    let href = parse_opf_cover_href(&opf)?;
    let opf_dir = opf_path
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    let image_path = resolve_zip_path(opf_dir, &href)?;
    let bytes = read_zip_entry_by_name(&mut archive, &image_path, MAX_COVER_SOURCE_BYTES)?;
    decode_thumbnail(&bytes)
}

fn find_epub_opf<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<String> {
    if let Some(container) =
        read_zip_entry_by_name(archive, "META-INF/container.xml", MAX_EPUB_XML_BYTES)
    {
        let container = String::from_utf8_lossy(&container);
        let mut reader = Reader::from_str(&container);
        reader.config_mut().trim_text(true);
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(Event::Start(ref event)) | Ok(Event::Empty(ref event)) => {
                    if local_name(event.name().as_ref()).eq_ignore_ascii_case("rootfile")
                        && let Some(path) = attribute(event, "full-path")
                    {
                        return Some(path);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buffer.clear();
        }
    }

    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(index) else {
            continue;
        };
        let Ok(name) = entry.name() else {
            continue;
        };
        if name.to_ascii_lowercase().ends_with(".opf") {
            return Some(name.to_string());
        }
    }
    None
}

fn parse_opf_cover_href(opf: &str) -> Option<String> {
    let mut reader = Reader::from_str(opf);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut items = Vec::new();
    let mut cover_id = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref event)) | Ok(Event::Empty(ref event)) => {
                let qualified_name = event.name();
                let name = local_name(qualified_name.as_ref());
                if name.eq_ignore_ascii_case("meta") {
                    let meta_name = attribute(event, "name").unwrap_or_default();
                    if meta_name.eq_ignore_ascii_case("cover") {
                        cover_id = attribute(event, "content");
                    }
                } else if name.eq_ignore_ascii_case("item") {
                    let id = attribute(event, "id").unwrap_or_default();
                    let href = attribute(event, "href").unwrap_or_default();
                    let media_type = attribute(event, "media-type").unwrap_or_default();
                    let properties = attribute(event, "properties").unwrap_or_default();
                    if !href.is_empty() {
                        items.push(ManifestEntry {
                            id,
                            href,
                            media_type,
                            properties,
                        });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buffer.clear();
    }

    if let Some(id) = cover_id
        && let Some(item) = items.iter().find(|item| item.id == id)
    {
        return Some(item.href.clone());
    }
    if let Some(item) = items.iter().find(|item| {
        item.properties
            .split_whitespace()
            .any(|value| value.eq_ignore_ascii_case("cover-image"))
    }) {
        return Some(item.href.clone());
    }
    if let Some(item) = items.iter().find(|item| {
        item.media_type.to_ascii_lowercase().starts_with("image/")
            && (item.id.to_ascii_lowercase().contains("cover")
                || item.href.to_ascii_lowercase().contains("cover"))
    }) {
        return Some(item.href.clone());
    }
    items
        .into_iter()
        .find(|item| item.media_type.to_ascii_lowercase().starts_with("image/"))
        .map(|item| item.href)
}

fn fb2_cover(book_path: &Path) -> Option<DecodedImage> {
    let name = book_path
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();
    let bytes = if name.ends_with(".fb2.zip") || name.ends_with(".zip") {
        let file = File::open(book_path).ok()?;
        let mut archive = ZipArchive::new(file).ok()?;
        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return None;
        }
        let index = find_fb2_entry(&mut archive)?;
        let entry = archive.by_index(index).ok()?;
        if entry.size() > MAX_FB2_XML_BYTES {
            return None;
        }
        parse_fb2_cover(BufReader::new(entry))?
    } else {
        let metadata = fs::metadata(book_path).ok()?;
        if metadata.len() > MAX_FB2_XML_BYTES {
            return None;
        }
        let file = File::open(book_path).ok()?;
        parse_fb2_cover(BufReader::new(file))?
    };
    decode_thumbnail(&bytes)
}

fn find_fb2_entry<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Option<usize> {
    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(index) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let Ok(name) = entry.name() else {
            continue;
        };
        if name.to_ascii_lowercase().ends_with(".fb2") {
            return Some(index);
        }
    }
    None
}

fn parse_fb2_cover<R: BufRead>(reader: R) -> Option<Vec<u8>> {
    let mut reader = Reader::from_reader(reader);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut cover_href = None;
    let mut saw_coverpage = false;
    let mut in_coverpage = false;
    let mut in_binary = false;
    let mut binary_id = String::new();
    let mut binary_text = String::new();
    let mut binary_oversized = false;
    let mut candidate: Option<(String, String)> = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(ref event)) | Ok(Event::Empty(ref event)) => {
                let qualified_name = event.name();
                let name = local_name(qualified_name.as_ref());
                if name.eq_ignore_ascii_case("coverpage") {
                    in_coverpage = true;
                    saw_coverpage = true;
                } else if in_coverpage && name.eq_ignore_ascii_case("image") {
                    if let Some(href) = attribute(event, "href") {
                        let href = href.trim().to_string();
                        if !href.is_empty() {
                            cover_href = Some(href);
                        }
                    }
                } else if name.eq_ignore_ascii_case("binary") {
                    in_binary = true;
                    binary_id = attribute(event, "id").unwrap_or_default();
                    binary_text.clear();
                    binary_oversized = false;
                }
            }
            Ok(Event::Text(ref event)) => {
                if in_binary {
                    append_binary_text(event.as_ref(), &mut binary_text, &mut binary_oversized);
                }
            }
            Ok(Event::CData(ref event)) => {
                if in_binary {
                    append_binary_text(event.as_ref(), &mut binary_text, &mut binary_oversized);
                }
            }
            Ok(Event::End(ref event)) => {
                let qualified_name = event.name();
                let name = local_name(qualified_name.as_ref());
                if name.eq_ignore_ascii_case("coverpage") {
                    in_coverpage = false;
                } else if name.eq_ignore_ascii_case("binary") && in_binary {
                    in_binary = false;
                    if !binary_oversized && !binary_text.trim().is_empty() {
                        let should_store = match cover_href.as_deref() {
                            Some(href) => fb2_ids_match(href, &binary_id),
                            None => candidate.is_none(),
                        };
                        if should_store {
                            candidate = Some((binary_id.clone(), std::mem::take(&mut binary_text)));
                        }
                    }
                    binary_text.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
        buffer.clear();
    }

    if !saw_coverpage {
        return None;
    }
    let (id, text) = candidate?;
    if let Some(href) = cover_href
        && !fb2_ids_match(&href, &id)
    {
        return None;
    }
    decode_fb2_binary(&text)
}

fn append_binary_text(text: &str, output: &mut String, oversized: &mut bool) {
    if *oversized {
        return;
    }
    if output.len().saturating_add(text.len()) > MAX_BASE64_CHARS {
        *oversized = true;
        return;
    }
    output.push_str(text);
}

fn fb2_ids_match(href: &str, id: &str) -> bool {
    let href = href.trim().trim_start_matches('#');
    let id = id.trim().trim_start_matches('#');
    !href.is_empty() && (href == id || href.rsplit('/').next() == Some(id))
}

fn decode_fb2_binary(text: &str) -> Option<Vec<u8>> {
    let cleaned: String = text
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    if cleaned.is_empty() || cleaned.len() > MAX_BASE64_CHARS {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(cleaned.as_bytes())
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(cleaned.as_bytes()))
        .ok()?;
    if decoded.len() as u64 > MAX_COVER_SOURCE_BYTES {
        return None;
    }
    Some(decoded)
}

fn cbz_cover(book_path: &Path) -> Option<DecodedImage> {
    let file = File::open(book_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return None;
    }
    let mut pages = Vec::new();
    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(index) else {
            continue;
        };
        if entry.is_dir() {
            continue;
        }
        let Ok(name) = entry.name() else {
            continue;
        };
        if is_safe_archive_path(&name) && is_supported_image(&name) {
            pages.push((name.to_string(), index));
        }
    }
    pages.sort_by(|left, right| natural_cmp(&left.0, &right.0));
    for (_, index) in pages {
        let Ok(entry) = archive.by_index(index) else {
            continue;
        };
        if entry.size() > MAX_COVER_SOURCE_BYTES {
            continue;
        }
        let mut total = 0u64;
        let Ok(bytes) = read_bounded(
            entry,
            MAX_COVER_SOURCE_BYTES,
            &mut total,
            MAX_COVER_SOURCE_BYTES,
        ) else {
            continue;
        };
        if let Some(image) = decode_thumbnail(&bytes) {
            return Some(image);
        }
    }
    None
}

fn comic_folder_cover(directory: &Path) -> Option<DecodedImage> {
    let entries = fs::read_dir(directory).ok()?;
    let mut pages = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if is_supported_image(&name) {
            pages.push(entry.path());
        }
    }
    pages.sort_by(|left, right| natural_cmp(&left.to_string_lossy(), &right.to_string_lossy()));
    for page in pages {
        if let Some(image) = decode_file_thumbnail(&page) {
            return Some(image);
        }
    }
    None
}

fn read_zip_entry_by_name<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    max_bytes: u64,
) -> Option<Vec<u8>> {
    let index = zip_entry_index(archive, name)?;
    let entry = archive.by_index(index).ok()?;
    if entry.size() > max_bytes {
        return None;
    }
    let mut total = 0u64;
    read_bounded(entry, max_bytes, &mut total, max_bytes).ok()
}

fn zip_entry_index<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<usize> {
    let normalized = name.replace('\\', "/");
    let decoded = crate::document::epub::percent_decode_str(&normalized);
    let mut fallback = None;
    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(index) else {
            continue;
        };
        let Ok(entry_name) = entry.name() else {
            continue;
        };
        let entry_name = entry_name.replace('\\', "/");
        if entry_name == normalized || entry_name == decoded {
            return Some(index);
        }
        if fallback.is_none()
            && (entry_name.eq_ignore_ascii_case(&normalized)
                || entry_name.eq_ignore_ascii_case(&decoded))
        {
            fallback = Some(index);
        }
    }
    fallback
}

fn resolve_zip_path(base_directory: &str, href: &str) -> Option<String> {
    let href = href.split('#').next().unwrap_or(href);
    let href = crate::document::epub::percent_decode_str(href);
    let base_directory = base_directory.trim_matches('/');
    let combined = if base_directory.is_empty() {
        href
    } else {
        format!("{base_directory}/{href}")
    };
    let mut parts = Vec::new();
    for part in combined.replace('\\', "/").split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part.to_string()),
        }
    }
    let path = parts.join("/");
    if path.is_empty() || !is_safe_archive_path(&path) {
        return None;
    }
    Some(path)
}

fn attribute(event: &BytesStart<'_>, name: &str) -> Option<String> {
    for attribute in event.attributes().flatten() {
        if local_name(attribute.key.as_ref()).eq_ignore_ascii_case(name) {
            return Some(
                quick_xml::escape::unescape(&attribute.value)
                    .map(|value| value.into_owned())
                    .unwrap_or_else(|_| attribute.value.to_string()),
            );
        }
    }
    None
}

fn local_name(name: &str) -> &str {
    match name.rfind(':') {
        Some(position) => &name[position + 1..],
        None => name,
    }
}

struct ManifestEntry {
    id: String,
    href: String,
    media_type: String,
    properties: String,
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
