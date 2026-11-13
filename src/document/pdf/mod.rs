pub mod links;
pub mod outline;
pub mod text;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, DocumentViewKind,
    PageContent, PageInfo, PageTextSpan, TocItem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PdfPermissions {
    pub can_print: bool,
    pub can_copy: bool,
    pub can_modify: bool,
    pub can_annotate: bool,
}

impl Default for PdfPermissions {
    fn default() -> Self {
        Self {
            can_print: true,
            can_copy: true,
            can_modify: true,
            can_annotate: true,
        }
    }
}

pub struct PdfDocument {
    metadata: BookMetadata,
    _path: PathBuf,
    pdf: hayro::hayro_syntax::Pdf,
    page_count: usize,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
    links: std::collections::HashMap<usize, Vec<crate::document::PageLink>>,
    permissions: PdfPermissions,
}

impl PdfDocument {
    #[allow(dead_code)]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        Self::from_file_with_password(path, None)
    }

    pub fn from_file_with_password<P: AsRef<Path>>(
        path: P,
        password: Option<&str>,
    ) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| {
            DocumentError::Io(crate::document::errors::io_open(
                &path_ref.display().to_string(),
                &e,
            ))
        })?;

        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        const MAX_PDF_SIZE: u64 = 300 * 1024 * 1024;
        if file_len > MAX_PDF_SIZE {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::file_too_large_mb("PDF", 300),
            ));
        }
        if file_len == 0 {
            return Err(DocumentError::Corrupted(
                crate::document::errors::file_empty("PDF"),
            ));
        }

        let mut bytes = Vec::new();
        let read = file
            .take(MAX_PDF_SIZE.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| {
                DocumentError::Io(crate::document::errors::io_read(
                    &path_ref.display().to_string(),
                    &e,
                ))
            })?;
        if read as u64 > MAX_PDF_SIZE {
            return Err(DocumentError::LimitExceeded(
                crate::document::errors::file_too_large_mb("PDF", 300),
            ));
        }

        let data = hayro::hayro_syntax::PdfData::from(bytes);
        Self::from_pdf_data_with_password(data, path_ref, password)
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: Vec<u8>, path: &Path) -> Result<Self, DocumentError> {
        Self::from_bytes_with_password(bytes, path, None)
    }

    #[allow(dead_code)]
    pub fn from_bytes_with_password(
        bytes: Vec<u8>,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Self, DocumentError> {
        Self::from_pdf_data_with_password(bytes.into(), path, password)
    }

    fn from_pdf_data_with_password(
        data: hayro::hayro_syntax::PdfData,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Self, DocumentError> {
        let bytes = data.as_ref();
        if bytes.len() < 4 || !bytes.starts_with(b"%PDF") {
            if let Some(pos) = bytes[..bytes.len().min(1024)]
                .windows(4)
                .position(|w| w == b"%PDF")
            {
                let _ = pos;
            } else {
                return Err(DocumentError::Corrupted(
                    crate::document::errors::invalid_header("PDF", "%PDF"),
                ));
            }
        }

        let pass = password.unwrap_or("");
        let pdf = match hayro::hayro_syntax::Pdf::new_with_password(data, pass) {
            Ok(pdf) => pdf,
            Err(e) => match e {
                hayro::hayro_syntax::LoadPdfError::Decryption(_) => {
                    return Err(DocumentError::PasswordRequired);
                }
                hayro::hayro_syntax::LoadPdfError::Invalid => {
                    return Err(DocumentError::Corrupted(
                        crate::document::errors::parse_corrupted("PDF"),
                    ));
                }
            },
        };

        let permissions = extract_pdf_permissions_with_xref(pdf.data().as_ref(), Some(pdf.xref()));

        let page_count = pdf.pages().len();
        if page_count == 0 {
            return Err(DocumentError::Corrupted(crate::document::errors::no_pages(
                "PDF",
            )));
        }

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(crate::document::errors::tokens::TITLE_PDF)
            .to_string();

        let meta = pdf.metadata();
        let title = meta
            .title
            .as_ref()
            .map(|t| outline::decode_pdf_string(t))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(default_title);

        let author = meta
            .author
            .as_ref()
            .map(|a| outline::decode_pdf_string(a))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| crate::document::errors::tokens::UNKNOWN_AUTHOR.to_string());

        let toc = outline::extract_pdf_toc(&pdf, page_count);
        let links = links::extract_pdf_links(&pdf, page_count);

        let mut chapters = Vec::with_capacity(toc.len());
        for (i, item) in toc.iter().enumerate() {
            chapters.push(BookChapter {
                id: item.id.clone(),
                title: item.title.clone(),
                paragraphs: vec![crate::document::errors::tokens::section_page(
                    i + 1,
                    item.chapter_index + 1,
                )],
            });
        }

        Ok(Self {
            metadata: BookMetadata { title, author },
            _path: path.to_path_buf(),
            pdf,
            page_count,
            chapters,
            toc,
            links,
            permissions,
        })
    }

    #[allow(dead_code)]
    pub fn permissions(&self) -> PdfPermissions {
        self.permissions
    }
}

impl Document for PdfDocument {
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
        "PDF"
    }

    fn has_custom_toc(&self) -> bool {
        !self.toc.is_empty()
    }

    fn view_kind(&self) -> DocumentViewKind {
        DocumentViewKind::Pdf
    }

    fn can_copy(&self) -> bool {
        self.permissions.can_copy
    }

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }
}

impl PageContent for PdfDocument {
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_name(&self, _index: usize) -> &str {
        ""
    }

    fn page_info(&self, _index: usize) -> Option<&PageInfo> {
        None
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError> {
        self.load_page_image_scaled(index, 2.5)
    }

    fn load_page_image_scaled(
        &self,
        index: usize,
        scale: f32,
    ) -> Result<DecodedImage, DocumentError> {
        if index >= self.page_count {
            return Err(DocumentError::Corrupted(
                crate::document::errors::page_out_of_bounds(index + 1),
            ));
        }

        let pages = self.pdf.pages();
        let page = pages.get(index).ok_or_else(|| {
            DocumentError::Corrupted(crate::document::errors::page_not_found(index + 1))
        })?;

        let interp_settings = hayro::hayro_interpret::InterpreterSettings::default();
        let render_settings = hayro::RenderSettings::default();

        let (page_w, page_h) = page.render_dimensions();
        let page_w = page_w.max(1.0);
        let page_h = page_h.max(1.0);

        let mut render_scale = scale.clamp(0.5, 4.0);
        if page_w * render_scale > 4096.0 {
            render_scale = render_scale.min(4096.0 / page_w);
        }
        if page_h * render_scale > 4096.0 {
            render_scale = render_scale.min(4096.0 / page_h);
        }
        let total_pixels = (page_w * render_scale) * (page_h * render_scale);
        if total_pixels > 25_000_000.0 {
            render_scale = render_scale.min((25_000_000.0 / (page_w * page_h)).sqrt());
        }
        let render_scale = render_scale.max(0.1);

        let pixmap_settings = hayro::PixmapSettings {
            x_scale: render_scale,
            y_scale: render_scale,
            bg_color: hayro::vello_cpu::color::palette::css::WHITE,
        };

        let cache = hayro::RenderCache::new();

        let pixmap = hayro::render(
            page,
            &cache,
            &interp_settings,
            &render_settings,
            &pixmap_settings,
        );

        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;

        if width == 0 || height == 0 {
            return Err(DocumentError::Corrupted(
                crate::document::errors::page_zero_size(index + 1),
            ));
        }

        let raw_pixels = pixmap.data();
        let total_bytes = (width * height * 4) as usize;
        let mut rgba = vec![0u8; total_bytes];

        for (i, p) in raw_pixels.iter().enumerate() {
            let offset = i * 4;
            if p.a == 255 {
                rgba[offset] = p.r;
                rgba[offset + 1] = p.g;
                rgba[offset + 2] = p.b;
                rgba[offset + 3] = 255;
            } else if p.a == 0 {
                continue;
            } else {
                let inv_a = 255.0 / p.a as f32;
                rgba[offset] = (p.r as f32 * inv_a).min(255.0) as u8;
                rgba[offset + 1] = (p.g as f32 * inv_a).min(255.0) as u8;
                rgba[offset + 2] = (p.b as f32 * inv_a).min(255.0) as u8;
                rgba[offset + 3] = p.a;
            }
        }

        Ok(DecodedImage {
            width,
            height,
            rgba,
        })
    }

    fn extract_page_text(&self, index: usize) -> Option<String> {
        if index >= self.page_count {
            return None;
        }

        let pages = self.pdf.pages();
        let page = pages.get(index)?;

        text::extract_page_text(page)
    }

    fn page_links(&self, index: usize) -> &[crate::document::PageLink] {
        self.links.get(&index).map(Vec::as_slice).unwrap_or(&[])
    }

    fn extract_page_text_spans(&self, index: usize) -> Option<Vec<PageTextSpan>> {
        if index >= self.page_count {
            return None;
        }

        let pages = self.pdf.pages();
        let page = pages.get(index)?;

        text::extract_page_spans(page)
    }

    fn is_manga(&self) -> bool {
        false
    }
}

#[cfg(all(test, feature = "enable_tests"))]
pub fn extract_pdf_permissions(data: &[u8]) -> PdfPermissions {
    extract_pdf_permissions_with_xref(data, None)
}

fn extract_pdf_permissions_with_xref(
    data: &[u8],
    xref: Option<&hayro::hayro_syntax::xref::XRef>,
) -> PdfPermissions {
    use hayro::hayro_syntax::object::{Dict, FromBytes, Name, Number};

    if let Some((has_encrypt, perms)) = extract_from_trailer_or_xref(data, xref) {
        if has_encrypt {
            return perms.unwrap_or_default();
        }
        return PdfPermissions::default();
    }

    let mut pos = 0;
    while let Some((dict_start, dict_end)) = find_next_dict(data, pos) {
        if let Some(dict) = Dict::from_bytes(&data[dict_start..dict_end]) {
            let is_standard_filter = dict
                .get::<Name<'_>>(b"Filter")
                .is_some_and(|name| name.as_str() == "Standard");
            let looks_like_encrypt_dict = is_standard_filter
                || (dict.contains_key(b"Filter")
                    && dict.contains_key(b"V")
                    && dict.contains_key(b"R"));
            if looks_like_encrypt_dict && let Some(p_num) = dict.get::<Number>(b"P") {
                return permissions_from_p_value(p_num.as_i64());
            }
        }
        pos = dict_start + 2;
    }

    PdfPermissions::default()
}

fn permissions_from_p_value(p_val: i64) -> PdfPermissions {
    let p = (p_val as i32) as u32;
    PdfPermissions {
        can_print: (p & 0x0004) != 0,
        can_modify: (p & 0x0008) != 0,
        can_copy: (p & 0x0010) != 0,
        can_annotate: (p & 0x0020) != 0,
    }
}

fn extract_from_trailer_or_xref(
    data: &[u8],
    xref: Option<&hayro::hayro_syntax::xref::XRef>,
) -> Option<(bool, Option<PdfPermissions>)> {
    use hayro::hayro_syntax::object::{Dict, FromBytes, Name};

    let mut search_end = data.len();
    while let Some(idx) = find_last_token(&data[..search_end], b"trailer") {
        if let Some((dict_start, dict_end)) = extract_balanced_dict(data, idx + b"trailer".len())
            && let Some(trailer_dict) = Dict::from_bytes(&data[dict_start..dict_end])
        {
            if let Some(result) = permissions_from_container_dict(data, &trailer_dict, xref) {
                return Some(result);
            }
        }
        search_end = idx;
    }

    let mut pos = 0;
    let mut last_xref_result = None;
    while let Some((dict_start, dict_end)) = find_next_dict(data, pos) {
        if let Some(dict) = Dict::from_bytes(&data[dict_start..dict_end])
            && dict
                .get::<Name<'_>>(b"Type")
                .is_some_and(|t| t.as_str() == "XRef")
        {
            last_xref_result = permissions_from_container_dict(data, &dict, xref);
        }
        pos = dict_end;
    }
    if last_xref_result.is_some() {
        return last_xref_result;
    }

    None
}

fn permissions_from_container_dict(
    data: &[u8],
    container: &hayro::hayro_syntax::object::Dict<'_>,
    xref: Option<&hayro::hayro_syntax::xref::XRef>,
) -> Option<(bool, Option<PdfPermissions>)> {
    use hayro::hayro_syntax::object::{Dict, FromBytes, Number};

    if !container.contains_key(b"Encrypt") {
        if container.contains_key(b"Root") || container.contains_key(b"Size") {
            return Some((false, None));
        }
        return None;
    }

    if let Some(inline_enc) = container.get::<Dict<'_>>(b"Encrypt") {
        let perms = inline_enc
            .get::<Number>(b"P")
            .map(|n| permissions_from_p_value(n.as_i64()));
        return Some((true, perms));
    }

    if let Some(enc_ref) = container.get_ref(b"Encrypt") {
        if let Some(xr) = xref
            && let Some(enc_dict) = xr.get::<Dict<'_>>(enc_ref.into())
            && let Some(p_num) = enc_dict.get::<Number>(b"P")
        {
            return Some((true, Some(permissions_from_p_value(p_num.as_i64()))));
        }
        if let Some(obj_dict_slice) =
            find_indirect_object_dict(data, enc_ref.obj_number, enc_ref.gen_number)
            && let Some(enc_dict) = Dict::from_bytes(obj_dict_slice)
            && let Some(p_num) = enc_dict.get::<Number>(b"P")
        {
            return Some((true, Some(permissions_from_p_value(p_num.as_i64()))));
        }
        return Some((true, None));
    }

    Some((true, None))
}

fn find_indirect_object_dict(data: &[u8], obj_num: u32, gen_num: i32) -> Option<&[u8]> {
    let header = format!("{obj_num} {gen_num} obj");
    let needle = header.as_bytes();
    let mut pos = 0;
    while pos + needle.len() <= data.len() {
        let rel = find_subsequence(&data[pos..], needle)?;
        let idx = pos + rel;
        let left_ok = idx == 0 || is_pdf_delimiter_or_ws(data[idx - 1]);
        let right_idx = idx + needle.len();
        let right_ok = right_idx >= data.len() || is_pdf_delimiter_or_ws(data[right_idx]);
        if left_ok
            && right_ok
            && let Some((dict_start, dict_end)) = extract_balanced_dict(data, right_idx)
        {
            return Some(&data[dict_start..dict_end]);
        }
        pos = idx + needle.len();
    }
    None
}

fn find_last_token(data: &[u8], token: &[u8]) -> Option<usize> {
    if token.is_empty() || data.len() < token.len() {
        return None;
    }
    let mut i = data.len() - token.len();
    loop {
        if &data[i..i + token.len()] == token {
            let left_ok = i == 0 || is_pdf_delimiter_or_ws(data[i - 1]);
            let right_idx = i + token.len();
            let right_ok = right_idx >= data.len() || is_pdf_delimiter_or_ws(data[right_idx]);
            if left_ok && right_ok {
                return Some(i);
            }
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }
    None
}

fn is_pdf_delimiter_or_ws(b: u8) -> bool {
    b.is_ascii_whitespace() || matches!(b, b'<' | b'>' | b'[' | b']' | b'(' | b')' | b'/' | b'%')
}

fn skip_pdf_ws_and_comments(data: &[u8], mut i: usize) -> usize {
    while i < data.len() {
        if data[i].is_ascii_whitespace() {
            i += 1;
        } else if data[i] == b'%' {
            while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                i += 1;
            }
        } else {
            break;
        }
    }
    i
}

fn find_next_dict(data: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut i = start;
    while i + 1 < data.len() {
        if data[i] == b'%' {
            while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        if data[i] == b'(' {
            i = skip_pdf_literal_string(data, i);
            continue;
        }
        if data[i] == b'<' && data[i + 1] == b'<' {
            if let Some(bounds) = extract_balanced_dict(data, i) {
                return Some(bounds);
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

fn extract_balanced_dict(data: &[u8], start: usize) -> Option<(usize, usize)> {
    let dict_start = skip_pdf_ws_and_comments(data, start);
    if dict_start + 1 >= data.len() || &data[dict_start..dict_start + 2] != b"<<" {
        return None;
    }
    let mut i = dict_start + 2;
    let mut depth = 1usize;
    while i < data.len() {
        if data[i] == b'%' {
            while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        if data[i] == b'(' {
            i = skip_pdf_literal_string(data, i);
            continue;
        }
        if i + 1 < data.len() && data[i] == b'<' && data[i + 1] == b'<' {
            depth += 1;
            i += 2;
            continue;
        }
        if i + 1 < data.len() && data[i] == b'>' && data[i + 1] == b'>' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return Some((dict_start, i));
            }
            continue;
        }
        i += 1;
    }
    None
}

fn skip_pdf_literal_string(data: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    let mut paren_depth = 1usize;
    while i < data.len() && paren_depth > 0 {
        if data[i] == b'\\' {
            i = (i + 2).min(data.len());
            continue;
        }
        if data[i] == b'(' {
            paren_depth += 1;
        } else if data[i] == b')' {
            paren_depth -= 1;
        }
        i += 1;
    }
    i
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
