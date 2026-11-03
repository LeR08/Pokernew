//! Database backups: `<data>/backups/database-YYYYMMDD-HHMMSS.sqlite`,
//! made with SQLite's online backup API (safe while the app is running).
//! Restoring is done at start-up, before the database is opened: the chosen
//! backup is recorded in `restore.pending` and copied over on next launch.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::DbError;

pub const PENDING: &str = "restore.pending";

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub size: u64,
    pub created_at: i64,
}

fn stamp(unix: i64) -> String {
    // UTC date for a stable, sortable name.
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Backs up `conn` into `dir`, then keeps only the `keep` most recent backups.
pub fn backup(
    conn: &Connection,
    dir: &Path,
    keep: usize,
    now_unix: i64,
) -> Result<PathBuf, DbError> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join(format!("database-{}.sqlite", stamp(now_unix)));
    conn.backup(rusqlite::MAIN_DB, &target, None)?;
    let mut all = list(dir)?;
    all.sort_by(|a, b| b.name.cmp(&a.name));
    for old in all.iter().skip(keep.max(1)) {
        let _ = std::fs::remove_file(dir.join(&old.name));
    }
    Ok(target)
}

pub fn list(dir: &Path) -> Result<Vec<BackupInfo>, DbError> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(out);
    };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("database-") && name.ends_with(".sqlite") {
            let meta = e.metadata()?;
            let created_at = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            out.push(BackupInfo {
                name,
                size: meta.len(),
                created_at,
            });
        }
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// The newest backup's age decides whether a scheduled backup is due.
pub fn is_due(dir: &Path, every_secs: i64, now_unix: i64) -> bool {
    list(dir)
        .ok()
        .and_then(|l| l.first().map(|b| now_unix - b.created_at >= every_secs))
        .unwrap_or(true)
}

/// Schedules `name` to replace the database at the next start.
pub fn request_restore(data_dir: &Path, backups_dir: &Path, name: &str) -> Result<(), DbError> {
    if name.contains(['/', '\\']) || !backups_dir.join(name).is_file() {
        return Err(DbError::Other(format!("unknown backup {name}")));
    }
    std::fs::write(data_dir.join(PENDING), name)?;
    Ok(())
}

/// Applies a pending restore. Call before opening the database.
pub fn apply_pending_restore(
    data_dir: &Path,
    backups_dir: &Path,
    db_path: &Path,
) -> Result<Option<String>, DbError> {
    let pending = data_dir.join(PENDING);
    let Ok(name) = std::fs::read_to_string(&pending) else {
        return Ok(None);
    };
    let name = name.trim().to_string();
    let src = backups_dir.join(&name);
    if src.is_file() {
        // Keep the current database next to the backups, just in case.
        if db_path.exists() {
            let _ = std::fs::copy(db_path, backups_dir.join("before-restore.sqlite"));
        }
        for ext in ["-wal", "-shm"] {
            let mut p = db_path.as_os_str().to_owned();
            p.push(ext);
            let _ = std::fs::remove_file(PathBuf::from(p));
        }
        std::fs::copy(&src, db_path)?;
    }
    std::fs::remove_file(&pending)?;
    Ok(Some(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps() {
        assert_eq!(stamp(1_793_644_483), "20261102-183443");
        assert_eq!(stamp(0), "19700101-000000");
    }
}
