//! Commands of the Tracker and Replayer modules.

use lps_core_model::Hand;
use lps_stats_engine::defs::{ColourRange, Text};
use lps_stats_engine::query::{self, Filter, Graph, Group, HandRow, ReportRow, TournamentRow, TournamentTotals};
use lps_stats_engine::{compile_all, compile_stat, CompiledStat, StatDef, StatsFile};
use serde::Serialize;
use tauri::State;

use crate::commands::CommandError;
use crate::import_service::ImportService;
use crate::state::AppState;

type Res<T> = Result<T, CommandError>;

fn svc(s: &AppState) -> Res<&ImportService> {
    s.import.as_deref().ok_or_else(|| CommandError::from("database unavailable".to_string()))
}

fn sql_err(e: rusqlite::Error) -> lps_db::DbError {
    lps_db::DbError::Sqlite(e)
}

fn err(e: lps_db::DbError) -> CommandError {
    CommandError::from(e.to_string())
}

/// What the UI needs to display a stat.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatMeta {
    pub id: String,
    pub name: String,
    pub format: String,
    pub description: Option<Text>,
    pub colour_ranges: Vec<ColourRange>,
    pub min_sample: Option<i64>,
    /// File the stat comes from (stats/<file>.toml).
    pub file: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatProblem {
    pub id: String,
    pub file: String,
    pub message: String,
}

/// Every stat of every `stats/*.toml`, compiled.
pub fn load_stats(s: &AppState) -> (Vec<CompiledStat>, Vec<StatMeta>, Vec<StatProblem>) {
    let mut compiled = Vec::new();
    let mut metas = Vec::new();
    let mut problems = Vec::new();
    for file in s.store.list("stats") {
        let Some(v) = s.store.get("stats", &file) else { continue };
        let parsed: StatsFile = match serde_json::from_value(v) {
            Ok(p) => p,
            Err(e) => {
                problems.push(StatProblem { id: String::new(), file: file.clone(), message: e.to_string() });
                continue;
            }
        };
        let (ok, errs) = compile_all(std::slice::from_ref(&parsed));
        for c in ok {
            metas.push(StatMeta {
                id: c.id.clone(),
                name: c.def.name.clone(),
                format: c.def.format.clone(),
                description: c.def.description.clone(),
                colour_ranges: c.def.colour_ranges.clone(),
                min_sample: c.def.min_sample,
                file: file.clone(),
            });
            compiled.push(c);
        }
        for (id, e) in errs {
            problems.push(StatProblem { id, file: file.clone(), message: e.to_string() });
        }
    }
    (compiled, metas, problems)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResponse {
    pub rows: Vec<ReportRow>,
    pub stats: Vec<StatMeta>,
    pub problems: Vec<StatProblem>,
}

#[tauri::command]
pub fn tracker_report(s: State<'_, AppState>, filter: Filter, group: Group) -> Res<ReportResponse> {
    let (compiled, stats, problems) = load_stats(&s);
    let rows = svc(&s)?
        .with_reader(|c| query::report(c, &filter, &compiled, group).map_err(sql_err))
        .map_err(err)?;
    Ok(ReportResponse { rows, stats, problems })
}

#[tauri::command]
pub fn tracker_graph(s: State<'_, AppState>, filter: Filter) -> Res<Graph> {
    svc(&s)?.with_reader(|c| query::graph(c, &filter, 2000).map_err(sql_err)).map_err(err)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentsResponse {
    pub rows: Vec<TournamentRow>,
    pub totals: TournamentTotals,
}

#[tauri::command]
pub fn tracker_tournaments(s: State<'_, AppState>, filter: Filter) -> Res<TournamentsResponse> {
    let (rows, totals) = svc(&s)?.with_reader(|c| query::tournaments(c, &filter).map_err(sql_err)).map_err(err)?;
    Ok(TournamentsResponse { rows, totals })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandsResponse {
    pub rows: Vec<HandRow>,
    pub total: i64,
}

#[tauri::command]
pub fn tracker_hands(s: State<'_, AppState>, filter: Filter, limit: i64, offset: i64) -> Res<HandsResponse> {
    let (rows, total) = svc(&s)?
        .with_reader(|c| query::hands(c, &filter, limit.clamp(1, 1000), offset.max(0)).map_err(sql_err))
        .map_err(err)?;
    Ok(HandsResponse { rows, total })
}

#[tauri::command]
pub fn tracker_accounts(s: State<'_, AppState>) -> Res<Vec<String>> {
    svc(&s)?.with_reader(|c| query::hero_accounts(c).map_err(sql_err)).map_err(err)
}

/// One stored hand, for the replayer.
#[tauri::command]
pub fn hand_get(s: State<'_, AppState>, hand_id: i64) -> Res<Hand> {
    svc(&s)?.with_reader(|c| lps_db::load::load_hand(c, hand_id).map(|(h, _)| h)).map_err(err)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub value: Option<f64>,
    pub sample: i64,
    pub hands: i64,
}

/// Validates a stat definition and computes it on the current filter (stat editor).
#[tauri::command]
pub fn stat_preview(s: State<'_, AppState>, def: StatDef, filter: Filter) -> Res<Preview> {
    let st = compile_stat("preview", &def).map_err(|e| CommandError::from(e.to_string()))?;
    let rows = svc(&s)?
        .with_reader(|c| query::report(c, &filter, std::slice::from_ref(&st), Group::None).map_err(sql_err))
        .map_err(err)?;
    let row = rows.first();
    Ok(Preview {
        value: row.and_then(|r| r.stats[0].value),
        sample: row.map(|r| r.stats[0].sample).unwrap_or(0),
        hands: row.map(|r| r.hands).unwrap_or(0),
    })
}

// ---- Sessions & review, leaks, grid ----

use lps_stats_engine::grid::Cell;
use lps_stats_engine::leaks::{LeakRow, TargetsFile};
use lps_stats_engine::review::{Session, SessionReview};

#[tauri::command]
pub fn tracker_sessions(s: State<'_, AppState>, filter: Filter) -> Res<Vec<Session>> {
    svc(&s)?.with_reader(|c| lps_stats_engine::review::sessions(c, &filter).map_err(sql_err)).map_err(err)
}

/// Post-session review: rules of thumb on the finished hands of a session.
#[tauri::command]
pub fn session_review(s: State<'_, AppState>, filter: Filter, start: i64, end: i64) -> Res<SessionReview> {
    svc(&s)?.with_reader(|c| lps_stats_engine::review::review_session(c, &filter, start, end)).map_err(err)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaksResponse {
    pub rows: Vec<LeakRow>,
    pub stats: Vec<StatMeta>,
}

#[tauri::command]
pub fn tracker_leaks(s: State<'_, AppState>, filter: Filter) -> Res<LeaksResponse> {
    let (compiled, stats, _) = load_stats(&s);
    let mut targets = TargetsFile::default();
    for id in s.store.list("leaks") {
        if let Some(v) = s.store.get("leaks", &id) {
            if let Ok(t) = serde_json::from_value::<TargetsFile>(v) {
                targets.target.extend(t.target);
            }
        }
    }
    let rows = svc(&s)?
        .with_reader(|c| lps_stats_engine::leaks::find(c, &filter, &compiled, &targets).map_err(sql_err))
        .map_err(err)?;
    Ok(LeaksResponse { rows, stats })
}

#[tauri::command]
pub fn tracker_grid(s: State<'_, AppState>, filter: Filter) -> Res<Vec<Cell>> {
    svc(&s)?.with_reader(|c| lps_stats_engine::grid::grid(c, &filter).map_err(sql_err)).map_err(err)
}

// ---- Notes ----

#[tauri::command]
pub fn notes_get(s: State<'_, AppState>, room: String, name: String) -> Res<lps_db::user::PlayerNotes> {
    svc(&s)?.with_reader(|c| lps_db::user::notes(c, &room, &name)).map_err(err)
}

#[tauri::command]
pub fn note_add(s: State<'_, AppState>, room: String, name: String, text: String) -> Res<i64> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    svc(&s)?.with_reader(|c| lps_db::user::add_note(c, &room, &name, &text, now)).map_err(err)
}

#[tauri::command]
pub fn note_delete(s: State<'_, AppState>, id: i64) -> Res<()> {
    svc(&s)?.with_reader(|c| lps_db::user::delete_note(c, id)).map_err(err)
}

#[tauri::command]
pub fn player_colour(s: State<'_, AppState>, room: String, name: String, colour: Option<String>) -> Res<()> {
    svc(&s)?.with_reader(|c| lps_db::user::set_colour(c, &room, &name, colour.as_deref())).map_err(err)
}

// ---- Bankroll ----

#[tauri::command]
pub fn bankroll_get(s: State<'_, AppState>) -> Res<lps_stats_engine::bankroll::Bankroll> {
    svc(&s)?.with_reader(lps_stats_engine::bankroll::bankroll).map_err(err)
}

#[tauri::command]
pub fn bankroll_tx_list(s: State<'_, AppState>) -> Res<Vec<lps_db::user::Tx>> {
    svc(&s)?.with_reader(lps_db::user::list_tx).map_err(err)
}

#[tauri::command]
pub fn bankroll_tx_add(s: State<'_, AppState>, tx: lps_db::user::Tx) -> Res<i64> {
    svc(&s)?.with_reader(|c| lps_db::user::add_tx(c, &tx)).map_err(err)
}

#[tauri::command]
pub fn bankroll_tx_delete(s: State<'_, AppState>, id: i64) -> Res<()> {
    svc(&s)?.with_reader(|c| lps_db::user::delete_tx(c, id)).map_err(err)
}

// ---- CSV export ----

fn csv_cell(v: &str) -> String {
    if v.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// Builds CSV text: `;` separator and a UTF-8 BOM, so Excel (French locale) opens it directly.
pub fn to_csv(rows: &[Vec<String>]) -> String {
    let mut out = String::from("\u{feff}");
    for r in rows {
        out.push_str(&r.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(";"));
        out.push_str("\r\n");
    }
    out
}

/// Writes `<data>/exports/<name>-<time>.csv`; returns its path.
#[tauri::command]
pub fn export_csv(s: State<'_, AppState>, name: String, rows: Vec<Vec<String>>) -> Res<String> {
    let safe: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).take(40).collect();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let dir = s.data_dir.path.join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| CommandError::from(e.to_string()))?;
    let path = dir.join(format!("{safe}-{now}.csv"));
    std::fs::write(&path, to_csv(&rows)).map_err(|e| CommandError::from(e.to_string()))?;
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn csv_is_excel_friendly() {
        let csv = super::to_csv(&[vec!["Position".into(), "VPIP".into()], vec!["BTN".into(), "31,2 %".into()], vec!["a;b".into(), "say \"hi\"".into()]]);
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("BTN;31,2 %\r\n"));
        assert!(csv.contains("\"a;b\";\"say \"\"hi\"\"\""));
    }
}
