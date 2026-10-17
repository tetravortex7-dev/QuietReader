use crate::document::PageTextSpan;
use hayro::hayro_interpret::TransformExt;
use hayro::hayro_syntax::page::Page;

#[derive(Debug, Clone, PartialEq)]
pub struct GlyphEntry {
    pub c: char,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub width: f64,
}

struct SpanExtractDevice {
    glyphs: Vec<GlyphEntry>,
}

impl<'a> hayro::hayro_interpret::Device<'a> for SpanExtractDevice {
    fn draw_path(
        &mut self,
        _: &hayro::kurbo::BezPath,
        _: hayro::hayro_interpret::DrawProps<'a>,
        _: &hayro::hayro_interpret::DrawMode,
    ) {
    }
    fn push_clip_path(&mut self, _: &hayro::hayro_interpret::ClipPath) {}
    fn push_transparency_group(
        &mut self,
        _: f32,
        _: Option<hayro::hayro_interpret::SoftMask<'a>>,
        _: hayro::hayro_interpret::BlendMode,
    ) {
    }
    fn draw_image(
        &mut self,
        _: hayro::hayro_interpret::Image<'a, '_>,
        _: hayro::hayro_interpret::ImageDrawProps<'a>,
    ) {
    }
    fn pop_clip(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph_run(
        &mut self,
        glyph_run: &hayro::hayro_interpret::font::GlyphRun<'_, 'a>,
        props: hayro::hayro_interpret::DrawProps<'a>,
        _draw_mode: &hayro::hayro_interpret::DrawMode,
    ) {
        let run_transform = props.transform;
        let glyphs = glyph_run.glyphs();
        for (i, g) in glyphs.iter().enumerate() {
            let glyph_transform = run_transform * g.transform();
            let coeffs = glyph_transform.as_coeffs();
            let x = coeffs[4];
            let y = coeffs[5];

            if !x.is_finite() || !y.is_finite() {
                continue;
            }

            let scale_x = (coeffs[0] * coeffs[0] + coeffs[1] * coeffs[1]).sqrt();
            let scale_y = (coeffs[2] * coeffs[2] + coeffs[3] * coeffs[3]).sqrt();
            let basis_scale = if scale_y > 0.001 { scale_y } else { scale_x };
            let font_size = if basis_scale < 0.2 {
                basis_scale * 1000.0
            } else {
                basis_scale
            }
            .clamp(6.0, 140.0);

            let width = if i + 1 < glyphs.len() {
                let next_coeffs = (run_transform * glyphs[i + 1].transform()).as_coeffs();
                let dx = (next_coeffs[4] - x).abs();
                let dy = (next_coeffs[5] - y).abs();
                let step = if dx > dy { dx } else { dy };
                if step > 0.0 && step < font_size * 2.5 {
                    step
                } else {
                    font_size * 0.52
                }
            } else {
                font_size * 0.52
            };

            if let Some(unicode) = g.as_unicode() {
                match unicode {
                    hayro::hayro_interpret::hayro_cmap::BfString::Char(c) => {
                        if !c.is_control() || c == ' ' || c == '\t' {
                            self.glyphs.push(GlyphEntry {
                                c,
                                x,
                                y,
                                font_size,
                                width,
                            });
                        }
                    }
                    hayro::hayro_interpret::hayro_cmap::BfString::String(s) => {
                        for c in s.chars() {
                            if !c.is_control() || c == ' ' || c == '\t' {
                                self.glyphs.push(GlyphEntry {
                                    c,
                                    x,
                                    y,
                                    font_size,
                                    width,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn extract_page_spans(page: &Page<'_>) -> Option<Vec<PageTextSpan>> {
    let (width, height) = page.render_dimensions();
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let interp_cache = hayro::hayro_interpret::InterpreterCache::new();
    let interp_settings = hayro::hayro_interpret::InterpreterSettings::default();

    let mut state = hayro::hayro_interpret::Context::new(
        page.initial_transform(true).to_kurbo(),
        hayro::kurbo::Rect::new(0.0, 0.0, width as f64, height as f64),
        &interp_cache,
        page.xref(),
        interp_settings,
    );

    let mut device = SpanExtractDevice { glyphs: Vec::new() };
    hayro::hayro_interpret::interpret_page(page, &mut state, &mut device);

    let spans = group_and_sort_glyphs(device.glyphs, width, height);
    if spans.is_empty() { None } else { Some(spans) }
}

#[derive(Debug, Clone)]
pub struct RawSpan {
    pub text: String,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

pub fn group_and_sort_glyphs(
    mut glyphs: Vec<GlyphEntry>,
    width: f32,
    height: f32,
) -> Vec<PageTextSpan> {
    if glyphs.is_empty() || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    glyphs.sort_by(|a, b| a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x)));

    let mut lines: Vec<Vec<GlyphEntry>> = Vec::new();
    let mut current_line: Vec<GlyphEntry> = Vec::new();
    let mut line_y = 0.0;
    let mut line_h = 12.0;

    for g in glyphs {
        if current_line.is_empty() {
            line_y = g.y;
            line_h = g.font_size;
            current_line.push(g);
        } else {
            let dy = (g.y - line_y).abs();
            let threshold = (line_h + g.font_size) * 0.45;
            if dy <= threshold {
                current_line.push(g);
            } else {
                lines.push(std::mem::take(&mut current_line));
                line_y = g.y;
                line_h = g.font_size;
                current_line.push(g);
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    let mut raw_spans: Vec<RawSpan> = Vec::new();

    for mut line in lines {
        line.sort_by(|a, b| a.x.total_cmp(&b.x));

        let mut seg_text = String::new();
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        let mut prev_x = 0.0;
        let mut prev_w = 0.0;
        let mut first = true;

        for g in line {
            if first {
                first = false;
            } else {
                let gap = g.x - (prev_x + prev_w);
                if gap > (g.font_size * 3.5).max(30.0) {
                    let trimmed = seg_text.trim();
                    if !trimmed.is_empty() && max_x > min_x && max_y > min_y {
                        raw_spans.push(RawSpan {
                            text: trimmed.to_string(),
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                        });
                    }
                    seg_text.clear();
                    min_x = f64::MAX;
                    max_x = f64::MIN;
                    min_y = f64::MAX;
                    max_y = f64::MIN;
                } else if gap > g.font_size * 0.22 && !seg_text.ends_with(' ') {
                    seg_text.push(' ');
                }
            }

            seg_text.push(g.c);
            min_x = min_x.min(g.x);
            max_x = max_x.max(g.x + g.width);
            min_y = min_y.min(g.y - g.font_size * 0.88);
            max_y = max_y.max(g.y + g.font_size * 0.24);

            prev_x = g.x;
            prev_w = g.width;
        }

        let trimmed = seg_text.trim();
        if !trimmed.is_empty() && max_x > min_x && max_y > min_y {
            raw_spans.push(RawSpan {
                text: trimmed.to_string(),
                min_x,
                max_x,
                min_y,
                max_y,
            });
        }
    }

    let ordered = order_spans_reading_flow(raw_spans, width as f64, height as f64);
    let mut spans = Vec::with_capacity(ordered.len());
    for s in ordered {
        let norm_x0 = (s.min_x / width as f64).clamp(0.0, 1.0) as f32;
        let norm_y0 = (s.min_y / height as f64).clamp(0.0, 1.0) as f32;
        let norm_x1 = (s.max_x / width as f64).clamp(0.0, 1.0) as f32;
        let norm_y1 = (s.max_y / height as f64).clamp(0.0, 1.0) as f32;
        if norm_x1 > norm_x0 && norm_y1 > norm_y0 {
            spans.push(PageTextSpan {
                text: s.text,
                bbox: [norm_x0, norm_y0, norm_x1, norm_y1],
            });
        }
    }

    spans
}

pub fn order_spans_reading_flow(
    mut raw_spans: Vec<RawSpan>,
    width: f64,
    height: f64,
) -> Vec<RawSpan> {
    if raw_spans.len() < 4 {
        raw_spans.sort_by(|a, b| {
            a.min_y
                .total_cmp(&b.min_y)
                .then_with(|| a.min_x.total_cmp(&b.min_x))
        });
        return raw_spans;
    }

    let mid_x = width * 0.5;
    let gutter_margin = width * 0.02;
    let header_limit = height * 0.18;
    let footer_limit = height * 0.85;

    let mut left_count = 0;
    let mut right_count = 0;
    let mut middle_crossing_count = 0;

    for s in &raw_spans {
        if s.min_y >= header_limit && s.max_y <= footer_limit {
            if s.max_x < mid_x - gutter_margin {
                left_count += 1;
            } else if s.min_x > mid_x + gutter_margin {
                right_count += 1;
            } else {
                middle_crossing_count += 1;
            }
        }
    }

    if left_count >= 3 && right_count >= 3 && middle_crossing_count == 0 {
        let mut headers = Vec::new();
        let mut col_left = Vec::new();
        let mut col_right = Vec::new();
        let mut footers = Vec::new();

        for s in raw_spans {
            if s.max_y <= header_limit {
                headers.push(s);
            } else if s.min_y >= footer_limit {
                footers.push(s);
            } else if s.max_x <= mid_x {
                col_left.push(s);
            } else {
                col_right.push(s);
            }
        }

        headers.sort_by(|a, b| {
            a.min_y
                .total_cmp(&b.min_y)
                .then_with(|| a.min_x.total_cmp(&b.min_x))
        });
        col_left.sort_by(|a, b| {
            a.min_y
                .total_cmp(&b.min_y)
                .then_with(|| a.min_x.total_cmp(&b.min_x))
        });
        col_right.sort_by(|a, b| {
            a.min_y
                .total_cmp(&b.min_y)
                .then_with(|| a.min_x.total_cmp(&b.min_x))
        });
        footers.sort_by(|a, b| {
            a.min_y
                .total_cmp(&b.min_y)
                .then_with(|| a.min_x.total_cmp(&b.min_x))
        });

        let mut result =
            Vec::with_capacity(headers.len() + col_left.len() + col_right.len() + footers.len());
        result.extend(headers);
        result.extend(col_left);
        result.extend(col_right);
        result.extend(footers);
        result
    } else {
        raw_spans.sort_by(|a, b| {
            a.min_y
                .total_cmp(&b.min_y)
                .then_with(|| a.min_x.total_cmp(&b.min_x))
        });
        raw_spans
    }
}

pub fn extract_page_text(page: &Page<'_>) -> Option<String> {
    let spans = extract_page_spans(page)?;
    let lines: Vec<String> = spans.into_iter().map(|s| s.text).collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}
