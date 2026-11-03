//! Commands of the HUD module and of the overlay windows.

use std::sync::Arc;

use lps_stats_engine::hud::HudTable;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, State};

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
pub fn hud_simulate(app: AppHandle, hud: State<'_, Arc<HudService>>, seats: u8) -> Res<String> {
    hud.simulate(&app, seats).map_err(CommandError::from)
}

#[tauri::command]
pub fn hud_clear_simulated(hud: State<'_, Arc<HudService>>) {
    hud.clear_simulated();
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayData {
    pub table: Option<HudTable>,
    pub layout: Value,
    pub layout_id: String,
    pub edit: bool,
    pub simulated: bool,
    pub stats: Vec<StatMeta>,
}

/// Everything an overlay window needs to draw its table.
#[tauri::command]
pub fn hud_overlay(s: State<'_, AppState>, hud: State<'_, Arc<HudService>>, window_id: u64) -> Res<OverlayData> {
    let lay = layout(&s);
    let lid = layout_id(&s);
    let status = hud.status();
    let Some((room, table_name, _, simulated)) = hud.overlay_table(window_id) else {
        return Ok(OverlayData { table: None, layout: lay, layout_id: lid, edit: status.edit, simulated: false, stats: vec![] });
    };
    let rotate = s.store.get("rooms", &room).and_then(|r| r["rotate_hero_to_bottom"].as_bool()).unwrap_or(true);
    let (compiled, metas, _) = load_stats(&s);
    let wanted: Vec<String> = lay["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|r| r.as_array().cloned().unwrap_or_default())
        .chain(lay["popup"].as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let used: Vec<_> = compiled.into_iter().filter(|c| wanted.contains(&c.id)).collect();
    let table = s
        .import
        .as_ref()
        .ok_or_else(|| CommandError::from("database unavailable".to_string()))?
        .with_reader(|c| lps_stats_engine::hud::table(c, &room, &table_name, &used, rotate).map_err(lps_db::DbError::Sqlite))
        .map_err(|e| CommandError::from(e.to_string()))?;
    Ok(OverlayData {
        table,
        layout: lay,
        layout_id: lid,
        edit: status.edit,
        simulated,
        stats: metas.into_iter().filter(|m| wanted.contains(&m.id)).collect(),
    })
}

/// Saves a panel position (edit mode) into the active layout file.
#[tauri::command]
pub fn hud_save_seat(s: State<'_, AppState>, table_size: u8, slot: usize, x: f64, y: f64) -> Res<()> {
    let id = layout_id(&s);
    let mut lay = s.store.get("hud", &id).ok_or_else(|| CommandError::from(format!("hud/{id}.toml not found")))?;
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
