fn is_cjk(c: char) -> bool {
    matches!(
        c,
        '\u{4e00}'..='\u{9fff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{3040}'..='\u{309f}'
            | '\u{30a0}'..='\u{30ff}'
            | '\u{ac00}'..='\u{d7af}'
    )
}

fn is_arabic(c: char) -> bool {
    matches!(
        c,
        '\u{0600}'..='\u{06ff}'
            | '\u{0750}'..='\u{077f}'
            | '\u{08a0}'..='\u{08ff}'
            | '\u{fb50}'..='\u{fdff}'
            | '\u{fe70}'..='\u{feff}'
    )
}

pub fn split_bionic_spans(text: &str) -> Vec<(&str, bool)> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let total_chars = chars.len();
    let mut i = 0;

    while i < total_chars {
        let (ch_byte, ch) = chars[i];

        if is_cjk(ch) {
            let mut cjk_count = 0;
            while i < total_chars && is_cjk(chars[i].1) {
                let (c_byte, _) = chars[i];
                let next_byte = if i + 1 < total_chars {
                    chars[i + 1].0
                } else {
                    text.len()
                };
                let is_bold = cjk_count % 2 == 0;
                result.push((&text[c_byte..next_byte], is_bold));
                cjk_count += 1;
                i += 1;
            }
            continue;
        }

        if is_arabic(ch) {
            let start_byte = ch_byte;
            while i < total_chars && (is_arabic(chars[i].1) || chars[i].1 == '\u{0640}') {
                i += 1;
            }
            let end_byte = if i < total_chars { chars[i].0 } else { text.len() };
            result.push((&text[start_byte..end_byte], true));
            continue;
        }

        if ch.is_alphabetic() {
            let start_byte = ch_byte;
            let mut word_char_count = 0;
            while i < total_chars
                && chars[i].1.is_alphabetic()
                && !is_cjk(chars[i].1)
                && !is_arabic(chars[i].1)
            {
                word_char_count += 1;
                i += 1;
            }
            let end_byte = if i < total_chars { chars[i].0 } else { text.len() };
            let word = &text[start_byte..end_byte];

            let fix = if word_char_count <= 3 {
                (word_char_count + 1) / 2
            } else {
                (word_char_count * 4 + 5) / 10
            }
            .max(1);

            let split_idx = word
                .char_indices()
                .nth(fix)
                .map(|(idx, _)| idx)
                .unwrap_or(word.len());

            if split_idx > 0 {
                result.push((&word[..split_idx], true));
            }
            if split_idx < word.len() {
                result.push((&word[split_idx..], false));
            }
            continue;
        }

        let start_byte = ch_byte;
        while i < total_chars
            && !chars[i].1.is_alphabetic()
            && !is_cjk(chars[i].1)
            && !is_arabic(chars[i].1)
        {
            i += 1;
        }
        let end_byte = if i < total_chars { chars[i].0 } else { text.len() };
        result.push((&text[start_byte..end_byte], false));
    }

    result
}
