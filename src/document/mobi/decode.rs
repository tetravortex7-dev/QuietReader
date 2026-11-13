use crate::document::DocumentError;
use mobi::Mobi;
use mobi::headers::Compression;

pub const MAX_MOBI_TEXT_BYTES: usize = 300 * 1024 * 1024;

fn ensure_mobi_capacity(out: &[u8], extra: usize) -> Result<(), DocumentError> {
    if out.len().saturating_add(extra) > MAX_MOBI_TEXT_BYTES {
        return Err(DocumentError::LimitExceeded(
            crate::document::errors::decompressed_too_large_mb("MOBI/AZW", 300),
        ));
    }
    Ok(())
}

fn extend_mobi_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), DocumentError> {
    ensure_mobi_capacity(out, bytes.len())?;
    out.extend_from_slice(bytes);
    Ok(())
}

pub fn decompress_palmdoc(data: &[u8]) -> Result<Vec<u8>, DocumentError> {
    let init_cap = data.len().saturating_mul(2).min(16 * 1024 * 1024);
    let mut out = Vec::with_capacity(init_cap);
    let mut i = 0;

    while i < data.len() {
        let byte = data[i];
        i += 1;

        match byte {
            0x00 | 0x09..=0x7F => {
                ensure_mobi_capacity(&out, 1)?;
                out.push(byte);
            }
            0x01..=0x08 => {
                let count = byte as usize;
                let end = (i + count).min(data.len());
                let len = end - i;
                ensure_mobi_capacity(&out, len)?;
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
                        ensure_mobi_capacity(&out, length)?;
                        let start = out.len() - distance;
                        for k in 0..length {
                            let src_idx = start + (k % distance);
                            let copy_byte = out[src_idx];
                            out.push(copy_byte);
                        }
                    }
                } else {
                    ensure_mobi_capacity(&out, 1)?;
                    out.push(byte);
                }
            }
            0xC0..=0xFF => {
                ensure_mobi_capacity(&out, 2)?;
                out.push(b' ');
                out.push(byte ^ 0x80);
            }
        }
    }

    Ok(out)
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

pub fn try_extract_kf8_boundary_raw_bytes(m: &Mobi) -> Result<Option<Vec<u8>>, DocumentError> {
    let raw_records = m.raw_records();
    let records = raw_records.records();

    let Some(boundary_idx) = records
        .iter()
        .position(|r| r.content == b"BOUNDARY" || r.content.starts_with(b"BOUNDARY"))
    else {
        return Ok(None);
    };

    let header_idx = boundary_idx + 1;
    if header_idx >= records.len() {
        return Ok(None);
    }

    let header_rec = records[header_idx].content;
    if header_rec.len() < 16 {
        return Ok(None);
    }

    let compression = u16::from_be_bytes([header_rec[0], header_rec[1]]);
    let text_count = u16::from_be_bytes([header_rec[8], header_rec[9]]) as usize;
    if text_count == 0 {
        return Ok(None);
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
            let bytes = decompress_palmdoc(valid_slice)?;
            extend_mobi_bytes(&mut decompressed, &bytes)?;
        } else if compression == 1 {
            extend_mobi_bytes(&mut decompressed, valid_slice)?;
        }
    }

    if decompressed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(decompressed))
    }
}

pub fn extract_raw_mobi_bytes(m: &Mobi) -> Result<(Vec<u8>, bool), DocumentError> {
    if let Some(kf8_bytes) = try_extract_kf8_boundary_raw_bytes(m)? {
        return Ok((kf8_bytes, true));
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
                let bytes = decompress_palmdoc(valid_slice)?;
                extend_mobi_bytes(&mut out, &bytes)?;
            }
        }
        Compression::No => {
            for r in raw.range(m.readable_records_range()) {
                let trailing = get_trailing_data_size(r.content, extra_flags);
                let valid_slice = &r.content[..r.content.len().saturating_sub(trailing)];
                extend_mobi_bytes(&mut out, valid_slice)?;
            }
        }
        Compression::Huff => {
            let text = m
                .content_as_string()
                .unwrap_or_else(|_| m.content_as_string_lossy());
            extend_mobi_bytes(&mut out, text.as_bytes())?;
        }
    }

    if out.is_empty() {
        let text = m
            .content_as_string()
            .unwrap_or_else(|_| m.content_as_string_lossy());
        extend_mobi_bytes(&mut out, text.as_bytes())?;
    }

    Ok((out, false))
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

fn skip_tag_or_entity(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes[i] == b'<'
        && let Some(next) = bytes.get(i + 1).copied()
        && (next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?'))
    {
        let limit = (i + 512).min(bytes.len());
        for j in (i + 1)..limit {
            if bytes[j] == b'<' {
                return None;
            }
            if bytes[j] == b'>' {
                return Some(j + 1);
            }
        }
    }
    if bytes[i] == b'&' {
        let limit = (i + 14).min(bytes.len());
        for j in (i + 1)..limit {
            let b = bytes[j];
            if b == b';' && j > i + 1 {
                return Some(j + 1);
            }
            if !(b.is_ascii_alphanumeric() || b == b'#') {
                return None;
            }
        }
    }
    None
}

fn byte_cp1251_to_char(b: u8) -> Option<char> {
    match b {
        0xA8 => Some('Ё'),
        0xB8 => Some('ё'),
        0xC0..=0xFF => char::from_u32(0x0410 + (b as u32 - 0xC0)),
        _ => None,
    }
}

fn cp1252_char_to_byte(c: char) -> Option<u8> {
    match c {
        '\u{0000}'..='\u{007F}' | '\u{00A0}'..='\u{00FF}' => Some(c as u8),
        '€' => Some(0x80),
        '‚' => Some(0x82),
        'ƒ' => Some(0x83),
        '„' => Some(0x84),
        '…' => Some(0x85),
        '†' => Some(0x86),
        '‡' => Some(0x87),
        'ˆ' => Some(0x88),
        '‰' => Some(0x89),
        'Š' => Some(0x8A),
        '‹' => Some(0x8B),
        'Œ' => Some(0x8C),
        'Ž' => Some(0x8E),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '•' => Some(0x95),
        '–' => Some(0x96),
        '—' => Some(0x97),
        '˜' => Some(0x98),
        '™' => Some(0x99),
        'š' => Some(0x9A),
        '›' => Some(0x9B),
        'œ' => Some(0x9C),
        'ž' => Some(0x9E),
        'Ÿ' => Some(0x9F),
        '№' => Some(0xB9),
        _ => None,
    }
}

fn is_russian_vowel(c: char) -> bool {
    matches!(
        c,
        'а' | 'е'
            | 'ё'
            | 'и'
            | 'о'
            | 'у'
            | 'ы'
            | 'э'
            | 'ю'
            | 'я'
            | 'А'
            | 'Е'
            | 'Ё'
            | 'И'
            | 'О'
            | 'У'
            | 'Ы'
            | 'Э'
            | 'Ю'
            | 'Я'
    )
}

fn has_russian_script_profile(decoded: &str, pure_chars: usize) -> bool {
    if pure_chars < 4 {
        return false;
    }
    let mut total = 0usize;
    let mut vowels = 0usize;
    for c in decoded.chars() {
        if c != ' ' {
            total += 1;
            if is_russian_vowel(c) {
                vowels += 1;
            }
        }
    }
    if total < 4 || vowels == 0 || vowels * 5 < total {
        return false;
    }
    if total < 5 {
        return true;
    }
    let lower = decoded.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut bigrams = 0usize;
    for window in chars.windows(2) {
        let pair = [window[0], window[1]];
        if matches!(
            pair,
            ['с', 'т']
                | ['н', 'о']
                | ['т', 'о']
                | ['н', 'а']
                | ['е', 'н']
                | ['о', 'в']
                | ['н', 'и']
                | ['р', 'а']
                | ['в', 'о']
                | ['к', 'о']
                | ['п', 'р']
                | ['р', 'о']
                | ['п', 'о']
                | ['г', 'о']
                | ['о', 'с']
                | ['л', 'и']
                | ['р', 'е']
                | ['а', 'л']
                | ['е', 'р']
                | ['т', 'ь']
                | ['с', 'я']
                | ['н', 'е']
                | ['о', 'м']
                | ['о', 'т']
                | ['к', 'а']
                | ['т', 'а']
                | ['т', 'е']
                | ['л', 'а']
                | ['л', 'о']
                | ['а', 'н']
                | ['о', 'р']
                | ['д', 'е']
                | ['и', 'т']
                | ['з', 'а']
                | ['и', 'з']
                | ['е', 'л']
                | ['л', 'ь']
                | ['н', 'ы']
                | ['б', 'ы']
                | ['ь', 'н']
                | ['е', 'й']
                | ['р', 'и']
                | ['и', 'в']
                | ['в', 'е']
                | ['е', 'т']
                | ['ч', 'т']
                | ['э', 'т']
                | ['а', 'к']
                | ['д', 'л']
                | ['и', 'л']
                | ['в', 'а']
                | ['м', 'а']
                | ['д', 'а']
                | ['о', 'н']
                | ['о', 'д']
                | ['с', 'к']
                | ['ч', 'н']
                | ['и', 'н']
                | ['а', 'т']
                | ['а', 'с']
                | ['м', 'и']
                | ['м', 'у']
                | ['г', 'л']
                | ['е', 'в']
                | ['г', 'е']
                | ['о', 'л']
                | ['о', 'й']
                | ['с', 'о']
                | ['о', 'б']
                | ['а', 'р']
                | ['е', 'с']
                | ['п', 'е']
        ) {
            bigrams += 1;
        }
    }
    if total >= 20 {
        bigrams * 20 >= total
    } else {
        bigrams >= 1
    }
}

pub fn is_cyrillic_windows1251(bytes: &[u8]) -> bool {
    let mut pure_high = 0usize;
    let mut mixed_high = 0usize;
    let mut pure_tokens_ge2 = 0usize;
    let mut decoded = String::new();

    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(next_i) = skip_tag_or_entity(bytes, i) {
            i = next_i;
            continue;
        }
        let b = bytes[i];
        let is_token_byte = b.is_ascii_alphabetic() || b >= 0xC0 || b == 0xA8 || b == 0xB8;
        if !is_token_byte {
            i += 1;
            continue;
        }
        let start = i;
        let mut ascii = 0usize;
        let mut high = 0usize;
        while i < bytes.len() {
            let cur = bytes[i];
            if cur.is_ascii_alphabetic() {
                ascii += 1;
                i += 1;
            } else if cur >= 0xC0 || cur == 0xA8 || cur == 0xB8 {
                if cur >= 0xC0 {
                    high += 1;
                }
                i += 1;
            } else {
                break;
            }
        }
        if high > 0 {
            if ascii > 0 {
                mixed_high += high;
            } else {
                pure_high += high;
                if high >= 2 {
                    pure_tokens_ge2 += 1;
                }
                if !decoded.is_empty() {
                    decoded.push(' ');
                }
                for &tb in &bytes[start..i] {
                    if let Some(c) = byte_cp1251_to_char(tb) {
                        decoded.push(c);
                    }
                }
            }
        }
    }

    pure_high >= 4
        && pure_tokens_ge2 >= 1
        && (mixed_high == 0 || mixed_high * 5 <= pure_high)
        && has_russian_script_profile(&decoded, pure_high)
}

pub fn is_cp1252_mojibake(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut pure_moj_chars = 0usize;
    let mut mixed_moj_chars = 0usize;
    let mut pure_tokens_ge2 = 0usize;
    let mut total_cyrillic = 0usize;
    let mut decoded = String::new();
    let mut token_buf = String::new();

    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(next_i) = skip_tag_or_entity(bytes, i) {
            i = next_i;
            continue;
        }
        let Some(first_ch) = text[i..].chars().next() else {
            break;
        };
        let is_token_char = |c: char| {
            c.is_ascii_alphabetic()
                || ('\u{00C0}'..='\u{00FF}').contains(&c)
                || matches!(c, '¨' | '¸')
                || ('\u{0400}'..='\u{04FF}').contains(&c)
                || c.is_alphabetic()
        };
        if !is_token_char(first_ch) {
            i += first_ch.len_utf8();
            continue;
        }
        token_buf.clear();
        let mut ascii = 0usize;
        let mut moj = 0usize;
        let mut cyr = 0usize;
        while i < bytes.len() {
            let Some(c) = text[i..].chars().next() else {
                break;
            };
            if !is_token_char(c) {
                break;
            }
            if c.is_ascii_alphabetic() {
                ascii += 1;
            } else if ('\u{0400}'..='\u{04FF}').contains(&c) {
                cyr += 1;
            } else if ('\u{00C0}'..='\u{00FF}').contains(&c) {
                moj += 1;
            }
            token_buf.push(c);
            i += c.len_utf8();
        }
        total_cyrillic += cyr;
        if moj > 0 && cyr == 0 {
            if ascii > 0 {
                mixed_moj_chars += moj;
            } else {
                pure_moj_chars += moj;
                if moj >= 2 {
                    pure_tokens_ge2 += 1;
                }
                if !decoded.is_empty() {
                    decoded.push(' ');
                }
                for c in token_buf.chars() {
                    if let Some(b) = cp1252_char_to_byte(c)
                        && let Some(rc) = byte_cp1251_to_char(b)
                    {
                        decoded.push(rc);
                    }
                }
            }
        }
    }

    if total_cyrillic >= 3 && total_cyrillic * 5 >= pure_moj_chars {
        return false;
    }

    pure_moj_chars >= 4
        && pure_tokens_ge2 >= 1
        && (mixed_moj_chars == 0 || mixed_moj_chars * 5 <= pure_moj_chars)
        && has_russian_script_profile(&decoded, pure_moj_chars)
}

fn is_cp1251_mojibake_tail(c: char) -> bool {
    matches!(
        c,
        '\u{0402}'..='\u{040F}'
            | '\u{0452}'..='\u{045F}'
            | '\u{0490}'..='\u{0491}'
            | 'µ'
            | '¶'
            | '·'
            | '‚'
            | '„'
            | '…'
            | '†'
            | '‡'
            | '€'
            | '‰'
            | '‹'
            | '›'
            | '™'
            | '‘'
            | '’'
            | '“'
            | '”'
            | '•'
            | '–'
            | '—'
    )
}

pub fn cp1251_mojibake_score(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut score = 0usize;
    for window in chars.windows(2) {
        if matches!(window[0], 'Р' | 'С' | 'р' | 'с') && is_cp1251_mojibake_tail(window[1]) {
            score += 1;
        }
    }
    score
}

pub fn is_cp1251_mojibake(text: &str) -> bool {
    cp1251_mojibake_score(text) >= 3
}

pub fn repair_cp1251_mojibake(text: &str) -> Option<String> {
    if !is_cp1251_mojibake(text) {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    let mut changed = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !token.is_empty() {
                if let Some(repaired) = repair_cp1251_mojibake_token(&token) {
                    out.push_str(&repaired);
                    changed = true;
                } else {
                    out.push_str(&token);
                }
                token.clear();
            }
            out.push(ch);
        } else {
            token.push(ch);
        }
    }
    if !token.is_empty() {
        if let Some(repaired) = repair_cp1251_mojibake_token(&token) {
            out.push_str(&repaired);
            changed = true;
        } else {
            out.push_str(&token);
        }
    }
    if changed { Some(out) } else { None }
}

fn repair_cp1251_mojibake_token(token: &str) -> Option<String> {
    if cp1251_mojibake_score(token) == 0 {
        return None;
    }
    let (bytes, _, had_errors) = encoding_rs::WINDOWS_1251.encode(token);
    if had_errors {
        return None;
    }
    let repaired = std::str::from_utf8(bytes.as_ref()).ok()?;
    if repaired.is_empty() {
        return None;
    }
    Some(repaired.to_string())
}

fn flush_cp1251_bytes(raw_bytes: &mut Vec<u8>, out: &mut String) {
    if !raw_bytes.is_empty() {
        let (cow, _, _) = encoding_rs::WINDOWS_1251.decode(raw_bytes);
        out.push_str(&cow);
        raw_bytes.clear();
    }
}

pub fn repair_cp1252_mojibake(text: &str) -> String {
    if !is_cp1252_mojibake(text) {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut raw_bytes = Vec::new();
    let mut token = String::new();
    let mut i = 0usize;
    let is_token_char = |c: char| {
        c.is_ascii_alphabetic()
            || ('\u{00C0}'..='\u{00FF}').contains(&c)
            || matches!(c, '¨' | '¸')
            || ('\u{0400}'..='\u{04FF}').contains(&c)
            || c.is_alphabetic()
    };

    while i < bytes.len() {
        if let Some(next_i) = skip_tag_or_entity(bytes, i) {
            flush_cp1251_bytes(&mut raw_bytes, &mut out);
            out.push_str(&text[i..next_i]);
            i = next_i;
            continue;
        }
        let Some(ch) = text[i..].chars().next() else {
            break;
        };
        if !is_token_char(ch) {
            if let Some(b) = cp1252_char_to_byte(ch) {
                raw_bytes.push(b);
            } else {
                flush_cp1251_bytes(&mut raw_bytes, &mut out);
                out.push(ch);
            }
            i += ch.len_utf8();
            continue;
        }
        token.clear();
        let mut ascii = 0usize;
        let mut moj = 0usize;
        let mut cyr = 0usize;
        while i < bytes.len() {
            let Some(c) = text[i..].chars().next() else {
                break;
            };
            if !is_token_char(c) {
                break;
            }
            if c.is_ascii_alphabetic() {
                ascii += 1;
            } else if ('\u{0400}'..='\u{04FF}').contains(&c) {
                cyr += 1;
            } else if ('\u{00C0}'..='\u{00FF}').contains(&c) || matches!(c, '¨' | '¸') {
                moj += 1;
            }
            token.push(c);
            i += c.len_utf8();
        }
        if moj > 0 && ascii == 0 && cyr == 0 {
            for c in token.chars() {
                if let Some(b) = cp1252_char_to_byte(c) {
                    raw_bytes.push(b);
                } else {
                    flush_cp1251_bytes(&mut raw_bytes, &mut out);
                    out.push(c);
                }
            }
        } else {
            flush_cp1251_bytes(&mut raw_bytes, &mut out);
            out.push_str(&token);
        }
    }
    flush_cp1251_bytes(&mut raw_bytes, &mut out);
    out
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
    if is_cp1251_mojibake(&text)
        && let Some(repaired) = repair_cp1251_mojibake(&text)
    {
        return repaired;
    }
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
        let res = if is_cp1251_mojibake(trimmed) {
            repair_cp1251_mojibake(trimmed).unwrap_or_else(|| trimmed.to_string())
        } else if is_cp1252_mojibake(trimmed) {
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
    let res = if is_cp1251_mojibake(&decoded) {
        repair_cp1251_mojibake(&decoded).unwrap_or_else(|| decoded.clone())
    } else if is_cp1252_mojibake(&decoded) {
        repair_cp1252_mojibake(&decoded)
    } else {
        decoded
    };
    sanitize_metadata_str(&res)
}
