use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct RtfDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for RtfDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for RtfDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

impl Document for RtfDocument {
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
        "RTF"
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

impl RtfDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path = path.as_ref();
        let meta = std::fs::metadata(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать файл RTF: {e}")))?;
        if meta.len() > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Размер файла RTF превышает лимит".to_string(),
            ));
        }

        let mut file = File::open(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось открыть файл RTF: {e}")))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Ошибка чтения файла RTF: {e}")))?;

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ RTF");

        Self::from_bytes(&bytes, default_title)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str) -> Result<Self, DocumentError> {
        if bytes.len() > crate::document::MAX_TOTAL_UNPACK_BYTES as usize {
            return Err(DocumentError::LimitExceeded(
                "Размер данных RTF превышает лимит".to_string(),
            ));
        }

        let (chapters, toc, metadata) = parse_rtf(bytes, default_title)?;

        Ok(Self {
            metadata,
            chapters,
            toc,
        })
    }
}

#[derive(Clone, Copy)]
enum Destination {
    Normal,
    Skip,
    Title,
    Author,
}

#[derive(Clone, Copy)]
struct RtfState {
    dest: Destination,
    uc: usize,
}

fn parse_rtf(
    bytes: &[u8],
    default_title: &str,
) -> Result<(Vec<BookChapter>, Vec<TocItem>, BookMetadata), DocumentError> {
    if bytes.len() < 5 {
        return Err(DocumentError::Corrupted(
            "Файл слишком короткий для RTF".to_string(),
        ));
    }

    let mut encoding: &'static encoding_rs::Encoding = encoding_rs::WINDOWS_1252;
    let mut title = String::new();
    let mut author = String::new();

    let mut chapters: Vec<BookChapter> = Vec::new();
    let mut current_paras: Vec<String> = Vec::new();
    let mut current_para = String::new();
    let mut hex_buffer: Vec<u8> = Vec::new();

    let mut state_stack: Vec<RtfState> = Vec::new();
    state_stack.push(RtfState {
        dest: Destination::Normal,
        uc: 1,
    });

    let flush_hex = |hex_buf: &mut Vec<u8>,
                     enc: &'static encoding_rs::Encoding,
                     cur_p: &mut String,
                     dest: Destination,
                     tit: &mut String,
                     aut: &mut String| {
        if hex_buf.is_empty() {
            return;
        }
        let (decoded, _, _) = enc.decode(hex_buf);
        match dest {
            Destination::Normal => cur_p.push_str(&decoded),
            Destination::Title => tit.push_str(&decoded),
            Destination::Author => aut.push_str(&decoded),
            Destination::Skip => {}
        }
        hex_buf.clear();
    };

    let flush_paragraph = |cur_p: &mut String, paras: &mut Vec<String>| {
        let trimmed = cur_p.trim();
        if !trimmed.is_empty() {
            paras.push(trimmed.to_string());
        }
        cur_p.clear();
    };

    let mut i = 0;
    let len = bytes.len();

    while i < len {
        let b = bytes[i];
        if b == b'{' {
            flush_hex(
                &mut hex_buffer,
                encoding,
                &mut current_para,
                state_stack
                    .last()
                    .map(|s| s.dest)
                    .unwrap_or(Destination::Normal),
                &mut title,
                &mut author,
            );
            let parent = state_stack.last().copied().unwrap_or(RtfState {
                dest: Destination::Normal,
                uc: 1,
            });
            state_stack.push(parent);
            i += 1;
        } else if b == b'}' {
            flush_hex(
                &mut hex_buffer,
                encoding,
                &mut current_para,
                state_stack
                    .last()
                    .map(|s| s.dest)
                    .unwrap_or(Destination::Normal),
                &mut title,
                &mut author,
            );
            if state_stack.len() > 1 {
                state_stack.pop();
            }
            i += 1;
        } else if b == b'\\' {
            flush_hex(
                &mut hex_buffer,
                encoding,
                &mut current_para,
                state_stack
                    .last()
                    .map(|s| s.dest)
                    .unwrap_or(Destination::Normal),
                &mut title,
                &mut author,
            );
            i += 1;
            if i >= len {
                break;
            }
            let next = bytes[i];
            if next == b'\\' || next == b'{' || next == b'}' {
                let current_dest = state_stack
                    .last()
                    .map(|s| s.dest)
                    .unwrap_or(Destination::Normal);
                match current_dest {
                    Destination::Normal => current_para.push(next as char),
                    Destination::Title => title.push(next as char),
                    Destination::Author => author.push(next as char),
                    Destination::Skip => {}
                }
                i += 1;
            } else if next == b'\'' {
                i += 1;
                if i + 1 < len {
                    let h1 = bytes[i];
                    let h2 = bytes[i + 1];
                    i += 2;
                    if let (Some(d1), Some(d2)) = (hex_digit(h1), hex_digit(h2)) {
                        let byte_val = (d1 << 4) | d2;
                        let current_dest = state_stack
                            .last()
                            .map(|s| s.dest)
                            .unwrap_or(Destination::Normal);
                        if !matches!(current_dest, Destination::Skip) {
                            hex_buffer.push(byte_val);
                        }
                    }
                }
            } else if next == b'*' {
                if let Some(top) = state_stack.last_mut() {
                    top.dest = Destination::Skip;
                }
                i += 1;
            } else if next == b'~' {
                current_para.push('\u{00A0}');
                i += 1;
            } else if next == b'_' {
                current_para.push('-');
                i += 1;
            } else if next.is_ascii_alphabetic() {
                let start = i;
                while i < len && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = String::from_utf8_lossy(&bytes[start..i]).to_string();

                let mut has_param = false;
                let mut param_neg = false;
                let param_start = i;
                if i < len && bytes[i] == b'-' {
                    param_neg = true;
                    i += 1;
                }
                while i < len && bytes[i].is_ascii_digit() {
                    has_param = true;
                    i += 1;
                }
                let param_val: Option<i32> = if has_param {
                    let s = String::from_utf8_lossy(&bytes[param_start..i]);
                    s.parse::<i32>().ok()
                } else if param_neg {
                    None
                } else {
                    None
                };

                if i < len && bytes[i] == b' ' {
                    i += 1;
                }

                match word.as_str() {
                    "ansicpg" => {
                        if let Some(cpg) = param_val {
                            encoding = match cpg {
                                1251 => encoding_rs::WINDOWS_1251,
                                1250 => encoding_rs::WINDOWS_1250,
                                1252 => encoding_rs::WINDOWS_1252,
                                1253 => encoding_rs::WINDOWS_1253,
                                1254 => encoding_rs::WINDOWS_1254,
                                1257 => encoding_rs::WINDOWS_1257,
                                932 => encoding_rs::SHIFT_JIS,
                                936 => encoding_rs::GBK,
                                950 => encoding_rs::BIG5,
                                949 => encoding_rs::EUC_KR,
                                65001 => encoding_rs::UTF_8,
                                _ => encoding_rs::WINDOWS_1252,
                            };
                        }
                    }
                    "fonttbl" | "colortbl" | "stylesheet" | "pict" | "object" | "header"
                    | "footer" | "footnote" => {
                        if let Some(top) = state_stack.last_mut() {
                            top.dest = Destination::Skip;
                        }
                    }
                    "title" => {
                        if let Some(top) = state_stack.last_mut() {
                            top.dest = Destination::Title;
                        }
                    }
                    "author" => {
                        if let Some(top) = state_stack.last_mut() {
                            top.dest = Destination::Author;
                        }
                    }
                    "uc" => {
                        if let Some(v) = param_val {
                            if let Some(top) = state_stack.last_mut() {
                                top.uc = v.max(0) as usize;
                            }
                        }
                    }
                    "u" => {
                        if let Some(v) = param_val {
                            let code = if v < 0 { (v + 65536) as u32 } else { v as u32 };
                            if let Some(ch) = char::from_u32(code) {
                                let current_dest = state_stack
                                    .last()
                                    .map(|s| s.dest)
                                    .unwrap_or(Destination::Normal);
                                match current_dest {
                                    Destination::Normal => current_para.push(ch),
                                    Destination::Title => title.push(ch),
                                    Destination::Author => author.push(ch),
                                    Destination::Skip => {}
                                }
                            }
                            let skip_count = state_stack.last().map(|s| s.uc).unwrap_or(1);
                            let mut skipped = 0;
                            while skipped < skip_count && i < len {
                                if bytes[i] == b'\\' {
                                    if i + 1 < len && bytes[i + 1] == b'\'' {
                                        i += 4;
                                        skipped += 1;
                                    } else {
                                        break;
                                    }
                                } else {
                                    i += 1;
                                    skipped += 1;
                                }
                            }
                        }
                    }
                    "par" | "line" => {
                        flush_paragraph(&mut current_para, &mut current_paras);
                    }
                    "page" | "sect" => {
                        flush_paragraph(&mut current_para, &mut current_paras);
                        if !current_paras.is_empty() {
                            chapters.push(BookChapter {
                                id: Some(format!("chap_{}", chapters.len())),
                                title: format!("Раздел {}", chapters.len() + 1),
                                paragraphs: std::mem::take(&mut current_paras),
                            });
                        }
                    }
                    "tab" => {
                        current_para.push('\t');
                    }
                    _ => {}
                }
            } else {
                i += 1;
            }
        } else if b == b'\r' || b == b'\n' {
            i += 1;
        } else {
            let current_dest = state_stack
                .last()
                .map(|s| s.dest)
                .unwrap_or(Destination::Normal);
            if !matches!(current_dest, Destination::Skip) {
                flush_hex(
                    &mut hex_buffer,
                    encoding,
                    &mut current_para,
                    current_dest,
                    &mut title,
                    &mut author,
                );
                match current_dest {
                    Destination::Normal => current_para.push(b as char),
                    Destination::Title => title.push(b as char),
                    Destination::Author => author.push(b as char),
                    Destination::Skip => {}
                }
            }
            i += 1;
        }
    }

    flush_hex(
        &mut hex_buffer,
        encoding,
        &mut current_para,
        state_stack
            .last()
            .map(|s| s.dest)
            .unwrap_or(Destination::Normal),
        &mut title,
        &mut author,
    );
    flush_paragraph(&mut current_para, &mut current_paras);

    if !current_paras.is_empty() {
        let chap_idx = chapters.len();
        let chap_title = if chap_idx == 0 {
            default_title.to_string()
        } else {
            format!("Раздел {}", chap_idx + 1)
        };
        chapters.push(BookChapter {
            id: Some(format!("chap_{chap_idx}")),
            title: chap_title,
            paragraphs: current_paras,
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

    let metadata = BookMetadata {
        title: if title.trim().is_empty() {
            default_title.to_string()
        } else {
            title.trim().to_string()
        },
        author: if author.trim().is_empty() {
            "Неизвестный автор".to_string()
        } else {
            author.trim().to_string()
        },
    };

    Ok((chapters, toc, metadata))
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
