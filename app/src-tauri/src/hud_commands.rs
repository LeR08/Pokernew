//! Commands of the HUD module and of the overlay windows.

use std::sync::Arc;

use lps_stats_engine::hud::HudTable;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::CommandError;
use crate::hud::{layout, layout_id, HudService, HudStatus};
use crate::state::AppState;
use crate::tracker_commands::{load_stats, StatMeta};

type Res<T> = Result<T, CommandError>;

#[tauri::command]
pub fn hud_status(hud: State<'_, Arc<HudService>>) -> HudStatus {
    hud.status()
}

#[tauri::command]
pub fn play_mode_get(hud: State<'_, Arc<HudService>>) -> bool {
    hud.play_mode.load(std::sync::atomic::Ordering::SeqCst)
}

#[tauri::command]
pub fn hud_set_visible(app: AppHandle, hud: State<'_, Arc<HudService>>, visible: bool) {
    hud.set_visible(&app, visible);
}

#[tauri::command]
pub fn hud_set_edit(app: AppHandle, hud: State<'_, Arc<HudService>>, edit: bool) {
    hud.set_edit(&app, edit);
}

#[tauri::command]
pub fn hud_set_edit_table(app: AppHandle, hud: State<'_, Arc<HudService>>, window_id: Option<u64>) {
    hud.set_edit_table(&app, window_id);
}

#[tauri::command]
pub fn hud_set_table_hidden(
    app: AppHandle,
    hud: State<'_, Arc<HudService>>,
    window_id: u64,
    hidden: bool,
) {
    hud.set_hidden(&app, window_id, hidden);
}

/// Where the overlay's panels are (CSS px, relative to the overlay window).
#[tauri::command]
pub fn hud_panel_rects(
    app: AppHandle,
    hud: State<'_, Arc<HudService>>,
    window_id: u64,
    rects: Vec<[f64; 4]>,
) {
    hud.set_panels(&app, window_id, rects);
}

/// Table size chosen by hand for one table (None = the detected size).
#[tauri::command]
pub fn hud_set_table_seats(
    app: AppHandle,
    hud: State<'_, Arc<HudService>>,
    window_id: u64,
    seats: Option<u8>,
) {
    hud.set_seats_override(&app, window_id, seats);
}

/// Double-click on a HUD panel: brings the main window up on the player's sheet.
#[tauri::command]
pub fn hud_open_player(app: AppHandle, name: String) -> Res<()> {
    let w = app
        .get_webview_window("main")
        .ok_or_else(|| CommandError::from("main window not found".to_string()))?;
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
    let q: String = name
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    app.emit_to(
        "main",
        "app://navigate",
        format!("/m/tracker/opponents?player={q}"),
    )
    .map_err(|e| CommandError::from(e.to_string()))
}

/// Saves the panel size of one table size ("6", "9"…) into the active layout.
#[tauri::command]
pub fn hud_save_scale(s: State<'_, AppState>, table_size: u8, scale: f64) -> Res<()> {
    let id = layout_id(&s);
    let mut lay = s
        .store
        .get("hud", &id)
        .ok_or_else(|| CommandError::from(format!("hud/{id}.toml not found")))?;
    if !lay["panel_scale"].is_object() {
        lay["panel_scale"] = json!({});
    }
    lay["panel_scale"][table_size.to_string()] =
        json!((scale.clamp(0.5, 2.5) * 100.0).round() / 100.0);
    crate::commands::logic::save_value(&s, "hud", &id, &lay)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayData {
    pub table: Option<HudTable>,
    pub layout: Value,
    pub layout_id: String,
    pub edit: bool,
    pub window_id: u64,
    pub stats: Vec<StatMeta>,
    /// Table size chosen by hand for this table, if any.
    pub seats_override: Option<u8>,
}

/// Everything an overlay window needs to draw its table.
#[tauri::command]
pub fn hud_overlay(
    s: State<'_, AppState>,
    hud: State<'_, Arc<HudService>>,
    window_id: u64,
) -> Res<OverlayData> {
    let lay = layout(&s);
    let lid = layout_id(&s);
    let Some((room, table_name, _, editing)) = hud.overlay_table(window_id) else {
        return Ok(OverlayData {
            table: None,
            layout: lay,
            layout_id: lid,
            edit: false,
            window_id,
            stats: vec![],
            seats_override: None,
        });
    };
    let rotate = s
        .store
        .get("rooms", &room)
        .and_then(|r| r["rotate_hero_to_bottom"].as_bool())
        .unwrap_or(true);
    let (compiled, metas, _) = load_stats(&s);
    let wanted: Vec<String> = lay["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|r| r.as_array().cloned().unwrap_or_default())
        .chain(lay["popup"].as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let used: Vec<_> = compiled
        .into_iter()
        .filter(|c| wanted.contains(&c.id))
        .collect();
    let over = hud.seats_override(window_id);
    let mut table = s
        .import
        .as_ref()
        .ok_or_else(|| CommandError::from("database unavailable".to_string()))?
        .with_reader(|c| {
            lps_stats_engine::hud::table(c, &room, &table_name, &used, rotate)
                .map_err(lps_db::DbError::Sqlite)
        })
        .map_err(|e| CommandError::from(e.to_string()))?;
    if let (Some(n), Some(t)) = (over, table.as_mut()) {
        let seats: Vec<(u8, bool)> = t.seats.iter().map(|x| (x.seat, x.is_hero)).collect();
        for (x, slot) in t
            .seats
            .iter_mut()
            .zip(lps_stats_engine::hud::slots(&seats, n, rotate))
        {
            x.slot = slot;
        }
        t.max_seats = n;
    }
    Ok(OverlayData {
        table,
        seats_override: over,
        layout: lay,
        layout_id: lid,
        edit: editing,
        window_id,
        stats: metas
            .into_iter()
            .filter(|m| wanted.contains(&m.id))
            .collect(),
    })
}

/// Saves a panel position (edit mode) into the active layout file.
#[tauri::command]
pub fn hud_save_seat(
    s: State<'_, AppState>,
    table_size: u8,
    slot: usize,
    x: f64,
    y: f64,
) -> Res<()> {
    let id = layout_id(&s);
    let mut lay = s
        .store
        .get("hud", &id)
        .ok_or_else(|| CommandError::from(format!("hud/{id}.toml not found")))?;
    let key = table_size.to_string();
    let mut seats: Vec<Value> = lay["seats"][&key].as_array().cloned().unwrap_or_default();
    while seats.len() < table_size as usize {
        seats.push(json!([0.42, 0.42]));
    }
    if slot >= seats.len() {
        return Err(CommandError::from(format!("slot {slot} out of range")));
    }
    let r = |v: f64| (v.clamp(0.0, 1.0) * 1000.0).round() / 1000.0;
    seats[slot] = json!([r(x), r(y)]);
    lay["seats"][&key] = Value::Array(seats);
    crate::commands::logic::save_value(&s, "hud", &id, &lay)
}
