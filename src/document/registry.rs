use crate::document::{
    ComicDocument, Document, DocumentError, EpubDocument, Fb2Document, PdfDocument, TxtDocument,
};
use std::path::Path;

/// Интерфейс обработчика конкретного формата документа (принцип OCP)
pub trait DocumentFormat: Send + Sync {
    /// Человекочитаемое имя формата
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    /// Поддерживаемые расширения файлов (без точки, в нижнем регистре)
    fn extensions(&self) -> &'static [&'static str];
    /// Проверяет, подходит ли данный формат для указанного пути (по заголовку или структуре)
    fn probe(&self, path: &Path) -> bool;
    /// Открывает документ
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
        if matches!(ext.as_str(), "cbz" | "cbr" | "cb7" | "zip" | "rar" | "7z") {
            return true;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut magic = [0u8; 8];
            if let Ok(n) = f.read(&mut magic)
                && ((n >= 4 && &magic[0..4] == b"PK\x03\x04")
                    || (n >= 7 && &magic[0..7] == b"Rar!\x1a\x07\x00")
                    || (n >= 6 && &magic[0..6] == b"7z\xbc\xaf\x27\x1c"))
            {
                return true;
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
        &["fb2", "xml"]
    }
    fn probe(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = [0u8; 512];
            if let Ok(n) = f.read(&mut buf) {
                let s = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                return s.contains("<fictionbook") || s.contains("<?xml");
            }
        }
        false
    }
    fn open(&self, path: &Path) -> Result<Box<dyn Document>, DocumentError> {
        let doc = Fb2Document::from_file(path)?;
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

/// Центральный реестр форматов документов
pub static FORMAT_REGISTRY: &[&dyn DocumentFormat] = &[
    &PdfFormat,
    &ComicFormat,
    &EpubFormat,
    &Fb2Format,
    &TxtFormat,
];

/// Открывает документ через центральный реестр форматов
pub fn open_with_registry(path: &Path) -> Result<Box<dyn Document>, DocumentError> {
    if path.is_dir() {
        return ComicFormat.open(path);
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    // 1. Сначала ищем точное совпадение по расширению в реестре
    if !ext.is_empty() {
        for fmt in FORMAT_REGISTRY {
            if fmt.extensions().contains(&ext.as_str()) {
                return fmt.open(path);
            }
        }
    }

    // 2. Если расширение архивное или нестандартное — проверяем через probe() и пробуем открыть
    let mut last_err = None;
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
