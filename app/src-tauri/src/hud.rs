//! HUD service: follows poker table windows and keeps one transparent,
//! always-on-top, click-through overlay window over each of them.
//!
//! * Tables = windows whose title contains the name of a table with hands in
//!   the last [`RECENT`] (see `lps_hud_overlay::matching`), restricted by the
//!   room settings; `hud_allowed = false` rooms never get an overlay.
//! * Overlays ignore the mouse so they never block the poker client. In
//!   **edit mode** (global hotkey or the HUD screen) they take the mouse, so
//!   panels can be dragged and popups opened.
//! * **Play mode** (fair-play, not a setting): as soon as one real table is
//!   detected, study modules are locked; they unlock [`UNLOCK_AFTER`] after
//!   the last table closed.
//! * Each table can be **edited on its own** (its overlay takes the mouse so
//!   panels can be dragged on the real table, with a "Done" button) or hidden.
//! * Windows of this app are never taken for tables.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lps_hud_overlay::matching::{match_tables, ActiveTable, RoomRules, TableWindow};
use lps_hud_overlay::{source, Rect, WindowInfo};
use serde::Serialize;
use serde_json::Value;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};

use crate::state::AppState;

/// Tables with hands in this window count as active.
pub const RECENT: i64 = 15 * 60;
/// Study modules unlock this long after the last table closed.
pub const UNLOCK_AFTER: Duration = Duration::from_secs(60);

pub enum Msg {
    Windows,
    Data,
    Config,
    Tick,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudStatus {
    pub supported: bool,
    pub visible: bool,
    pub edit: bool,
    pub play_mode: bool,
    /// Seconds before study modules unlock (0 when unlocked or still playing).
    pub unlock_in: u64,
    pub tables: Vec<TableWindow>,
    /// Table being edited on its own (window id).
    pub edit_table: Option<u64>,
    /// Tables whose HUD the user hid (window ids).
    pub hidden: Vec<u64>,
    /// Windows that matched no table (for the inspector).
    pub windows: Vec<WindowInfo>,
}

#[derive(Default)]
struct Inner {
    visible: bool,
    edit: bool,
    play_mode: bool,
    last_real_table: Option<Instant>,
    /// Overlay label -> (room, table name, last rect).
    overlays: HashMap<String, (String, String, Rect)>,
    matches: Vec<TableWindow>,
    windows: Vec<WindowInfo>,
    edit_table: Option<u64>,
    hidden: HashSet<u64>,
}

impl Inner {
    fn editing(&self, window_id: u64) -> bool {
        self.edit || self.edit_table == Some(window_id)
    }
}

/// File name of our own executable (its windows are never tables).
fn own_process() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

pub struct HudService {
    tx: Mutex<mpsc::Sender<Msg>>,
    inner: Mutex<Inner>,
    pub play_mode: Arc<AtomicBool>,
}

/// Fair-play rule: playing while a table is open, and for
/// [`UNLOCK_AFTER`] after the last one closed.
pub fn is_play_mode(real_table_open: bool, last_real_table: Option<Instant>) -> bool {
    real_table_open || last_real_table.is_some_and(|t| t.elapsed() < UNLOCK_AFTER)
}

pub fn overlay_label(window_id: u64) -> String {
    format!("hud-{window_id}")
}

impl HudService {
    pub fn start(app: AppHandle) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<Msg>();
        let svc = Arc::new(Self {
            tx: Mutex::new(tx.clone()),
            inner: Mutex::new(Inner {
                visible: true,
                ..Default::default()
            }),
            play_mode: Arc::new(AtomicBool::new(false)),
        });
        let wtx = Mutex::new(tx);
        source::watch(move || {
            if let Ok(t) = wtx.lock() {
                let _ = t.send(Msg::Windows);
            }
        });
        let svc2 = svc.clone();
        let _ = std::thread::Builder::new()
            .name("lps-hud".into())
            .spawn(move || svc2.run(app, rx));
        svc
    }

    pub fn send(&self, m: Msg) {
        if let Ok(t) = self.tx.lock() {
            let _ = t.send(m);
        }
    }

    fn run(&self, app: AppHandle, rx: mpsc::Receiver<Msg>) {
        loop {
            let first = match rx.recv_timeout(Duration::from_millis(1000)) {
                Ok(m) => m,
                Err(RecvTimeoutError::Timeout) => Msg::Tick,
                Err(RecvTimeoutError::Disconnected) => return,
            };
            // Coalesce bursts (window drags send many events): wait 16 ms, drain.
            let mut data = matches!(first, Msg::Data | Msg::Config);
            std::thread::sleep(Duration::from_millis(16));
            while let Ok(m) = rx.try_recv() {
                data |= matches!(m, Msg::Data | Msg::Config);
            }
            self.refresh(&app, data);
        }
    }

    fn rules(state: &AppState) -> Vec<RoomRules> {
        state
            .store
            .list("rooms")
            .into_iter()
            .filter_map(|id| {
                let v = state.store.get("rooms", &id)?;
                Some(RoomRules {
                    room: id,
                    processes: v["processes"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                    title_pattern: v["title_pattern"]
                        .as_str()
                        .filter(|p| !p.is_empty())
                        .and_then(|p| {
                            regex::Regex::new(p)
                                .map_err(|e| log::warn!("bad title_pattern: {e}"))
                                .ok()
                        }),
                    hud_allowed: v["hud_allowed"].as_bool().unwrap_or(false),
                })
            })
            .collect()
    }

    fn refresh(&self, app: &AppHandle, data_changed: bool) {
        let state = app.state::<AppState>();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let own = own_process();
        let windows: Vec<WindowInfo> = source::snapshot()
            .into_iter()
            .filter(|w| !w.process.eq_ignore_ascii_case(&own))
            .collect();
        let tables: Vec<ActiveTable> = state
            .import
            .as_ref()
            .and_then(|svc| {
                svc.with_reader(|c| {
                    lps_stats_engine::hud::recent_tables(c, now - RECENT)
                        .map_err(lps_db::DbError::Sqlite)
                })
                .ok()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|t| ActiveTable {
                room: t.room,
                table_name: t.table_name,
                tournament_id: t.tournament_id,
                table_number: t.table_number,
                max_seats: t.max_seats,
                last_hand_at: t.last_hand_at,
            })
            .collect();
        let rules = Self::rules(&state);
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let matches = match_tables(&windows, &tables, &rules);

        // ---- Play mode ----
        let real = !matches.is_empty();
        if real {
            inner.last_real_table = Some(Instant::now());
        }
        let play = is_play_mode(real, inner.last_real_table);
        if play != inner.play_mode {
            inner.play_mode = play;
            self.play_mode.store(play, Ordering::SeqCst);
            let _ = app.emit("playmode://changed", play);
            log::info!("play mode {}", if play { "on" } else { "off" });
        }

        // ---- Overlays ----
        let allowed = |room: &str| rules.iter().any(|r| r.room == room && r.hud_allowed);
        let wanted: Vec<&TableWindow> = if inner.visible {
            matches
                .iter()
                .filter(|m| allowed(&m.table.room) && !inner.hidden.contains(&m.window.id))
                .collect()
        } else {
            vec![]
        };
        let wanted_labels: Vec<String> =
            wanted.iter().map(|m| overlay_label(m.window.id)).collect();
        let stale: Vec<String> = inner
            .overlays
            .keys()
            .filter(|l| !wanted_labels.contains(l))
            .cloned()
            .collect();
        for label in stale {
            if let Some(w) = app.get_webview_window(&label) {
                let _ = w.close();
            }
            inner.overlays.remove(&label);
        }
        for m in wanted {
            let editing = inner.editing(m.window.id);
            let label = overlay_label(m.window.id);
            let r = m.window.client;
            match app.get_webview_window(&label) {
                Some(w) => {
                    let prev = inner.overlays.get(&label).map(|x| x.2);
                    if prev != Some(r) {
                        let _ = w.set_position(PhysicalPosition::new(r.x, r.y));
                        let _ = w.set_size(PhysicalSize::new(r.w.max(1) as u32, r.h.max(1) as u32));
                    }
                }
                None => {
                    let built = WebviewWindowBuilder::new(
                        app,
                        &label,
                        WebviewUrl::App(format!("index.html#/hud/{}", m.window.id).into()),
                    )
                    .title("HUD")
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .always_on_top(true)
                    .skip_taskbar(true)
                    .resizable(false)
                    .focused(false)
                    .visible(true)
                    .build();
                    match built {
                        Ok(w) => {
                            let _ = w.set_position(PhysicalPosition::new(r.x, r.y));
                            let _ =
                                w.set_size(PhysicalSize::new(r.w.max(1) as u32, r.h.max(1) as u32));
                            let _ = w.set_ignore_cursor_events(!editing);
                        }
                        Err(e) => log::warn!("cannot open HUD window: {e}"),
                    }
                }
            }
            inner
                .overlays
                .insert(label, (m.table.room.clone(), m.table.table_name.clone(), r));
        }
        let changed = inner.matches != matches;
        inner.matches = matches;
        inner.windows = windows;
        drop(inner);
        if data_changed || changed {
            let _ = app.emit("hud://refresh", ());
        }
    }

    pub fn status(&self) -> HudStatus {
        let inner = self.inner.lock().ok();
        let Some(i) = inner else {
            return HudStatus {
                supported: source::supported(),
                visible: false,
                edit: false,
                play_mode: false,
                unlock_in: 0,
                tables: vec![],
                edit_table: None,
                hidden: vec![],
                windows: vec![],
            };
        };
        let matched: Vec<u64> = i.matches.iter().map(|m| m.window.id).collect();
        let unlock_in = match (i.play_mode, !i.matches.is_empty(), i.last_real_table) {
            (true, false, Some(t)) => UNLOCK_AFTER.saturating_sub(t.elapsed()).as_secs(),
            _ => 0,
        };
        HudStatus {
            supported: source::supported(),
            visible: i.visible,
            edit: i.edit,
            play_mode: i.play_mode,
            unlock_in,
            tables: i.matches.clone(),
            edit_table: i.edit_table,
            hidden: i.hidden.iter().copied().collect(),
            windows: i
                .windows
                .iter()
                .filter(|w| !matched.contains(&w.id) && w.client.w > 200)
                .cloned()
                .collect(),
        }
    }

    /// Which table an overlay shows, and whether it is being edited.
    pub fn overlay_table(&self, window_id: u64) -> Option<(String, String, u8, bool)> {
        let i = self.inner.lock().ok()?;
        i.matches
            .iter()
            .find(|m| m.window.id == window_id)
            .map(|m| {
                (
                    m.table.room.clone(),
                    m.table.table_name.clone(),
                    m.table.max_seats,
                    i.editing(window_id),
                )
            })
    }

    pub fn set_visible(&self, app: &AppHandle, v: bool) {
        if let Ok(mut i) = self.inner.lock() {
            i.visible = v;
        }
        self.send(Msg::Config);
        let _ = app.emit("hud://status", ());
    }

    pub fn toggle_visible(&self, app: &AppHandle) {
        let v = self.inner.lock().map(|i| !i.visible).unwrap_or(true);
        self.set_visible(app, v);
    }

    fn apply_edit(&self, app: &AppHandle) {
        let states: Vec<(String, bool)> = match self.inner.lock() {
            Ok(i) => i
                .matches
                .iter()
                .map(|m| (overlay_label(m.window.id), i.editing(m.window.id)))
                .collect(),
            Err(_) => return,
        };
        for (l, editing) in states {
            if let Some(w) = app.get_webview_window(&l) {
                let _ = w.set_ignore_cursor_events(!editing);
                if editing {
                    let _ = w.set_focus();
                }
            }
        }
        let _ = app.emit("hud://edit", ());
        let _ = app.emit("hud://status", ());
    }

    pub fn set_edit(&self, app: &AppHandle, edit: bool) {
        if let Ok(mut i) = self.inner.lock() {
            i.edit = edit;
            if !edit {
                i.edit_table = None;
            }
        }
        self.apply_edit(app);
    }

    /// Edit one table's HUD on the table itself (None = stop).
    pub fn set_edit_table(&self, app: &AppHandle, window_id: Option<u64>) {
        if let Ok(mut i) = self.inner.lock() {
            i.edit_table = window_id;
            if window_id.is_none() {
                i.edit = false;
            }
        }
        self.apply_edit(app);
    }

    /// Hides (or shows again) the HUD of one table.
    pub fn set_hidden(&self, app: &AppHandle, window_id: u64, hidden: bool) {
        if let Ok(mut i) = self.inner.lock() {
            if hidden {
                i.hidden.insert(window_id);
            } else {
                i.hidden.remove(&window_id);
            }
        }
        self.send(Msg::Config);
        let _ = app.emit("hud://status", ());
    }

    pub fn toggle_edit(&self, app: &AppHandle) {
        let e = self
            .inner
            .lock()
            .map(|i| !(i.edit || i.edit_table.is_some()))
            .unwrap_or(false);
        self.set_edit(app, e);
    }
}

/// "Ctrl+Alt+H" from hotkeys.toml -> the plugin's format ("Meta" is "Super").
pub fn shortcut_string(s: &str) -> String {
    s.split('+')
        .map(|p| if p == "Meta" { "Super" } else { p })
        .collect::<Vec<_>>()
        .join("+")
}

pub fn hotkey(state: &AppState, action: &str, default: &str) -> String {
    state
        .store
        .get("hotkeys", "hotkeys")
        .and_then(|v| v["actions"][action].as_str().map(String::from))
        .unwrap_or_else(|| default.to_string())
}

/// The HUD layout of the active profile (`hud/<profile.hud>.toml`, default "standard").
pub fn layout(state: &AppState) -> Value {
    let profile = state
        .store
        .get("settings", "settings")
        .and_then(|s| s["general"]["profile"].as_str().map(String::from))
        .and_then(|p| state.store.get("profiles", &p))
        .and_then(|p| p["hud"].as_str().map(String::from))
        .unwrap_or_else(|| "standard".into());
    state
        .store
        .get("hud", &profile)
        .or_else(|| state.store.get("hud", "standard"))
        .unwrap_or(Value::Null)
}

pub fn layout_id(state: &AppState) -> String {
    state
        .store
        .get("settings", "settings")
        .and_then(|s| s["general"]["profile"].as_str().map(String::from))
        .and_then(|p| state.store.get("profiles", &p))
        .and_then(|p| p["hud"].as_str().map(String::from))
        .filter(|id| state.store.get("hud", id).is_some())
        .unwrap_or_else(|| "standard".into())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    #[test]
    fn play_mode_rule() {
        assert!(super::is_play_mode(true, None));
        assert!(
            !super::is_play_mode(false, None),
            "never played: study mode"
        );
        assert!(
            super::is_play_mode(false, Some(Instant::now())),
            "just closed: still locked"
        );
        let long_ago = Instant::now().checked_sub(Duration::from_secs(61)).unwrap();
        assert!(
            !super::is_play_mode(false, Some(long_ago)),
            "unlocked after 60 s"
        );
    }

    #[test]
    fn shortcut_strings() {
        assert_eq!(super::shortcut_string("Ctrl+Alt+H"), "Ctrl+Alt+H");
        assert_eq!(super::shortcut_string("Meta+Shift+K"), "Super+Shift+K");
    }
}
