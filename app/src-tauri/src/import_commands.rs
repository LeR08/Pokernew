//! Commands of the Import & Database module and the first-run wizard.

use std::path::PathBuf;

use lps_db::backup::{self, BackupInfo};
use lps_db::import::{list_errors, retry_errors, ImportErrorRow};
use lps_db::{stats, DbStats};
use lps_hh_parsers::install::{detect_accounts, BaseDirs, FoundAccount};
use tauri::{AppHandle, State};

use crate::commands::CommandError;
use crate::import_service::{ImportService, ImportStatus};
use crate::state::AppState;

type Res<T> = Result<T, CommandError>;

fn svc(s: &AppState) -> Res<&ImportService> {
    s.import.as_deref().ok_or_else(|| {
        CommandError::from(format!("database unavailable: {}", s.db_error.clone().unwrap_or_default()))
    })
}

fn err(e: lps_db::DbError) -> CommandError {
    CommandError::from(e.to_string())
}

#[tauri::command]
pub fn db_stats(s: State<'_, AppState>) -> Res<DbStats> {
    svc(&s)?.with_reader(stats).map_err(err)
}

#[tauri::command]
pub fn import_status(s: State<'_, AppState>) -> Res<ImportStatus> {
    Ok(svc(&s)?.status())
}

/// Manual import of files, folders or zip archives (runs in the background;
/// progress arrives as `import://event`).
#[tauri::command]
pub fn import_paths(s: State<'_, AppState>, paths: Vec<String>) -> Res<()> {
    let svc = svc(&s)?;
    for p in paths {
        svc.import_now(PathBuf::from(p)).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub fn import_errors(s: State<'_, AppState>, limit: Option<i64>) -> Res<Vec<ImportErrorRow>> {
    svc(&s)?.with_reader(|c| list_errors(c, limit.unwrap_or(200))).map_err(err)
}

/// Re-parses files that had errors (after a parser update).
#[tauri::command]
pub fn import_retry_errors(s: State<'_, AppState>) -> Res<usize> {
    let svc = svc(&s)?;
    let n = svc.with_reader(retry_errors).map_err(err)?;
    svc.restart_watcher(&s.store).map_err(err)?;
    Ok(n)
}

pub fn base_dirs() -> BaseDirs {
    BaseDirs { app_data: dirs::config_dir(), local_app_data: dirs::data_local_dir(), documents: dirs::document_dir() }
}

#[tauri::command]
pub fn detect_rooms() -> Vec<FoundAccount> {
    detect_accounts(&base_dirs())
}

#[tauri::command]
pub fn backup_list(s: State<'_, AppState>) -> Res<Vec<BackupInfo>> {
    backup::list(&svc(&s)?.backups_dir).map_err(err)
}

#[tauri::command]
pub fn backup_now(s: State<'_, AppState>) -> Res<String> {
    let keep = s
        .store
        .get("settings", "settings")
        .and_then(|v| v["backup"]["keep"].as_u64())
        .unwrap_or(7) as usize;
    let p = svc(&s)?.backup_now(keep).map_err(err)?;
    Ok(p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}

/// Schedules the restore; it is applied when the app restarts.
#[tauri::command]
pub fn backup_restore(s: State<'_, AppState>, name: String) -> Res<()> {
    let svc = svc(&s)?;
    backup::request_restore(&svc.data_dir, &svc.backups_dir, &name).map_err(err)
}

#[tauri::command]
pub fn app_restart(app: AppHandle) {
    app.restart();
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use lps_config::paths::{DataDir, DataDirSource};
    use serde_json::json;

    use crate::bus::AppEvent;
    use crate::commands::logic;
    use crate::state::AppState;

    fn fixtures() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/hh-parsers/fixtures/winamax").display().to_string()
    }

    #[test]
    fn configuring_a_room_folder_imports_its_hands() {
        let tmp = tempfile::tempdir().unwrap();
        let s = AppState::init_at(DataDir { path: tmp.path().join("data"), source: DataDirSource::Portable }, Instant::now()).unwrap();
        let svc = s.import.clone().expect("database opened");
        let mut rx = s.bus.subscribe();

        let mut room = logic::config_get(&s, "rooms", "winamax").unwrap();
        room["folders"] = json!([fixtures()]);
        room["hero_names"] = json!(["Tantaculejen"]);
        logic::save_value(&s, "rooms", "winamax", &room).unwrap();
        // In the app the bus loop restarts the watcher; do it directly here.
        svc.restart_watcher(&s.store).unwrap();

        let deadline = Instant::now() + Duration::from_secs(30);
        let mut inserted = 0;
        while inserted < 288 && Instant::now() < deadline {
            if let Ok(AppEvent::HandsImported(ev)) = rx.try_recv() {
                inserted += ev.report.hands_inserted;
            } else {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        assert_eq!(inserted, 288);
        let stats = svc.with_reader(lps_db::stats).unwrap();
        assert_eq!((stats.hands, stats.tournaments, stats.errors), (288, 3, 0));
        assert_eq!(svc.status().folders.len(), 1);
        assert!(tmp.path().join("data/database.sqlite").is_file());
    }
}
