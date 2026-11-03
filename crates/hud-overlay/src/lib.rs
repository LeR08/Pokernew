//! Finds poker table windows and follows them, for the HUD overlay.
//!
//! * [`WindowInfo`]: a top-level window as the OS reports it (title, class,
//!   program, client rectangle in physical pixels, DPI scale);
//! * [`source`]: where windows come from — Win32 on Windows (EnumWindows +
//!   SetWinEventHook), nothing elsewhere, plus simulated tables;
//! * [`matching`]: which window is which table, from the tables of the
//!   recently imported hands (pure logic, tested).

pub mod matching;
pub mod source;

use serde::Serialize;

/// Rectangle in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// Native handle (HWND) or a simulated id.
    pub id: u64,
    pub title: String,
    pub class: String,
    /// Executable file name, e.g. "Winamax Poker.exe" (lowercase comparison).
    pub process: String,
    /// Client area (without the title bar and borders).
    pub client: Rect,
    /// DPI / 96.
    pub scale: f64,
    pub minimized: bool,
    pub visible: bool,
    pub simulated: bool,
}
