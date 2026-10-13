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
        pan_offset.x = pan_offset.x.clamp(-max_pan_x, max_pan_x);
        pan_offset.y = pan_offset.y.clamp(-max_pan_y, max_pan_y);
        ctx.request_repaint();
    } else if scroll_delta != Vec2::ZERO {
        *pan_offset += scroll_delta;
        pan_offset.x = pan_offset.x.clamp(-max_pan_x, max_pan_x);
        pan_offset.y = pan_offset.y.clamp(-max_pan_y, max_pan_y);
    }
}
