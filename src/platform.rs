//! Windows-spezifische Helfer: Monitore auflisten, Fenster verschieben,
//! Bildschirm/System wach halten.

#[derive(Clone, Debug, PartialEq)]
pub struct MonitorInfo {
    /// z. B. `\\.\DISPLAY2`
    pub device: String,
    /// left, top, right, bottom in physischen Pixeln
    pub rect: [i32; 4],
    pub primary: bool,
}

impl MonitorInfo {
    pub fn width(&self) -> i32 {
        self.rect[2] - self.rect[0]
    }
    pub fn height(&self) -> i32 {
        self.rect[3] - self.rect[1]
    }
    pub fn number(&self) -> Option<u32> {
        let digits: String = self
            .device
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        digits.parse().ok()
    }
    pub fn label(&self, fallback_index: usize) -> String {
        let nr = self.number().unwrap_or(fallback_index as u32 + 1);
        use crate::i18n::t;
        let mut s = format!(
            "{} {nr}  ·  {} × {}",
            t("Bildschirm", "Display"),
            self.width(),
            self.height()
        );
        if self.primary {
            s.push_str(t("  ·  Haupt", "  ·  Main"));
        }
        s
    }
}

#[cfg(windows)]
mod imp {
    use super::MonitorInfo;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, MonitorFromRect, HDC, HMONITOR,
        MONITORINFO, MONITORINFOEXW, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows_sys::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowPlacement, GetWindowRect, IsIconic, IsZoomed, SetWindowPos, ShowWindow,
        SWP_NOACTIVATE, SWP_NOZORDER, SW_MAXIMIZE, SW_RESTORE, WINDOWPLACEMENT,
    };

    const MONITORINFOF_PRIMARY: u32 = 1;

    unsafe extern "system" fn enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let list = &mut *(data as *mut Vec<MonitorInfo>);
        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) != 0 {
            let r = info.monitorInfo.rcMonitor;
            let len = info
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szDevice.len());
            list.push(MonitorInfo {
                device: String::from_utf16_lossy(&info.szDevice[..len]),
                rect: [r.left, r.top, r.right, r.bottom],
                primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            });
        }
        1
    }

    pub fn monitors() -> Vec<MonitorInfo> {
        let mut list: Vec<MonitorInfo> = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(enum_proc),
                &mut list as *mut Vec<MonitorInfo> as LPARAM,
            );
        }
        list.sort_by_key(|m| (m.number().unwrap_or(99), m.rect[0]));
        list
    }

    fn h(hwnd: isize) -> HWND {
        hwnd as HWND
    }

    pub fn window_rect(hwnd: isize) -> Option<[i32; 4]> {
        unsafe {
            if IsIconic(h(hwnd)) != 0 || IsZoomed(h(hwnd)) != 0 {
                return None;
            }
            let mut r: RECT = std::mem::zeroed();
            if GetWindowRect(h(hwnd), &mut r) != 0 {
                Some([r.left, r.top, r.right, r.bottom])
            } else {
                None
            }
        }
    }

    /// Verschiebung zwischen Arbeitsbereichs- und Bildschirmkoordinaten
    /// (nur relevant, wenn die Taskleiste oben oder links liegt).
    fn workspace_offset() -> (i32, i32) {
        unsafe {
            let m = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(m, &mut info) != 0 {
                (
                    info.rcWork.left - info.rcMonitor.left,
                    info.rcWork.top - info.rcMonitor.top,
                )
            } else {
                (0, 0)
            }
        }
    }

    /// Normale Fensterposition (auch wenn maximiert) und ob maximiert.
    /// `None`, wenn das Fenster minimiert ist.
    pub fn window_state(hwnd: isize) -> Option<([i32; 4], bool)> {
        unsafe {
            if IsIconic(h(hwnd)) != 0 {
                return None;
            }
            if IsZoomed(h(hwnd)) == 0 {
                return window_rect(hwnd).map(|r| (r, false));
            }
            let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
            wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            if GetWindowPlacement(h(hwnd), &mut wp) == 0 {
                return None;
            }
            let (dx, dy) = workspace_offset();
            let r = wp.rcNormalPosition;
            Some(([r.left + dx, r.top + dy, r.right + dx, r.bottom + dy], true))
        }
    }

    pub fn maximize(hwnd: isize) {
        unsafe {
            ShowWindow(h(hwnd), SW_MAXIMIZE);
        }
    }

    pub fn set_window_rect(hwnd: isize, r: [i32; 4]) {
        unsafe {
            if IsZoomed(h(hwnd)) != 0 || IsIconic(h(hwnd)) != 0 {
                ShowWindow(h(hwnd), SW_RESTORE);
            }
            SetWindowPos(
                h(hwnd),
                std::ptr::null_mut(),
                r[0],
                r[1],
                r[2] - r[0],
                r[3] - r[1],
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    /// Ist das Rechteck (zumindest teilweise) auf einem angeschlossenen Monitor?
    pub fn rect_visible(r: [i32; 4]) -> bool {
        let rc = RECT {
            left: r[0],
            top: r[1],
            right: r[2],
            bottom: r[3],
        };
        unsafe { !MonitorFromRect(&rc, MONITOR_DEFAULTTONULL).is_null() }
    }

    /// Verschiebt das Fenster mittig auf den Ziel-Monitor (Größe passt sich an).
    pub fn move_to_monitor(hwnd: isize, m: &MonitorInfo) {
        let cur = window_rect(hwnd).unwrap_or([0, 0, 960, 540]);
        let w = (cur[2] - cur[0]).min(m.width() * 3 / 4).max(320);
        let hgt = (cur[3] - cur[1]).min(m.height() * 3 / 4).max(200);
        let x = m.rect[0] + (m.width() - w) / 2;
        let y = m.rect[1] + (m.height() - hgt) / 2;
        set_window_rect(hwnd, [x, y, x + w, y + hgt]);
    }

    pub fn set_awake(system: bool, display: bool) {
        let mut flags = ES_CONTINUOUS;
        if system {
            flags |= ES_SYSTEM_REQUIRED;
        }
        if display {
            flags |= ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED;
        }
        unsafe {
            SetThreadExecutionState(flags);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::MonitorInfo;
    pub fn monitors() -> Vec<MonitorInfo> {
        Vec::new()
    }
    pub fn window_rect(_hwnd: isize) -> Option<[i32; 4]> {
        None
    }
    pub fn set_window_rect(_hwnd: isize, _r: [i32; 4]) {}
    pub fn window_state(_hwnd: isize) -> Option<([i32; 4], bool)> {
        None
    }
    pub fn maximize(_hwnd: isize) {}
    pub fn rect_visible(_r: [i32; 4]) -> bool {
        false
    }
    pub fn move_to_monitor(_hwnd: isize, _m: &MonitorInfo) {}
    pub fn set_awake(_system: bool, _display: bool) {}
}

pub use imp::*;

/// Fenster-Handle aus eframe holen.
pub fn hwnd_of(w: &impl raw_window_handle::HasWindowHandle) -> Option<isize> {
    use raw_window_handle::RawWindowHandle;
    let handle = w.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
        _ => None,
    }
}
