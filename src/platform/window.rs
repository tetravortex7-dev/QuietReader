pub fn is_window_accessible_in_work_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
) -> bool {
    if width <= 0 || height <= 0 || work_right <= work_left || work_bottom <= work_top {
        return false;
    }

    let grab_left = x + 16;
    let grab_right = (x + width - 16).max(grab_left + 64);
    let grab_top = y;
    let grab_bottom = y + 40;

    let grab_overlap_w = grab_right.min(work_right) - grab_left.max(work_left);
    let grab_overlap_h = grab_bottom.min(work_bottom) - grab_top.max(work_top);

    let body_overlap_w = (x + width).min(work_right) - x.max(work_left);
    let body_overlap_h = (y + height).min(work_bottom) - y.max(work_top);

    y >= work_top - 8
        && grab_overlap_w >= 80
        && grab_overlap_h >= 20
        && body_overlap_w >= 120
        && body_overlap_h >= 80
}

pub fn reposition_window_into_work_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
) -> (i32, i32, i32, i32) {
    let work_w = (work_right - work_left).max(400);
    let work_h = (work_bottom - work_top).max(300);

    let new_w = width.clamp(400.min(work_w), work_w);
    let new_h = height.clamp(300.min(work_h), work_h);

    let completely_outside =
        x + new_w <= work_left || x >= work_right || y + new_h <= work_top || y >= work_bottom;

    if completely_outside {
        let centered_x = work_left + (work_w - new_w) / 2;
        let centered_y = work_top + (work_h - new_h) / 2;
        (centered_x, centered_y, new_w, new_h)
    } else {
        let clamped_x = x.clamp(work_left, (work_right - new_w).max(work_left));
        let clamped_y = y.clamp(work_top, (work_bottom - new_h).max(work_top));
        (clamped_x, clamped_y, new_w, new_h)
    }
}

#[cfg(windows)]
mod windows_fullscreen {
    use super::{is_window_accessible_in_work_area, reposition_window_into_work_area};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    const MONITOR_DEFAULTTONULL: u32 = 0;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_OVERLAPPEDWINDOW: i32 = 0x00CF0000;
    const WS_POPUP: i32 = -2147483648;
    const WS_VISIBLE: i32 = 0x10000000;
    const WS_CLIPSIBLINGS: i32 = 0x04000000;
    const WS_CLIPCHILDREN: i32 = 0x02000000;
    const WS_MAXIMIZE: i32 = 0x01000000;
    const WS_EX_WINDOWEDGE: i32 = 0x00000100;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOOWNERZORDER: u32 = 0x0200;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_SHOWWINDOW: u32 = 0x0040;
    const SW_MAXIMIZE: i32 = 3;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct MonitorInfo {
        cb_size: u32,
        rc_monitor: Rect,
        rc_work: Rect,
        dw_flags: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct WindowPlacement {
        length: u32,
        flags: u32,
        show_cmd: u32,
        pt_min_position: Point,
        pt_max_position: Point,
        rc_normal_position: Rect,
    }

    #[derive(Clone, Copy)]
    struct SavedWindowInfo {
        style: i32,
        ex_style: i32,
        rect: Rect,
        maximized: bool,
    }

    static SAVED_WINDOW_INFO: Mutex<Option<SavedWindowInfo>> = Mutex::new(None);
    static IS_FULLSCREEN: AtomicBool = AtomicBool::new(false);

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MonitorFromRect(lprc: *const Rect, dw_flags: u32) -> isize;
        fn MonitorFromWindow(hwnd: isize, dw_flags: u32) -> isize;
        fn GetMonitorInfoW(hmonitor: isize, lpmi: *mut MonitorInfo) -> i32;
        fn GetWindowLongW(hwnd: isize, n_index: i32) -> i32;
        fn SetWindowLongW(hwnd: isize, n_index: i32, dw_new_long: i32) -> i32;
        fn GetWindowRect(hwnd: isize, lp_rect: *mut Rect) -> i32;
        fn GetWindowPlacement(hwnd: isize, lpwndpl: *mut WindowPlacement) -> i32;
        fn IsZoomed(hwnd: isize) -> i32;
        fn IsWindow(hwnd: isize) -> i32;
        fn ShowWindow(hwnd: isize, n_cmd_show: i32) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            hwnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
    }

    pub fn optimize_window_for_rendering(_hwnd: isize) {}

    pub fn is_window_fullscreen() -> bool {
        IS_FULLSCREEN.load(Ordering::Relaxed)
    }

    fn apply_fullscreen_win32(hwnd: isize, fullscreen: bool) {
        unsafe {
            if hwnd == 0 || IsWindow(hwnd) == 0 {
                return;
            }
            if fullscreen {
                let style = GetWindowLongW(hwnd, GWL_STYLE);
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let maximized = IsZoomed(hwnd) != 0;

                let mut rect = Rect::default();
                if maximized {
                    let mut wp = WindowPlacement {
                        length: std::mem::size_of::<WindowPlacement>() as u32,
                        ..Default::default()
                    };
                    if GetWindowPlacement(hwnd, &mut wp) != 0 {
                        rect = wp.rc_normal_position;
                    } else {
                        GetWindowRect(hwnd, &mut rect);
                    }
                } else {
                    GetWindowRect(hwnd, &mut rect);
                }

                if let Ok(mut guard) = SAVED_WINDOW_INFO.lock() {
                    *guard = Some(SavedWindowInfo {
                        style,
                        ex_style,
                        rect,
                        maximized,
                    });
                }

                let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if mon != 0 && GetMonitorInfoW(mon, &mut mi) != 0 {
                    let new_style = (style & !(WS_OVERLAPPEDWINDOW | WS_MAXIMIZE))
                        | WS_POPUP
                        | WS_VISIBLE
                        | WS_CLIPSIBLINGS
                        | WS_CLIPCHILDREN;
                    let new_ex_style = ex_style & !WS_EX_WINDOWEDGE;

                    SetWindowLongW(hwnd, GWL_STYLE, new_style);
                    SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style);

                    let w = mi.rc_monitor.right - mi.rc_monitor.left;
                    let h = (mi.rc_monitor.bottom - mi.rc_monitor.top) + 1;

                    SetWindowPos(
                        hwnd,
                        0,
                        mi.rc_monitor.left,
                        mi.rc_monitor.top,
                        w,
                        h,
                        SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                    );
                }
            } else {
                let saved = SAVED_WINDOW_INFO.lock().ok().and_then(|mut g| g.take());
                if let Some(info) = saved {
                    let restored_style = (info.style & !WS_MAXIMIZE) | WS_VISIBLE;
                    SetWindowLongW(hwnd, GWL_STYLE, restored_style);
                    SetWindowLongW(hwnd, GWL_EXSTYLE, info.ex_style);

                    let w = (info.rect.right - info.rect.left).max(600);
                    let h = (info.rect.bottom - info.rect.top).max(400);

                    SetWindowPos(
                        hwnd,
                        0,
                        info.rect.left,
                        info.rect.top,
                        w,
                        h,
                        SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                    );

                    if info.maximized {
                        ShowWindow(hwnd, SW_MAXIMIZE);
                    }
                }
            }
        }
    }

    pub fn set_window_fullscreen(hwnd: isize, fullscreen: bool) -> bool {
        if hwnd == 0 || unsafe { IsWindow(hwnd) == 0 } {
            return false;
        }
        if IS_FULLSCREEN.swap(fullscreen, Ordering::SeqCst) == fullscreen {
            return true;
        }
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1));
            apply_fullscreen_win32(hwnd, fullscreen);
        });
        true
    }

    pub fn sanitize_window_geometry(
        x: Option<f32>,
        y: Option<f32>,
        width: f32,
        height: f32,
    ) -> (Option<f32>, Option<f32>, f32, f32) {
        let w = if width.is_finite() {
            width.clamp(600.0, 7680.0)
        } else {
            1024.0
        };
        let h = if height.is_finite() {
            height.clamp(400.0, 4320.0)
        } else {
            768.0
        };

        let (Some(px), Some(py)) = (x, y) else {
            return (None, None, w, h);
        };
        if !px.is_finite() || !py.is_finite() || px <= -15000.0 || py <= -15000.0 {
            return (None, None, w, h);
        }

        let ix = px.round() as i32;
        let iy = py.round() as i32;
        let iw = w.round() as i32;
        let ih = h.round() as i32;

        unsafe {
            let grab_rect = Rect {
                left: ix + 16,
                top: iy,
                right: (ix + iw - 16).max(ix + 80),
                bottom: iy + 40,
            };
            let mon = MonitorFromRect(&grab_rect, MONITOR_DEFAULTTONULL);
            if mon != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(mon, &mut mi) != 0
                    && is_window_accessible_in_work_area(
                        ix,
                        iy,
                        iw,
                        ih,
                        mi.rc_work.left,
                        mi.rc_work.top,
                        mi.rc_work.right,
                        mi.rc_work.bottom,
                    )
                {
                    let work_w = (mi.rc_work.right - mi.rc_work.left).max(600) as f32;
                    let work_h = (mi.rc_work.bottom - mi.rc_work.top).max(400) as f32;
                    return (Some(px), Some(py), w.min(work_w), h.min(work_h));
                }
            }

            let win_rect = Rect {
                left: ix,
                top: iy,
                right: ix + iw,
                bottom: iy + ih,
            };
            let nearest = MonitorFromRect(&win_rect, MONITOR_DEFAULTTONEAREST);
            if nearest != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(nearest, &mut mi) != 0 {
                    let (nx, ny, nw, nh) = reposition_window_into_work_area(
                        ix,
                        iy,
                        iw,
                        ih,
                        mi.rc_work.left,
                        mi.rc_work.top,
                        mi.rc_work.right,
                        mi.rc_work.bottom,
                    );
                    return (
                        Some(nx as f32),
                        Some(ny as f32),
                        (nw as f32).max(600.0),
                        (nh as f32).max(400.0),
                    );
                }
            }
        }

        (None, None, w, h)
    }

    #[allow(dead_code)]
    pub fn ensure_window_on_screen() -> bool {
        false
    }
}

#[cfg(windows)]
pub fn sanitize_window_geometry(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
) -> (Option<f32>, Option<f32>, f32, f32) {
    windows_fullscreen::sanitize_window_geometry(x, y, width, height)
}

#[cfg(not(windows))]
pub fn sanitize_window_geometry(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
) -> (Option<f32>, Option<f32>, f32, f32) {
    let w = if width.is_finite() {
        width.clamp(600.0, 7680.0)
    } else {
        1024.0
    };
    let h = if height.is_finite() {
        height.clamp(400.0, 4320.0)
    } else {
        768.0
    };
    if let (Some(px), Some(py)) = (x, y)
        && px.is_finite()
        && py.is_finite()
        && (-10000.0..=10000.0).contains(&px)
        && (-10000.0..=10000.0).contains(&py)
    {
        (Some(px), Some(py), w, h)
    } else {
        (None, None, w, h)
    }
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn ensure_window_on_screen() -> bool {
    windows_fullscreen::ensure_window_on_screen()
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn ensure_window_on_screen() -> bool {
    false
}

#[cfg(windows)]
pub fn optimize_window_for_rendering(hwnd: isize) {
    windows_fullscreen::optimize_window_for_rendering(hwnd);
}

#[cfg(not(windows))]
pub fn optimize_window_for_rendering(_hwnd: isize) {}

#[cfg(windows)]
pub fn set_window_fullscreen(hwnd: isize, fullscreen: bool) -> bool {
    windows_fullscreen::set_window_fullscreen(hwnd, fullscreen)
}

#[cfg(not(windows))]
pub fn set_window_fullscreen(_hwnd: isize, _fullscreen: bool) -> bool {
    false
}

#[cfg(windows)]
pub fn is_window_fullscreen() -> bool {
    windows_fullscreen::is_window_fullscreen()
}

#[cfg(not(windows))]
pub fn is_window_fullscreen() -> bool {
    false
}
