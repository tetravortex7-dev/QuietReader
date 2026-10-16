use eframe::egui::{self, CursorIcon, PointerButton, Vec2};

pub fn parse_page_input(text: &str, page_count: usize) -> Option<usize> {
    text.trim()
        .parse::<usize>()
        .ok()
        .map(|p| p.saturating_sub(1).min(page_count.saturating_sub(1)))
}

pub fn zoom_in(zoom_percent: &mut f32, fit_mode: &mut String) {
    *fit_mode = "Custom".to_string();
    *zoom_percent = (*zoom_percent + 15.0).clamp(30.0, 400.0);
}

pub fn zoom_out(zoom_percent: &mut f32, fit_mode: &mut String) {
    *fit_mode = "Custom".to_string();
    *zoom_percent = (*zoom_percent - 15.0).clamp(30.0, 400.0);
}

pub fn zoom_reset(zoom_percent: &mut f32, fit_mode: &mut String, pan_offset: &mut Vec2) {
    *fit_mode = "Original".to_string();
    *zoom_percent = 100.0;
    *pan_offset = Vec2::ZERO;
}

pub fn calculate_paged_dimensions(
    orig_w: f32,
    orig_h: f32,
    avail_w: f32,
    avail_h: f32,
    fit_mode: &str,
    zoom_percent: f32,
) -> (f32, f32) {
    let zoom_factor = (zoom_percent / 100.0).clamp(0.3, 4.0);
    match fit_mode {
        "FitHeight" => {
            let h = (avail_h - 16.0).max(100.0);
            let aspect = orig_w / orig_h.max(1.0);
            (h * aspect, h)
        }
        "FitWidth" => {
            let w = (avail_w - 16.0).max(100.0);
            let aspect = orig_h / orig_w.max(1.0);
            (w, w * aspect)
        }
        "FitWindow" => {
            let scale_x = (avail_w - 16.0) / orig_w.max(1.0);
            let scale_y = (avail_h - 16.0) / orig_h.max(1.0);
            let s = scale_x.min(scale_y).max(0.05);
            (orig_w * s, orig_h * s)
        }
        _ => {
            let s = zoom_factor;
            (orig_w * s, orig_h * s)
        }
    }
}

pub fn handle_middle_mouse_scroll(
    ctx: &egui::Context,
    scroll_output_offset_y: f32,
    max_scroll_y: f32,
    pending_scroll_y: &mut Option<f32>,
) {
    let (m_down, m_delta) = ctx.input(|i| {
        (
            i.pointer.button_down(PointerButton::Middle),
            i.pointer.delta(),
        )
    });
    if m_down {
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        if m_delta.y.abs() > 0.0 {
            let base_y = pending_scroll_y.unwrap_or(scroll_output_offset_y);
            *pending_scroll_y = Some((base_y - m_delta.y).clamp(0.0, max_scroll_y));
            ctx.request_repaint();
        }
    }
}

#[derive(Debug, Clone)]
pub struct PagedGeometry {
    pub page_sizes: Vec<Vec2>,
    pub max_pan: Vec2,
    pub pad_x: f32,
}

pub fn compute_paged_geometry(
    avail_size: Vec2,
    fit_mode: &str,
    zoom_percent: f32,
    rotation: i32,
    tex_sizes: &[Vec2],
) -> PagedGeometry {
    let num_pages = tex_sizes.len().max(1) as f32;
    let target_w_per_page = (avail_size.x - 24.0) / num_pages;
    let zoom_factor = (zoom_percent / 100.0).clamp(0.3, 4.0);

    let page_sizes: Vec<Vec2> = tex_sizes
        .iter()
        .map(|&tex_size| {
            let (mut w, mut h) = match fit_mode {
                "FitHeight" => {
                    let h = (avail_size.y - 20.0).max(100.0);
                    let w = h * (tex_size.x / tex_size.y.max(1.0));
                    (w, h)
                }
                "FitWidth" => {
                    let w = (target_w_per_page - 12.0).max(100.0);
                    let h = w * (tex_size.y / tex_size.x.max(1.0));
                    (w, h)
                }
                "Original" => (tex_size.x, tex_size.y),
                "Custom" => {
                    let w = (target_w_per_page - 12.0) * zoom_factor;
                    let h = w * (tex_size.y / tex_size.x.max(1.0));
                    (w, h)
                }
                _ => {
                    let max_w = (target_w_per_page - 12.0).max(100.0);
                    let max_h = (avail_size.y - 20.0).max(100.0);
                    let scale = (max_w / tex_size.x.max(1.0)).min(max_h / tex_size.y.max(1.0));
                    (tex_size.x * scale, tex_size.y * scale)
                }
            };
            if rotation == 90 || rotation == 270 {
                std::mem::swap(&mut w, &mut h);
            }
            Vec2::new(w, h)
        })
        .collect();

    let total_w = page_sizes.iter().map(|s| s.x).sum::<f32>()
        + (page_sizes.len().saturating_sub(1) as f32 * 12.0);
    let total_h = page_sizes.iter().map(|s| s.y).fold(0.0f32, f32::max);

    let max_pan_x = ((total_w - avail_size.x) / 2.0).max(0.0);
    let max_pan_y = ((total_h - avail_size.y) / 2.0).max(0.0);
    let pad_x = ((avail_size.x - total_w) / 2.0).max(8.0);

    PagedGeometry {
        page_sizes,
        max_pan: Vec2::new(max_pan_x, max_pan_y),
        pad_x,
    }
}

pub fn handle_paged_pan(
    ctx: &egui::Context,
    pan_offset: &mut Vec2,
    max_pan_x: f32,
    max_pan_y: f32,
) {
    let (m_down, m_delta, scroll_delta) = ctx.input(|i| {
        (
            i.pointer.button_down(PointerButton::Middle),
            i.pointer.delta(),
            i.smooth_scroll_delta,
        )
    });
    if m_down {
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        *pan_offset += m_delta;
        ctx.request_repaint();
    } else if scroll_delta != Vec2::ZERO {
        *pan_offset += scroll_delta;
    }
    pan_offset.x = pan_offset.x.clamp(-max_pan_x, max_pan_x);
    pan_offset.y = pan_offset.y.clamp(-max_pan_y, max_pan_y);
}
