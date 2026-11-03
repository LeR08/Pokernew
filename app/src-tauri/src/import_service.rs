//! Runs the hand-history import in the background and answers the UI.
//!
//! * the folder watcher (own writer connection) imports new hands and
//!   publishes [`AppEvent::HandsImported`];
//! * a reader connection serves stats and the error log;
//! * a scheduler thread makes the periodic database backup.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lps_config::ConfigStore;
use lps_db::watch::{FolderWatcher, WatchEvent};
use lps_db::{backup, import::Importer, DbError};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;

use crate::bus::{AppEvent, EventBus};

pub const DB_FILE: &str = "database.sqlite";
pub const BACKUPS_DIR: &str = "backups";
const HISTORY: usize = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRecord {
    pub at: i64,
    pub trigger: &'static str,
    pub report: lps_db::import::ImportReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStatus {
    pub room: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStatus {
    pub folders: Vec<FolderStatus>,
    pub watching: bool,
    pub history: Vec<ImportRecord>,
    pub db_path: String,
}

pub struct ImportService {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub backups_dir: PathBuf,
    reader: Mutex<Connection>,
    watcher: Mutex<Option<FolderWatcher>>,
    folders: Mutex<Vec<FolderStatus>>,
    history: Arc<Mutex<VecDeque<ImportRecord>>>,
    bus: EventBus,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Enabled rooms' folders from `rooms/*.toml`.
pub fn configured_folders(store: &ConfigStore) -> Vec<FolderStatus> {
    let mut out = Vec::new();
    for id in store.list("rooms") {
        let Some(room) = store.get("rooms", &id) else { continue };
        if room["enabled"] != Value::Bool(true) {
            continue;
        }
        for f in room["folders"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            out.push(FolderStatus { room: id.clone(), path: f.to_string(), exists: Path::new(f).is_dir() });
        }
    }
    out
}

fn worker_threads(store: &ConfigStore) -> usize {
    store
        .get("settings", "settings")
        .and_then(|s| s["performance"]["worker_threads"].as_u64())
        .unwrap_or(0) as usize
}

impl ImportService {
    /// Applies a pending restore, opens/migrates the database and starts the watcher.
    pub fn start(data_dir: &Path, store: &ConfigStore, bus: EventBus) -> Result<Arc<Self>, DbError> {
        let db_path = data_dir.join(DB_FILE);
        let backups_dir = data_dir.join(BACKUPS_DIR);
        if let Some(name) = backup::apply_pending_restore(data_dir, &backups_dir, &db_path)? {
            log::info!("restored backup {name}");
        }
        drop(lps_db::open(&db_path)?); // migrate once before anyone reads
        let svc = Arc::new(Self {
            data_dir: data_dir.to_path_buf(),
            reader: Mutex::new(lps_db::open_reader(&db_path)?),
            db_path,
            backups_dir,
            watcher: Mutex::new(None),
            folders: Mutex::new(vec![]),
            history: Arc::new(Mutex::new(VecDeque::new())),
            bus,
        });
        svc.restart_watcher(store)?;
        Ok(svc)
    }

    /// (Re)starts the watcher with the folders currently configured.
    pub fn restart_watcher(&self, store: &ConfigStore) -> Result<(), DbError> {
        let folders = configured_folders(store);
        let paths: Vec<PathBuf> = folders.iter().filter(|f| f.exists).map(|f| PathBuf::from(&f.path)).collect();
        let mut guard = self.watcher.lock().map_err(|_| DbError::Other("watcher lock poisoned".into()))?;
        *guard = None; // stop the old one first (joins its thread)
        let importer = Importer::new(lps_db::open(&self.db_path)?, worker_threads(store))?;
        let bus = self.bus.clone();
        let history = self.history.clone();
        *guard = Some(FolderWatcher::start(importer, paths, move |ev: WatchEvent| {
            if let Ok(mut h) = history.lock() {
                h.push_front(ImportRecord { at: now(), trigger: ev.trigger, report: ev.report.clone() });
                h.truncate(HISTORY);
            }
            bus.publish(AppEvent::HandsImported(ev));
        })?);
        if let Ok(mut f) = self.folders.lock() {
            *f = folders;
        }
        Ok(())
    }

    pub fn import_now(&self, path: PathBuf) -> Result<(), DbError> {
        let guard = self.watcher.lock().map_err(|_| DbError::Other("watcher lock poisoned".into()))?;
        guard.as_ref().ok_or_else(|| DbError::Other("import service stopped".into()))?.import_now(path);
        Ok(())
    }

    pub fn status(&self) -> ImportStatus {
        ImportStatus {
            folders: self.folders.lock().map(|f| f.clone()).unwrap_or_default(),
            watching: self.watcher.lock().map(|w| w.is_some()).unwrap_or(false),
            history: self.history.lock().map(|h| h.iter().cloned().collect()).unwrap_or_default(),
            db_path: self.db_path.display().to_string(),
        }
    }

    pub fn with_reader<T>(&self, f: impl FnOnce(&Connection) -> Result<T, DbError>) -> Result<T, DbError> {
        let conn = self.reader.lock().map_err(|_| DbError::Other("reader lock poisoned".into()))?;
        f(&conn)
    }

    pub fn backup_now(&self, keep: usize) -> Result<PathBuf, DbError> {
        let conn = lps_db::open_reader(&self.db_path)?;
        backup::backup(&conn, &self.backups_dir, keep, now())
    }

    /// Checks every 10 minutes whether the scheduled backup is due.
    pub fn spawn_backup_scheduler(self: &Arc<Self>, store: Arc<ConfigStore>) {
        let svc = Arc::clone(self);
        let _ = std::thread::Builder::new().name("lps-backup".into()).spawn(move || loop {
            std::thread::sleep(Duration::from_secs(60));
            let cfg = store.get("settings", "settings").map(|s| s["backup"].clone()).unwrap_or(Value::Null);
            let enabled = cfg["enabled"].as_bool().unwrap_or(true);
            let every = cfg["every_hours"].as_i64().unwrap_or(24) * 3600;
            let keep = cfg["keep"].as_u64().unwrap_or(7) as usize;
            if enabled && backup::is_due(&svc.backups_dir, every, now()) {
                match svc.backup_now(keep) {
                    Ok(p) => log::info!("backup written to {}", p.display()),
                    Err(e) => log::warn!("backup failed: {e}"),
                }
            }
            std::thread::sleep(Duration::from_secs(540));
        });
    }
}
