//! Where windows come from.
//!
//! * [`snapshot`] lists the top-level windows (Windows: `EnumWindows`; other
//!   systems: none — macOS support is best-effort and not implemented yet);
//! * [`watch`] calls back whenever a window is created, destroyed, shown,
//!   hidden, moved/resized, renamed, minimised/restored or brought to the
//!   foreground (Windows: `SetWinEventHook`, out of context, skipping our own
//!   process so the HUD's own windows never trigger it).

use crate::WindowInfo;

/// Current top-level windows.
pub fn snapshot() -> Vec<WindowInfo> {
    #[cfg(windows)]
    {
        win32::snapshot()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Starts watching window events; `on_change` runs on a dedicated thread
/// (possibly many times per second while a window is dragged — debounce it).
pub fn watch<F: Fn() + Send + Sync + 'static>(on_change: F) {
    #[cfg(windows)]
    {
        win32::watch(Box::new(on_change));
    }
    #[cfg(not(windows))]
    {
        let _ = on_change;
        log::info!("window tracking is only available on Windows");
    }
}

/// True when this build can see real poker windows.
pub const fn supported() -> bool {
    cfg!(windows)
}

#[cfg(windows)]
mod win32 {
    use std::sync::OnceLock;

    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EnumWindows, GetClassNameW, GetClientRect, GetMessageW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, TranslateMessage, EVENT_OBJECT_CREATE,
        EVENT_OBJECT_NAMECHANGE, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, MSG, OBJID_WINDOW,
        WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    };

    use crate::{Rect, WindowInfo};

    fn text(hwnd: HWND) -> String {
        // SAFETY: plain Win32 calls on a window handle given by the system.
        unsafe {
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return String::new();
            }
            let mut buf = vec![0u16; len as usize + 1];
            let n = GetWindowTextW(hwnd, &mut buf);
            String::from_utf16_lossy(&buf[..n.max(0) as usize])
        }
    }

    fn class(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        // SAFETY: buffer is valid for its length.
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn process(hwnd: HWND) -> String {
        let mut pid = 0u32;
        // SAFETY: Win32 calls; the process handle is closed below.
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
            let mut buf = [0u16; 1024];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size).is_ok();
            let _ = CloseHandle(h);
            if !ok {
                return String::new();
            }
            let path = String::from_utf16_lossy(&buf[..size as usize]);
            path.rsplit(['\\', '/']).next().unwrap_or_default().to_string()
        }
    }

    fn info(hwnd: HWND) -> Option<WindowInfo> {
        // SAFETY: Win32 queries on a live handle.
        unsafe {
            let visible = IsWindowVisible(hwnd).as_bool();
            if !visible {
                return None;
            }
            let title = text(hwnd);
            if title.is_empty() {
                return None;
            }
            let mut rc = RECT::default();
            GetClientRect(hwnd, &mut rc).ok()?;
            let mut origin = POINT { x: 0, y: 0 };
            let _ = ClientToScreen(hwnd, &mut origin);
            let dpi = GetDpiForWindow(hwnd);
            Some(WindowInfo {
                id: hwnd.0 as usize as u64,
                title,
                class: class(hwnd),
                process: process(hwnd),
                client: Rect { x: origin.x, y: origin.y, w: rc.right - rc.left, h: rc.bottom - rc.top },
                scale: if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 },
                minimized: IsIconic(hwnd).as_bool(),
                visible,
                simulated: false,
            })
        }
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the &mut Vec passed by `snapshot` for the duration of EnumWindows.
        let out = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
        if let Some(w) = info(hwnd) {
            out.push(w);
        }
        BOOL(1)
    }

    pub fn snapshot() -> Vec<WindowInfo> {
        let mut out: Vec<WindowInfo> = Vec::new();
        // SAFETY: the callback only uses `out` during this call.
        unsafe {
            let _ = EnumWindows(Some(enum_cb), LPARAM(&mut out as *mut Vec<WindowInfo> as isize));
        }
        out
    }

    type Callback = Box<dyn Fn() + Send + Sync>;
    static CALLBACK: OnceLock<Callback> = OnceLock::new();

    unsafe extern "system" fn event_cb(
        _hook: HWINEVENTHOOK,
        _event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        // Only whole windows (not the caret, cursor or child objects).
        if id_object == OBJID_WINDOW.0 && id_child == 0 && !hwnd.is_invalid() {
            if let Some(cb) = CALLBACK.get() {
                cb();
            }
        }
    }

    pub fn watch(cb: Callback) {
        if CALLBACK.set(cb).is_err() {
            log::warn!("window watcher already running");
            return;
        }
        let _ = std::thread::Builder::new().name("lps-win-events".into()).spawn(|| {
            let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
            // SAFETY: hooks live for the whole process; the message loop below services them.
            unsafe {
                // create, destroy, show, hide, … location change, name change
                SetWinEventHook(EVENT_OBJECT_CREATE, EVENT_OBJECT_NAMECHANGE, None, Some(event_cb), 0, 0, flags);
                // foreground, move/size start/end, minimise start/end
                SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, None, Some(event_cb), 0, 0, flags);
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        });
    }
}
