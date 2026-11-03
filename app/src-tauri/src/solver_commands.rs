//! Commands of the postflop solver module. Study module: refused in play mode.
//!
//! One solve runs at a time. Finished solves go to `<data>/solver-cache/`;
//! the viewer works on up to two loaded solutions (slots 0 and 1, for side
//! by side comparison).

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use lps_equity::breakdown::{breakdown, Breakdown};
use lps_equity::Range;
use lps_solver::cache::{self, Entry};
use lps_solver::job::{Job, Lock, Progress, SolveParams, State as JobState};
use lps_solver::{estimate, view, Estimate, NodeView, SpotConfig};
use postflop_solver::PostFlopGame;
use tauri::State;

use crate::commands::CommandError;
use crate::hud::HudService;
use crate::state::AppState;

type Res<T> = Result<T, CommandError>;

fn e(x: impl std::fmt::Display) -> CommandError {
    CommandError::from(x.to_string())
}

fn study_guard(hud: &HudService) -> Res<()> {
    if hud.play_mode.load(Ordering::SeqCst) {
        return Err(e("study modules are locked while a poker table is open"));
    }
    Ok(())
}

struct Loaded {
    hash: String,
    spot: SpotConfig,
    game: PostFlopGame,
}

#[derive(Default)]
pub struct SolverState {
    job: Mutex<Option<(Arc<Job>, String)>>,
    slots: Mutex<[Option<Loaded>; 2]>,
}

fn cache_dir(s: &AppState) -> PathBuf {
    s.data_dir.path.join("solver-cache")
}

#[tauri::command]
pub fn solver_estimate(hud: State<'_, Arc<HudService>>, spot: SpotConfig) -> Res<Estimate> {
    study_guard(&hud)?;
    estimate(&spot).map_err(e)
}

#[tauri::command]
pub fn solver_start(
    hud: State<'_, Arc<HudService>>,
    st: State<'_, SolverState>,
    spot: SpotConfig,
    params: SolveParams,
    locks: Vec<Lock>,
    name: String,
) -> Res<()> {
    study_guard(&hud)?;
    let mut job = st.job.lock().unwrap();
    if let Some((j, _)) = job.as_ref() {
        if matches!(
            j.progress().state,
            JobState::Building | JobState::Solving | JobState::Paused
        ) {
            return Err(e("a solve is already running"));
        }
    }
    spot.build().map_err(e)?; // report input errors right away
    *job = Some((Job::start(spot, params, locks), name));
    Ok(())
}

#[tauri::command]
pub fn solver_progress(st: State<'_, SolverState>) -> Option<Progress> {
    st.job.lock().unwrap().as_ref().map(|(j, _)| j.progress())
}

#[tauri::command]
pub fn solver_control(st: State<'_, SolverState>, action: String) -> Res<()> {
    let job = st.job.lock().unwrap();
    let (j, _) = job.as_ref().ok_or_else(|| e("no solve running"))?;
    match action.as_str() {
        "pause" => j.pause(),
        "resume" => j.resume(),
        "cancel" => j.cancel(),
        _ => return Err(e("unknown action")),
    }
    Ok(())
}

/// Once the solve is done: saves it to the cache and loads it in slot 0.
#[tauri::command]
pub fn solver_finish(s: State<'_, AppState>, st: State<'_, SolverState>) -> Res<Entry> {
    let (job, name) = st.job.lock().unwrap().take().ok_or_else(|| e("no solve"))?;
    let p = job.progress();
    let game = job
        .take_game()
        .ok_or_else(|| e("the solve is not finished"))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let entry = cache::save(
        &cache_dir(&s),
        &name,
        &job.spot,
        &job.locks,
        &game,
        p.exploitability_pct,
        p.iteration,
        now,
    )
    .map_err(e)?;
    st.slots.lock().unwrap()[0] = Some(Loaded {
        hash: entry.hash.clone(),
        spot: job.spot.clone(),
        game,
    });
    Ok(entry)
}

#[tauri::command]
pub fn solver_cache_list(s: State<'_, AppState>) -> Vec<Entry> {
    cache::list(&cache_dir(&s))
}

#[tauri::command]
pub async fn solver_cache_load(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    st: State<'_, SolverState>,
    hash: String,
    slot: usize,
) -> Res<SpotConfig> {
    study_guard(&hud)?;
    if slot > 1 {
        return Err(e("slot 0 or 1"));
    }
    let dir = cache_dir(&s);
    let h = hash.clone();
    let (game, spot, _) = tauri::async_runtime::spawn_blocking(move || cache::load(&dir, &h))
        .await
        .map_err(e)?
        .map_err(e)?;
    st.slots.lock().unwrap()[slot] = Some(Loaded {
        hash,
        spot: spot.clone(),
        game,
    });
    Ok(spot)
}

#[tauri::command]
pub fn solver_cache_delete(
    s: State<'_, AppState>,
    st: State<'_, SolverState>,
    hash: String,
) -> Res<()> {
    let mut slots = st.slots.lock().unwrap();
    for slot in slots.iter_mut() {
        if slot.as_ref().is_some_and(|l| l.hash == hash) {
            *slot = None;
        }
    }
    cache::delete(&cache_dir(&s), &hash).map_err(e)
}

#[tauri::command]
pub fn solver_cache_rename(s: State<'_, AppState>, hash: String, name: String) -> Res<()> {
    cache::rename(&cache_dir(&s), &hash, name.trim()).map_err(e)
}

#[tauri::command]
pub fn solver_view(
    hud: State<'_, Arc<HudService>>,
    st: State<'_, SolverState>,
    slot: usize,
    path: Vec<usize>,
) -> Res<NodeView> {
    study_guard(&hud)?;
    let mut slots = st.slots.lock().unwrap();
    let l = slots
        .get_mut(slot)
        .and_then(|x| x.as_mut())
        .ok_or_else(|| e("no solution loaded"))?;
    view(&mut l.game, &path).map_err(e)
}

/// The range of `player` at a node, broken down by made hand and draws (Equity Lab).
#[tauri::command]
pub fn solver_breakdown(
    hud: State<'_, Arc<HudService>>,
    st: State<'_, SolverState>,
    slot: usize,
    path: Vec<usize>,
    player: usize,
) -> Res<Breakdown> {
    study_guard(&hud)?;
    let mut slots = st.slots.lock().unwrap();
    let l = slots
        .get_mut(slot)
        .and_then(|x| x.as_mut())
        .ok_or_else(|| e("no solution loaded"))?;
    let v = view(&mut l.game, &path).map_err(e)?;
    let p = v
        .players
        .get(player)
        .ok_or_else(|| e("only at decision nodes"))?;
    let mut w = vec![0.0f32; lps_equity::combo::NUM_COMBOS];
    for h in &p.hands {
        w[h.combo as usize] = h.weight;
    }
    let board: Vec<u8> = v
        .board
        .iter()
        .filter_map(|c| lps_evaluator::parse_card(c))
        .collect();
    Ok(breakdown(&Range::from_weights(w), &board, &[]))
}

/// Writes the node's strategy to <data>/exports as CSV or JSON; returns the file.
#[tauri::command]
pub fn solver_export(
    s: State<'_, AppState>,
    st: State<'_, SolverState>,
    slot: usize,
    path: Vec<usize>,
    format: String,
) -> Res<String> {
    let mut slots = st.slots.lock().unwrap();
    let l = slots
        .get_mut(slot)
        .and_then(|x| x.as_mut())
        .ok_or_else(|| e("no solution loaded"))?;
    let v = view(&mut l.game, &path).map_err(e)?;
    let dir = s.data_dir.path.join("exports");
    std::fs::create_dir_all(&dir).map_err(e)?;
    let stem = format!(
        "solver-{}-{}",
        l.hash,
        path.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("-")
    );
    let (file, text) = if format == "json" {
        (
            dir.join(format!("{stem}.json")),
            serde_json::to_string_pretty(&v).map_err(e)?,
        )
    } else {
        (
            dir.join(format!("{stem}.csv")),
            lps_solver::view::to_csv(&v),
        )
    };
    std::fs::write(&file, text).map_err(e)?;
    Ok(file.display().to_string())
}

/// Spot of a loaded solution (to re-solve it with locks).
#[tauri::command]
pub fn solver_loaded_spot(st: State<'_, SolverState>, slot: usize) -> Option<(String, SpotConfig)> {
    st.slots
        .lock()
        .unwrap()
        .get(slot)
        .and_then(|x| x.as_ref())
        .map(|l| (l.hash.clone(), l.spot.clone()))
}
