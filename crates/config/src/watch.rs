//! Hot reload: watches the data folder and reloads changed config files.
//!
//! Events are debounced (200 ms) so an editor's save (often several writes
//! or a rename) results in a single reload.

use std::sync::Arc;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};

use crate::store::{ConfigEvent, ConfigStore};
use crate::ConfigError;

/// Debounce window for file events.
pub const DEBOUNCE: Duration = Duration::from_millis(200);

/// Keeps the watcher alive; dropping it stops hot reload.
pub struct ConfigWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

/// Starts watching `store.root()`; `on_event` is called from the watcher
/// thread for every effective change.
pub fn watch<F>(store: Arc<ConfigStore>, on_event: F) -> Result<ConfigWatcher, ConfigError>
where
    F: Fn(ConfigEvent) + Send + 'static,
{
    let root = store.root().to_path_buf();
    let mut debouncer = new_debouncer(DEBOUNCE, move |res: DebounceEventResult| match res {
        Ok(events) => {
            let mut paths: Vec<_> = events.into_iter().map(|e| e.path).collect();
            paths.sort();
            paths.dedup();
            for p in paths {
                if let Some(ev) = store.reload_path(&p) {
                    on_event(ev);
                }
            }
        }
        Err(e) => log::warn!("config watcher error: {e}"),
    })
    .map_err(|e| ConfigError::Watch(e.to_string()))?;
    debouncer
        .watcher()
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| ConfigError::Watch(e.to_string()))?;
    Ok(ConfigWatcher { _debouncer: debouncer })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    #[test]
    fn edit_on_disk_is_reloaded_within_a_second() {
        let tmp = tempfile::tempdir().unwrap();
        crate::defaults::install(tmp.path()).unwrap();
        let store = Arc::new(ConfigStore::open(tmp.path()).unwrap());
        let (tx, rx) = mpsc::channel();
        let _w = watch(store.clone(), move |e| {
            let _ = tx.send(e);
        })
        .unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let path = tmp.path().join("settings.toml");
        let text = std::fs::read_to_string(&path).unwrap().replace("language = \"fr\"", "language = \"en\"");
        let start = Instant::now();
        std::fs::write(&path, text).unwrap();

        let ev = rx.recv_timeout(Duration::from_secs(5)).expect("no reload event");
        assert_eq!(ev, ConfigEvent::Changed { family: "settings".into(), id: "settings".into() });
        assert!(start.elapsed() < Duration::from_secs(1), "took {:?}", start.elapsed());
        assert_eq!(store.get("settings", "settings").unwrap()["general"]["language"], "en");
    }
}
