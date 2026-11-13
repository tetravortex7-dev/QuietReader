#[test]
fn test_doc_document_resilience_and_parsing() {
    let empty = b"";
    assert!(crate::document::DocDocument::from_bytes(empty, "Empty").is_err());

    let garbage = vec![0x12u8; 1024];
    assert!(crate::document::DocDocument::from_bytes(&garbage, "Garbage").is_err());

    let mut ole_header = vec![0u8; 512];
    ole_header[0..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    let res = crate::document::DocDocument::from_bytes(&ole_header, "TruncatedOLE");
    assert!(res.is_err());
}

#[test]
fn test_doc_story_split_footnotes_and_headers() {
    let main = "Main text\u{0002}";
    let notes = "Note text.\r";
    let headers = "Header text\rFooter text\r";
    let doc_text = format!("{main}{notes}{headers}");
    let mut fib = vec![0u8; 0x58];
    let main_len = main.chars().count() as u32;
    let notes_len = notes.chars().count() as u32;
    let headers_len = headers.chars().count() as u32;
    fib[0x4C..0x50].copy_from_slice(&main_len.to_le_bytes());
    fib[0x50..0x54].copy_from_slice(&notes_len.to_le_bytes());
    fib[0x54..0x58].copy_from_slice(&headers_len.to_le_bytes());

    let (text, footnotes, header, footer) = super::split_doc_stories(doc_text, &fib);
    assert!(text.contains("{{note:doc_fn_0}}"));
    assert_eq!(
        footnotes.get("doc_fn_0").map(String::as_str),
        Some("Note text.")
    );
    assert_eq!(header.as_deref(), Some("Header text"));
    assert_eq!(footer.as_deref(), Some("Footer text"));
}

#[test]
fn test_doc_story_split_endnotes_and_annotations() {
    let main = "Text\u{0002}\u{0002}\r";
    let notes = "Footnote text\r";
    let headers = "Head\rFoot\r";
    let annotations = "Comment text\r";
    let endnotes = "Endnote text\r";
    let doc_text = format!("{main}{notes}{headers}{annotations}{endnotes}");
    let mut fib = vec![0u8; 0x6C];
    fib[0x4C..0x50].copy_from_slice(&(main.chars().count() as u32).to_le_bytes());
    fib[0x50..0x54].copy_from_slice(&(notes.chars().count() as u32).to_le_bytes());
    fib[0x54..0x58].copy_from_slice(&(headers.chars().count() as u32).to_le_bytes());
    fib[0x5C..0x60].copy_from_slice(&(annotations.chars().count() as u32).to_le_bytes());
    fib[0x60..0x64].copy_from_slice(&(endnotes.chars().count() as u32).to_le_bytes());

    let (text, footnotes, header, footer) = super::split_doc_stories(doc_text, &fib);
    assert!(text.contains("{{note:doc_fn_0}}{{note:doc_fn_1}}"));
    assert_eq!(
        footnotes.get("doc_fn_0").map(String::as_str),
        Some("Footnote text")
    );
    assert_eq!(
        footnotes.get("doc_fn_1").map(String::as_str),
        Some("Endnote text")
    );
    assert!(!footnotes.values().any(|note| note.contains("Comment")));
    assert_eq!(header.as_deref(), Some("Head"));
    assert_eq!(footer.as_deref(), Some("Foot"));
}

#[test]
fn test_doc_fat_chain_reads_sector_bodies() {
    let sector_size = 512;
    let mut bytes = vec![0u8; sector_size * 3];
    bytes[512..516].copy_from_slice(b"S01!");
    bytes[1024..1028].copy_from_slice(b"S02!");
    let fat = vec![1u32, 0xFFFF_FFFE];
    let data = super::read_fat_chain(&bytes, sector_size, &fat, 0, sector_size + 4);
    assert_eq!(data.len(), sector_size + 4);
    assert_eq!(&data[0..4], b"S01!");
    assert_eq!(&data[sector_size..], b"S02!");
}

#[test]
fn test_doc_mini_stream_chain() {
    let mut container = vec![0u8; 64 * 3];
    container[0..8].copy_from_slice(b"AAAAAAA\n");
    container[64..72].copy_from_slice(b"BBBBBBB\n");
    container[128..136].copy_from_slice(b"CCCCCCC\n");
    let mini_fat = vec![1u32, 2, 0xFFFF_FFFE];
    let data = super::read_mini_chain(&container, &mini_fat, 0, 64 * 2 + 8, 64);
    assert_eq!(data.len(), 64 * 2 + 8);
    assert_eq!(&data[0..8], b"AAAAAAA\n");
    assert_eq!(&data[64..72], b"BBBBBBB\n");
    assert_eq!(&data[128..136], b"CCCCCCC\n");
}

#[test]
fn test_doc_mini_stream_cycle_and_bounds_are_safe() {
    let container = vec![7u8; 64 * 2];
    let mini_fat = vec![1u32, 0];
    let data = super::read_mini_chain(&container, &mini_fat, 0, 1024, 64);
    assert_eq!(data.len(), 128);

    let short = super::read_mini_chain(&container, &[], 0, 64, 64);
    assert_eq!(short.len(), 64);

    let out_of_range = super::read_mini_chain(&container, &[], 5, 64, 64);
    assert!(out_of_range.is_empty());
}

fn build_synthetic_doc_with_difat(compressed_text: &[u8], use_difat: bool) -> Vec<u8> {
    let sector_size = 512usize;
    let entries_per_fat = sector_size / 4;
    let num_fat_sectors = if use_difat { 110usize } else { 1usize };
    let total_sectors = num_fat_sectors * entries_per_fat;
    let mut ole = vec![0u8; (total_sectors + 1) * sector_size];

    ole[0..8].copy_from_slice(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1");
    ole[30..32].copy_from_slice(&9u16.to_le_bytes());
    ole[32..34].copy_from_slice(&6u16.to_le_bytes());
    ole[44..48].copy_from_slice(&(num_fat_sectors as u32).to_le_bytes());
    let dir_sector: u32 = if use_difat { 13960 } else { 1 };
    ole[48..52].copy_from_slice(&dir_sector.to_le_bytes());
    ole[56..60].copy_from_slice(&4096u32.to_le_bytes());
    ole[60..64].copy_from_slice(&0xFFFF_FFFEu32.to_le_bytes());
    ole[64..68].copy_from_slice(&0u32.to_le_bytes());

    let difat_sector: u32 = if use_difat { 109 } else { 0xFFFF_FFFE };
    let num_difat_sectors: u32 = if use_difat { 1 } else { 0 };
    ole[68..72].copy_from_slice(&difat_sector.to_le_bytes());
    ole[72..76].copy_from_slice(&num_difat_sectors.to_le_bytes());

    let mut fat = vec![0xFFFF_FFFFu32; total_sectors];
    let mut fat_sector_list: Vec<u32> = Vec::with_capacity(num_fat_sectors);
    for idx in 0..num_fat_sectors {
        let sec = if idx < 109 {
            idx as u32
        } else {
            (idx + 1) as u32
        };
        fat[sec as usize] = 0xFFFF_FFFD;
        fat_sector_list.push(sec);
    }
    if use_difat {
        fat[difat_sector as usize] = 0xFFFF_FFFC;
    }

    for i in 0..109 {
        let off = 76 + i * 4;
        let val = if i < num_fat_sectors {
            fat_sector_list[i]
        } else {
            0xFFFF_FFFF
        };
        ole[off..off + 4].copy_from_slice(&val.to_le_bytes());
    }
    if use_difat {
        let difat_off = (difat_sector as usize + 1) * sector_size;
        for slot in 0..(entries_per_fat - 1) {
            let off = difat_off + slot * 4;
            let val = fat_sector_list
                .get(109 + slot)
                .copied()
                .unwrap_or(0xFFFF_FFFF);
            ole[off..off + 4].copy_from_slice(&val.to_le_bytes());
        }
        let next_off = difat_off + (entries_per_fat - 1) * 4;
        ole[next_off..next_off + 4].copy_from_slice(&0xFFFF_FFFEu32.to_le_bytes());
    }

    let word_start: u32 = dir_sector + 1;
    let mut word_doc = vec![0u8; 0x280];
    let text_offset = 0x200usize;
    word_doc[text_offset..text_offset + compressed_text.len()].copy_from_slice(compressed_text);
    word_doc[10..12].copy_from_slice(&0x0200u16.to_le_bytes());
    let char_len = compressed_text.len() as u32;
    word_doc[0x4C..0x50].copy_from_slice(&char_len.to_le_bytes());

    let mut clx = Vec::new();
    clx.push(0x02u8);
    let pcdt_lcb = 16u32;
    clx.extend_from_slice(&pcdt_lcb.to_le_bytes());
    clx.extend_from_slice(&0u32.to_le_bytes());
    clx.extend_from_slice(&char_len.to_le_bytes());
    clx.extend_from_slice(&0u16.to_le_bytes());
    let fc_compressed = ((text_offset as u32) * 2) | 0x4000_0000;
    clx.extend_from_slice(&fc_compressed.to_le_bytes());
    clx.extend_from_slice(&0u16.to_le_bytes());

    word_doc[0x01A2..0x01A6].copy_from_slice(&0u32.to_le_bytes());
    word_doc[0x01A6..0x01AA].copy_from_slice(&(clx.len() as u32).to_le_bytes());

    let word_sectors = word_doc.len().div_ceil(sector_size);
    for s in 0..word_sectors {
        let sec = word_start + s as u32;
        fat[sec as usize] = if s + 1 < word_sectors {
            sec + 1
        } else {
            0xFFFF_FFFE
        };
        let off = (sec as usize + 1) * sector_size;
        let src_start = s * sector_size;
        let src_end = (src_start + sector_size).min(word_doc.len());
        ole[off..off + (src_end - src_start)].copy_from_slice(&word_doc[src_start..src_end]);
    }

    let table_start = word_start + word_sectors as u32;
    fat[table_start as usize] = 0xFFFF_FFFE;
    let table_off = (table_start as usize + 1) * sector_size;
    ole[table_off..table_off + clx.len()].copy_from_slice(&clx);

    fat[dir_sector as usize] = 0xFFFF_FFFE;
    let dir_off = (dir_sector as usize + 1) * sector_size;
    let write_dir_entry = |dst: &mut [u8], name: &str, obj_type: u8, start: u32, size: u32| {
        let mut units: Vec<u16> = name.encode_utf16().collect();
        units.push(0);
        for (idx, u) in units.iter().enumerate() {
            dst[idx * 2..idx * 2 + 2].copy_from_slice(&u.to_le_bytes());
        }
        dst[64..66].copy_from_slice(&((units.len() * 2) as u16).to_le_bytes());
        dst[66] = obj_type;
        dst[116..120].copy_from_slice(&start.to_le_bytes());
        dst[120..124].copy_from_slice(&size.to_le_bytes());
    };
    write_dir_entry(
        &mut ole[dir_off..dir_off + 128],
        "Root Entry",
        5,
        0xFFFF_FFFE,
        0,
    );
    write_dir_entry(
        &mut ole[dir_off + 128..dir_off + 256],
        "WordDocument",
        2,
        word_start,
        word_doc.len() as u32,
    );
    write_dir_entry(
        &mut ole[dir_off + 256..dir_off + 384],
        "1Table",
        2,
        table_start,
        clx.len() as u32,
    );

    for (fat_idx, &fat_sec) in fat_sector_list.iter().enumerate() {
        let sec_off = (fat_sec as usize + 1) * sector_size;
        for j in 0..entries_per_fat {
            let entry = fat[fat_idx * entries_per_fat + j];
            ole[sec_off + j * 4..sec_off + j * 4 + 4].copy_from_slice(&entry.to_le_bytes());
        }
    }

    ole
}

#[test]
fn test_doc_compressed_pieces_use_windows_1252() {
    use crate::document::Document;

    let cp1252 = b"Caf\xE9 \xFCber \xDF \x93Quote\x94\r";
    let bytes = build_synthetic_doc_with_difat(cp1252, false);
    let doc = crate::document::DocDocument::from_bytes(&bytes, "Win1252").unwrap();
    let para = &doc.chapters()[0].paragraphs[0];
    assert_eq!(para, "Café über ß \u{201C}Quote\u{201D}");
}

#[test]
fn test_doc_difat_chain_beyond_109_fat_sectors() {
    use crate::document::Document;

    let cp1252 = b"DIFAT chapter text \xE9\r";
    let bytes = build_synthetic_doc_with_difat(cp1252, true);
    let doc = crate::document::DocDocument::from_bytes(&bytes, "DifatDoc").unwrap();
    assert_eq!(doc.chapters()[0].paragraphs[0], "DIFAT chapter text é");
}

#[test]
fn test_doc_difat_cycle_protection() {
    let sector_size = 128usize;
    let mut ole = vec![0u8; sector_size * 5];
    let next_off = (1usize + 1) * sector_size + sector_size - 4;
    ole[next_off..next_off + 4].copy_from_slice(&2u32.to_le_bytes());
    let next_off2 = (2usize + 1) * sector_size + sector_size - 4;
    ole[next_off2..next_off2 + 4].copy_from_slice(&1u32.to_le_bytes());

    let ids = super::collect_fat_sector_ids(&ole, sector_size, 300, 1, 10);
    assert_eq!(ids.len(), 109 + 2 * 31);
}
