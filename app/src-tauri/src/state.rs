//! Application state shared by every command, independent of Tauri so it
//! can be unit-tested.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use lps_config::paths::{self, DataDir, ResolveInput};
use lps_config::watch::{self, ConfigWatcher};
use lps_config::{defaults, ConfigError, ConfigStore};

use crate::bus::{AppEvent, EventBus};
use crate::import_service::ImportService;

pub struct AppState {
    pub data_dir: DataDir,
    pub store: Arc<ConfigStore>,
    pub bus: EventBus,
    pub started: Instant,
    pub backend_ready_ms: u64,
    /// `None` when the database could not be opened (the error is in `db_error`).
    pub import: Option<Arc<ImportService>>,
    pub db_error: Option<String>,
    _watcher: Mutex<Option<ConfigWatcher>>,
}

impl AppState {
    /// Resolves the data folder, installs missing defaults, loads every
    /// config file and starts hot reload.
    pub fn init(input: &ResolveInput, started: Instant) -> Result<Self, String> {
        let data_dir =
            paths::resolve(input).ok_or("cannot determine a data folder (use --data-dir)")?;
        Self::init_at(data_dir, started)
    }

    pub fn init_at(data_dir: DataDir, started: Instant) -> Result<Self, String> {
        let report = defaults::install(&data_dir.path)
            .map_err(|e| format!("cannot write data folder {}: {e}", data_dir.path.display()))?;
        for p in &report.created {
            log::info!("created default {}", p.display());
        }
        let store =
            Arc::new(ConfigStore::open(&data_dir.path).map_err(|e: ConfigError| e.to_string())?);
        for issue in store.issues() {
            log::warn!(
                "config issue {}/{}{}: {}",
                issue.family,
                issue.id,
                issue.pointer,
                issue.message
            );
        }
        let bus = EventBus::default();
        let bus2 = bus.clone();
        let watcher = watch::watch(store.clone(), move |e| bus2.publish(AppEvent::Config(e)))
            .map_err(|e| log::warn!("hot reload disabled: {e}"))
            .ok();
        let (import, db_error) = match ImportService::start(&data_dir.path, &store, bus.clone()) {
            Ok(svc) => {
                svc.spawn_backup_scheduler(store.clone());
                (Some(svc), None)
            }
            Err(e) => {
                log::error!("database unavailable: {e}");
                (None, Some(e.to_string()))
            }
        };
        Ok(Self {
            data_dir,
            store,
            bus,
            import,
            db_error,
            started,
            backend_ready_ms: started.elapsed().as_millis() as u64,
            _watcher: Mutex::new(watcher),
        })
    }
}
