use crate::document::{
    BookChapter, BookMetadata, Document, DocumentError, HasToc, TextContent, TocItem,
};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct DocDocument {
    metadata: BookMetadata,
    chapters: Vec<BookChapter>,
    toc: Vec<TocItem>,
}

impl TextContent for DocDocument {
    fn chapters(&self) -> &[BookChapter] {
        &self.chapters
    }
}

impl HasToc for DocDocument {
    fn toc(&self) -> &[TocItem] {
        &self.toc
    }
}

impl Document for DocDocument {
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
        "DOC"
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

impl DocDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DocumentError> {
        let path = path.as_ref();
        let meta = std::fs::metadata(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось прочитать файл DOC: {e}")))?;
        if meta.len() > crate::document::MAX_TOTAL_UNPACK_BYTES {
            return Err(DocumentError::LimitExceeded(
                "Размер файла DOC превышает лимит".to_string(),
            ));
        }

        let mut file = File::open(path)
            .map_err(|e| DocumentError::Io(format!("Не удалось открыть файл DOC: {e}")))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| DocumentError::Io(format!("Ошибка чтения файла DOC: {e}")))?;

        let default_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Документ Word");

        Self::from_bytes(&bytes, default_title)
    }

    pub fn from_bytes(bytes: &[u8], default_title: &str) -> Result<Self, DocumentError> {
        if bytes.len() > crate::document::MAX_TOTAL_UNPACK_BYTES as usize {
            return Err(DocumentError::LimitExceeded(
                "Размер данных DOC превышает лимит".to_string(),
            ));
        }

        let (chapters, toc, metadata) = parse_doc(bytes, default_title)?;

        Ok(Self {
            metadata,
            chapters,
            toc,
        })
    }
}

struct OleStream {
    name: String,
    start_sector: u32,
    size: usize,
}

fn parse_doc(
    bytes: &[u8],
    default_title: &str,
) -> Result<(Vec<BookChapter>, Vec<TocItem>, BookMetadata), DocumentError> {
    if bytes.len() < 512 {
        return Err(DocumentError::Corrupted(
            "Файл слишком короткий для формата DOC".to_string(),
        ));
    }

    if &bytes[0..8] != b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
        return Err(DocumentError::Corrupted(
            "Некорректная сигнатура OLE2 в файле DOC".to_string(),
        ));
    }

    let sector_shift = u16::from_le_bytes([bytes[30], bytes[31]]) as usize;
    if sector_shift < 7 || sector_shift > 14 {
        return Err(DocumentError::Corrupted(
            "Недопустимый размер сектора в файле DOC".to_string(),
        ));
    }
    let sector_size = 1 << sector_shift;

    let num_fat_sectors = u32::from_le_bytes([bytes[44], bytes[45], bytes[46], bytes[47]]) as usize;
    let first_dir_sector = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]);

    let mut fat_sector_ids = Vec::new();
    let msat_entries = num_fat_sectors.min(109);
    for idx in 0..msat_entries {
        let off = 76 + idx * 4;
        let sec = u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
        if sec < 0xFFFFFFFD {
            fat_sector_ids.push(sec);
        }
    }

    let mut fat = Vec::new();
    for &sec in &fat_sector_ids {
        let sec_offset = (sec as usize + 1) * sector_size;
        if sec_offset + sector_size <= bytes.len() {
            let sec_bytes = &bytes[sec_offset..sec_offset + sector_size];
            for chunk in sec_bytes.chunks_exact(4) {
                fat.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
            }
        }
    }

    let read_stream = |start_sec: u32, size: usize| -> Vec<u8> {
        let mut out = Vec::new();
        let mut cur = start_sec;
        let mut visited = std::collections::HashSet::new();
        while cur < 0xFFFFFFFD && out.len() < size {
            if !visited.insert(cur) {
                break;
            }
            let off = (cur as usize + 1) * sector_size;
            if off + sector_size > bytes.len() {
                break;
            }
            out.extend_from_slice(&bytes[off..off + sector_size]);
            if let Some(&next) = fat.get(cur as usize) {
                cur = next;
            } else {
                break;
            }
        }
        out.truncate(size);
        out
    };

    let dir_bytes = read_stream(first_dir_sector, 1024 * 1024);
    let mut streams: Vec<OleStream> = Vec::new();

    for chunk in dir_bytes.chunks_exact(128) {
        let name_len = u16::from_le_bytes([chunk[64], chunk[65]]) as usize;
        let obj_type = chunk[66];
        if (obj_type == 2 || obj_type == 5) && name_len > 2 && name_len <= 64 {
            let raw_chars = &chunk[0..name_len.saturating_sub(2)];
            let mut name = String::new();
            for c in raw_chars.chunks_exact(2) {
                let code = u16::from_le_bytes([c[0], c[1]]);
                if let Some(ch) = char::from_u32(code as u32) {
                    name.push(ch);
                }
            }
            let start_sec = u32::from_le_bytes([chunk[116], chunk[117], chunk[118], chunk[119]]);
            let size = u32::from_le_bytes([chunk[120], chunk[121], chunk[122], chunk[123]]) as usize;
            streams.push(OleStream {
                name,
                start_sector: start_sec,
                size,
            });
        }
    }

    let word_stream = streams.iter().find(|s| s.name == "WordDocument");
    let word_doc_bytes = if let Some(s) = word_stream {
        read_stream(s.start_sector, s.size)
    } else {
        Vec::new()
    };

    let mut doc_text = String::new();

    if word_doc_bytes.len() >= 0x0200 {
        let flags = u16::from_le_bytes([word_doc_bytes[10], word_doc_bytes[11]]);
        let use_1table = (flags & 0x0200) != 0;
        let table_name = if use_1table { "1Table" } else { "0Table" };
        let table_stream = streams.iter().find(|s| s.name == table_name);

        let table_bytes = if let Some(s) = table_stream {
            read_stream(s.start_sector, s.size)
        } else {
            Vec::new()
        };

        if word_doc_bytes.len() >= 0x01AA {
            let fc_clx = u32::from_le_bytes([
                word_doc_bytes[0x01A2],
                word_doc_bytes[0x01A3],
                word_doc_bytes[0x01A4],
                word_doc_bytes[0x01A5],
            ]) as usize;
            let lcb_clx = u32::from_le_bytes([
                word_doc_bytes[0x01A6],
                word_doc_bytes[0x01A7],
                word_doc_bytes[0x01A8],
                word_doc_bytes[0x01A9],
            ]) as usize;

            if fc_clx + lcb_clx <= table_bytes.len() && lcb_clx > 0 {
                let clx_data = &table_bytes[fc_clx..fc_clx + lcb_clx];
                let mut pos = 0;
                while pos < clx_data.len() {
                    let clxt = clx_data[pos];
                    if clxt == 0x01 {
                        if pos + 3 <= clx_data.len() {
                            let cb_grpprl = u16::from_le_bytes([clx_data[pos + 1], clx_data[pos + 2]]) as usize;
                            pos += 3 + cb_grpprl;
                        } else {
                            break;
                        }
                    } else if clxt == 0x02 {
                        if pos + 5 <= clx_data.len() {
                            let lcb = u32::from_le_bytes([
                                clx_data[pos + 1],
                                clx_data[pos + 2],
                                clx_data[pos + 3],
                                clx_data[pos + 4],
                            ]) as usize;
                            pos += 5;
                            if pos + lcb <= clx_data.len() && lcb >= 4 {
                                let pcdt = &clx_data[pos..pos + lcb];
                                let n_pieces = (lcb.saturating_sub(4)) / 12;
                                let cp_table_bytes = (n_pieces + 1) * 4;
                                if cp_table_bytes + n_pieces * 8 <= pcdt.len() {
                                    for piece_idx in 0..n_pieces {
                                        let cp1 = u32::from_le_bytes([
                                            pcdt[piece_idx * 4],
                                            pcdt[piece_idx * 4 + 1],
                                            pcdt[piece_idx * 4 + 2],
                                            pcdt[piece_idx * 4 + 3],
                                        ]) as usize;
                                        let cp2 = u32::from_le_bytes([
                                            pcdt[(piece_idx + 1) * 4],
                                            pcdt[(piece_idx + 1) * 4 + 1],
                                            pcdt[(piece_idx + 1) * 4 + 2],
                                            pcdt[(piece_idx + 1) * 4 + 3],
                                        ]) as usize;
                                        let char_count = cp2.saturating_sub(cp1);

                                        let pcd_off = cp_table_bytes + piece_idx * 8;
                                        let fc = u32::from_le_bytes([
                                            pcdt[pcd_off + 2],
                                            pcdt[pcd_off + 3],
                                            pcdt[pcd_off + 4],
                                            pcdt[pcd_off + 5],
                                        ]);

                                        let is_compressed = (fc & 0x4000_0000) != 0;
                                        let file_offset = if is_compressed {
                                            ((fc & !0x4000_0000) / 2) as usize
                                        } else {
                                            (fc & !0x4000_0000) as usize
                                        };

                                        if is_compressed {
                                            if file_offset + char_count <= word_doc_bytes.len() {
                                                let raw = &word_doc_bytes[file_offset..file_offset + char_count];
                                                let (dec, _, _) = encoding_rs::WINDOWS_1251.decode(raw);
                                                doc_text.push_str(&dec);
                                            }
                                        } else {
                                            let byte_count = char_count * 2;
                                            if file_offset + byte_count <= word_doc_bytes.len() {
                                                let raw = &word_doc_bytes[file_offset..file_offset + byte_count];
                                                for c in raw.chunks_exact(2) {
                                                    let code = u16::from_le_bytes([c[0], c[1]]);
                                                    if let Some(ch) = char::from_u32(code as u32) {
                                                        doc_text.push(ch);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    if doc_text.trim().is_empty() && !word_doc_bytes.is_empty() {
        let mut cur_run = Vec::new();
        for &b in &word_doc_bytes {
            if b >= 0x20 || b == b'\r' || b == b'\n' || b == b'\t' {
                cur_run.push(b);
            } else {
                if cur_run.len() >= 4 {
                    let (dec, _, _) = encoding_rs::WINDOWS_1251.decode(&cur_run);
                    doc_text.push_str(&dec);
                    doc_text.push('\n');
                }
                cur_run.clear();
            }
        }
        if cur_run.len() >= 4 {
            let (dec, _, _) = encoding_rs::WINDOWS_1251.decode(&cur_run);
            doc_text.push_str(&dec);
        }
    }

    let mut chapters = Vec::new();
    let current_paras = Vec::new();

    let raw_sections: Vec<&str> = doc_text.split('\x0C').collect();
    for (sec_idx, sec) in raw_sections.iter().enumerate() {
        let mut sec_paras = Vec::new();
        for line in sec.split(&['\r', '\n'][..]) {
            let clean = clean_word_text(line);
            if !clean.is_empty() {
                sec_paras.push(clean);
            }
        }
        if !sec_paras.is_empty() {
            let title = if raw_sections.len() > 1 {
                format!("Раздел {}", sec_idx + 1)
            } else {
                default_title.to_string()
            };
            chapters.push(BookChapter {
                id: Some(format!("chap_{sec_idx}")),
                title,
                paragraphs: sec_paras,
            });
        }
    }

    if chapters.is_empty() {
        chapters.push(BookChapter {
            id: Some("chap_0".to_string()),
            title: default_title.to_string(),
            paragraphs: if current_paras.is_empty() {
                vec!["Документ не содержит текста.".to_string()]
            } else {
                current_paras
            },
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
        title: default_title.to_string(),
        author: "Неизвестный автор".to_string(),
    };

    Ok((chapters, toc, metadata))
}

fn clean_word_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch == '\u{0013}' || ch == '\u{0014}' || ch == '\u{0015}' || ch == '\u{0007}' || ch == '\u{0008}' {
            continue;
        }
        if ch == '\u{0001}' || ch == '\u{0002}' || ch == '\u{0003}' || ch == '\u{0004}' || ch == '\u{0005}' {
            continue;
        }
        if ch.is_control() && ch != '\t' {
            continue;
        }
        out.push(ch);
    }
    out.trim().to_string()
}
