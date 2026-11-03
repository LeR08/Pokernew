//! Solution cache: `<data>/solver-cache/<hash>.bin` (zstd-compressed engine
//! files) plus `index.json` with what each file holds.

use std::path::{Path, PathBuf};

use postflop_solver::{load_data_from_file, save_data_to_file, PostFlopGame};
use serde::{Deserialize, Serialize};

use crate::job::Lock;
use crate::spot::SpotConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub hash: String,
    pub name: String,
    pub created_at: i64,
    pub spot: SpotConfig,
    pub locks: Vec<Lock>,
    pub exploitability_pct: f64,
    pub iterations: u32,
    pub bytes: u64,
}

/// What a cache file's memo holds.
#[derive(Serialize, Deserialize)]
struct Memo {
    spot: SpotConfig,
    locks: Vec<Lock>,
}

/// Key of a solution: the spot plus its locks.
pub fn key(spot: &SpotConfig, locks: &[Lock]) -> String {
    if locks.is_empty() {
        return spot.hash();
    }
    let mut s = spot.clone();
    s.oop_range
        .push_str(&serde_json::to_string(locks).unwrap_or_default());
    format!("{}-l", s.hash())
}

fn index_path(dir: &Path) -> PathBuf {
    dir.join("index.json")
}

pub fn list(dir: &Path) -> Vec<Entry> {
    std::fs::read_to_string(index_path(dir))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_index(dir: &Path, entries: &[Entry]) -> Result<(), String> {
    std::fs::write(
        index_path(dir),
        serde_json::to_string_pretty(entries).unwrap(),
    )
    .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn save(
    dir: &Path,
    name: &str,
    spot: &SpotConfig,
    locks: &[Lock],
    game: &PostFlopGame,
    exploitability_pct: f64,
    iterations: u32,
    now: i64,
) -> Result<Entry, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let hash = key(spot, locks);
    let file = dir.join(format!("{hash}.bin"));
    let memo = serde_json::to_string(&Memo {
        spot: spot.clone(),
        locks: locks.to_vec(),
    })
    .unwrap();
    save_data_to_file(game, &memo, &file, Some(3))?;
    let bytes = std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0);
    let entry = Entry {
        hash: hash.clone(),
        name: name.to_string(),
        created_at: now,
        spot: spot.clone(),
        locks: locks.to_vec(),
        exploitability_pct,
        iterations,
        bytes,
    };
    let mut all: Vec<Entry> = list(dir).into_iter().filter(|e| e.hash != hash).collect();
    all.insert(0, entry.clone());
    write_index(dir, &all)?;
    Ok(entry)
}

pub fn load(dir: &Path, hash: &str) -> Result<(PostFlopGame, SpotConfig, Vec<Lock>), String> {
    if !hash
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-' || c == 'l')
    {
        return Err("invalid solution id".into());
    }
    let (game, memo): (PostFlopGame, String) =
        load_data_from_file(dir.join(format!("{hash}.bin")), None)?;
    let m: Memo = serde_json::from_str(&memo).map_err(|e| e.to_string())?;
    Ok((game, m.spot, m.locks))
}

pub fn delete(dir: &Path, hash: &str) -> Result<(), String> {
    let all: Vec<Entry> = list(dir).into_iter().filter(|e| e.hash != hash).collect();
    let _ = std::fs::remove_file(dir.join(format!("{hash}.bin")));
    write_index(dir, &all)
}

pub fn rename(dir: &Path, hash: &str, name: &str) -> Result<(), String> {
    let mut all = list(dir);
    for e in all.iter_mut().filter(|e| e.hash == hash) {
        e.name = name.to_string();
    }
    write_index(dir, &all)
}
