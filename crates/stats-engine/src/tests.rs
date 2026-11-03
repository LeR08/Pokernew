use std::path::PathBuf;

use lps_db::import::Importer;
use rusqlite::Connection;

use crate::query::{self, Filter, Group};
use crate::*;

fn db() -> Connection {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../hh-parsers/fixtures/winamax");
    let mut imp = Importer::new(lps_db::open_in_memory().unwrap(), 1).unwrap();
    let r = imp.import_path(&fixtures).unwrap();
    assert_eq!(r.errors, 0);
    imp.into_conn()
}

fn core() -> Vec<CompiledStat> {
    let f: StatsFile = toml::from_str(include_str!("../../../config/defaults/stats/core.toml")).unwrap();
    let (ok, errs) = compile_all(&[f]);
    assert!(errs.is_empty(), "{errs:?}");
    ok
}

fn value(row: &query::ReportRow, id: &str) -> (Option<f64>, i64) {
    let s = row.stats.iter().find(|s| s.id == id).unwrap();
    (s.value, s.sample)
}

/// Phase 2 acceptance: the stats match an independent recount
/// (tools/reference_stats.py, straight from the text files).
#[test]
fn core_stats_match_the_independent_reference() {
    #[derive(serde::Deserialize)]
    struct Ref {
        hands: i64,
        vpip: i64,
        pfr: i64,
        opp3: i64,
        cnt3: i64,
        flop: i64,
        wtsd: i64,
        wsd: i64,
    }
    let r: Ref = serde_json::from_str(include_str!("../tests/reference.json")).unwrap();
    let conn = db();
    let rows = query::report(&conn, &Filter::default(), &core(), Group::None).unwrap();
    let all = &rows[0];
    assert_eq!(all.hands, r.hands);
    let ratio = |a: i64, b: i64| Some(a as f64 / b as f64);
    assert_eq!(value(all, "vpip"), (ratio(r.vpip, r.hands), r.hands));
    assert_eq!(value(all, "pfr"), (ratio(r.pfr, r.hands), r.hands));
    assert_eq!(value(all, "three_bet"), (ratio(r.cnt3, r.opp3), r.opp3));
    assert_eq!(value(all, "wtsd"), (ratio(r.wtsd, r.flop), r.flop));
    assert_eq!(value(all, "wsd"), (ratio(r.wsd, r.wtsd), r.wtsd));
}

#[test]
fn graph_ends_on_the_sum_of_results() {
    let conn = db();
    let f = Filter::default();
    let g = query::graph(&conn, &f, 50).unwrap();
    let sum_bb: f64 = conn.query_row("SELECT SUM(net_won_bb) FROM hand_players WHERE is_hero = 1", [], |r| r.get(0)).unwrap();
    let sum_cash: i64 = conn
        .query_row(
            "SELECT SUM(hp.net_won) FROM hand_players hp JOIN hands h ON h.id = hp.hand_id WHERE hp.is_hero = 1 AND h.unit = 'cents'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!((g.total_bb.last().unwrap() - sum_bb).abs() < 1e-9);
    assert!((g.cash_eur.last().unwrap() - sum_cash as f64 / 100.0).abs() < 1e-9);
    assert_eq!(*g.x.last().unwrap(), g.hands);
    assert!(g.x.len() <= 51);
    let last = g.total_bb.len() - 1;
    assert!((g.showdown_bb[last] + g.non_showdown_bb[last] - g.total_bb[last]).abs() < 1e-9);
}

#[test]
fn groups_and_filters() {
    let conn = db();
    let stats = core();
    let by_pos = query::report(&conn, &Filter::default(), &stats, Group::Position).unwrap();
    assert_eq!(by_pos.iter().map(|r| r.hands).sum::<i64>(), 496);
    assert_eq!(by_pos.last().unwrap().key, "BB");
    let by_game = query::report(&conn, &Filter::default(), &stats, Group::GameType).unwrap();
    let keys: Vec<&str> = by_game.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["cash", "mtt", "spin"]);
    let cash = Filter { game_types: vec!["cash".into()], ..Default::default() };
    let rows = query::report(&conn, &cash, &stats, Group::None).unwrap();
    assert_eq!(rows[0].hands, 22);
    assert!(rows[0].net_cash_cents != 0);
    let acc = Filter { accounts: vec!["Tantaculejen".into()], ..Default::default() };
    assert_eq!(query::report(&conn, &acc, &stats, Group::None).unwrap()[0].hands, 288);
    assert_eq!(query::hero_accounts(&conn).unwrap(), ["GTOverBet", "Tantaculejen"]);
}

#[test]
fn where_clause_restricts_a_stat() {
    let conn = db();
    let def = StatDef {
        name: "VPIP BB".into(),
        formula: "cnt_vpip / cnt_hands".into(),
        where_: Some("position = 'BB'".into()),
        format: "pct1".into(),
        sample: None,
        min_sample: None,
        description: None,
        colour_ranges: vec![],
    };
    let st = compile_stat("vpip_bb", &def).unwrap();
    let all = query::report(&conn, &Filter::default(), std::slice::from_ref(&st), Group::None).unwrap();
    let bb_only = Filter { positions: vec!["BB".into()], ..Default::default() };
    let pos = query::report(&conn, &bb_only, &core(), Group::None).unwrap();
    assert_eq!(all[0].stats[0].sample, pos[0].hands);
    assert_eq!(all[0].stats[0].value, value(&pos[0], "vpip").0);
}

#[test]
fn tournaments_and_roi() {
    let conn = db();
    let (rows, tot) = query::tournaments(&conn, &Filter::default()).unwrap();
    assert_eq!(tot.tournaments as usize, rows.len());
    let ko = rows.iter().find(|r| r.name == "SPACE KO" && r.hands == 176).unwrap();
    assert_eq!((ko.cost_cents, ko.won_cents, ko.profit_cents), (50, Some(103), Some(53)));
    assert_eq!(tot.with_result as usize, rows.iter().filter(|r| r.won_cents.is_some()).count());
    assert_eq!(*tot.curve_eur.last().unwrap(), tot.profit_cents as f64 / 100.0);
    assert!(tot.roi.is_some());
}

#[test]
fn hand_list_pages() {
    let conn = db();
    let (page, total) = query::hands(&conn, &Filter::default(), 25, 0).unwrap();
    assert_eq!((page.len(), total), (25, 496));
    assert!(page.windows(2).all(|w| w[0].played_at >= w[1].played_at));
    let (last, _) = query::hands(&conn, &Filter::default(), 25, 475).unwrap();
    assert_eq!(last.len(), 21);
}

#[test]
fn every_default_stat_compiles_and_runs() {
    let conn = db();
    let stats = core();
    assert!(stats.len() >= 13);
    let rows = query::report(&conn, &Filter::default(), &stats, Group::Month).unwrap();
    assert!(!rows.is_empty());
}
