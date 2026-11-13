pub mod catalog;
pub use catalog::*;

use crate::document::comic::zip_contains_supported_image;
use crate::document::fb2::zip_contains_fb2;
use crate::document::{
    ComicDocument, DjvuDocument, DocDocument, Document, DocumentError, DocxDocument, EpubDocument,
    Fb2Document, HtmlDocument, MobiDocument, PdfDocument, RtfDocument, TxtDocument,
};
use std::path::Path;
use std::sync::LazyLock;

pub trait DocumentFormat: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn probe(&self, path: &Path) -> bool;
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError>;
    fn open_with_password(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Box<dyn Document>, DocumentError> {
        let _ = password;
        self.open(path)
    }
    fn open_with_options(
        &self,
        path: &Path,
        encoding: Option<&str>,
        password: Option<&str>,
    ) -> Result<Box<dyn Document>, DocumentError> {
        let _ = encoding;
        self.open_with_password(path, password)
    }
}

pub struct PdfFormat;
impl DocumentFormat for PdfFormat {
    fn name(&self) -> &'static str {
        "PDF"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Pdf));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 64];
            if let Ok(n) = f.read(&mut buf) {
                return buf[..n].windows(4).any(|w| w == b"%PDF");
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        self.open_with_password(path, None)
    }
    fn open_with_password(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Box<dyn Document>, DocumentError> {
        let doc = PdfDocument::from_file_with_password(path, password)?;
        Ok(Box::new(doc))
    }
}

pub struct DjvuFormat;
impl DocumentFormat for DjvuFormat {
    fn name(&self) -> &'static str {
        "DjVu"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Djvu));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 64];
            if let Ok(n) = f.read(&mut buf) {
                return buf[..n].windows(8).any(|w| w == b"AT&TFORM");
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = DjvuDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct Fb2ZipFormat;
impl DocumentFormat for Fb2ZipFormat {
    fn name(&self) -> &'static str {
        "FB2.ZIP"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Fb2Zip));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        path.is_file() && zip_contains_fb2(path)
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = Fb2Document::from_zip_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct ComicFormat;
impl DocumentFormat for ComicFormat {
    fn name(&self) -> &'static str {
        "Comic / Manga"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Comic));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if path.is_dir() {
            return true;
        }
        let ext = extension_lower(path);
        if ext == "zip" {
            return zip_contains_supported_image(path);
        }
        if self.extensions().contains(&ext.as_str()) {
            return true;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut magic = [0u8; 262];
            if let Ok(n) = f.read(&mut magic) {
                if n >= 4 && &magic[0..4] == b"PK\x03\x04" {
                    return zip_contains_supported_image(path);
                }
                if (n >= 7 && &magic[0..7] == b"Rar!\x1a\x07\x00")
                    || (n >= 8 && &magic[0..8] == b"Rar!\x1a\x07\x01\x00")
                    || (n >= 6 && &magic[0..6] == b"7z\xbc\xaf\x27\x1c")
                    || (n >= 262 && &magic[257..262] == b"ustar")
                {
                    return true;
                }
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = ComicDocument::from_path(path)?;
        Ok(Box::new(doc))
    }
}

pub struct EpubFormat;
impl DocumentFormat for EpubFormat {
    fn name(&self) -> &'static str {
        "EPUB"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Epub));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut magic = [0u8; 4];
            if let Ok(4) = f.read(&mut magic) {
                return &magic == b"PK\x03\x04";
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = EpubDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct Fb2Format;
impl DocumentFormat for Fb2Format {
    fn name(&self) -> &'static str {
        "FB2"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Fb2));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 4096];
            if let Ok(n) = f.read(&mut buf) {
                let s = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                return s.contains("<fictionbook");
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = Fb2Document::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct DocxFormat;
impl DocumentFormat for DocxFormat {
    fn name(&self) -> &'static str {
        "DOCX"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Docx));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = extension_lower(path);
        if self.extensions().contains(&ext.as_str()) {
            return true;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut magic = [0u8; 4];
            if let Ok(4) = f.read(&mut magic) {
                return &magic == b"PK\x03\x04";
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = DocxDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct RtfFormat;
impl DocumentFormat for RtfFormat {
    fn name(&self) -> &'static str {
        "RTF"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Rtf));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 16];
            if let Ok(n) = f.read(&mut buf) {
                let s = String::from_utf8_lossy(&buf[..n]);
                return s.starts_with("{\\rtf");
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = RtfDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct DocFormat;
impl DocumentFormat for DocFormat {
    fn name(&self) -> &'static str {
        "DOC"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Doc));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut magic = [0u8; 8];
            if let Ok(8) = f.read(&mut magic) {
                return &magic == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1";
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = DocDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct TxtFormat;
impl DocumentFormat for TxtFormat {
    fn name(&self) -> &'static str {
        "TXT / MD / LOG"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Txt));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = extension_lower(path);
        self.extensions().contains(&ext.as_str())
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        self.open_with_options(path, None, None)
    }
    fn open_with_options(
        &self,
        path: &Path,
        encoding: Option<&str>,
        _password: Option<&str>,
    ) -> Result<Box<dyn Document>, DocumentError> {
        let doc = TxtDocument::from_file_with_encoding(path, encoding)?;
        Ok(Box::new(doc))
    }
}

pub struct HtmlFormat;
impl DocumentFormat for HtmlFormat {
    fn name(&self) -> &'static str {
        "HTML"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Html));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = extension_lower(path);
        if self.extensions().contains(&ext.as_str()) {
            return true;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 2048];
            if let Ok(n) = f.read(&mut buf) {
                let head = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                return head.contains("<html")
                    || head.contains("<!doctype html")
                    || head.contains("<body");
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        self.open_with_options(path, None, None)
    }
    fn open_with_options(
        &self,
        path: &Path,
        encoding: Option<&str>,
        _password: Option<&str>,
    ) -> Result<Box<dyn Document>, DocumentError> {
        let doc = HtmlDocument::from_file_with_encoding(path, encoding)?;
        Ok(Box::new(doc))
    }
}

pub struct MobiFormat;
impl DocumentFormat for MobiFormat {
    fn name(&self) -> &'static str {
        "MOBI / AZW / AZW3"
    }
    fn extensions(&self) -> &'static [&'static str] {
        static EXTS: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| collect_backend_extensions(FormatBackend::Mobi));
        &EXTS
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = extension_lower(path);
        if self.extensions().contains(&ext.as_str()) {
            return true;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 68];
            if let Ok(n) = f.read(&mut buf)
                && n >= 68
                && (&buf[60..68] == b"BOOKMOBI"
                    || &buf[60..68] == b"TEXtRECD"
                    || &buf[64..68] == b"MOBI"
                    || &buf[60..64] == b"BOOK")
            {
                return true;
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = MobiDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub static FORMAT_REGISTRY: &[&dyn DocumentFormat] = &[
    &PdfFormat,
    &DjvuFormat,
    &Fb2ZipFormat,
    &ComicFormat,
    &EpubFormat,
    &Fb2Format,
    &HtmlFormat,
    &MobiFormat,
    &DocxFormat,
    &RtfFormat,
    &DocFormat,
    &TxtFormat,
];

pub fn is_supported_document_path(path: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return false;
    }
    let ext = extension_lower(path);
    if ext.is_empty() {
        return false;
    }
    FORMAT_REGISTRY
        .iter()
        .any(|format| format.extensions().contains(&ext.as_str()))
}

pub fn open_with_registry_and_encoding_and_password(
    path: &Path,
    encoding: Option<&str>,
    password: Option<&str>,
) -> Result<Box<dyn Document>, DocumentError> {
    if path.is_dir() {
        return ComicFormat.open_with_options(path, encoding, password);
    }

    let ext = extension_lower(path);
    let mut last_err = None;

    if !ext.is_empty() {
        for fmt in FORMAT_REGISTRY {
            if fmt.extensions().contains(&ext.as_str()) {
                match fmt.open_with_options(path, encoding, password) {
                    Ok(doc) => return Ok(doc),
                    Err(e) => {
                        if matches!(e, DocumentError::PasswordRequired) {
                            return Err(e);
                        }
                        last_err = Some(e);
                    }
                }
            }
        }
    }

    for fmt in FORMAT_REGISTRY {
        if fmt.probe(path) {
            match fmt.open_with_options(path, encoding, password) {
                Ok(doc) => return Ok(doc),
                Err(e) => {
                    if matches!(e, DocumentError::PasswordRequired) {
                        return Err(e);
                    }
                    last_err = Some(e);
                }
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        DocumentError::Unsupported(crate::document::errors::unsupported_format(
            &path.display().to_string(),
        ))
    }))
}

#[allow(dead_code)]
pub fn open_with_registry_and_encoding(
    path: &Path,
    encoding: Option<&str>,
) -> Result<Box<dyn Document>, DocumentError> {
    open_with_registry_and_encoding_and_password(path, encoding, None)
}

pub fn open_with_registry(path: &Path) -> Result<Box<dyn Document>, DocumentError> {
    open_with_registry_and_encoding_and_password(path, None, None)
}

#[allow(dead_code)]
pub fn open_with_registry_and_password(
    path: &Path,
    password: Option<&str>,
) -> Result<Box<dyn Document>, DocumentError> {
    open_with_registry_and_encoding_and_password(path, None, password)
}
