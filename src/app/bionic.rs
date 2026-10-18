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

pub fn render_reading_ruler(
    ui: &eframe::egui::Ui,
    viewport: eframe::egui::Rect,
    font_size: f32,
    theme: &crate::app::theme::ReaderTheme,
    settings: &crate::settings::AppSettings,
) {
    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let pointer_y = match pointer_pos {
        Some(pos) if viewport.contains(pos) => pos.y,
        Some(pos) => pos.y.clamp(viewport.min.y + 40.0, viewport.max.y - 40.0),
        None => viewport.center().y,
    };

    let line_height = font_size * 1.5;
    let lines_count = settings.reading_ruler_lines.clamp(1, 5) as f32;
    let ruler_height = (line_height * lines_count).max(32.0);

    let half = ruler_height * 0.5;
    let band_top = (pointer_y - half).clamp(viewport.min.y, viewport.max.y);
    let band_bot = (pointer_y + half).clamp(viewport.min.y, viewport.max.y);

    let dim_alpha = ((settings.reading_ruler_opacity.clamp(0.1, 0.85)) * 255.0) as u8;
    let dim_color = if theme.is_dark() {
        eframe::egui::Color32::from_black_alpha(dim_alpha)
    } else {
        eframe::egui::Color32::from_rgba_unmultiplied(20, 20, 25, dim_alpha)
    };

    let painter = ui.painter();

    if band_top > viewport.min.y {
        let top_rect =
            eframe::egui::Rect::from_min_max(viewport.min, eframe::egui::pos2(viewport.max.x, band_top));
        painter.rect_filled(top_rect, 0.0, dim_color);
    }

    if band_bot < viewport.max.y {
        let bot_rect =
            eframe::egui::Rect::from_min_max(eframe::egui::pos2(viewport.min.x, band_bot), viewport.max);
        painter.rect_filled(bot_rect, 0.0, dim_color);
    }

    let guide_stroke = eframe::egui::Stroke::new(1.5, theme.accent_color().gamma_multiply(0.75));
    painter.line_segment(
        [
            eframe::egui::pos2(viewport.min.x, band_top),
            eframe::egui::pos2(viewport.max.x, band_top),
        ],
        guide_stroke,
    );
    painter.line_segment(
        [
            eframe::egui::pos2(viewport.min.x, band_bot),
            eframe::egui::pos2(viewport.max.x, band_bot),
        ],
        guide_stroke,
    );
}
