//! Folder watcher: imports new and growing files as soon as they change.
//!
//! On start, every folder is scanned once (only new bytes are read thanks to
//! the stored offsets, so this is how the watcher "resumes after a restart");
//! then file events are debounced ([`DEBOUNCE`]) and imported.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use notify_debouncer_mini::notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};

use crate::import::{ImportReport, Importer};
use crate::DbError;

pub const DEBOUNCE: Duration = Duration::from_millis(250);

enum Msg {
    Paths(Vec<PathBuf>),
    ImportNow(PathBuf),
    Rescan,
    Stop,
}

/// Running watcher. Dropping it stops the thread.
pub struct FolderWatcher {
    tx: mpsc::Sender<Msg>,
    thread: Option<JoinHandle<()>>,
    _debouncer:
        Option<notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>>,
    pub folders: Vec<PathBuf>,
}

/// What the watcher reports after each import pass.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WatchEvent {
    pub report: ImportReport,
    /// "scan" (startup / rescan), "watch" (file change) or "manual".
    pub trigger: &'static str,
}

impl FolderWatcher {
    /// Starts watching `folders` with `importer`; `on_import` runs on the
    /// watcher thread after every pass that did something.
    pub fn start<F>(
        importer: Importer,
        folders: Vec<PathBuf>,
        on_import: F,
    ) -> Result<Self, DbError>
    where
        F: Fn(WatchEvent) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Msg>();
        let tx2 = tx.clone();
        let mut debouncer = new_debouncer(DEBOUNCE, move |res: DebounceEventResult| {
            if let Ok(events) = res {
                let _ = tx2.send(Msg::Paths(events.into_iter().map(|e| e.path).collect()));
            }
        })
        .map_err(|e| DbError::Other(e.to_string()))?;
        for f in &folders {
            if let Err(e) = debouncer.watcher().watch(f, RecursiveMode::Recursive) {
                log::warn!("cannot watch {}: {e}", f.display());
            }
        }
        let importer = Arc::new(Mutex::new(importer));
        let folders2 = folders.clone();
        let thread = std::thread::Builder::new()
            .name("lps-import".into())
            .spawn(move || run(importer, folders2, rx, on_import))
            .map_err(|e| DbError::Other(e.to_string()))?;
        tx.send(Msg::Rescan).ok();
        Ok(Self {
            tx,
            thread: Some(thread),
            _debouncer: Some(debouncer),
            folders,
        })
    }

    /// Imports a file, folder or zip on the watcher thread (manual import).
    pub fn import_now(&self, path: PathBuf) {
        let _ = self.tx.send(Msg::ImportNow(path));
    }

    pub fn rescan(&self) {
        let _ = self.tx.send(Msg::Rescan);
    }
}

impl Drop for FolderWatcher {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run<F: Fn(WatchEvent)>(
    importer: Arc<Mutex<Importer>>,
    folders: Vec<PathBuf>,
    rx: mpsc::Receiver<Msg>,
    on_import: F,
) {
    let emit = |r: Result<ImportReport, DbError>, trigger: &'static str| match r {
        Ok(report) if report.files_imported > 0 => on_import(WatchEvent { report, trigger }),
        Ok(_) => {}
        Err(e) => log::warn!("import failed: {e}"),
    };
    while let Ok(msg) = rx.recv() {
        let Ok(mut imp) = importer.lock() else { return };
        match msg {
            Msg::Stop => return,
            Msg::Rescan => {
                let mut total = ImportReport::default();
                let start = std::time::Instant::now();
                for f in &folders {
                    match imp.import_path(f) {
                        Ok(r) => total.add(&r),
                        Err(e) => log::warn!("scan {}: {e}", f.display()),
                    }
                }
                total.elapsed_ms = start.elapsed().as_millis() as u64;
                emit(Ok(total), "scan");
            }
            Msg::ImportNow(p) => emit(imp.import_path(&p), "manual"),
            Msg::Paths(first) => {
                // Coalesce bursts of events.
                let mut set: BTreeSet<PathBuf> = first.into_iter().collect();
                while let Ok(Msg::Paths(more)) = rx.try_recv() {
                    set.extend(more);
                }
                let mut total = ImportReport::default();
                let start = std::time::Instant::now();
                for p in set.iter().filter(|p| is_candidate(p)) {
                    match imp.import_file(p) {
                        Ok(r) => total.add(&r),
                        Err(e) => log::warn!("import {}: {e}", p.display()),
                    }
                }
                total.elapsed_ms = start.elapsed().as_millis() as u64;
                emit(Ok(total), "watch");
            }
        }
    }
}

fn is_candidate(p: &Path) -> bool {
    p.is_file() && !p.extension().is_some_and(|e| e == "tmp" || e == "part")
}
