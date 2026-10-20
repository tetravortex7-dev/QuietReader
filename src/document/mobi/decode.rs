use mobi::Mobi;
use mobi::headers::Compression;

pub fn decompress_palmdoc(data: &[u8]) -> Vec<u8> {
    let init_cap = data.len().saturating_mul(2).min(16 * 1024 * 1024);
    let mut out = Vec::with_capacity(init_cap);
    let mut i = 0;

    while i < data.len() {
        let byte = data[i];
        i += 1;

        match byte {
            0x00 | 0x09..=0x7F => {
                out.push(byte);
            }
            0x01..=0x08 => {
                let count = byte as usize;
                let end = (i + count).min(data.len());
                out.extend_from_slice(&data[i..end]);
                i += count;
            }
            0x80..=0xBF => {
                if i < data.len() {
                    let next = data[i];
                    i += 1;
                    let b1 = byte as usize;
                    let b2 = next as usize;
                    let distance = ((b1 & 0x3F) << 5) | (b2 >> 3);
                    let length = (b2 & 0x07) + 3;

                    if distance > 0 && distance <= out.len() {
                        let start = out.len() - distance;
                        for k in 0..length {
                            let src_idx = start + (k % distance);
                            let copy_byte = out[src_idx];
                            out.push(copy_byte);
                        }
                    }
                } else {
                    out.push(byte);
                }
            }
            0xC0..=0xFF => {
                out.push(b' ');
                out.push(byte ^ 0x80);
            }
        }
    }

    out
}

pub fn get_trailing_data_size(data: &[u8], flags: u16) -> usize {
    let mut num = 0usize;
    let mut testflags = flags >> 1;
    while testflags > 0 {
        if (testflags & 1) != 0 {
            let mut bitpos = 0usize;
            let mut result = 0usize;
            let mut cur_size = data.len().saturating_sub(num);
            while cur_size > 0 {
                let v = data[cur_size - 1] as usize;
                result |= (v & 0x7F) << bitpos;
                bitpos += 7;
                cur_size -= 1;
                if (v & 0x80) != 0 || bitpos >= 28 || cur_size == 0 {
                    break;
                }
            }
            num += result;
        }
        testflags >>= 1;
    }
    if (flags & 1) != 0 && num < data.len() {
        let v = data[data.len() - num - 1] as usize;
        num += (v & 3) + 1;
    }
    num.min(data.len())
}

pub fn try_extract_kf8_boundary_raw_bytes(m: &Mobi) -> Option<Vec<u8>> {
    let raw_records = m.raw_records();
    let records = raw_records.records();

    let boundary_idx = records
        .iter()
        .position(|r| r.content == b"BOUNDARY" || r.content.starts_with(b"BOUNDARY"))?;

    let header_idx = boundary_idx + 1;
    if header_idx >= records.len() {
        return None;
    }

    let header_rec = records[header_idx].content;
    if header_rec.len() < 16 {
        return None;
    }

    let compression = u16::from_be_bytes([header_rec[0], header_rec[1]]);
    let text_count = u16::from_be_bytes([header_rec[8], header_rec[9]]) as usize;
    if text_count == 0 {
        return None;
    }

    let extra_flags = if header_rec.len() >= 244 {
        u16::from_be_bytes([header_rec[242], header_rec[243]])
    } else {
        0
    };

    let start_rec = header_idx + 1;
    let end_rec = (start_rec + text_count).min(records.len());
    let mut decompressed = Vec::new();

    for r in &records[start_rec..end_rec] {
        let trailing = get_trailing_data_size(r.content, extra_flags);
        let valid_slice = &r.content[..r.content.len().saturating_sub(trailing)];
        if compression == 2 {
            decompressed.extend_from_slice(&decompress_palmdoc(valid_slice));
        } else if compression == 1 {
            decompressed.extend_from_slice(valid_slice);
        }
    }

    if decompressed.is_empty() {
        None
    } else {
        Some(decompressed)
    }
}

pub fn extract_raw_mobi_bytes(m: &Mobi) -> (Vec<u8>, bool) {
    if let Some(kf8_bytes) = try_extract_kf8_boundary_raw_bytes(m) {
        return (kf8_bytes, true);
    }

    let raw = m.raw_records();
    let records = raw.records();
    let extra_flags = if !records.is_empty() && records[0].content.len() >= 244 {
        u16::from_be_bytes([records[0].content[242], records[0].content[243]])
    } else {
        0
    };

    let mut out = Vec::new();
    match m.compression() {
        Compression::PalmDoc => {
            for r in raw.range(m.readable_records_range()) {
                let trailing = get_trailing_data_size(r.content, extra_flags);
                let valid_slice = &r.content[..r.content.len().saturating_sub(trailing)];
                out.extend_from_slice(&decompress_palmdoc(valid_slice));
            }
        }
        Compression::No => {
            for r in raw.range(m.readable_records_range()) {
                let trailing = get_trailing_data_size(r.content, extra_flags);
                let valid_slice = &r.content[..r.content.len().saturating_sub(trailing)];
                out.extend_from_slice(valid_slice);
            }
        }
        Compression::Huff => {
            let text = m
                .content_as_string()
                .unwrap_or_else(|_| m.content_as_string_lossy());
            out.extend_from_slice(text.as_bytes());
        }
    }

    if out.is_empty() {
        let text = m
            .content_as_string()
            .unwrap_or_else(|_| m.content_as_string_lossy());
        out.extend_from_slice(text.as_bytes());
    }

    (out, false)
}

pub fn detect_meta_charset(bytes: &[u8]) -> Option<&'static encoding_rs::Encoding> {
    let limit = bytes.len().min(8192);
    let slice = &bytes[..limit];
    let lower = slice.to_ascii_lowercase();

    let pattern = b"charset=";
    let mut search_idx = 0;
    while let Some(rel) = lower[search_idx..]
        .windows(pattern.len())
        .position(|w| w == pattern)
    {
        let start = search_idx + rel + pattern.len();
        let mut cur = start;
        while cur < lower.len() && (lower[cur] == b' ' || lower[cur] == b'"' || lower[cur] == b'\'')
        {
            cur += 1;
        }
        let token_start = cur;
        while cur < lower.len()
            && lower[cur] != b'"'
            && lower[cur] != b'\''
            && lower[cur] != b';'
            && lower[cur] != b'>'
            && lower[cur] != b' '
            && lower[cur] != b'\r'
            && lower[cur] != b'\n'
        {
            cur += 1;
        }
        let token = &lower[token_start..cur];
        if let Some(enc) = encoding_rs::Encoding::for_label(token) {
            return Some(enc);
        }
        search_idx = cur;
    }

    let xml_pattern = b"encoding=";
    search_idx = 0;
    while let Some(rel) = lower[search_idx..]
        .windows(xml_pattern.len())
        .position(|w| w == xml_pattern)
    {
        let start = search_idx + rel + xml_pattern.len();
        let mut cur = start;
        while cur < lower.len() && (lower[cur] == b' ' || lower[cur] == b'"' || lower[cur] == b'\'')
        {
            cur += 1;
        }
        let token_start = cur;
        while cur < lower.len()
            && lower[cur] != b'"'
            && lower[cur] != b'\''
            && lower[cur] != b'?'
            && lower[cur] != b'>'
            && lower[cur] != b' '
            && lower[cur] != b'\r'
            && lower[cur] != b'\n'
        {
            cur += 1;
        }
        let token = &lower[token_start..cur];
        if let Some(enc) = encoding_rs::Encoding::for_label(token) {
            return Some(enc);
        }
        search_idx = cur;
    }

    None
}

pub fn encoding_from_codepage(codepage: u32) -> Option<&'static encoding_rs::Encoding> {
    match codepage {
        65001 => Some(encoding_rs::UTF_8),
        1251 => Some(encoding_rs::WINDOWS_1251),
        1252 => Some(encoding_rs::WINDOWS_1252),
        28591 => Some(encoding_rs::WINDOWS_1252),
        28595 => Some(encoding_rs::ISO_8859_5),
        20866 => Some(encoding_rs::KOI8_R),
        21866 => Some(encoding_rs::KOI8_U),
        866 => Some(encoding_rs::IBM866),
        932 => Some(encoding_rs::SHIFT_JIS),
        936 => Some(encoding_rs::GBK),
        949 => Some(encoding_rs::EUC_KR),
        950 => Some(encoding_rs::BIG5),
        _ => None,
    }
}

pub fn is_cyrillic_windows1251(bytes: &[u8]) -> bool {
    let mut total_alpha = 0usize;
    let mut cyrillic_count = 0usize;

    for &b in bytes {
        if b >= 0xC0 {
            cyrillic_count += 1;
            total_alpha += 1;
        } else if b.is_ascii_alphabetic() {
            total_alpha += 1;
        }
    }

    total_alpha > 0 && cyrillic_count * 10 >= total_alpha * 4
}

pub fn is_cp1252_mojibake(text: &str) -> bool {
    let mut mojibake_chars = 0usize;
    let mut total_alpha = 0usize;
    let mut consecutive_pairs = 0usize;
    let mut prev_mojibake = false;

    for c in text.chars() {
        let is_moj = matches!(
            c,
            '\u{00C0}'
                ..='\u{00FF}'
                    | '—'
                    | '–'
                    | '“'
                    | '”'
                    | '„'
                    | '«'
                    | '»'
                    | '…'
                    | '№'
                    | '†'
                    | '‡'
                    | '€'
                    | '‰'
                    | '‹'
                    | '›'
                    | '•'
                    | '™'
                    | '¤'
                    | '¦'
                    | '§'
                    | '©'
                    | '¬'
                    | '®'
                    | '°'
                    | '±'
                    | 'µ'
                    | '¶'
                    | '·'
        );

        if c.is_alphabetic() {
            total_alpha += 1;
        }
        if is_moj {
            mojibake_chars += 1;
            if prev_mojibake {
                consecutive_pairs += 1;
            }
        }
        prev_mojibake = is_moj;
    }

    total_alpha > 0
        && consecutive_pairs >= 3
        && (mojibake_chars * 10 >= total_alpha * 4 || consecutive_pairs >= 8)
}

pub fn repair_cp1252_mojibake(text: &str) -> String {
    let mut raw_bytes = Vec::with_capacity(text.len());
    for c in text.chars() {
        let cp = c as u32;
        if cp <= 0xFF {
            raw_bytes.push(cp as u8);
        } else {
            match c {
                '—' => raw_bytes.push(0x97),
                '–' => raw_bytes.push(0x96),
                '“' => raw_bytes.push(0x93),
                '”' => raw_bytes.push(0x94),
                '„' => raw_bytes.push(0x84),
                '«' => raw_bytes.push(0xAB),
                '»' => raw_bytes.push(0xBB),
                '…' => raw_bytes.push(0x85),
                '№' => raw_bytes.push(0xB9),
                _ => {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    raw_bytes.extend_from_slice(s.as_bytes());
                }
            }
        }
    }
    let (cow, _, _) = encoding_rs::WINDOWS_1251.decode(&raw_bytes);
    cow.into_owned()
}

pub fn resolve_mobi_encoding(bytes: &[u8], header_codepage: u32) -> &'static encoding_rs::Encoding {
    if bytes.starts_with(b"\xEF\xBB\xBF") || std::str::from_utf8(bytes).is_ok() {
        return encoding_rs::UTF_8;
    }

    if let Some(enc) = detect_meta_charset(bytes) {
        return enc;
    }

    if is_cyrillic_windows1251(bytes) {
        return encoding_rs::WINDOWS_1251;
    }

    if let Some(enc) = encoding_from_codepage(header_codepage) {
        return enc;
    }

    encoding_rs::WINDOWS_1252
}

pub fn decode_mobi_content(bytes: &[u8], header_codepage: u32) -> String {
    let clean_bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let enc = resolve_mobi_encoding(clean_bytes, header_codepage);
    let (cow, _, _) = enc.decode(clean_bytes);
    let text = cow.into_owned();
    if is_cp1252_mojibake(&text) {
        repair_cp1252_mojibake(&text)
    } else {
        text
    }
}

fn sanitize_metadata_str(s: &str) -> String {
    let filtered: String = s
        .chars()
        .filter(|&c| !c.is_control() && c != '\u{FFFD}')
        .collect();
    let mut words = filtered.split_whitespace();
    let mut res = String::new();
    if let Some(first) = words.next() {
        res.push_str(first);
        for w in words {
            res.push(' ');
            res.push_str(w);
        }
    }
    res
}

pub fn decode_metadata_string(
    bytes: &[u8],
    preferred_encoding: Option<&'static encoding_rs::Encoding>,
    header_codepage: u32,
) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let clean = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(clean) {
        let trimmed = s.trim();
        let res = if is_cp1252_mojibake(trimmed) {
            repair_cp1252_mojibake(trimmed)
        } else {
            trimmed.to_string()
        };
        return sanitize_metadata_str(&res);
    }
    let enc = preferred_encoding
        .or_else(|| {
            if is_cyrillic_windows1251(clean) {
                Some(encoding_rs::WINDOWS_1251)
            } else {
                encoding_from_codepage(header_codepage)
            }
        })
        .unwrap_or(encoding_rs::WINDOWS_1252);

    let (cow, _, _) = enc.decode(clean);
    let decoded = cow.trim().to_string();
    let res = if is_cp1252_mojibake(&decoded) {
        repair_cp1252_mojibake(&decoded)
    } else {
        decoded
    };
    sanitize_metadata_str(&res)
}
