use std::path::{Path, PathBuf};

use lps_hh_parsers::HandHistoryParser;
use rusqlite::Connection;

use crate::import::Importer;
use crate::*;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hh-parsers/fixtures/winamax")
}

const TRIDENT: &str = "20261102_TRIDENT_SPACE_KO1195602081_real_holdem_no-limit.txt";

fn importer(db: &Path) -> Importer {
    Importer::new(open(db).unwrap(), 2).unwrap()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn imports_all_fixtures_without_errors_and_no_duplicates_on_reimport() {
    let tmp = tempfile::tempdir().unwrap();
    let mut imp = importer(&tmp.path().join("db.sqlite"));
    let r = imp.import_path(&fixtures()).unwrap();
    assert_eq!((r.hands_inserted, r.errors, r.summaries), (288, 0, 3), "{r:?}");
    let s = stats(imp.conn()).unwrap();
    assert_eq!((s.hands, s.hero_hands, s.tournaments), (288, 288, 4));

    // Same folder again: files unchanged -> nothing read.
    let r2 = imp.import_path(&fixtures()).unwrap();
    assert_eq!((r2.hands_parsed, r2.hands_inserted), (0, 0));

    // Same hands under another path: parsed again, all recognised as duplicates.
    let copy = tmp.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    std::fs::copy(fixtures().join(TRIDENT), copy.join("x.txt")).unwrap();
    let r3 = imp.import_path(&copy).unwrap();
    assert_eq!((r3.hands_inserted, r3.duplicates), (0, 24));
    assert_eq!(count(imp.conn(), "SELECT COUNT(*) FROM hands"), 288);
}

#[test]
fn summaries_complete_the_tournament_rows() {
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    imp.import_path(&fixtures()).unwrap();
    let (name, buy_in, bounty, fee, pos, won, bwon, hands): (String, i64, i64, i64, i64, i64, i64, i64) = imp
        .conn()
        .query_row(
            "SELECT t.name, buy_in_cents, bounty_cents, fee_cents, finish_position, won_cents, bounty_won_cents,
                    (SELECT COUNT(*) FROM hands h WHERE h.tournament_id = t.id)
             FROM tournaments t WHERE room_tournament_id = '1195603616'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
        )
        .unwrap();
    assert_eq!((name.as_str(), buy_in, bounty, fee, pos, won, bwon, hands), ("SPACE KO", 45, 25, 5, 203, 52, 51, 176));
}

#[test]
fn live_file_is_imported_incrementally_and_resumes_after_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("db.sqlite");
    let file = tmp.path().join("live.txt");
    let full = std::fs::read_to_string(fixtures().join(TRIDENT)).unwrap();
    // Cut in the middle of hand 11.
    let cut = full.match_indices("Winamax Poker - ").nth(10).unwrap().0 + 300;
    std::fs::write(&file, &full[..cut]).unwrap();

    {
        let mut imp = importer(&db);
        let r = imp.import_file(&file).unwrap();
        assert_eq!(r.hands_inserted, 10, "the hand being written is left for later");
    }
    // "Restart": new importer on the same database; the file has grown by one hand.
    let next_hand_end = full.match_indices("Winamax Poker - ").nth(11).unwrap().0;
    std::fs::write(&file, &full[..next_hand_end]).unwrap();
    let mut imp = importer(&db);
    let r = imp.import_file(&file).unwrap();
    assert_eq!((r.hands_parsed, r.hands_inserted), (1, 1), "only the new hand is parsed");

    std::fs::write(&file, &full).unwrap();
    let r = imp.import_file(&file).unwrap();
    assert_eq!((r.hands_parsed, r.hands_inserted, r.errors), (13, 13, 0));
    assert_eq!(count(imp.conn(), "SELECT COUNT(*) FROM hands"), 24);
    assert_eq!(import::file_offset(imp.conn(), &file.to_string_lossy()).unwrap(), Some(full.len() as i64));
}

#[test]
fn stored_hands_reload_identically() {
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    imp.import_file(&fixtures().join(TRIDENT)).unwrap();
    let text = std::fs::read_to_string(fixtures().join(TRIDENT)).unwrap();
    let wx = lps_hh_parsers::winamax::Winamax;
    for raw in wx.split(&text) {
        let parsed = wx.parse(&raw).unwrap();
        let id = write::hand_id(imp.conn(), "winamax", &parsed.hand_no).unwrap().unwrap();
        let (loaded, _) = load::load_hand(imp.conn(), id).unwrap();
        assert_eq!(loaded, parsed);
    }
}

#[test]
fn net_won_sums_to_zero_and_matches_stack_changes() {
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    imp.import_path(&fixtures()).unwrap();
    // No rake in tournaments: every hand is zero-sum.
    assert_eq!(count(imp.conn(), "SELECT COUNT(*) FROM (SELECT hand_id FROM hand_players GROUP BY hand_id HAVING SUM(net_won) <> 0)"), 0);
    // Hero stack continuity inside the 3-max table: stack(n) + net(n) = stack(n+1).
    let mut st = imp
        .conn()
        .prepare(
            "SELECT hp.stack, hp.net_won FROM hand_players hp JOIN hands h ON h.id = hp.hand_id
             WHERE hp.is_hero = 1 AND h.table_name LIKE 'TRIDENT%' ORDER BY h.played_at",
        )
        .unwrap();
    let rows: Vec<(i64, i64)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(Result::unwrap).collect();
    for w in rows.windows(2) {
        assert_eq!(w[0].0 + w[0].1, w[1].0);
    }
}

/// Counters of the first TRIDENT hand, checked by hand against the text:
/// BTN (Player34) limps, SB (Player10) raises, hero (BB) folds, BTN calls;
/// SB c-bets flop, BTN calls; bet/call turn and river; showdown, BTN wins.
#[test]
fn counters_on_a_hand_checked_by_hand() {
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    imp.import_file(&fixtures().join(TRIDENT)).unwrap();
    let get = |name: &str, col: &str| -> i64 {
        imp.conn()
            .query_row(
                &format!(
                    "SELECT s.{col} FROM hand_player_stats s JOIN players p ON p.id = s.player_id
                     JOIN hands h ON h.id = s.hand_id WHERE h.hand_no = '5135071836924543030-33-1793644483' AND p.name = ?"
                ),
                [name],
                |r| r.get(0),
            )
            .unwrap()
    };
    let btn = "Player34";
    let sb = "Player10";
    let hero = "Tantaculejen";
    assert_eq!((get(btn, "cnt_vpip"), get(btn, "cnt_pfr"), get(btn, "cnt_limp")), (1, 0, 1));
    assert_eq!((get(btn, "opp_steal"), get(btn, "cnt_steal")), (1, 0));
    assert_eq!((get(btn, "opp_3bet"), get(btn, "cnt_3bet")), (1, 0));
    assert_eq!((get(sb, "cnt_vpip"), get(sb, "cnt_pfr"), get(sb, "opp_steal")), (1, 1, 0));
    assert_eq!((get(hero, "cnt_vpip"), get(hero, "opp_3bet"), get(hero, "cnt_3bet")), (0, 1, 0));
    assert_eq!(get(hero, "opp_fold_to_steal"), 0, "a limp is not a steal");
    assert_eq!((get(sb, "opp_cbet_flop"), get(sb, "cnt_cbet_flop")), (1, 1));
    assert_eq!((get(btn, "opp_fold_to_cbet_flop"), get(btn, "cnt_fold_to_cbet_flop")), (1, 0));
    assert_eq!((get(btn, "cnt_saw_flop"), get(hero, "cnt_saw_flop")), (1, 0));
    assert_eq!((get(btn, "cnt_wtsd"), get(btn, "cnt_wsd"), get(sb, "cnt_wsd")), (1, 1, 0));
    assert_eq!((get(sb, "cnt_postflop_bets_raises"), get(btn, "cnt_postflop_calls")), (3, 3));
    assert_eq!(get(btn, "cnt_allin_preflop"), 0);
}

#[test]
fn steal_and_fold_to_steal() {
    // TRIDENT hand 37: BTN (Player10) raises, SB hero folds, BB folds.
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    imp.import_file(&fixtures().join(TRIDENT)).unwrap();
    let row = |name: &str| -> (i64, i64, i64, i64) {
        imp.conn()
            .query_row(
                "SELECT opp_steal, cnt_steal, opp_fold_to_steal, cnt_fold_to_steal FROM hand_player_stats s
                 JOIN players p ON p.id = s.player_id JOIN hands h ON h.id = s.hand_id
                 WHERE h.hand_no = '5135071836924543030-37-1793644598' AND p.name = ?",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap()
    };
    assert_eq!(row("Player10"), (1, 1, 0, 0));
    assert_eq!(row("Tantaculejen"), (0, 0, 1, 1));
    assert_eq!(row("Player34"), (0, 0, 1, 1));
}

#[test]
fn recompute_gives_identical_counters() {
    let mut conn = open_in_memory().unwrap();
    {
        let mut imp = Importer::new(conn, 1).unwrap();
        imp.import_path(&fixtures()).unwrap();
        conn = Connection::open_in_memory().unwrap();
        // Snapshot the counters, recompute, compare.
        let snap = |c: &Connection| -> Vec<Vec<i64>> {
            let cols = counters::COUNTERS.join(", ");
            let mut st = c.prepare(&format!("SELECT {cols} FROM hand_player_stats ORDER BY hand_id, player_id")).unwrap();
            st.query_map([], |r| (0..counters::COUNTERS.len()).map(|i| r.get(i)).collect())
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        let before = snap(imp.conn());
        let mut c2 = std::mem::replace(&mut imp, Importer::new(open_in_memory().unwrap(), 1).unwrap()).into_conn();
        c2.execute("UPDATE hand_player_stats SET cnt_vpip = 0, cnt_wtsd = 0", []).unwrap();
        assert_eq!(recompute_counters(&mut c2).unwrap(), 288);
        assert_eq!(snap(&c2), before);
    }
    drop(conn);
}

#[test]
fn parse_errors_are_stored_with_the_raw_hand() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("bad.txt");
    let text = std::fs::read_to_string(fixtures().join(TRIDENT)).unwrap().replacen("Tantaculejen folds", "Tantaculejen dances", 1);
    std::fs::write(&file, text).unwrap();
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    let r = imp.import_file(&file).unwrap();
    assert_eq!((r.hands_inserted, r.errors), (23, 1));
    let errs = import::list_errors(imp.conn(), 10).unwrap();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, "Tantaculejen dances");
    assert!(errs[0].raw.starts_with("Winamax Poker - Tournament"));
    // After a fix (here: restoring the file), retry imports the hand.
    std::fs::copy(fixtures().join(TRIDENT), &file).unwrap();
    import::retry_errors(imp.conn()).unwrap();
    let r = imp.import_file(&file).unwrap();
    assert_eq!((r.hands_inserted, r.duplicates, r.errors), (1, 23, 0));
}

#[test]
fn zip_archives_are_imported() {
    let tmp = tempfile::tempdir().unwrap();
    let zpath = tmp.path().join("hh.zip");
    {
        let f = std::fs::File::create(&zpath).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let opts: zip::write::SimpleFileOptions = Default::default();
        z.start_file("a/trident.txt", opts).unwrap();
        std::io::Write::write_all(&mut z, &std::fs::read(fixtures().join(TRIDENT)).unwrap()).unwrap();
        z.finish().unwrap();
    }
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    assert_eq!(imp.import_path(&zpath).unwrap().hands_inserted, 24);
    assert_eq!(imp.import_path(&zpath).unwrap().hands_parsed, 0);
}

#[test]
fn backup_and_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("database.sqlite");
    let backups = tmp.path().join("backups");
    let mut imp = importer(&db);
    imp.import_file(&fixtures().join(TRIDENT)).unwrap();
    let b = backup::backup(imp.conn(), &backups, 2, 1_793_644_483).unwrap();
    backup::backup(imp.conn(), &backups, 2, 1_793_644_484).unwrap();
    backup::backup(imp.conn(), &backups, 2, 1_793_644_485).unwrap();
    assert_eq!(backup::list(&backups).unwrap().len(), 2, "only the 2 newest are kept");
    assert!(!b.exists());
    imp.import_path(&fixtures()).unwrap();
    drop(imp);
    let newest = backup::list(&backups).unwrap()[0].name.clone();
    backup::request_restore(tmp.path(), &backups, &newest).unwrap();
    assert_eq!(backup::apply_pending_restore(tmp.path(), &backups, &db).unwrap(), Some(newest));
    let conn = open(&db).unwrap();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM hands"), 24);
}

/// `cargo test -p lps-db --release -- --ignored --nocapture bench`
#[test]
#[ignore]
fn bench_100k_hands() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("hh");
    std::fs::create_dir(&dir).unwrap();
    // 288 real hands, renumbered: 348 copies ≈ 100 224 hands.
    let mut sources = Vec::new();
    for f in std::fs::read_dir(fixtures()).unwrap().flatten() {
        let name = f.file_name().to_string_lossy().into_owned();
        if !name.ends_with("summary.txt") {
            sources.push(std::fs::read_to_string(f.path()).unwrap());
        }
    }
    for copy in 0..348 {
        for (k, src) in sources.iter().enumerate() {
            let text = src.replace("HandId: #", &format!("HandId: #{copy:04}{k}-"));
            std::fs::write(dir.join(format!("{copy}_{k}.txt")), text).unwrap();
        }
    }
    let threads: usize = std::env::var("LPS_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut imp = Importer::new(open(&tmp.path().join("db.sqlite")).unwrap(), threads).unwrap();
    let r = imp.import_path(&dir).unwrap();
    println!(
        "BENCH: {} hands in {} ms = {:.0} hands/s ({} errors, {} threads)",
        r.hands_inserted,
        r.elapsed_ms,
        r.hands_per_sec(),
        r.errors,
        if threads == 0 { rayon::current_num_threads() } else { threads }
    );
    assert_eq!(r.errors, 0);
    assert!(r.hands_inserted >= 100_000);
}

#[test]
fn re_entries_make_one_tournament_with_two_entries() {
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    imp.import_path(&fixtures()).unwrap();
    let row: (i64, i64, i64, i64, i64) = imp
        .conn()
        .query_row(
            "SELECT entries, finish_position, won_cents, bounty_won_cents, duration_secs FROM tournaments
             WHERE room_tournament_id = '1185497705'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(row, (2, 241, 0, 107, 142 + 35 * 60 + 9));
}

/// Regression (user's PC, 3 Nov. 2026): a file with non-UTF-8 bytes was
/// re-read from the start at every launch, because the stored offset was a
/// position in the decoded text (longer than the file).
#[test]
fn non_utf8_file_is_not_reimported_at_every_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("cp1252.txt");
    let utf8 = std::fs::read_to_string(fixtures().join(TRIDENT)).unwrap().replace("\r\n", "\n");
    let bytes: Vec<u8> = utf8.chars().map(|c| if c == '€' { 0x80 } else { c as u8 }).collect();
    std::fs::write(&file, &bytes).unwrap();
    let mut imp = Importer::new(open_in_memory().unwrap(), 1).unwrap();
    let r = imp.import_file(&file).unwrap();
    assert_eq!((r.hands_inserted, r.errors), (24, 0));
    assert_eq!(import::file_offset(imp.conn(), &file.to_string_lossy()).unwrap(), Some(bytes.len() as i64));
    let r2 = imp.import_file(&file).unwrap();
    assert_eq!(r2.hands_parsed, 0);
    // Appending one more (Windows-1252) hand reads only that hand.
    let last = utf8.trim_end().rsplit("\n\n\n").next().unwrap().replace("-56-1793645123", "-57-1793645200");
    let mut more = bytes.clone();
    more.extend(b"\n\n\n");
    more.extend(last.chars().map(|c| if c == '€' { 0x80u8 } else { c as u8 }));
    more.extend(b"\n\n\n");
    std::fs::write(&file, &more).unwrap();
    let r3 = imp.import_file(&file).unwrap();
    assert_eq!((r3.hands_parsed, r3.hands_inserted, r3.errors), (1, 1, 0));
}
