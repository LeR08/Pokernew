//! Commands of the Trainer module: drills on cached solutions, the drill
//! history, and hand analysis against the solver (background queue).
//! Study module: refused in play mode; the analysis queue waits while a
//! poker table is open.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use lps_core_model::Hand;
use lps_solver::cache;
use lps_solver::job::{solve_now, SolveParams};
use lps_solver::Tree;
use lps_trainer::analysis::{grade_line, prepare, HandAnalysis};
use lps_trainer::drill::{self, Answer, Mode, Question};
use lps_trainer::Rng;
use postflop_solver::PostFlopGame;
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::commands::CommandError;
use crate::equity_commands::library_file;
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

#[derive(Default)]
pub struct TrainerState {
    /// Solutions loaded for drills (by cache hash), at most two.
    games: Mutex<Vec<(String, PostFlopGame)>>,
    analysis: Mutex<Option<Arc<AnalysisJob>>>,
}

fn cache_dir(s: &AppState) -> PathBuf {
    s.data_dir.path.join("solver-cache")
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

fn with_game<T>(
    s: &AppState,
    st: &TrainerState,
    hash: &str,
    f: impl FnOnce(&mut PostFlopGame) -> T,
) -> Res<T> {
    let mut games = st.games.lock().unwrap();
    if !games.iter().any(|(h, _)| h == hash) {
        let (game, _, _) = cache::load(&cache_dir(s), hash).map_err(e)?;
        games.insert(0, (hash.to_string(), game));
        games.truncate(2);
    }
    let g = games.iter_mut().find(|(h, _)| h == hash).unwrap();
    Ok(f(&mut g.1))
}

fn history_file(s: &AppState) -> PathBuf {
    s.data_dir.path.join("trainer").join("history.jsonl")
}

fn read_history(path: &Path, limit: usize) -> Vec<Value> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut v: Vec<Value> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if v.len() > limit {
        v.drain(..v.len() - limit);
    }
    v
}

/// Average loss (% of pot) per node key, from the postflop history.
fn weak_spots(path: &Path) -> HashMap<String, f64> {
    let mut sum: HashMap<String, (f64, f64)> = HashMap::new();
    for v in read_history(path, 2000) {
        if let (Some(k), Some(l)) = (v["nodeKey"].as_str(), v["lossPct"].as_f64()) {
            let x = sum.entry(k.to_string()).or_default();
            x.0 += l;
            x.1 += 1.0;
        }
    }
    sum.into_iter().map(|(k, (s, n))| (k, s / n)).collect()
}

#[tauri::command]
pub fn trainer_deal(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    st: State<'_, TrainerState>,
    hash: String,
    mode: Mode,
    hero: Option<usize>,
    weak: bool,
) -> Res<Question> {
    study_guard(&hud)?;
    let w = if weak {
        Some(weak_spots(&history_file(&s)))
    } else {
        None
    };
    with_game(&s, &st, &hash, |g| {
        drill::deal(g, mode, hero, w.as_ref(), &mut Rng(seed()))
    })?
    .ok_or_else(|| e("no question could be dealt from this solution"))
}

#[tauri::command]
pub fn trainer_answer(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    st: State<'_, TrainerState>,
    hash: String,
    question: Question,
    chosen: usize,
) -> Res<Answer> {
    study_guard(&hud)?;
    with_game(&s, &st, &hash, |g| drill::answer(g, &question, chosen))?
        .ok_or_else(|| e("invalid answer"))
}

#[tauri::command]
pub fn trainer_next(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    st: State<'_, TrainerState>,
    hash: String,
    question: Question,
    chosen: usize,
) -> Res<Option<Question>> {
    study_guard(&hud)?;
    with_game(&s, &st, &hash, |g| {
        drill::next(g, &question, chosen, &mut Rng(seed()))
    })
}

/// Appends one answered question (any JSON object) to <data>/trainer/history.jsonl.
#[tauri::command]
pub fn trainer_record(s: State<'_, AppState>, entry: Value) -> Res<()> {
    let f = history_file(&s);
    std::fs::create_dir_all(f.parent().unwrap()).map_err(e)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&f)
        .map_err(e)?;
    writeln!(file, "{}", serde_json::to_string(&entry).map_err(e)?).map_err(e)
}

#[tauri::command]
pub fn trainer_history(s: State<'_, AppState>, limit: usize) -> Vec<Value> {
    read_history(&history_file(&s), limit.clamp(1, 100_000))
}

// ---- hand analysis queue ----

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRow {
    pub hand_id: i64,
    pub played_at: i64,
    pub analysis: Option<HandAnalysis>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProgress {
    pub total: usize,
    pub done: usize,
    pub running: bool,
    /// Waiting because a poker table is open.
    pub waiting_for_tables: bool,
    pub rows: Vec<AnalysisRow>,
}

pub struct AnalysisJob {
    progress: Mutex<AnalysisProgress>,
    cancel: AtomicBool,
}

#[allow(clippy::too_many_arguments)]
fn run_analysis(
    job: &AnalysisJob,
    hands: Vec<(i64, Hand)>,
    format: String,
    tree: Tree,
    params: SolveParams,
    data: PathBuf,
    play_mode: Arc<HudService>,
) {
    let charts = |name: &str| {
        library_file(&data, name)
            .ok()
            .and_then(|f| std::fs::read_to_string(f).ok())
    };
    let dir = data.join("solver-cache");
    for (id, hand) in hands {
        // never solve while a table is open (fair play)
        while play_mode.play_mode.load(Ordering::SeqCst) && !job.cancel.load(Ordering::SeqCst) {
            job.progress.lock().unwrap().waiting_for_tables = true;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        job.progress.lock().unwrap().waiting_for_tables = false;
        if job.cancel.load(Ordering::SeqCst) {
            break;
        }
        let res = prepare(&hand, &format, &tree, &charts)
            .map_err(|x| x.to_string())
            .and_then(|prep| {
                let key = cache::key(&prep.spot, &[]);
                let mut game = match cache::load(&dir, &key) {
                    Ok((g, _, _)) => g,
                    Err(_) => {
                        let est = lps_solver::estimate(&prep.spot).map_err(|x| x.to_string())?;
                        let mb = (if params.compression {
                            est.memory_compressed
                        } else {
                            est.memory
                        }) as f64
                            / 1048576.0;
                        if mb > params.memory_cap_mb as f64 {
                            return Err(format!(
                                "{mb:.0} MB needed, above the {} MB cap",
                                params.memory_cap_mb
                            ));
                        }
                        let (g, expl) =
                            solve_now(&prep.spot, &params, &[]).map_err(|x| x.to_string())?;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let _ = cache::save(
                            &dir,
                            &format!("analyse · {} · {}", prep.spot.board, hand.hand_no),
                            &prep.spot,
                            &[],
                            &g,
                            expl,
                            params.max_iterations,
                            now,
                        );
                        g
                    }
                };
                grade_line(&mut game, &hand, &prep).map_err(|x| x.to_string())
            });
        let mut p = job.progress.lock().unwrap();
        let (analysis, error) = match res {
            Ok(a) => (Some(a), None),
            Err(x) => (None, Some(x)),
        };
        p.rows.push(AnalysisRow {
            hand_id: id,
            played_at: hand.played_at,
            analysis,
            error,
        });
        p.done += 1;
    }
    job.progress.lock().unwrap().running = false;
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn analysis_start(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    st: State<'_, TrainerState>,
    hand_ids: Vec<i64>,
    format: String,
    tree: Tree,
    params: SolveParams,
) -> Res<()> {
    study_guard(&hud)?;
    let mut slot = st.analysis.lock().unwrap();
    if slot
        .as_ref()
        .is_some_and(|j| j.progress.lock().unwrap().running)
    {
        return Err(e("an analysis is already running"));
    }
    let svc = s
        .import
        .as_deref()
        .ok_or_else(|| e("database unavailable"))?;
    let hands: Vec<(i64, Hand)> = svc
        .with_reader(|c| {
            hand_ids
                .iter()
                .map(|&id| lps_db::load::load_hand(c, id).map(|(h, _)| (id, h)))
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(e)?;
    let job = Arc::new(AnalysisJob {
        progress: Mutex::new(AnalysisProgress {
            total: hands.len(),
            done: 0,
            running: true,
            waiting_for_tables: false,
            rows: vec![],
        }),
        cancel: AtomicBool::new(false),
    });
    let j = job.clone();
    let data = s.data_dir.path.clone();
    let hud = hud.inner().clone();
    std::thread::Builder::new()
        .name("lps-analysis".into())
        .spawn(move || run_analysis(&j, hands, format, tree, params, data, hud))
        .map_err(e)?;
    *slot = Some(job);
    Ok(())
}

#[tauri::command]
pub fn analysis_progress(st: State<'_, TrainerState>) -> Option<AnalysisProgress> {
    st.analysis
        .lock()
        .unwrap()
        .as_ref()
        .map(|j| j.progress.lock().unwrap().clone())
}

#[tauri::command]
pub fn analysis_cancel(st: State<'_, TrainerState>) {
    if let Some(j) = st.analysis.lock().unwrap().as_ref() {
        j.cancel.store(true, Ordering::SeqCst);
    }
}
