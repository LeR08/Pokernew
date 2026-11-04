//! Commands of the Preflop module: solver and range bundles (import / export of
//! the range library). Study module: refused in play mode.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use lps_preflop::{bundle, Config, Solution};
use tauri::State;

use crate::commands::CommandError;
use crate::equity_commands::{library_file, list_library};
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

#[tauri::command]
pub async fn preflop_solve(
    hud: State<'_, Arc<HudService>>,
    config: Config,
    target: f64,
    max_iterations: usize,
) -> Res<Solution> {
    study_guard(&hud)?;
    tauri::async_runtime::spawn_blocking(move || {
        lps_preflop::solve(
            &config,
            target.max(0.0),
            max_iterations.clamp(10, 20_000),
            |_| true,
        )
        .map_err(e)
    })
    .await
    .map_err(e)?
}

/// Every saved range under `prefix` ("" = all) as one bundle text.
pub fn export_bundle(data: &std::path::Path, prefix: &str) -> Res<String> {
    let prefix = prefix.trim().trim_end_matches('/');
    let mut out = vec![];
    for name in list_library(data) {
        if prefix.is_empty() || name == prefix || name.starts_with(&format!("{prefix}/")) {
            let f = library_file(data, &name)?;
            out.push((name, std::fs::read_to_string(f).map_err(e)?));
        }
    }
    Ok(bundle::export(&out))
}

/// Saves every range of a bundle under `prefix`; returns how many were written.
pub fn import_bundle(
    data: &std::path::Path,
    text: &str,
    prefix: &str,
    overwrite: bool,
) -> Res<usize> {
    let ranges = bundle::import(text).map_err(e)?;
    let prefix = prefix.trim().trim_matches('/');
    let mut n = 0;
    for (name, body) in ranges {
        let full = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let f = library_file(data, &full)?;
        if f.exists() && !overwrite {
            continue;
        }
        if let Some(dir) = f.parent() {
            std::fs::create_dir_all(dir).map_err(e)?;
        }
        std::fs::write(&f, format!("{}\n", body.trim())).map_err(e)?;
        n += 1;
    }
    Ok(n)
}

#[tauri::command]
pub fn ranges_export(s: State<'_, AppState>, prefix: String) -> Res<String> {
    export_bundle(&s.data_dir.path, &prefix)
}

#[tauri::command]
pub fn ranges_import(
    s: State<'_, AppState>,
    text: String,
    prefix: String,
    overwrite: bool,
) -> Res<usize> {
    import_bundle(&s.data_dir.path, &text, &prefix, overwrite)
}

/// Writes a bundle to <data>/exports/<name>.lpsranges; returns the file.
#[tauri::command]
pub fn ranges_export_file(s: State<'_, AppState>, prefix: String) -> Res<String> {
    let text = export_bundle(&s.data_dir.path, &prefix)?;
    let dir = s.data_dir.path.join("exports");
    std::fs::create_dir_all(&dir).map_err(e)?;
    let stem: String = if prefix.trim().is_empty() {
        "ranges".into()
    } else {
        prefix.replace(['/', '\\', ' '], "_")
    };
    let file = dir.join(format!("{stem}.lpsranges"));
    std::fs::write(&file, text).map_err(e)?;
    Ok(file.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Acceptance test of phase 7: export then import gives back the same ranges.
    #[test]
    fn library_round_trip() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        for (name, text) in [
            (
                "cash 6max 100bb/BTN/open",
                "22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+",
            ),
            ("cash 6max 100bb/BB/3bet vs BTN", "QQ+,AKs,A5s:50%,KQo:0.25"),
            ("other/x", "AA"),
        ] {
            let f = library_file(a.path(), name).unwrap();
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, text).unwrap();
        }
        let text = export_bundle(a.path(), "cash 6max 100bb").unwrap();
        assert_eq!(import_bundle(b.path(), &text, "", false).unwrap(), 2);
        assert_eq!(
            list_library(b.path()),
            vec![
                "cash 6max 100bb/BB/3bet vs BTN".to_string(),
                "cash 6max 100bb/BTN/open".to_string()
            ]
        );
        for name in list_library(b.path()) {
            let x = std::fs::read_to_string(library_file(a.path(), &name).unwrap()).unwrap();
            let y = std::fs::read_to_string(library_file(b.path(), &name).unwrap()).unwrap();
            assert_eq!(
                lps_equity::Range::parse(&x).unwrap().weights,
                lps_equity::Range::parse(&y).unwrap().weights
            );
        }
        // nothing overwritten without asking
        assert_eq!(import_bundle(b.path(), &text, "", false).unwrap(), 0);
        assert_eq!(import_bundle(b.path(), &text, "copie", false).unwrap(), 2);
    }
}
