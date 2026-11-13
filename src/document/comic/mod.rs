use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use zip::ZipArchive;

use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, DocumentViewKind,
    PageContent, PageInfo, TocItem,
};

const MAX_COMIC_INFO_BYTES: u64 = 5 * 1024 * 1024;

static SEVENZ_CACHE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct ExtractedCache {
    dir: PathBuf,
    files: HashMap<String, PathBuf>,
}

impl ExtractedCache {
    fn new() -> Result<Self, DocumentError> {
        let parent = std::env::temp_dir().join("QuietReader").join("comic-cache");
        std::fs::create_dir_all(&parent)
            .map_err(|e| DocumentError::Io(crate::document::errors::cache_dir_create(&e)))?;
        let id = SEVENZ_CACHE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = parent.join(format!("{}-{id}", std::process::id()));
        std::fs::create_dir(&dir)
            .map_err(|e| DocumentError::Io(crate::document::errors::cache_file_create(&e)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)) {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(DocumentError::Io(
                    crate::document::errors::cache_permissions(&e),
                ));
            }
        }
        Ok(Self {
            dir,
            files: HashMap::new(),
        })
    }

    fn store(&mut self, name: &str, bytes: &[u8]) -> Result<(), DocumentError> {
        let cache_name = format!("page_{}.img", self.files.len());
        let cache_path = self.dir.join(cache_name);
        std::fs::write(&cache_path, bytes)
            .map_err(|e| DocumentError::Io(crate::document::errors::cache_save_page(&e)))?;
        self.files.insert(name.to_string(), cache_path);
        Ok(())
    }
}

impl Drop for ExtractedCache {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComicArchiveKind {
    Zip,
    Rar,
    SevenZ,
    Tar,
    Folder,
}

pub struct ComicDocument {
    metadata: BookMetadata,
    path: PathBuf,
    kind: ComicArchiveKind,
    pages: Vec<PageInfo>,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    is_manga: bool,
    extracted_cache: Option<ExtractedCache>,
}

impl ComicDocument {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(DocumentError::Io(crate::document::errors::io_not_found(
                &path_ref.display().to_string(),
            )));
        }

        let kind = if path_ref.is_dir() {
            ComicArchiveKind::Folder
        } else {
            let ext = path_ref
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            match ext.as_str() {
                "cbz" | "zip" => ComicArchiveKind::Zip,
                "cbr" | "rar" => ComicArchiveKind::Rar,
                "cb7" | "7z" => ComicArchiveKind::SevenZ,
                "cbt" | "tar" => ComicArchiveKind::Tar,
                _ => {
                    let mut header = [0u8; 262];
                    let n = File::open(path_ref)
                        .and_then(|mut f| f.read(&mut header))
                        .map_err(|e| {
                            DocumentError::Io(crate::document::errors::io_read(
                                &path_ref.display().to_string(),
                                &e,
                            ))
                        })?;
                    if n >= 4 && &header[0..4] == b"PK\x03\x04" {
                        ComicArchiveKind::Zip
                    } else if (n >= 7 && &header[0..7] == b"Rar!\x1a\x07\x00")
                        || (n >= 8 && &header[0..8] == b"Rar!\x1a\x07\x01\x00")
                    {
                        ComicArchiveKind::Rar
                    } else if n >= 6 && &header[0..6] == b"7z\xbc\xaf\x27\x1c" {
                        ComicArchiveKind::SevenZ
                    } else if n >= 262 && &header[257..262] == b"ustar" {
                        ComicArchiveKind::Tar
                    } else {
                        return Err(DocumentError::Unsupported(
                            crate::document::errors::comic_unsupported_format(
                                &path_ref.display().to_string(),
                            ),
                        ));
                    }
                }
            }
        };

        let (mut pages, comic_info_xml, extracted_cache) = match kind {
            ComicArchiveKind::Folder => {
                let (pages, info) = scan_folder(path_ref)?;
                (pages, info, None)
            }
            ComicArchiveKind::Zip => {
                let (pages, info) = scan_zip(path_ref)?;
                (pages, info, None)
            }
            ComicArchiveKind::Rar => {
                let (pages, info) = scan_rar(path_ref)?;
                (pages, info, None)
            }
            ComicArchiveKind::SevenZ => scan_sevenz(path_ref)?,
            ComicArchiveKind::Tar => scan_tar(path_ref)?,
        };

        if pages.is_empty() {
            return Err(DocumentError::Corrupted(
                crate::document::errors::comic_no_images(),
            ));
        }

        const MAX_PAGES_LIMIT: usize = 5000;
        if pages.len() > MAX_PAGES_LIMIT {
            pages.truncate(MAX_PAGES_LIMIT);
        }

        pages.sort_by(|a, b| natural_cmp(&a.archive_path, &b.archive_path));

        let default_title = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(crate::document::errors::tokens::TITLE_COMIC)
            .to_string();

        let mut title = default_title;
        let mut author = crate::document::errors::tokens::UNKNOWN_AUTHOR.to_string();
        let mut is_manga = false;

        let path_lower = path_ref.to_string_lossy().to_lowercase();
        if path_lower.contains("manga") || path_lower.contains("манга") {
            is_manga = true;
        }

        if let Some(xml_bytes) = comic_info_xml
            && let Ok(info) = parse_comic_info(&xml_bytes)
        {
            if let Some(t) = info.title
                && !t.trim().is_empty()
            {
                title = t;
            }
            if let Some(w) = info.writer
                && !w.trim().is_empty()
            {
                author = w;
            }
            if let Some(m) = info.is_manga {
                is_manga = m;
            }
        }

        let (chapters, toc) = build_comic_chapters(&pages, &title);

        Ok(Self {
            metadata: BookMetadata { title, author },
            path: path_ref.to_path_buf(),
            kind,
            pages,
            chapters,
            toc,
            is_manga,
            extracted_cache,
        })
    }
}

impl Document for ComicDocument {
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
        "Comic"
    }

    fn has_custom_toc(&self) -> bool {
        self.toc.len() > 1
    }

    fn view_kind(&self) -> DocumentViewKind {
        DocumentViewKind::Comic
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }
}

impl PageContent for ComicDocument {
    fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn page_name(&self, index: usize) -> &str {
        self.pages
            .get(index)
            .map(|p| p.display_name.as_str())
            .unwrap_or("")
    }

    fn page_info(&self, index: usize) -> Option<&PageInfo> {
        self.pages.get(index)
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError> {
        let page = self.pages.get(index).ok_or_else(|| {
            DocumentError::Corrupted(crate::document::errors::page_out_of_bounds(index + 1))
        })?;
        let max_single = crate::document::MAX_SINGLE_ENTRY_BYTES;
        let max_total = crate::document::MAX_TOTAL_UNPACK_BYTES;

        let bytes = match self.kind {
            ComicArchiveKind::Folder => {
                if !is_safe_path(&page.archive_path) {
                    return Err(DocumentError::Corrupted(
                        crate::document::errors::unsafe_path(),
                    ));
                }
                let full = self.path.join(&page.archive_path);
                let meta = std::fs::metadata(&full)?;
                if meta.len() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        crate::document::errors::archive_entry_too_large_mb(&page.archive_path, 50),
                    ));
                }
                std::fs::read(&full)?
            }
            ComicArchiveKind::Zip => {
                let mut zip = ZipArchive::new(File::open(&self.path)?).map_err(|e| {
                    DocumentError::Corrupted(crate::document::errors::archive_parse("ZIP", &e))
                })?;
                let entry = zip.by_name(&page.archive_path).map_err(|_| {
                    DocumentError::Corrupted(crate::document::errors::archive_entry_not_found(
                        "ZIP",
                        &page.archive_path,
                    ))
                })?;
                if entry.size() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        crate::document::errors::archive_entry_too_large_mb(&page.archive_path, 50),
                    ));
                }
                let mut total = 0u64;
                crate::document::read_bounded(entry, max_single, &mut total, max_total)?
            }
            ComicArchiveKind::Rar => {
                let mut archive =
                    unrar_rs::RarArchive::open(File::open(&self.path)?).map_err(|e| {
                        DocumentError::Corrupted(crate::document::errors::archive_parse("RAR", &e))
                    })?;
                let handle = archive.by_name(&page.archive_path).map_err(|_| {
                    DocumentError::Corrupted(crate::document::errors::archive_entry_not_found(
                        "RAR",
                        &page.archive_path,
                    ))
                })?;
                let mut total = 0u64;
                crate::document::read_bounded(handle, max_single, &mut total, max_total)?
            }
            ComicArchiveKind::SevenZ => {
                let cache = self.extracted_cache.as_ref().ok_or_else(|| {
                    DocumentError::Corrupted(crate::document::errors::archive_cache_unavailable(
                        "CB7",
                    ))
                })?;
                let cached = cache.files.get(&page.archive_path).ok_or_else(|| {
                    DocumentError::Corrupted(crate::document::errors::archive_entry_not_found(
                        "7z",
                        &page.archive_path,
                    ))
                })?;
                let meta = std::fs::metadata(cached)?;
                if meta.len() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        crate::document::errors::archive_entry_too_large_mb(&page.archive_path, 50),
                    ));
                }
                std::fs::read(cached)?
            }
            ComicArchiveKind::Tar => {
                let cache = self.extracted_cache.as_ref().ok_or_else(|| {
                    DocumentError::Corrupted(crate::document::errors::archive_cache_unavailable(
                        "CBT",
                    ))
                })?;
                let cached = cache.files.get(&page.archive_path).ok_or_else(|| {
                    DocumentError::Corrupted(crate::document::errors::archive_entry_not_found(
                        "TAR",
                        &page.archive_path,
                    ))
                })?;
                let meta = std::fs::metadata(cached)?;
                if meta.len() > max_single {
                    return Err(DocumentError::LimitExceeded(
                        crate::document::errors::archive_entry_too_large_mb(&page.archive_path, 50),
                    ));
                }
                std::fs::read(cached)?
            }
        };

        decode_image_bytes(&bytes)
    }

    fn is_manga(&self) -> bool {
        self.is_manga
    }
}

fn decode_image_bytes(bytes: &[u8]) -> Result<DecodedImage, DocumentError> {
    let dim_reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DocumentError::Corrupted(crate::document::errors::image_format_error(&e)))?;
    if let Ok((w, h)) = dim_reader.into_dimensions() {
        if w == 0 || h == 0 {
            return Err(DocumentError::Corrupted(
                crate::document::errors::image_zero_size(),
            ));
        }
        if w > 8192 || h > 16384 || (w as u64) * (h as u64) > 25_000_000 {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::image_too_large(w, h),
            ));
        }
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| DocumentError::Corrupted(crate::document::errors::image_format_error(&e)))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(150 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|e| DocumentError::Corrupted(crate::document::errors::image_decode_failed(&e)))?;
    let img = if img.width() > 4096 || img.height() > 4096 {
        img.thumbnail(4096, 4096)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    Ok(DecodedImage {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

pub fn is_supported_image(name: &str) -> bool {
    let filename = Path::new(name)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");
    if filename.starts_with('.')
        || filename.eq_ignore_ascii_case("thumbs.db")
        || name.contains("__MACOSX")
    {
        return false;
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    matches!(
        ext.as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp"
    )
}

pub fn zip_contains_supported_image(path: &Path) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    let Ok(mut zip) = ZipArchive::new(file) else {
        return false;
    };
    if zip.len() > crate::document::MAX_ARCHIVE_ENTRIES {
        return false;
    }
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i)
            && !entry.is_dir()
            && let Ok(name) = entry.name()
            && crate::document::is_safe_archive_path(&name)
            && is_supported_image(&name)
        {
            return true;
        }
    }
    false
}

fn is_safe_path(p: &str) -> bool {
    crate::document::is_safe_archive_path(p)
}

fn is_comic_info_xml(p: &Path) -> bool {
    p.file_name()
        .map(|f| f.to_string_lossy().eq_ignore_ascii_case("comicinfo.xml"))
        .unwrap_or(false)
}

fn make_page_info(archive_path: String, path_obj: &Path) -> PageInfo {
    let display_name = path_obj
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| archive_path.clone());
    let chapter_title = path_obj
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().to_string());
    PageInfo {
        archive_path,
        display_name,
        chapter_title,
    }
}

fn scan_folder(folder: &Path) -> Result<(Vec<PageInfo>, Option<Vec<u8>>), DocumentError> {
    let mut pages = Vec::new();
    let mut comic_info = None;

    fn visit_dir(
        root: &Path,
        current: &Path,
        depth: usize,
        pages: &mut Vec<PageInfo>,
        comic_info: &mut Option<Vec<u8>>,
    ) {
        if depth > crate::document::MAX_FOLDER_DEPTH {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(current) {
            for entry in entries.flatten() {
                if pages.len() >= crate::document::MAX_ARCHIVE_ENTRIES {
                    break;
                }
                let path = entry.path();
                let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                    continue;
                };
                if metadata.file_type().is_symlink() {
                    continue;
                }
                if metadata.is_dir() {
                    visit_dir(root, &path, depth + 1, pages, comic_info);
                } else if metadata.is_file() {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let rel_str = rel.to_string_lossy().replace('\\', "/");

                    if is_comic_info_xml(rel) {
                        if let Ok(file) = File::open(&path) {
                            let mut total_xml = 0u64;
                            if let Ok(bytes) = crate::document::read_bounded(
                                file,
                                MAX_COMIC_INFO_BYTES,
                                &mut total_xml,
                                MAX_COMIC_INFO_BYTES,
                            ) {
                                *comic_info = Some(bytes);
                            }
                        }
                    } else if is_supported_image(&rel_str) && is_safe_path(&rel_str) {
                        pages.push(make_page_info(rel_str, rel));
                    }
                }
            }
        }
    }

    visit_dir(folder, folder, 0, &mut pages, &mut comic_info);
    Ok((pages, comic_info))
}

fn scan_zip(path: &Path) -> Result<(Vec<PageInfo>, Option<Vec<u8>>), DocumentError> {
    let file = File::open(path)
        .map_err(|e| DocumentError::Io(crate::document::errors::io_open("ZIP", &e)))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| DocumentError::Corrupted(crate::document::errors::archive_parse("ZIP", &e)))?;

    if zip.len() > crate::document::MAX_ARCHIVE_ENTRIES {
        return Err(DocumentError::LimitExceeded(
            crate::document::errors::archive_too_many_files(
                "ZIP",
                crate::document::MAX_ARCHIVE_ENTRIES,
            ),
        ));
    }

    let mut pages = Vec::new();
    let mut comic_info = None;
    let mut total_unpacked: u64 = 0;

    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            let name = entry
                .name()
                .map_err(|e| {
                    DocumentError::Corrupted(crate::document::errors::archive_read("ZIP", &e))
                })?
                .to_string();
            if !is_safe_path(&name) {
                return Err(DocumentError::Corrupted(
                    crate::document::errors::archive_unsafe_path("ZIP", &name),
                ));
            }
            if entry.size() > crate::document::MAX_SINGLE_ENTRY_BYTES {
                return Err(DocumentError::LimitExceeded(
                    crate::document::errors::archive_entry_too_large_mb(&name, 50),
                ));
            }
            total_unpacked = total_unpacked.saturating_add(entry.size());
            if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
                return Err(DocumentError::LimitExceeded(
                    crate::document::errors::archive_total_too_large_mb("ZIP", 500),
                ));
            }
            if name.ends_with('/') || name.ends_with('\\') {
                continue;
            }
            let path_obj = Path::new(&name);
            if is_comic_info_xml(path_obj) {
                let mut total_xml = 0u64;
                if let Ok(bytes) = crate::document::read_bounded(
                    entry,
                    MAX_COMIC_INFO_BYTES,
                    &mut total_xml,
                    MAX_COMIC_INFO_BYTES,
                ) {
                    comic_info = Some(bytes);
                }
            } else if is_supported_image(&name) {
                pages.push(make_page_info(name.clone(), path_obj));
            }
        }
    }

    Ok((pages, comic_info))
}

fn scan_rar(path: &Path) -> Result<(Vec<PageInfo>, Option<Vec<u8>>), DocumentError> {
    let file = File::open(path)
        .map_err(|e| DocumentError::Io(crate::document::errors::io_open("RAR", &e)))?;
    let mut archive = unrar_rs::RarArchive::open(file)
        .map_err(|e| DocumentError::Corrupted(crate::document::errors::archive_parse("RAR", &e)))?;

    let mut pages = Vec::new();
    let mut comic_info_entry_name = None;

    let entry_names: Vec<String> = archive
        .entries()
        .take(crate::document::MAX_ARCHIVE_ENTRIES + 1)
        .map(|e| e.name.clone())
        .collect();
    if entry_names.len() > crate::document::MAX_ARCHIVE_ENTRIES {
        return Err(DocumentError::LimitExceeded(
            crate::document::errors::archive_too_many_files(
                "RAR",
                crate::document::MAX_ARCHIVE_ENTRIES,
            ),
        ));
    }

    for raw_name in entry_names {
        let name = raw_name.replace('\\', "/");
        if !is_safe_path(&name) {
            return Err(DocumentError::Corrupted(
                crate::document::errors::archive_unsafe_path("RAR", &name),
            ));
        }

        let path_obj = Path::new(&name);
        if is_comic_info_xml(path_obj) {
            comic_info_entry_name = Some(raw_name);
        } else if is_supported_image(&name) {
            pages.push(make_page_info(raw_name, path_obj));
        }
    }

    let mut comic_info = None;
    if let Some(ci_name) = comic_info_entry_name
        && let Ok(handle) = archive.by_name(&ci_name)
    {
        let mut total = 0u64;
        if let Ok(bytes) = crate::document::read_bounded(
            handle,
            MAX_COMIC_INFO_BYTES,
            &mut total,
            MAX_COMIC_INFO_BYTES,
        ) {
            comic_info = Some(bytes);
        }
    }

    Ok((pages, comic_info))
}

fn scan_sevenz(
    path: &Path,
) -> Result<(Vec<PageInfo>, Option<Vec<u8>>, Option<ExtractedCache>), DocumentError> {
    scan_sevenz_with_limits(
        path,
        crate::document::MAX_ARCHIVE_ENTRIES,
        crate::document::MAX_SINGLE_ENTRY_BYTES,
        crate::document::MAX_TOTAL_UNPACK_BYTES,
    )
}

pub(crate) fn scan_sevenz_with_limits(
    path: &Path,
    max_entries: usize,
    max_single: u64,
    max_total: u64,
) -> Result<(Vec<PageInfo>, Option<Vec<u8>>, Option<ExtractedCache>), DocumentError> {
    let mut pages = Vec::new();
    let mut comic_info = None;
    let mut entry_count = 0usize;
    let mut total_unpacked = 0u64;
    let mut actual_unpacked = 0u64;
    let mut sec_err: Option<DocumentError> = None;
    let mut cache = ExtractedCache::new()?;

    sevenz_rust::decompress_file_with_extract_fn(path, "", |entry, reader, _| {
        if sec_err.is_some() {
            return Ok(false);
        }
        entry_count += 1;
        if entry_count > max_entries {
            sec_err = Some(DocumentError::LimitExceeded(
                crate::document::errors::archive_too_many_files("7z", max_entries),
            ));
            return Ok(false);
        }
        let name = entry.name().replace('\\', "/");
        if !is_safe_path(&name) {
            sec_err = Some(DocumentError::Corrupted(
                crate::document::errors::archive_unsafe_path("7z", &name),
            ));
            return Ok(false);
        }
        if entry.size() > max_single {
            sec_err = Some(DocumentError::LimitExceeded(
                crate::document::errors::archive_entry_too_large_mb(&name, 50),
            ));
            return Ok(false);
        }
        total_unpacked = total_unpacked.saturating_add(entry.size());
        if total_unpacked > max_total {
            sec_err = Some(DocumentError::LimitExceeded(
                crate::document::errors::archive_total_too_large_mb("7z", 500),
            ));
            return Ok(false);
        }

        let path_obj = Path::new(&name);
        if is_comic_info_xml(path_obj) {
            let prev = actual_unpacked;
            match crate::document::read_bounded(
                reader,
                MAX_COMIC_INFO_BYTES,
                &mut actual_unpacked,
                max_total,
            ) {
                Ok(b) => comic_info = Some(b),
                Err(e)
                    if max_total < MAX_COMIC_INFO_BYTES
                        || prev.saturating_add(MAX_COMIC_INFO_BYTES) > max_total =>
                {
                    sec_err = Some(e);
                    return Ok(false);
                }
                Err(_) => {}
            }
        } else if is_supported_image(&name) {
            let bytes = match crate::document::read_bounded(
                reader,
                max_single,
                &mut actual_unpacked,
                max_total,
            ) {
                Ok(bytes) => bytes,
                Err(e) => {
                    sec_err = Some(e);
                    return Ok(false);
                }
            };
            if let Err(e) = cache.store(&entry.name().to_string(), &bytes) {
                sec_err = Some(e);
                return Ok(false);
            }
            pages.push(make_page_info(entry.name().to_string(), path_obj));
        }
        Ok(true)
    })
    .map_err(|e| DocumentError::Corrupted(crate::document::errors::archive_read("7z", &e)))?;

    if let Some(err) = sec_err {
        return Err(err);
    }
    Ok((pages, comic_info, Some(cache)))
}

fn scan_tar(
    path: &Path,
) -> Result<(Vec<PageInfo>, Option<Vec<u8>>, Option<ExtractedCache>), DocumentError> {
    let file = File::open(path)
        .map_err(|e| DocumentError::Io(crate::document::errors::io_open("TAR", &e)))?;
    let mut archive = tar::Archive::new(std::io::BufReader::new(file));
    let mut cache = ExtractedCache::new()?;
    let mut pages = Vec::new();
    let mut comic_info = None;
    let mut entry_count = 0usize;
    let mut total_unpacked = 0u64;
    let mut actual_unpacked = 0u64;

    let entries = archive
        .entries()
        .map_err(|e| DocumentError::Corrupted(crate::document::errors::archive_read("TAR", &e)))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| {
            DocumentError::Corrupted(crate::document::errors::archive_read("TAR", &e))
        })?;
        entry_count += 1;
        if entry_count > crate::document::MAX_ARCHIVE_ENTRIES {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::archive_too_many_files(
                    "TAR",
                    crate::document::MAX_ARCHIVE_ENTRIES,
                ),
            ));
        }
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .map_err(|e| {
                DocumentError::Corrupted(crate::document::errors::archive_read("TAR", &e))
            })?
            .to_string_lossy()
            .replace('\\', "/");
        if !is_safe_path(&name) {
            return Err(DocumentError::Corrupted(
                crate::document::errors::archive_unsafe_path("TAR", &name),
            ));
        }
        let size = entry.size();
        if size > crate::document::MAX_SINGLE_ENTRY_BYTES {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::archive_entry_too_large_mb(&name, 50),
            ));
        }
        total_unpacked = total_unpacked.saturating_add(size);
        if total_unpacked > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::archive_total_too_large_mb("TAR", 500),
            ));
        }

        let path_obj = Path::new(&name);
        if is_comic_info_xml(path_obj) {
            let prev = actual_unpacked;
            match crate::document::read_bounded(
                &mut entry,
                MAX_COMIC_INFO_BYTES,
                &mut actual_unpacked,
                crate::document::MAX_TOTAL_UNPACK_BYTES,
            ) {
                Ok(bytes) => comic_info = Some(bytes),
                Err(e)
                    if prev.saturating_add(MAX_COMIC_INFO_BYTES)
                        > crate::document::MAX_TOTAL_UNPACK_BYTES =>
                {
                    return Err(e);
                }
                Err(_) => {}
            }
        } else if is_supported_image(&name) {
            let bytes = crate::document::read_bounded(
                &mut entry,
                crate::document::MAX_SINGLE_ENTRY_BYTES,
                &mut actual_unpacked,
                crate::document::MAX_TOTAL_UNPACK_BYTES,
            )?;
            cache.store(&name, &bytes)?;
            pages.push(make_page_info(name.clone(), path_obj));
        }
    }

    Ok((pages, comic_info, Some(cache)))
}

fn build_comic_chapters(
    pages: &[PageInfo],
    default_title: &str,
) -> (Vec<BookChapter>, Vec<TocItem>) {
    let mut groups: Vec<(String, usize, usize)> = Vec::new();
    for (idx, page) in pages.iter().enumerate() {
        let name = page
            .chapter_title
            .clone()
            .unwrap_or_else(|| default_title.to_string());
        if let Some(last) = groups.last_mut()
            && last.0 == name
        {
            last.2 += 1;
        } else {
            groups.push((name, idx, 1));
        }
    }

    if groups.is_empty() {
        let ch = BookChapter {
            id: None,
            title: default_title.to_string(),
            paragraphs: vec![crate::document::errors::tokens::comic_total_pages(
                pages.len(),
            )],
        };
        let t = TocItem {
            id: None,
            title: default_title.to_string(),
            chapter_index: 0,
            children: Vec::new(),
        };
        return (vec![ch], vec![t]);
    }

    let mut chapters = Vec::with_capacity(groups.len());
    let mut toc = Vec::with_capacity(groups.len());
    for (group_title, start, count) in groups {
        let id = Some(format!("ch_{start}"));
        let desc =
            crate::document::errors::tokens::comic_pages_range(start + 1, start + count, count);
        chapters.push(BookChapter {
            id: id.clone(),
            title: group_title.clone(),
            paragraphs: vec![desc],
        });
        toc.push(TocItem {
            id,
            title: group_title,
            chapter_index: start,
            children: Vec::new(),
        });
    }
    (chapters, toc)
}

mod info;
pub use info::natural_cmp;
use info::parse_comic_info;

#[cfg(all(test, feature = "enable_tests"))]
mod test;
