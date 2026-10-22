use crate::document::comic::zip_contains_supported_image;
use crate::document::fb2::zip_contains_fb2;
use crate::document::{
    ComicDocument, DjvuDocument, DocDocument, Document, DocumentError, DocxDocument, EpubDocument,
    Fb2Document, MobiDocument, PdfDocument, RtfDocument, TxtDocument,
};
use std::path::Path;

pub trait DocumentFormat: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn probe(&self, path: &Path) -> bool;
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError>;
}

pub struct PdfFormat;
impl DocumentFormat for PdfFormat {
    fn name(&self) -> &'static str {
        "PDF"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
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
        let doc = PdfDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct DjvuFormat;
impl DocumentFormat for DjvuFormat {
    fn name(&self) -> &'static str {
        "DjVu"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["djvu", "djv"]
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
        &["fb2.zip", "zip"]
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
        &["cbz", "cbr", "cb7"]
    }
    fn probe(&self, path: &Path) -> bool {
        if path.is_dir() {
            return true;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        if matches!(ext.as_str(), "cbz" | "cbr" | "cb7" | "rar" | "7z") {
            return true;
        }
        if ext == "zip" {
            return zip_contains_supported_image(path);
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut magic = [0u8; 8];
            if let Ok(n) = f.read(&mut magic) {
                if n >= 4 && &magic[0..4] == b"PK\x03\x04" {
                    return zip_contains_supported_image(path);
                }
                if (n >= 7 && &magic[0..7] == b"Rar!\x1a\x07\x00")
                    || (n >= 8 && &magic[0..8] == b"Rar!\x1a\x07\x01\x00")
                    || (n >= 6 && &magic[0..6] == b"7z\xbc\xaf\x27\x1c")
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
        &["epub"]
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
        &["fb2"]
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
        &["docx"]
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        if ext == "docx" {
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
        &["rtf"]
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
        &["doc", "dot"]
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
        &["txt", "md", "markdown", "log"]
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        self.extensions().contains(&ext.as_str())
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = TxtDocument::from_file(path)?;
        Ok(Box::new(doc))
    }
}

pub struct MobiFormat;
impl DocumentFormat for MobiFormat {
    fn name(&self) -> &'static str {
        "MOBI / AZW / AZW3"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["mobi", "prc", "azw", "azw3"]
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        if matches!(ext.as_str(), "mobi" | "prc" | "azw" | "azw3") {
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
    &MobiFormat,
    &DocxFormat,
    &RtfFormat,
    &DocFormat,
    &TxtFormat,
];

fn extension_lower(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".fb2.zip") {
        return "fb2.zip".to_string();
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
}

pub fn open_with_registry(path: &Path) -> Result<Box<dyn Document>, DocumentError> {
    if path.is_dir() {
        return ComicFormat.open(path);
    }

    let ext = extension_lower(path);
    let mut last_err = None;

    if !ext.is_empty() {
        for fmt in FORMAT_REGISTRY {
            if fmt.extensions().contains(&ext.as_str()) {
                match fmt.open(path) {
                    Ok(doc) => return Ok(doc),
                    Err(e) => last_err = Some(e),
                }
            }
        }
    }

    for fmt in FORMAT_REGISTRY {
        if fmt.probe(path) {
            match fmt.open(path) {
                Ok(doc) => return Ok(doc),
                Err(e) => last_err = Some(e),
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        DocumentError::Unsupported(format!("Неподдерживаемый формат файла: {}", path.display()))
    }))
}
