//! Commands of the Equity Lab module and of the range library.
//!
//! Fair play: Equity Lab is a study module. Every command refuses to run while
//! the HUD sees an open table (play mode), in addition to the locked tile.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use lps_equity::breakdown::{self, Breakdown};
use lps_equity::texture::{self, FlopFilter, TextureReport};
use lps_equity::{equity, EquityResult, Mode, Options, Range};
use lps_evaluator::{parse_cards, Card};
use serde::Deserialize;
use tauri::State;

use crate::commands::CommandError;
use crate::hud::HudService;
use crate::state::AppState;

type Res<T> = Result<T, CommandError>;

/// Cancels the running calculation.
#[derive(Default)]
pub struct EquityJobs {
    cancel: Arc<AtomicBool>,
}

fn study_guard(hud: &HudService) -> Res<()> {
    if hud.play_mode.load(Ordering::SeqCst) {
        return Err(CommandError::from(
            "study modules are locked while a poker table is open".to_string(),
        ));
    }
    Ok(())
}

fn cards(s: &str) -> Res<Vec<Card>> {
    parse_cards(s).ok_or_else(|| CommandError::from(format!("invalid cards '{s}'")))
}

fn range(s: &str) -> Res<Range> {
    Range::parse(s).map_err(|e| CommandError::from(e.to_string()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalcRequest {
    pub players: Vec<String>,
    pub board: String,
    pub dead: String,
    #[serde(default)]
    pub mode: Mode,
    /// 95 % half-width, e.g. 0.001.
    pub precision: Option<f64>,
    pub max_ms: Option<u64>,
}

pub fn run_calc(req: &CalcRequest, cancel: Option<&AtomicBool>) -> Res<EquityResult> {
    let ranges = req
        .players
        .iter()
        .map(|p| range(p))
        .collect::<Res<Vec<_>>>()?;
    let opts = Options {
        mode: req.mode,
        precision: req.precision.unwrap_or(0.001).clamp(0.0001, 0.05),
        max_ms: req.max_ms.unwrap_or(30_000).clamp(100, 600_000),
        ..Options::default()
    };
    equity(
        &ranges,
        &cards(&req.board)?,
        &cards(&req.dead)?,
        &opts,
        cancel,
    )
    .map_err(|e| CommandError::from(e.to_string()))
}

#[tauri::command]
pub async fn equity_calc(
    hud: State<'_, Arc<HudService>>,
    jobs: State<'_, EquityJobs>,
    req: CalcRequest,
) -> Res<EquityResult> {
    study_guard(&hud)?;
    jobs.cancel.store(false, Ordering::SeqCst);
    let cancel = jobs.cancel.clone();
    tauri::async_runtime::spawn_blocking(move || run_calc(&req, Some(&cancel)))
        .await
        .map_err(|e| CommandError::from(e.to_string()))?
}

#[tauri::command]
pub fn equity_cancel(jobs: State<'_, EquityJobs>) {
    jobs.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn range_breakdown(
    hud: State<'_, Arc<HudService>>,
    range_text: String,
    board: String,
    dead: String,
) -> Res<Breakdown> {
    study_guard(&hud)?;
    let b = cards(&board)?;
    if !(3..=5).contains(&b.len()) {
        return Err(CommandError::from(
            "the board needs 3 to 5 cards".to_string(),
        ));
    }
    Ok(breakdown::breakdown(
        &range(&range_text)?,
        &b,
        &cards(&dead)?,
    ))
}

/// The combos of `range_text` having one of the selected classes or draws, as 1 326 weights.
#[tauri::command]
pub fn range_filter(
    hud: State<'_, Arc<HudService>>,
    range_text: String,
    board: String,
    dead: String,
    made: Vec<String>,
    draws: Vec<String>,
) -> Res<Vec<f32>> {
    study_guard(&hud)?;
    let made: Vec<_> = made
        .iter()
        .filter_map(|m| breakdown::parse_made(m))
        .collect();
    let draws: Vec<_> = draws
        .iter()
        .filter_map(|d| breakdown::parse_draw(d))
        .collect();
    let r = breakdown::filter(
        &range(&range_text)?,
        &cards(&board)?,
        &cards(&dead)?,
        &made,
        &draws,
    );
    Ok(r.weights)
}

#[tauri::command]
pub async fn flop_texture(
    hud: State<'_, Arc<HudService>>,
    range_text: String,
    filter: FlopFilter,
) -> Res<TextureReport> {
    study_guard(&hud)?;
    let r = range(&range_text)?;
    tauri::async_runtime::spawn_blocking(move || texture::texture(&r, &filter))
        .await
        .map_err(|e| CommandError::from(e.to_string()))
}

// ---- Range library: <data>/ranges/**/*.txt ----

pub fn ranges_dir(data: &Path) -> PathBuf {
    data.join("ranges")
}

/// Checks a library path like `6max/100bb/CO_open` (".txt" optional) and returns the file.
pub fn library_file(data: &Path, rel: &str) -> Res<PathBuf> {
    let rel = rel.trim().trim_end_matches(".txt");
    let ok_segment = |s: &str| {
        !s.is_empty()
            && s != "."
            && s != ".."
            && s.chars()
                .all(|c| c.is_alphanumeric() || " _-+.%()".contains(c))
    };
    if rel.is_empty() || !rel.split('/').all(ok_segment) {
        return Err(CommandError::from(format!("invalid range name '{rel}'")));
    }
    Ok(ranges_dir(data).join(format!("{rel}.txt")))
}

/// Every saved range, as `folder/sub/name` (without ".txt"), sorted.
pub fn list_library(data: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else if p.extension().is_some_and(|x| x == "txt") {
                if let Ok(rel) = p.strip_prefix(base) {
                    let s = rel.with_extension("").to_string_lossy().replace('\\', "/");
                    out.push(s);
                }
            }
        }
    }
    let base = ranges_dir(data);
    let mut out = Vec::new();
    walk(&base, &base, &mut out);
    out.sort();
    out
}

#[tauri::command]
pub fn ranges_list(s: State<'_, AppState>) -> Vec<String> {
    list_library(&s.data_dir.path)
}

#[tauri::command]
pub fn range_load(s: State<'_, AppState>, name: String) -> Res<String> {
    let f = library_file(&s.data_dir.path, &name)?;
    std::fs::read_to_string(&f).map_err(|e| CommandError::from(format!("{}: {e}", f.display())))
}

#[tauri::command]
pub fn range_save(s: State<'_, AppState>, name: String, text: String) -> Res<()> {
    range(&text)?; // never save an unreadable range
    let f = library_file(&s.data_dir.path, &name)?;
    if let Some(dir) = f.parent() {
        std::fs::create_dir_all(dir).map_err(|e| CommandError::from(e.to_string()))?;
    }
    std::fs::write(&f, format!("{}\n", text.trim())).map_err(|e| CommandError::from(e.to_string()))
}

#[tauri::command]
pub fn range_delete(s: State<'_, AppState>, name: String) -> Res<()> {
    let f = library_file(&s.data_dir.path, &name)?;
    std::fs::remove_file(&f).map_err(|e| CommandError::from(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        assert!(library_file(d, "../x").is_err());
        assert!(library_file(d, "a//b").is_err());
        assert!(library_file(d, "").is_err());
        let f = library_file(d, "6max/100bb/CO_open").unwrap();
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(&f, "22+,A2s+\n").unwrap();
        std::fs::write(library_file(d, "BTN 3bet").unwrap(), "QQ+\n").unwrap();
        assert_eq!(
            list_library(d),
            vec!["6max/100bb/CO_open".to_string(), "BTN 3bet".to_string()]
        );
    }

    #[test]
    fn calc_request() {
        let req = CalcRequest {
            players: vec!["AsAh".into(), "KsKh".into()],
            board: String::new(),
            dead: String::new(),
            mode: Mode::Auto,
            precision: None,
            max_ms: None,
        };
        let r = run_calc(&req, None).unwrap();
        assert!((r.players[0].equity - 0.82637).abs() < 1e-4);
        let bad = CalcRequest {
            players: vec!["AsAh".into(), "XYZ".into()],
            ..req
        };
        assert!(run_calc(&bad, None).is_err());
    }
}
