use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::document::{
    BookChapter, BookMetadata, DecodedImage, Document, DocumentError, HasToc, PageContent,
    PageInfo, TocItem,
};

/// Документ формата PDF с растеризацией страниц на чистом Rust (без внешних зависимостей)
pub struct PdfDocument {
    metadata: BookMetadata,
    _path: PathBuf,
    data: Arc<Vec<u8>>,
    page_count: usize,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

// Реализация Send + Sync безопасна, так как данные защищены Arc
unsafe impl Send for PdfDocument {}
unsafe impl Sync for PdfDocument {}

impl PdfDocument {
    /// Открывает PDF документ из файла
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref).map_err(|e| {
            DocumentError::Io(format!(
                "Не удалось открыть PDF файл {}: {e}",
                path_ref.display()
            ))
        })?;

        // Проверка лимита размера файла (макс. 300 МБ для безопасности памяти)
        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        const MAX_PDF_SIZE: u64 = 300 * 1024 * 1024;
        if file_len > MAX_PDF_SIZE {
            return Err(DocumentError::LimitExceeded(
                "Размер PDF файла превышает максимально допустимый лимит 300 МБ".to_string(),
            ));
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать PDF файл: {e}")))?;

        Self::from_bytes(bytes, path_ref)
    }

    /// Создает PDF документ из байтового буфера
    pub fn from_bytes(bytes: Vec<u8>, path: &Path) -> Result<Self, DocumentError> {
        if bytes.len() < 4 || !bytes.starts_with(b"%PDF") {
            // Некоторые PDF начинаются с небольшого мусора/BOM перед %PDF
            if let Some(pos) = bytes[..bytes.len().min(1024)]
                .windows(4)
                .position(|w| w == b"%PDF")
            {
                let _ = pos;
            } else {
                return Err(DocumentError::Corrupted(
                    "Файл не является валидным PDF документом (отсутствует заголовок %PDF)"
                        .to_string(),
                ));
            }
        }

        let data = Arc::new(bytes);

        let pdf = hayro::hayro_syntax::Pdf::new(data.clone()).map_err(|e| {
            DocumentError::Corrupted(format!("Ошибка парсинга PDF документа: {e:?}"))
        })?;

        let page_count = pdf.pages().len();
        if page_count == 0 {
            return Err(DocumentError::Corrupted(
                "PDF документ не содержит ни одной страницы".to_string(),
            ));
        }

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ PDF")
            .to_string();

        let meta = pdf.metadata();
        let title = meta
            .title
            .as_ref()
            .and_then(|t| decode_pdf_string(t))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(default_title);

        let author = meta
            .author
            .as_ref()
            .and_then(|a| decode_pdf_string(a))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Неизвестный автор".to_string());

        let mut chapters = Vec::with_capacity(page_count);
        let mut toc = Vec::with_capacity(page_count);
        for i in 0..page_count {
            let ch_title = format!("Страница {}", i + 1);
            chapters.push(BookChapter {
                id: Some(format!("page_{}", i + 1)),
                title: ch_title.clone(),
                paragraphs: vec![format!("Страница {} из {}", i + 1, page_count)],
            });
            toc.push(TocItem {
                id: Some(format!("page_{}", i + 1)),
                title: ch_title,
                chapter_index: i,
                children: Vec::new(),
            });
        }

        Ok(Self {
            metadata: BookMetadata { title, author },
            _path: path.to_path_buf(),
            data,
            page_count,
            chapters,
            toc,
        })
    }
}

impl HasToc for PdfDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
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

    fn page_content(&self) -> Option<&dyn PageContent> {
        Some(self)
    }

    fn toc_capability(&self) -> Option<&dyn HasToc> {
        Some(self)
    }
}

impl PageContent for PdfDocument {
    fn page_count(&self) -> usize {
        self.page_count
    }

    fn page_name(&self, index: usize) -> &str {
        let _ = index;
        ""
    }

    fn page_info(&self, _index: usize) -> Option<&PageInfo> {
        None
    }

    fn load_page_image(&self, index: usize) -> Result<DecodedImage, DocumentError> {
        if index >= self.page_count {
            return Err(DocumentError::Corrupted(format!(
                "Страница {} выходит за пределы документа",
                index + 1
            )));
        }

        // Парсим структуру документа
        let pdf = hayro::hayro_syntax::Pdf::new(self.data.clone())
            .map_err(|e| DocumentError::Corrupted(format!("Ошибка парсинга PDF: {e:?}")))?;

        let pages = pdf.pages();
        let page = pages.get(index).ok_or_else(|| {
            DocumentError::Corrupted(format!("Страница {} не найдена", index + 1))
        })?;

        // Настройки интерпретатора и растеризатора
        let interp_settings = hayro::hayro_interpret::InterpreterSettings::default();
        let render_settings = hayro::RenderSettings::default();

        // Масштабируем страницу с четким разрешением для комфортного чтения (1.5x)
        let pixmap_settings = hayro::PixmapSettings {
            x_scale: 1.5,
            y_scale: 1.5,
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
            return Err(DocumentError::Corrupted(format!(
                "Страница {} имеет нулевой размер",
                index + 1
            )));
        }

        // Конвертируем PremulRgba8 в стандартный прямой RGBA буфер
        let raw_pixels = pixmap.data();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);

        for p in raw_pixels {
            if p.a == 0 {
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
                rgba.push(0);
            } else if p.a == 255 {
                rgba.push(p.r);
                rgba.push(p.g);
                rgba.push(p.b);
                rgba.push(255);
            } else {
                // Unpremultiply alpha: c_unpremul = (c_premul * 255) / a
                let a_f = p.a as f32;
                let r = ((p.r as f32 / a_f) * 255.0).clamp(0.0, 255.0) as u8;
                let g = ((p.g as f32 / a_f) * 255.0).clamp(0.0, 255.0) as u8;
                let b = ((p.b as f32 / a_f) * 255.0).clamp(0.0, 255.0) as u8;
                rgba.push(r);
                rgba.push(g);
                rgba.push(b);
                rgba.push(p.a);
            }
        }

        Ok(DecodedImage {
            width,
            height,
            rgba,
        })
    }

    fn is_manga(&self) -> bool {
        false
    }
}

/// Декодирует строковые значения метаданных PDF (ASCII, UTF-8 или UTF-16BE с BOM)
fn decode_pdf_string(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    // UTF-16BE BOM: FE FF
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let u16_chars: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16(&u16_chars).ok();
    }

    // UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Some(s.to_string());
    }

    // PDFDocEncoding / Windows-1251 / ISO-8859-1 fallback
    let decoded: String = bytes
        .iter()
        .map(|&b| if b < 128 { b as char } else { '?' })
        .collect();

    Some(decoded)
}
