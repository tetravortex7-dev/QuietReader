use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct DocxDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for DocxDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for DocxDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

impl Document for DocxDocument {
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
        "DOCX"
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

impl DocxDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path = path.as_ref();
        let meta = std::fs::metadata(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать файл DOCX: {e}")))?;
        if meta.len() > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Размер файла DOCX превышает допустимый лимит".to_string(),
            ));
        }

        let file = File::open(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось открыть файл DOCX: {e}")))?;
        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ Word");

        Self::from_reader(file, default_title)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str) -> Result<Self, DocumentError> {
        if bytes.len() as u64 > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Размер данных DOCX превышает лимит".to_string(),
            ));
        }
        let cursor = std::io::Cursor::new(bytes);
        Self::from_reader(cursor, default_title)
    }

    fn from_reader<R: Read + std::io::Seek>(
        reader: R,
        default_title: &str,
    ) -> Result<Self, DocumentError> {
        let mut archive = zip::ZipArchive::new(reader)
            .map_err(|e| DocumentError::Corrupted(format!("Поврежденный архив DOCX: {e}")))?;

        if archive.len() > crate::document::MAX_ARCHIVE_ENTRIES {
            return Err(DocumentError::LimitExceeded(
                "Слишком много файлов в архиве DOCX".to_string(),
            ));
        }

        let mut total_unpacked = 0u64;
        let mut doc_xml_bytes: Option<Vec<u8>> = None;
        let mut core_xml_bytes: Option<Vec<u8>> = None;

        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| DocumentError::Corrupted(format!("Ошибка архива DOCX: {e}")))?;
            let name = file.name().to_string();
            if !crate::document::is_safe_archive_path(&name) {
                return Err(DocumentError::Corrupted(format!(
                    "Небезопасный путь в архиве DOCX: {name}"
                )));
            }

            if name == "word/document.xml" {
                doc_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            } else if name == "docProps/core.xml" {
                core_xml_bytes = Some(crate::document::read_bounded(
                    file,
                    crate::document::MAX_SINGLE_ENTRY_BYTES,
                    &mut total_unpacked,
                    crate::document::MAX_TOTAL_UNPACK_BYTES,
                )?);
            }
        }

        let doc_xml = doc_xml_bytes.ok_or_else(|| {
            DocumentError::Corrupted("В архиве DOCX отсутствует word/document.xml".to_string())
        })?;

        let doc_xml_str = String::from_utf8_lossy(&doc_xml);
        let metadata = core_xml_bytes
            .as_deref()
            .map(Self::parse_core_properties)
            .unwrap_or_else(|| BookMetadata {
                title: default_title.to_string(),
                author: "Неизвестный автор".to_string(),
            });

        let (chapters, toc) = Self::parse_document_xml(&doc_xml_str, &metadata.title)?;

        Ok(Self {
            metadata,
            chapters,
            toc,
        })
    }

    fn parse_core_properties(xml: &[u8]) -> BookMetadata {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let xml_str = String::from_utf8_lossy(xml);
        let mut reader = Reader::from_str(&xml_str);
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut author = String::new();
        let mut current_tag = String::new();
        let mut buf = Vec::new();

        while let Ok(event) = reader.read_event_into(&mut buf) {
            match event {
                Event::Start(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    current_tag = tag;
                }
                Event::Text(ref e) => {
                    let raw_str = e.as_ref();
                    let txt = quick_xml::escape::unescape(raw_str)
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| raw_str.to_string());
                    if current_tag == "title" && title.is_empty() {
                        title = txt.trim().to_string();
                    } else if (current_tag == "creator" || current_tag == "author")
                        && author.is_empty()
                    {
                        author = txt.trim().to_string();
                    }
                }
                Event::End(_) => {
                    current_tag.clear();
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        BookMetadata {
            title: if title.is_empty() {
                "Документ Word".to_string()
            } else {
                title
            },
            author: if author.is_empty() {
                "Неизвестный автор".to_string()
            } else {
                author
            },
        }
    }

    fn parse_document_xml(
        xml: &str,
        default_title: &str,
    ) -> Result<(Vec<BookChapter>, Vec<TocItem>), DocumentError> {
        use quick_xml::events::Event;
        use quick_xml::reader::Reader;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut chapters: Vec<BookChapter> = Vec::new();
        let mut current_chapter_title = String::new();
        let mut current_chapter_paras: Vec<String> = Vec::new();

        let mut in_p = false;
        let mut in_t = false;
        let mut current_p_text = String::new();
        let mut is_heading = false;

        let mut buf = Vec::new();

        while let Ok(event) = reader.read_event_into(&mut buf) {
            match event {
                Event::Start(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag == "p" {
                        in_p = true;
                        current_p_text.clear();
                        is_heading = false;
                    } else if in_p && tag == "pstyle" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let val = attr.value.to_ascii_lowercase();
                                if val.contains("heading")
                                    || val.contains("title")
                                    || val.contains("заголовок")
                                {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && tag == "outlinelvl" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let lvl = attr.value.trim();
                                if lvl == "0" || lvl == "1" {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && tag == "t" {
                        in_t = true;
                    }
                }
                Event::Empty(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if in_p && tag == "pstyle" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let val = attr.value.to_ascii_lowercase();
                                if val.contains("heading")
                                    || val.contains("title")
                                    || val.contains("заголовок")
                                {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && tag == "outlinelvl" {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref()).to_ascii_lowercase();
                            if key == "val" {
                                let lvl = attr.value.trim();
                                if lvl == "0" || lvl == "1" {
                                    is_heading = true;
                                }
                            }
                        }
                    } else if in_p && (tag == "br" || tag == "cr") {
                        current_p_text.push('\n');
                    } else if in_p && tag == "tab" {
                        current_p_text.push('\t');
                    }
                }
                Event::Text(ref e) => {
                    if in_t {
                        let raw = e.as_ref();
                        let txt = quick_xml::escape::unescape(raw)
                            .map(|c| c.into_owned())
                            .unwrap_or_else(|_| raw.to_string());
                        current_p_text.push_str(&txt);
                    }
                }
                Event::End(ref e) => {
                    let tag = local_name(e.name().as_ref()).to_ascii_lowercase();
                    if tag == "t" {
                        in_t = false;
                    } else if tag == "p" {
                        in_p = false;
                        let text = current_p_text.trim();
                        if !text.is_empty() {
                            if is_heading {
                                if !current_chapter_paras.is_empty()
                                    || !current_chapter_title.is_empty()
                                {
                                    let title = if current_chapter_title.is_empty() {
                                        if chapters.is_empty() {
                                            default_title.to_string()
                                        } else {
                                            format!("Раздел {}", chapters.len() + 1)
                                        }
                                    } else {
                                        std::mem::take(&mut current_chapter_title)
                                    };
                                    chapters.push(BookChapter {
                                        id: Some(format!("chap_{}", chapters.len())),
                                        title,
                                        paragraphs: std::mem::take(&mut current_chapter_paras),
                                    });
                                }
                                current_chapter_title = text.to_string();
                            } else {
                                current_chapter_paras.push(text.to_string());
                            }
                        }
                        current_p_text.clear();
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        if !current_chapter_paras.is_empty() || !current_chapter_title.is_empty() {
            let title = if current_chapter_title.is_empty() {
                if chapters.is_empty() {
                    default_title.to_string()
                } else {
                    format!("Раздел {}", chapters.len() + 1)
                }
            } else {
                current_chapter_title
            };
            chapters.push(BookChapter {
                id: Some(format!("chap_{}", chapters.len())),
                title,
                paragraphs: current_chapter_paras,
            });
        }

        if chapters.is_empty() {
            chapters.push(BookChapter {
                id: Some("chap_0".to_string()),
                title: default_title.to_string(),
                paragraphs: vec!["Документ не содержит текста.".to_string()],
            });
        }

        let mut toc = Vec::new();
        if chapters.len() > 1 {
            for (idx, chap) in chapters.iter().enumerate() {
                toc.push(TocItem {
                    id: chap.id.clone(),
                    title: chap.title.clone(),
                    chapter_index: idx,
                    children: Vec::new(),
                });
            }
        }

        Ok((chapters, toc))
    }
}

fn local_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(':') {
        &name[pos + 1..]
    } else {
        name
    }
}

#[cfg(all(test, feature = "enable_tests"))]
mod test;
