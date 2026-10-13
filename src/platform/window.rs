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

    const GWL_STYLE: i32 = -16;
    const GWL_EXSTYLE: i32 = -20;
    const WS_CAPTION: i32 = 0x00C00000;
    const WS_THICKFRAME: i32 = 0x00040000;
    const WS_EX_DLGMODALFRAME: i32 = 0x00000001;
    const WS_EX_WINDOWEDGE: i32 = 0x00000100;
    const WS_EX_CLIENTEDGE: i32 = 0x00000200;
    const WS_EX_STATICEDGE: i32 = 0x00020000;
    const MONITOR_DEFAULTTONULL: u32 = 0;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const HWND_TOP: isize = 0;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SWP_NOOWNERZORDER: u32 = 0x0200;

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
    struct Point {
        x: i32,
        y: i32,
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

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct MonitorInfo {
        cb_size: u32,
        rc_monitor: Rect,
        rc_work: Rect,
        dw_flags: u32,
    }

    #[derive(Clone, Copy)]
    struct SavedWindowState {
        hwnd: isize,
        style: i32,
        ex_style: i32,
        placement: WindowPlacement,
    }

    static SAVED_STATE: Mutex<Option<SavedWindowState>> = Mutex::new(None);

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetActiveWindow() -> isize;
        fn GetForegroundWindow() -> isize;
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
        fn IsIconic(hwnd: isize) -> i32;
        fn IsZoomed(hwnd: isize) -> i32;
        fn GetWindowRect(hwnd: isize, lprect: *mut Rect) -> i32;
        fn GetWindowLongW(hwnd: isize, n_index: i32) -> i32;
        fn SetWindowLongW(hwnd: isize, n_index: i32, dw_new_long: i32) -> i32;
        fn GetWindowPlacement(hwnd: isize, lpwndpl: *mut WindowPlacement) -> i32;
        fn SetWindowPlacement(hwnd: isize, lpwndpl: *const WindowPlacement) -> i32;
        fn MonitorFromWindow(hwnd: isize, dw_flags: u32) -> isize;
        fn MonitorFromRect(lprc: *const Rect, dw_flags: u32) -> isize;
        fn GetMonitorInfoW(hmonitor: isize, lpmi: *mut MonitorInfo) -> i32;
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

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcessId() -> u32;
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn is_own_process_window(hwnd: isize) -> bool {
        if hwnd == 0 {
            return false;
        }
        let mut proc_id = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut proc_id);
            proc_id != 0 && proc_id == GetCurrentProcessId()
        }
    }

    fn get_own_window_hwnd() -> isize {
        unsafe {
            let mut hwnd = GetActiveWindow();
            if is_own_process_window(hwnd) {
                return hwnd;
            }
            hwnd = GetForegroundWindow();
            if is_own_process_window(hwnd) {
                return hwnd;
            }
            let title = to_wide("QuietReader");
            hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if is_own_process_window(hwnd) {
                return hwnd;
            }
            0
        }
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

    pub fn ensure_window_on_screen() -> bool {
        let hwnd = get_own_window_hwnd();
        if hwnd == 0 {
            return false;
        }
        unsafe {
            if IsIconic(hwnd) != 0 || IsZoomed(hwnd) != 0 {
                return false;
            }

            let mut rect = Rect::default();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return false;
            }

            let ix = rect.left;
            let iy = rect.top;
            let iw = rect.right - rect.left;
            let ih = rect.bottom - rect.top;
            if iw <= 0 || ih <= 0 {
                return false;
            }

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
                    return false;
                }
            }

            let nearest = MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST);
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
                    SetWindowPos(
                        hwnd,
                        0,
                        nx,
                        ny,
                        nw,
                        nh,
                        SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
                    );
                    return true;
                }
            }
            false
        }
    }

    pub fn set_window_fullscreen(fullscreen: bool) -> bool {
        unsafe {
            if fullscreen {
                let hwnd = get_own_window_hwnd();
                if hwnd == 0 {
                    return false;
                }

                let style = GetWindowLongW(hwnd, GWL_STYLE);
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                let mut placement = WindowPlacement {
                    length: std::mem::size_of::<WindowPlacement>() as u32,
                    ..Default::default()
                };
                GetWindowPlacement(hwnd, &mut placement);

                if let Ok(mut guard) = SAVED_STATE.lock() {
                    *guard = Some(SavedWindowState {
                        hwnd,
                        style,
                        ex_style,
                        placement,
                    });
                }

                let new_style = style & !(WS_CAPTION | WS_THICKFRAME);
                let new_ex_style = ex_style
                    & !(WS_EX_DLGMODALFRAME
                        | WS_EX_WINDOWEDGE
                        | WS_EX_CLIENTEDGE
                        | WS_EX_STATICEDGE);

                SetWindowLongW(hwnd, GWL_STYLE, new_style);
                SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style);

                let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if monitor != 0 && GetMonitorInfoW(monitor, &mut mi) != 0 {
                    let x = mi.rc_monitor.left;
                    let y = mi.rc_monitor.top;
                    let w = mi.rc_monitor.right - mi.rc_monitor.left;
                    let h = (mi.rc_monitor.bottom - mi.rc_monitor.top) + 1;
                    SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        x,
                        y,
                        w,
                        h,
                        SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                    );
                    return true;
                }
                false
            } else {
                let saved = SAVED_STATE.lock().ok().and_then(|mut g| g.take());
                if let Some(saved) = saved {
                    SetWindowLongW(saved.hwnd, GWL_STYLE, saved.style);
                    SetWindowLongW(saved.hwnd, GWL_EXSTYLE, saved.ex_style);
                    SetWindowPlacement(saved.hwnd, &saved.placement);
                    SetWindowPos(
                        saved.hwnd,
                        0,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE
                            | SWP_NOSIZE
                            | SWP_NOZORDER
                            | SWP_NOOWNERZORDER
                            | SWP_FRAMECHANGED,
                    );
                    let _ = ensure_window_on_screen();
                    return true;
                }
                false
            }
        }
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
pub fn ensure_window_on_screen() -> bool {
    windows_fullscreen::ensure_window_on_screen()
}

#[cfg(not(windows))]
pub fn ensure_window_on_screen() -> bool {
    false
}

#[cfg(windows)]
pub fn set_window_fullscreen(fullscreen: bool) -> bool {
    windows_fullscreen::set_window_fullscreen(fullscreen)
}

#[cfg(not(windows))]
pub fn set_window_fullscreen(_fullscreen: bool) -> bool {
    false
}
