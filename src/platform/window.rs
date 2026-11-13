#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonitorWorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub scale: f32,
}

#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
pub fn normalize_scale_factor(scale: Option<f32>) -> Option<f32> {
    scale.filter(|s| s.is_finite() && (0.5..=6.0).contains(s))
}

#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
pub fn scale_factor_from_dpi(dpi: u32) -> f32 {
    if dpi == 0 {
        1.0
    } else {
        (dpi as f32 / 96.0).clamp(0.5, 6.0)
    }
}

#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
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
    is_window_accessible_in_work_area_scaled(
        x,
        y,
        width,
        height,
        work_left,
        work_top,
        work_right,
        work_bottom,
        1.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn is_window_accessible_in_work_area_scaled(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
    scale: f32,
) -> bool {
    if width <= 0 || height <= 0 || work_right <= work_left || work_bottom <= work_top {
        return false;
    }

    let s = normalize_scale_factor(Some(scale)).unwrap_or(1.0);
    let margin = (16.0 * s).round() as i32;
    let min_grab = (64.0 * s).round() as i32;
    let grab_h = (40.0 * s).round() as i32;
    let top_slop = (8.0 * s).round() as i32;
    let min_grab_w = (80.0 * s).round() as i32;
    let min_grab_h = (20.0 * s).round() as i32;
    let min_body_w = (120.0 * s).round() as i32;
    let min_body_h = (80.0 * s).round() as i32;

    let grab_left = x + margin;
    let grab_right = (x + width - margin).max(grab_left + min_grab);
    let grab_top = y;
    let grab_bottom = y + grab_h;

    let grab_overlap_w = grab_right.min(work_right) - grab_left.max(work_left);
    let grab_overlap_h = grab_bottom.min(work_bottom) - grab_top.max(work_top);

    let body_overlap_w = (x + width).min(work_right) - x.max(work_left);
    let body_overlap_h = (y + height).min(work_bottom) - y.max(work_top);

    y >= work_top - top_slop
        && grab_overlap_w >= min_grab_w
        && grab_overlap_h >= min_grab_h
        && body_overlap_w >= min_body_w
        && body_overlap_h >= min_body_h
}

#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
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
    reposition_window_into_work_area_scaled(
        x,
        y,
        width,
        height,
        work_left,
        work_top,
        work_right,
        work_bottom,
        1.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn reposition_window_into_work_area_scaled(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    work_left: i32,
    work_top: i32,
    work_right: i32,
    work_bottom: i32,
    scale: f32,
) -> (i32, i32, i32, i32) {
    let s = normalize_scale_factor(Some(scale)).unwrap_or(1.0);
    let min_w_bound = (400.0 * s).round().max(1.0) as i32;
    let min_h_bound = (300.0 * s).round().max(1.0) as i32;

    let work_w = (work_right - work_left).max(1);
    let work_h = (work_bottom - work_top).max(1);

    let new_w = width.clamp(min_w_bound.min(work_w), work_w);
    let new_h = height.clamp(min_h_bound.min(work_h), work_h);

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

pub fn sanitize_geometry_with_monitors<F>(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
    saved_scale: Option<f32>,
    primary_monitor: Option<MonitorWorkArea>,
    lookup_monitor: F,
) -> (Option<f32>, Option<f32>, f32, f32)
where
    F: Fn(i32, i32, i32, i32, f32) -> Option<MonitorWorkArea>,
{
    let w = if width.is_finite() {
        width.clamp(400.0, 7680.0)
    } else {
        1024.0
    };
    let h = if height.is_finite() {
        height.clamp(300.0, 4320.0)
    } else {
        768.0
    };

    let clamp_to_monitor = |w_pt: f32, h_pt: f32, mon: MonitorWorkArea| -> (f32, f32) {
        let mon_scale = normalize_scale_factor(Some(mon.scale)).unwrap_or(1.0);
        let max_w = ((mon.right - mon.left).max(1) as f32 / mon_scale).max(1.0);
        let max_h = ((mon.bottom - mon.top).max(1) as f32 / mon_scale).max(1.0);
        let min_w = 400.0_f32.min(max_w);
        let min_h = 300.0_f32.min(max_h);
        (w_pt.clamp(min_w, max_w), h_pt.clamp(min_h, max_h))
    };

    let (Some(px), Some(py)) = (x, y) else {
        let (cw, ch) = primary_monitor
            .map(|pm| clamp_to_monitor(w, h, pm))
            .unwrap_or_else(|| (w.max(600.0), h.max(400.0)));
        return (None, None, cw, ch);
    };
    if !px.is_finite() || !py.is_finite() || px <= -15000.0 || py <= -15000.0 {
        let (cw, ch) = primary_monitor
            .map(|pm| clamp_to_monitor(w, h, pm))
            .unwrap_or_else(|| (w.max(600.0), h.max(400.0)));
        return (None, None, cw, ch);
    }

    let primary_scale = primary_monitor
        .and_then(|m| normalize_scale_factor(Some(m.scale)))
        .unwrap_or(1.0);
    let input_scale = normalize_scale_factor(saved_scale).unwrap_or(primary_scale);

    let phys_x = (px * input_scale).round() as i32;
    let phys_y = (py * input_scale).round() as i32;
    let probe_w = (w * input_scale).round().max(1.0) as i32;
    let probe_h = (h * input_scale).round().max(1.0) as i32;

    if let Some(mon) =
        lookup_monitor(phys_x, phys_y, probe_w, probe_h, input_scale).or(primary_monitor)
    {
        let mon_scale = normalize_scale_factor(Some(mon.scale)).unwrap_or(1.0);
        let iw_px = (w * mon_scale).round().max(1.0) as i32;
        let ih_px = (h * mon_scale).round().max(1.0) as i32;

        if is_window_accessible_in_work_area_scaled(
            phys_x, phys_y, iw_px, ih_px, mon.left, mon.top, mon.right, mon.bottom, mon_scale,
        ) {
            let (out_w, out_h) = clamp_to_monitor(w, h, mon);
            let out_x = if (input_scale - primary_scale).abs() < 1e-4 {
                px
            } else {
                phys_x as f32 / primary_scale
            };
            let out_y = if (input_scale - primary_scale).abs() < 1e-4 {
                py
            } else {
                phys_y as f32 / primary_scale
            };
            return (Some(out_x), Some(out_y), out_w, out_h);
        }

        let (nx_px, ny_px, nw_px, nh_px) = reposition_window_into_work_area_scaled(
            phys_x, phys_y, iw_px, ih_px, mon.left, mon.top, mon.right, mon.bottom, mon_scale,
        );
        let (out_w, out_h) =
            clamp_to_monitor(nw_px as f32 / mon_scale, nh_px as f32 / mon_scale, mon);
        return (
            Some(nx_px as f32 / primary_scale),
            Some(ny_px as f32 / primary_scale),
            out_w,
            out_h,
        );
    }

    (None, None, w.max(600.0), h.max(400.0))
}

#[cfg(windows)]
mod windows_fullscreen {
    use super::{
        MonitorWorkArea, is_window_accessible_in_work_area_scaled, normalize_scale_factor,
        reposition_window_into_work_area_scaled, sanitize_geometry_with_monitors,
        scale_factor_from_dpi,
    };
    use std::ffi::c_void;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    const MONITOR_DEFAULTTONULL: u32 = 0;
    const MONITOR_DEFAULTTOPRIMARY: u32 = 1;
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
    const MDT_EFFECTIVE_DPI: u32 = 0;
    const LOGPIXELSX: i32 = 88;

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
        fn MonitorFromPoint(pt: Point, dw_flags: u32) -> isize;
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
        fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, hdc: isize) -> i32;
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
        fn GetCurrentThreadId() -> u32;
        fn GetCurrentProcessId() -> u32;
        fn LoadLibraryW(lp_lib_file_name: *const u16) -> isize;
        fn GetProcAddress(h_module: isize, lp_proc_name: *const u8) -> *const c_void;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn GetDeviceCaps(hdc: isize, index: i32) -> i32;
    }

    type GetDpiForMonitorFn = unsafe extern "system" fn(isize, u32, *mut u32, *mut u32) -> i32;
    type GetDpiForSystemFn = unsafe extern "system" fn() -> u32;

    fn load_proc(dll_wide: &[u16], proc_name: &[u8]) -> *const c_void {
        unsafe {
            let module = LoadLibraryW(dll_wide.as_ptr());
            if module == 0 {
                return std::ptr::null();
            }
            GetProcAddress(module, proc_name.as_ptr())
        }
    }

    fn monitor_scale_factor(hmonitor: isize) -> f32 {
        unsafe {
            if hmonitor != 0 {
                let shcore: Vec<u16> = "shcore.dll\0".encode_utf16().collect();
                let proc = load_proc(&shcore, b"GetDpiForMonitor\0");
                if !proc.is_null() {
                    let func: GetDpiForMonitorFn = std::mem::transmute(proc);
                    let mut dpi_x = 0u32;
                    let mut dpi_y = 0u32;
                    if func(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == 0 && dpi_x > 0 {
                        return scale_factor_from_dpi(dpi_x);
                    }
                }
            }

            let user32: Vec<u16> = "user32.dll\0".encode_utf16().collect();
            let sys_proc = load_proc(&user32, b"GetDpiForSystem\0");
            if !sys_proc.is_null() {
                let func: GetDpiForSystemFn = std::mem::transmute(sys_proc);
                let dpi = func();
                if dpi > 0 {
                    return scale_factor_from_dpi(dpi);
                }
            }

            let hdc = GetDC(0);
            if hdc != 0 {
                let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
                ReleaseDC(0, hdc);
                if dpi > 0 {
                    return scale_factor_from_dpi(dpi as u32);
                }
            }
        }
        1.0
    }

    fn read_monitor_work_area(hmonitor: isize) -> Option<MonitorWorkArea> {
        if hmonitor == 0 {
            return None;
        }
        let mut mi = MonitorInfo {
            cb_size: std::mem::size_of::<MonitorInfo>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(hmonitor, &mut mi) } == 0 {
            return None;
        }
        Some(MonitorWorkArea {
            left: mi.rc_work.left,
            top: mi.rc_work.top,
            right: mi.rc_work.right,
            bottom: mi.rc_work.bottom,
            scale: monitor_scale_factor(hmonitor),
        })
    }

    fn primary_monitor_work_area() -> Option<MonitorWorkArea> {
        let mon = unsafe { MonitorFromPoint(Point { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        read_monitor_work_area(mon)
    }

    fn monitor_for_physical_rect(
        phys_x: i32,
        phys_y: i32,
        probe_w: i32,
        probe_h: i32,
        input_scale: f32,
    ) -> Option<MonitorWorkArea> {
        let s = normalize_scale_factor(Some(input_scale)).unwrap_or(1.0);
        let margin = (16.0 * s).round() as i32;
        let min_span = (80.0 * s).round() as i32;
        let grab_h = (40.0 * s).round() as i32;

        let grab_rect = Rect {
            left: phys_x + margin,
            top: phys_y,
            right: (phys_x + probe_w - margin).max(phys_x + min_span),
            bottom: phys_y + grab_h,
        };
        let mon = unsafe { MonitorFromRect(&grab_rect, MONITOR_DEFAULTTONULL) };
        if let Some(area) = read_monitor_work_area(mon)
            && is_window_accessible_in_work_area_scaled(
                phys_x,
                phys_y,
                probe_w,
                probe_h,
                area.left,
                area.top,
                area.right,
                area.bottom,
                area.scale,
            )
        {
            return Some(area);
        }

        let win_rect = Rect {
            left: phys_x,
            top: phys_y,
            right: phys_x + probe_w,
            bottom: phys_y + probe_h,
        };
        let nearest = unsafe { MonitorFromRect(&win_rect, MONITOR_DEFAULTTONEAREST) };
        read_monitor_work_area(nearest)
    }

    pub fn is_current_thread_window_owner(hwnd: isize) -> bool {
        if hwnd == 0 {
            return false;
        }
        unsafe {
            if IsWindow(hwnd) == 0 {
                return false;
            }
            let mut proc_id = 0u32;
            let win_tid = GetWindowThreadProcessId(hwnd, &mut proc_id);
            win_tid != 0 && proc_id == GetCurrentProcessId() && win_tid == GetCurrentThreadId()
        }
    }

    pub fn optimize_window_for_rendering(_hwnd: isize) {}

    pub fn is_window_fullscreen() -> bool {
        IS_FULLSCREEN.load(Ordering::Relaxed)
    }

    fn apply_fullscreen_win32(hwnd: isize, fullscreen: bool) -> bool {
        unsafe {
            if !is_current_thread_window_owner(hwnd) {
                return false;
            }
            if fullscreen {
                let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    ..Default::default()
                };
                if mon == 0 || GetMonitorInfoW(mon, &mut mi) == 0 {
                    return false;
                }

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

                let moved = SetWindowPos(
                    hwnd,
                    0,
                    mi.rc_monitor.left,
                    mi.rc_monitor.top,
                    w,
                    h,
                    SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                );
                if moved == 0 {
                    SetWindowLongW(hwnd, GWL_STYLE, style);
                    SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style);
                    return false;
                }

                if let Ok(mut guard) = SAVED_WINDOW_INFO.lock() {
                    *guard = Some(SavedWindowInfo {
                        style,
                        ex_style,
                        rect,
                        maximized,
                    });
                }
                true
            } else {
                let saved = SAVED_WINDOW_INFO.lock().ok().and_then(|mut g| g.take());
                let Some(info) = saved else {
                    return false;
                };

                let restored_style = (info.style & !WS_MAXIMIZE) | WS_VISIBLE;
                SetWindowLongW(hwnd, GWL_STYLE, restored_style);
                SetWindowLongW(hwnd, GWL_EXSTYLE, info.ex_style);

                let raw_w = (info.rect.right - info.rect.left).max(400);
                let raw_h = (info.rect.bottom - info.rect.top).max(300);
                let (rx, ry, rw, rh) = if let Some(area) =
                    read_monitor_work_area(MonitorFromRect(&info.rect, MONITOR_DEFAULTTONEAREST))
                {
                    if is_window_accessible_in_work_area_scaled(
                        info.rect.left,
                        info.rect.top,
                        raw_w,
                        raw_h,
                        area.left,
                        area.top,
                        area.right,
                        area.bottom,
                        area.scale,
                    ) {
                        (info.rect.left, info.rect.top, raw_w, raw_h)
                    } else {
                        reposition_window_into_work_area_scaled(
                            info.rect.left,
                            info.rect.top,
                            raw_w,
                            raw_h,
                            area.left,
                            area.top,
                            area.right,
                            area.bottom,
                            area.scale,
                        )
                    }
                } else {
                    (info.rect.left, info.rect.top, raw_w, raw_h)
                };

                let moved = SetWindowPos(
                    hwnd,
                    0,
                    rx,
                    ry,
                    rw,
                    rh,
                    SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                );
                if info.maximized {
                    ShowWindow(hwnd, SW_MAXIMIZE);
                }
                moved != 0
            }
        }
    }

    pub fn set_window_fullscreen(hwnd: isize, fullscreen: bool) -> bool {
        if !is_current_thread_window_owner(hwnd) {
            return false;
        }
        if IS_FULLSCREEN.load(Ordering::SeqCst) == fullscreen {
            return true;
        }
        if apply_fullscreen_win32(hwnd, fullscreen) {
            IS_FULLSCREEN.store(fullscreen, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    pub fn sanitize_window_geometry_with_scale(
        x: Option<f32>,
        y: Option<f32>,
        width: f32,
        height: f32,
        scale_factor: Option<f32>,
    ) -> (Option<f32>, Option<f32>, f32, f32) {
        sanitize_geometry_with_monitors(
            x,
            y,
            width,
            height,
            scale_factor,
            primary_monitor_work_area(),
            monitor_for_physical_rect,
        )
    }

    #[allow(dead_code)]
    pub fn ensure_window_on_screen() -> bool {
        false
    }
}

#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
pub fn sanitize_window_geometry(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
) -> (Option<f32>, Option<f32>, f32, f32) {
    sanitize_window_geometry_with_scale(x, y, width, height, None)
}

#[cfg(windows)]
pub fn sanitize_window_geometry_with_scale(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
    scale_factor: Option<f32>,
) -> (Option<f32>, Option<f32>, f32, f32) {
    windows_fullscreen::sanitize_window_geometry_with_scale(x, y, width, height, scale_factor)
}

#[cfg(not(windows))]
pub fn sanitize_window_geometry_with_scale(
    x: Option<f32>,
    y: Option<f32>,
    width: f32,
    height: f32,
    _scale_factor: Option<f32>,
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
#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
pub fn is_current_thread_window_owner(hwnd: isize) -> bool {
    windows_fullscreen::is_current_thread_window_owner(hwnd)
}

#[cfg(not(windows))]
#[cfg_attr(not(all(test, feature = "enable_tests")), allow(dead_code))]
pub fn is_current_thread_window_owner(_hwnd: isize) -> bool {
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

pub fn sync_fullscreen_state(state: &mut bool) {
    #[cfg(windows)]
    {
        *state = windows_fullscreen::is_window_fullscreen();
    }
    #[cfg(not(windows))]
    {
        let _ = state;
    }
}

pub fn try_set_native_fullscreen(fullscreen: bool) -> bool {
    #[cfg(windows)]
    {
        let hwnd = crate::platform::get_primary_window_hwnd();
        crate::platform::set_window_fullscreen(hwnd, fullscreen)
    }
    #[cfg(not(windows))]
    {
        let _ = fullscreen;
        false
    }
}
