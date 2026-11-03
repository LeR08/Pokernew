//! Tauri commands. Each is a thin wrapper over a plain function taking
//! `&AppState`, so the logic is tested without a window.

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use lps_config::{ConfigError, ConfigEvent, Issue};

use crate::bus::AppEvent;
use crate::state::AppState;

/// Error shape returned to the front end.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub message: String,
    pub issues: Vec<Issue>,
}

impl From<ConfigError> for CommandError {
    fn from(e: ConfigError) -> Self {
        match e {
            ConfigError::Invalid(issues) => Self { message: "invalid config".into(), issues },
            other => Self { message: other.to_string(), issues: vec![] },
        }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self { message, issues: vec![] }
    }
}

type Res<T> = Result<T, CommandError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: &'static str,
    pub data_dir: String,
    pub data_dir_source: lps_config::paths::DataDirSource,
    pub backend_ready_ms: u64,
    pub mock: bool,
}

pub mod logic {
    use super::*;

    pub fn app_info(s: &AppState) -> AppInfo {
        AppInfo {
            version: env!("CARGO_PKG_VERSION"),
            data_dir: s.data_dir.path.display().to_string(),
            data_dir_source: s.data_dir.source,
            backend_ready_ms: s.backend_ready_ms,
            mock: false,
        }
    }

    pub fn config_get(s: &AppState, family: &str, id: &str) -> Res<Value> {
        s.store
            .get(family, id)
            .ok_or_else(|| format!("{family}/{id} not found or invalid").into())
    }

    /// Saves and publishes the change on the bus right away (the file
    /// watcher sees the write too, but the store dedups identical content).
    pub fn publish(s: &AppState, e: ConfigEvent) {
        s.bus.publish(AppEvent::Config(e));
    }

    pub fn save_text(s: &AppState, family: &str, id: &str, text: &str) -> Res<()> {
        let e = s.store.save_text(family, id, text)?;
        publish(s, e);
        Ok(())
    }

    pub fn save_value(s: &AppState, family: &str, id: &str, value: &Value) -> Res<()> {
        let e = s.store.save_value(family, id, value)?;
        publish(s, e);
        Ok(())
    }

    pub fn delete(s: &AppState, family: &str, id: &str) -> Res<()> {
        if family == "settings" || family == "hotkeys" {
            return Err(format!("{family} cannot be deleted").into());
        }
        if s.store.list(family).len() <= 1 {
            return Err(format!("the last file of {family} cannot be deleted").into());
        }
        let e = s.store.delete(family, id)?;
        publish(s, e);
        Ok(())
    }

    pub fn is_valid_module_id(id: &str) -> bool {
        !id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
}

#[tauri::command]
pub fn app_info(s: State<'_, AppState>) -> AppInfo {
    logic::app_info(&s)
}

#[tauri::command]
pub fn config_get(s: State<'_, AppState>, family: String, id: String) -> Res<Value> {
    logic::config_get(&s, &family, &id)
}

#[tauri::command]
pub fn config_list(s: State<'_, AppState>, family: String) -> Vec<String> {
    s.store.list(&family)
}

#[tauri::command]
pub fn config_raw(s: State<'_, AppState>, family: String, id: String) -> Res<String> {
    Ok(s.store.raw_text(&family, &id)?)
}

#[tauri::command]
pub fn config_validate(s: State<'_, AppState>, family: String, id: String, text: String) -> Vec<Issue> {
    s.store.validate_text(&family, &id, &text).err().unwrap_or_default()
}

#[tauri::command]
pub fn config_save_text(s: State<'_, AppState>, family: String, id: String, text: String) -> Res<()> {
    logic::save_text(&s, &family, &id, &text)
}

#[tauri::command]
pub fn config_save_value(s: State<'_, AppState>, family: String, id: String, value: Value) -> Res<()> {
    logic::save_value(&s, &family, &id, &value)
}

#[tauri::command]
pub fn config_delete(s: State<'_, AppState>, family: String, id: String) -> Res<()> {
    logic::delete(&s, &family, &id)
}

#[tauri::command]
pub fn config_issues(s: State<'_, AppState>) -> Vec<Issue> {
    s.store.issues()
}

#[tauri::command]
pub fn schema_get(s: State<'_, AppState>, family: String) -> Res<Value> {
    s.store.schema(&family).ok_or_else(|| format!("unknown family {family}").into())
}

/// Logs total start-up time (process start -> first screen rendered) to
/// `<data>/logs/startup.log`, to check the < 2 s target on the real PC.
#[tauri::command]
pub fn frontend_ready(s: State<'_, AppState>) {
    use std::io::Write;
    let total = s.started.elapsed().as_millis();
    log::info!("start-up: backend {} ms, first screen {} ms", s.backend_ready_ms, total);
    let dir = s.data_dir.path.join("logs");
    if std::fs::create_dir_all(&dir).is_ok() {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("startup.log")) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(f, "{ts}\tbackend_ms={}\tfirst_screen_ms={total}", s.backend_ready_ms);
        }
    }
}

/// Opens a module in its own window (or focuses it if already open).
#[tauri::command]
pub fn open_module_window(app: AppHandle, module_id: String) -> Res<()> {
    if !logic::is_valid_module_id(&module_id) {
        return Err(format!("invalid module id {module_id}").into());
    }
    let label = format!("module-{module_id}");
    if let Some(w) = app.get_webview_window(&label) {
        let _ = w.unminimize();
        return w.set_focus().map_err(|e| e.to_string().into());
    }
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App(format!("index.html#/window/{module_id}").into()))
        .title(format!("Local Poker Suite — {module_id}"))
        .inner_size(1100.0, 760.0)
        .min_inner_size(640.0, 480.0)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string().into())
}

#[cfg(test)]
mod tests {
    use super::logic;
    use crate::state::AppState;
    use lps_config::paths::{DataDir, DataDirSource};
    use serde_json::json;
    use std::time::Instant;

    fn state() -> (tempfile::TempDir, AppState) {
        let tmp = tempfile::tempdir().unwrap();
        let dd = DataDir { path: tmp.path().join("data"), source: DataDirSource::Portable };
        let s = AppState::init_at(dd, Instant::now()).unwrap();
        (tmp, s)
    }

    #[test]
    fn first_run_creates_defaults_and_reports_portable() {
        let (tmp, s) = state();
        assert!(tmp.path().join("data/settings.toml").is_file());
        let info = logic::app_info(&s);
        assert_eq!(serde_json::to_value(&info).unwrap()["dataDirSource"], "portable");
    }

    #[test]
    fn save_value_publishes_on_bus() {
        let (_tmp, s) = state();
        let mut rx = s.bus.subscribe();
        let mut v = logic::config_get(&s, "settings", "settings").unwrap();
        v["appearance"]["density"] = json!("compact");
        logic::save_value(&s, "settings", "settings", &v).unwrap();
        let e = rx.try_recv().unwrap();
        assert_eq!(serde_json::to_value(&e).unwrap()["payload"]["kind"], "changed");
        assert_eq!(logic::config_get(&s, "settings", "settings").unwrap()["appearance"]["density"], "compact");
    }

    #[test]
    fn invalid_save_returns_issues() {
        let (_tmp, s) = state();
        let err = logic::save_text(&s, "themes", "bad", "name = \"x\"\nbase = \"blue\"\n").unwrap_err();
        assert!(!err.issues.is_empty());
    }

    #[test]
    fn settings_and_last_file_cannot_be_deleted() {
        let (_tmp, s) = state();
        assert!(logic::delete(&s, "settings", "settings").is_err());
        logic::delete(&s, "themes", "light").unwrap();
        assert!(logic::delete(&s, "themes", "dark").is_err());
    }

    #[test]
    fn module_ids_are_checked() {
        assert!(logic::is_valid_module_id("solver"));
        assert!(!logic::is_valid_module_id("../x"));
        assert!(!logic::is_valid_module_id("a b"));
    }
}
