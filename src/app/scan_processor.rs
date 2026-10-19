use crate::document::DecodedImage;
use eframe::egui::{Color32, Pos2, Rect};

pub fn detect_content_bbox(img: &DecodedImage, tolerance: u8) -> Rect {
    let w = img.width as usize;
    let h = img.height as usize;
    if w < 20 || h < 20 || img.rgba.len() < w * h * 4 {
        return Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    }

    let tol = (tolerance as f32) / 255.0;

    let sample_corner = |x: usize, y: usize| -> f32 {
        let idx = (y * w + x) * 4;
        let r = img.rgba[idx] as f32;
        let g = img.rgba[idx + 1] as f32;
        let b = img.rgba[idx + 2] as f32;
        (0.299 * r + 0.587 * g + 0.114 * b) / 255.0
    };

    let cx = (w / 20).clamp(2, 20);
    let cy = (h / 20).clamp(2, 20);
    let bg_lum = (sample_corner(cx, cy)
        + sample_corner(w - 1 - cx, cy)
        + sample_corner(cx, h - 1 - cy)
        + sample_corner(w - 1 - cx, h - 1 - cy))
        * 0.25;

    let is_content_pixel = |r: u8, g: u8, b: u8, x: usize, y: usize| -> bool {
        let edge_dist_x = x.min(w - 1 - x);
        let edge_dist_y = y.min(h - 1 - y);
        let is_edge_band = edge_dist_x < 3 || edge_dist_y < 3;
        let rf = r as f32;
        let gf = g as f32;
        let bf = b as f32;
        let lum = (0.299 * rf + 0.587 * gf + 0.114 * bf) / 255.0;

        if is_edge_band && lum < 0.12 && bg_lum > 0.6 {
            return false;
        }

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let chroma = max_c - min_c;
        if chroma > 30 {
            return true;
        }

        if bg_lum > 0.6 {
            bg_lum - lum > tol
        } else {
            (lum - bg_lum).abs() > tol
        }
    };

    let col_step = (w / 500).max(1);
    let row_step = (h / 500).max(1);
    let content_threshold = (w / 250).clamp(3, 15);

    let mut min_y = 0;
    for y in (0..h).step_by(row_step) {
        let row_offset = y * w * 4;
        let mut count = 0;
        for x in (0..w).step_by(col_step) {
            let idx = row_offset + x * 4;
            if is_content_pixel(img.rgba[idx], img.rgba[idx + 1], img.rgba[idx + 2], x, y) {
                count += 1;
                if count >= content_threshold {
                    min_y = y;
                    break;
                }
            }
        }
        if count >= content_threshold {
            break;
        }
    }

    let mut max_y = h;
    for y in (min_y..h).rev().step_by(row_step) {
        let row_offset = y * w * 4;
        let mut count = 0;
        for x in (0..w).step_by(col_step) {
            let idx = row_offset + x * 4;
            if is_content_pixel(img.rgba[idx], img.rgba[idx + 1], img.rgba[idx + 2], x, y) {
                count += 1;
                if count >= content_threshold {
                    max_y = y;
                    break;
                }
            }
        }
        if count >= content_threshold {
            break;
        }
    }

    let y_start = min_y;
    let y_end = max_y.max(min_y + 1);
    let col_content_threshold = ((y_end - y_start) / 100).clamp(3, 15);

    let mut min_x = 0;
    for x in (0..w).step_by(col_step) {
        let mut count = 0;
        for y in (y_start..y_end).step_by(row_step * 2) {
            let idx = (y * w + x) * 4;
            if is_content_pixel(img.rgba[idx], img.rgba[idx + 1], img.rgba[idx + 2], x, y) {
                count += 1;
                if count >= col_content_threshold {
                    min_x = x;
                    break;
                }
            }
        }
        if count >= col_content_threshold {
            break;
        }
    }

    let mut max_x = w;
    for x in (min_x..w).rev().step_by(col_step) {
        let mut count = 0;
        for y in (y_start..y_end).step_by(row_step * 2) {
            let idx = (y * w + x) * 4;
            if is_content_pixel(img.rgba[idx], img.rgba[idx + 1], img.rgba[idx + 2], x, y) {
                count += 1;
                if count >= col_content_threshold {
                    max_x = x;
                    break;
                }
            }
        }
        if count >= col_content_threshold {
            break;
        }
    }

    if max_x <= min_x + 30 || max_y <= min_y + 30 {
        return Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    }

    let pad_x = ((max_x - min_x) as f32 * 0.015).clamp(4.0, 20.0) as usize;
    let pad_y = ((max_y - min_y) as f32 * 0.015).clamp(4.0, 20.0) as usize;

    let final_min_x = min_x.saturating_sub(pad_x);
    let final_min_y = min_y.saturating_sub(pad_y);
    let final_max_x = (max_x + pad_x).min(w);
    let final_max_y = (max_y + pad_y).min(h);

    let u0 = final_min_x as f32 / w as f32;
    let v0 = final_min_y as f32 / h as f32;
    let u1 = final_max_x as f32 / w as f32;
    let v1 = final_max_y as f32 / h as f32;

    Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1))
}

pub fn apply_smart_dark_mode(
    img: &mut DecodedImage,
    target_bg: Color32,
    target_text: Color32,
    is_dark_theme: bool,
) {
    let bg_r = target_bg.r() as f32;
    let bg_g = target_bg.g() as f32;
    let bg_b = target_bg.b() as f32;

    let tx_r = target_text.r() as f32;
    let tx_g = target_text.g() as f32;
    let tx_b = target_text.b() as f32;

    for chunk in img.rgba.chunks_exact_mut(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];
        if a == 0 {
            continue;
        }

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let chroma = max_c - min_c;
        let sat = if max_c > 0 {
            chroma as f32 / max_c as f32
        } else {
            0.0
        };

        let lum = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;

        let is_paper = lum > 0.85 && sat < 0.25;

        let color_factor = if is_paper {
            0.0
        } else if sat > 0.22 {
            1.0
        } else if sat < 0.12 {
            0.0
        } else {
            (sat - 0.12) / 0.10
        };

        let lum_clamped = ((lum - 0.08) / 0.84).clamp(0.0, 1.0);

        let (tint_r, tint_g, tint_b) = if is_dark_theme {
            let inv = 1.0 - lum_clamped;
            (
                bg_r + (tx_r - bg_r) * inv,
                bg_g + (tx_g - bg_g) * inv,
                bg_b + (tx_b - bg_b) * inv,
            )
        } else {
            (
                tx_r + (bg_r - tx_r) * lum_clamped,
                tx_g + (bg_g - tx_g) * lum_clamped,
                tx_b + (bg_b - tx_b) * lum_clamped,
            )
        };

        let orig_r = r as f32;
        let orig_g = g as f32;
        let orig_b = b as f32;

        let final_r =
            (tint_r * (1.0 - color_factor) + orig_r * color_factor).clamp(0.0, 255.0) as u8;
        let final_g =
            (tint_g * (1.0 - color_factor) + orig_g * color_factor).clamp(0.0, 255.0) as u8;
        let final_b =
            (tint_b * (1.0 - color_factor) + orig_b * color_factor).clamp(0.0, 255.0) as u8;

        chunk[0] = final_r;
        chunk[1] = final_g;
        chunk[2] = final_b;
    }
}
