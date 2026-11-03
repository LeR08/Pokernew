//! Import pipeline: files -> parser -> batched writes.
//!
//! * Files are recognised by their first bytes ([`lps_hh_parsers::detect`]).
//! * Imports are **incremental**: `import_files.offset` stores how many bytes
//!   were imported, always on a hand boundary, so a growing live file only
//!   costs the new hands. A hand still being written (no summary / no blank
//!   line after it yet) is left for the next pass.
//! * Hands are parsed in parallel (rayon) and written in transactions of
//!   [`BATCH`] hands. Duplicates — same room and hand number — are ignored.
//! * Parse errors are stored in `import_errors` with the raw hand text, never
//!   silently dropped.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use lps_hh_parsers::{detect, parser_for_room, Detected, HandHistoryParser, ParseError, RawHand};
use rayon::prelude::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::write::{insert_hand, upsert_summary, IdCache};
use crate::DbError;

/// Hands per write transaction.
pub const BATCH: usize = 1000;
/// Files larger than this are not hand histories.
const MAX_FILE: u64 = 512 * 1024 * 1024;
/// A file untouched for this long is finished: its last hand may be imported
/// even without the trailing blank lines.
const SETTLED_MS: i64 = 30_000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub files_seen: usize,
    pub files_imported: usize,
    pub hands_parsed: usize,
    pub hands_inserted: usize,
    pub duplicates: usize,
    pub errors: usize,
    pub summaries: usize,
    pub elapsed_ms: u64,
}

impl ImportReport {
    pub fn add(&mut self, o: &ImportReport) {
        self.files_seen += o.files_seen;
        self.files_imported += o.files_imported;
        self.hands_parsed += o.hands_parsed;
        self.hands_inserted += o.hands_inserted;
        self.duplicates += o.duplicates;
        self.errors += o.errors;
        self.summaries += o.summaries;
    }
    pub fn hands_per_sec(&self) -> f64 {
        if self.elapsed_ms == 0 {
            0.0
        } else {
            self.hands_parsed as f64 * 1000.0 / self.elapsed_ms as f64
        }
    }
}

struct FileRow {
    id: i64,
    size: i64,
    mtime: i64,
    offset: i64,
    room: Option<String>,
    kind: Option<String>,
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn mtime_ms(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Owns the writer connection. One importer per database.
pub struct Importer {
    conn: Connection,
    cache: IdCache,
    pool: rayon::ThreadPool,
}

impl Importer {
    /// `threads` = parse threads (0 = rayon default).
    pub fn new(conn: Connection, threads: usize) -> Result<Self, DbError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("lps-parse-{i}"))
            .build()
            .map_err(|e| DbError::Other(e.to_string()))?;
        Ok(Self { conn, cache: IdCache::default(), pool })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn into_conn(self) -> Connection {
        self.conn
    }

    /// Imports a file, a folder (recursively) or a `.zip` archive.
    pub fn import_path(&mut self, path: &Path) -> Result<ImportReport, DbError> {
        let start = Instant::now();
        let mut total = ImportReport::default();
        if path.is_dir() {
            let mut files = Vec::new();
            walk(path, &mut files);
            files.sort();
            for f in files {
                total.add(&self.import_any_file(&f)?);
            }
        } else {
            total.add(&self.import_any_file(path)?);
        }
        total.elapsed_ms = start.elapsed().as_millis() as u64;
        Ok(total)
    }

    fn import_any_file(&mut self, path: &Path) -> Result<ImportReport, DbError> {
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
            self.import_zip(path)
        } else {
            self.import_file(path)
        }
    }

    /// Imports one file, resuming from where the last import stopped.
    pub fn import_file(&mut self, path: &Path) -> Result<ImportReport, DbError> {
        let meta = match std::fs::metadata(path) {
            Ok(m) if m.is_file() && m.len() <= MAX_FILE => m,
            _ => return Ok(ImportReport::default()),
        };
        let key = path.to_string_lossy().into_owned();
        let size = meta.len() as i64;
        let mtime = mtime_ms(&meta);
        self.import_source(&key, size, mtime, |offset| {
            let mut f = std::fs::File::open(path)?;
            f.seek(SeekFrom::Start(offset))?;
            let mut buf = Vec::with_capacity((size as u64).saturating_sub(offset) as usize);
            f.read_to_end(&mut buf)?;
            Ok(buf)
        })
    }

    /// Imports every entry of a zip archive (e.g. a GGPoker/PokerCraft export).
    pub fn import_zip(&mut self, path: &Path) -> Result<ImportReport, DbError> {
        let file = std::fs::File::open(path)?;
        let mtime = file.metadata().map(|m| mtime_ms(&m)).unwrap_or(0);
        let mut zip = zip::ZipArchive::new(file).map_err(|e| DbError::Other(format!("{}: {e}", path.display())))?;
        let mut total = ImportReport::default();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| DbError::Other(e.to_string()))?;
            if !entry.is_file() || entry.size() > MAX_FILE {
                continue;
            }
            let key = format!("{}!{}", path.display(), entry.name());
            let size = entry.size() as i64;
            let mut buf = Vec::with_capacity(size as usize);
            entry.read_to_end(&mut buf)?;
            drop(entry);
            let r = self.import_source(&key, size, mtime, |offset| Ok(buf[offset as usize..].to_vec()))?;
            total.add(&r);
        }
        Ok(total)
    }

    fn file_row(&self, key: &str) -> Result<FileRow, DbError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO import_files (path, updated_at) VALUES (?, ?)",
            params![key, now_ms()],
        )?;
        Ok(self.conn.query_row(
            "SELECT id, size, mtime, offset, room, kind FROM import_files WHERE path = ?",
            [key],
            |r| {
                Ok(FileRow {
                    id: r.get(0)?,
                    size: r.get(1)?,
                    mtime: r.get(2)?,
                    offset: r.get(3)?,
                    room: r.get(4)?,
                    kind: r.get(5)?,
                })
            },
        )?)
    }

    fn import_source(
        &mut self,
        key: &str,
        size: i64,
        mtime: i64,
        read_from: impl FnOnce(u64) -> std::io::Result<Vec<u8>>,
    ) -> Result<ImportReport, DbError> {
        let start = Instant::now();
        let mut report = ImportReport { files_seen: 1, ..Default::default() };
        let row = self.file_row(key)?;
        let settled = now_ms() - mtime > SETTLED_MS;
        // Unchanged since last time: nothing to do (unless a pending tail can now be taken).
        if row.size == size && row.mtime == mtime && (row.offset == size || !settled || row.kind.as_deref() != Some("hands")) {
            return Ok(report);
        }
        // Shrunk or replaced: start over (duplicates are ignored anyway).
        let offset = if size < row.offset { 0 } else { row.offset };
        let bytes = read_from(offset as u64)?;
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
        };

        let detected: Option<(&'static dyn HandHistoryParser, &'static str)> = match (&row.room, &row.kind) {
            (Some(room), Some(kind)) if offset > 0 && kind != "unknown" => {
                parser_for_room(room).map(|p| (p, if kind == "summary" { "summary" } else { "hands" }))
            }
            _ => match detect(text.as_bytes()) {
                Some(Detected::Hands(p)) => Some((p, "hands")),
                Some(Detected::Summary(p)) => Some((p, "summary")),
                None => None,
            },
        };
        let Some((parser, kind)) = detected else {
            self.conn.execute(
                "UPDATE import_files SET kind = 'unknown', size = ?, mtime = ?, offset = ?, updated_at = ? WHERE id = ?",
                params![size, mtime, size, now_ms(), row.id],
            )?;
            return Ok(report);
        };
        report.files_imported = 1;

        let new_offset = if kind == "summary" {
            let tx = self.conn.transaction()?;
            match parser.parse_summary(&text) {
                Ok(s) => {
                    upsert_summary(&tx, &s)?;
                    report.summaries = 1;
                }
                Err(e) => {
                    store_error(&tx, row.id, offset, &e, &text)?;
                    report.errors = 1;
                }
            }
            tx.commit()?;
            size
        } else {
            let raws = parser.split(&text);
            let mut take: Vec<RawHand<'_>> = Vec::with_capacity(raws.len());
            for r in &raws {
                let tail_ok = settled && r.text.contains("*** SUMMARY ***");
                if r.complete || tail_ok {
                    take.push(*r);
                } else {
                    break;
                }
            }
            let consumed = take.last().map(|r| r.end).unwrap_or(0) as i64;
            let parsed: Vec<Result<lps_core_model::Hand, ParseError>> =
                self.pool.install(|| take.par_iter().map(|r| parser.parse(r)).collect());
            report.hands_parsed = parsed.len();
            for (chunk_raws, chunk) in take.chunks(BATCH).zip(parsed.chunks(BATCH)) {
                let tx = self.conn.transaction()?;
                for (raw, res) in chunk_raws.iter().zip(chunk) {
                    match res {
                        Ok(hand) => {
                            if insert_hand(&tx, &mut self.cache, hand, Some(row.id))? {
                                report.hands_inserted += 1;
                            } else {
                                report.duplicates += 1;
                            }
                        }
                        Err(e) => {
                            store_error(&tx, row.id, offset + raw.start as i64, e, raw.text)?;
                            report.errors += 1;
                        }
                    }
                }
                tx.commit()?;
            }
            offset + consumed
        };

        self.conn.execute(
            "UPDATE import_files SET room = ?, kind = ?, size = ?, mtime = ?, offset = ?,
                 hands = hands + ?, errors = errors + ?, updated_at = ? WHERE id = ?",
            params![
                parser.room_id(),
                kind,
                size,
                mtime,
                new_offset,
                report.hands_inserted as i64,
                report.errors as i64,
                now_ms(),
                row.id
            ],
        )?;
        report.elapsed_ms = start.elapsed().as_millis() as u64;
        Ok(report)
    }
}

fn store_error(tx: &rusqlite::Transaction<'_>, file_id: i64, offset: i64, e: &ParseError, raw: &str) -> Result<(), DbError> {
    tx.prepare_cached(
        "INSERT OR IGNORE INTO import_errors (file_id, byte_offset, line_no, line, message, raw, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )?
    .execute(params![file_id, offset, e.line_no as i64, e.line, e.message, raw, now_ms()])?;
    Ok(())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() => walk(&p, out),
            Ok(t) if t.is_file() => out.push(p),
            _ => {}
        }
    }
}

/// Error rows for the import log screen.
#[derive(Debug, Clone, Serialize)]
pub struct ImportErrorRow {
    pub id: i64,
    pub file: String,
    pub byte_offset: i64,
    pub line_no: i64,
    pub line: String,
    pub message: String,
    pub raw: String,
    pub created_at: i64,
}

pub fn list_errors(conn: &Connection, limit: i64) -> Result<Vec<ImportErrorRow>, DbError> {
    let mut st = conn.prepare(
        "SELECT e.id, f.path, e.byte_offset, e.line_no, e.line, e.message, e.raw, e.created_at
         FROM import_errors e JOIN import_files f ON f.id = e.file_id ORDER BY e.id DESC LIMIT ?",
    )?;
    let rows = st.query_map([limit], |r| {
        Ok(ImportErrorRow {
            id: r.get(0)?,
            file: r.get(1)?,
            byte_offset: r.get(2)?,
            line_no: r.get(3)?,
            line: r.get(4)?,
            message: r.get(5)?,
            raw: r.get(6)?,
            created_at: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Forgets stored errors and offsets of files with errors, so they are
/// parsed again (after a parser update).
pub fn retry_errors(conn: &Connection) -> Result<usize, DbError> {
    let n = conn.execute(
        "UPDATE import_files SET offset = 0, size = -1, errors = 0 WHERE id IN (SELECT DISTINCT file_id FROM import_errors)",
        [],
    )?;
    conn.execute("DELETE FROM import_errors", [])?;
    Ok(n)
}

pub fn file_offset(conn: &Connection, key: &str) -> Result<Option<i64>, DbError> {
    Ok(conn.query_row("SELECT offset FROM import_files WHERE path = ?", [key], |r| r.get(0)).optional()?)
}
