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

#[test]
fn grid_covers_every_hero_hand() {
    let conn = db();
    let cells = crate::grid::grid(&conn, &Filter::default()).unwrap();
    assert_eq!(cells.len(), 169);
    let total: i64 = cells.iter().map(|c| c.hands).sum();
    let with_cards: i64 =
        conn.query_row("SELECT COUNT(*) FROM hand_players WHERE is_hero = 1 AND length(hole_cards) = 4", [], |r| r.get(0)).unwrap();
    assert_eq!(total, with_cards);
    let sum_bb: f64 = cells.iter().map(|c| c.net_bb).sum();
    let ref_bb: f64 = conn.query_row("SELECT SUM(net_won_bb) FROM hand_players WHERE is_hero = 1", [], |r| r.get(0)).unwrap();
    assert!((sum_bb - ref_bb).abs() < 1e-6);
}

#[test]
fn sessions_split_on_30_minute_gaps() {
    let conn = db();
    let s = crate::review::sessions(&conn, &Filter::default()).unwrap();
    assert_eq!(s.iter().map(|x| x.hands).sum::<i64>(), 496);
    assert!(s.windows(2).all(|w| w[0].start > w[1].end + crate::review::SESSION_GAP));
    // The two Nov. 2 tournaments (SPACE KO + TRIDENT, 18:15–21:28) are one session.
    let nov2 = s.iter().find(|x| x.start <= 1_793_644_483 && x.end >= 1_793_644_483).unwrap();
    assert_eq!(nov2.tournaments, 2);
    assert_eq!(nov2.hands, 200);
}

#[test]
fn session_review_finds_mistakes_and_good_plays() {
    let conn = db();
    let s = crate::review::sessions(&conn, &Filter::default()).unwrap();
    let nov2 = s.iter().find(|x| x.start <= 1_793_644_483 && x.end >= 1_793_644_483).unwrap();
    let r = crate::review::review_session(&conn, &Filter::default(), nov2.start, nov2.end).unwrap();
    assert_eq!(r.session.as_ref().unwrap().hands, 200);
    assert!(r.findings.iter().any(|f| f.kind == crate::review::Kind::Good));
    // Reviews come first, worst result first.
    let kinds: Vec<_> = r.findings.iter().map(|f| f.kind).collect();
    let first_good = kinds.iter().position(|k| *k == crate::review::Kind::Good).unwrap_or(kinds.len());
    assert!(kinds[first_good..].iter().all(|k| *k == crate::review::Kind::Good));
    assert!(r.vpip.is_some() && r.vpip_overall.is_some());
    assert!(r.worst_hand.is_some() && r.best_hand.is_some());
}

/// Rules checked on real hands from the fixtures, read by hand.
#[test]
fn review_rules_on_known_hands() {
    let conn = db();
    let find = |hand_no: &str| -> Vec<&'static str> {
        let id = lps_db::write::hand_id(&conn, "winamax", hand_no).unwrap().unwrap();
        let (h, _) = lps_db::load::load_hand(&conn, id).unwrap();
        crate::review::review_hand(&h, id).into_iter().map(|f| f.rule).collect()
    };
    // TRIDENT #34: hero SB shoves K6s for 9 bb over a limp and wins 10 bb: nothing to flag
    // (Chen score 5 is too weak for the "good short shove" rule, 9 bb too short for "weak shove").
    assert!(find("5135071836924543030-34-1793644523").is_empty());
    // TRIDENT #47: hero BTN raises QJo, stacks off on the flop and wins 2893 (+21 bb).
    let r = find("5135071836924543030-47-1793644836");
    assert!(r.contains(&"big_pot"), "{r:?}");
    // TRIDENT #44: hero BTN raises KTo, blinds fold: steal won.
    assert!(find("5135071836924543030-44-1793644752").contains(&"steal_won"));
    // TRIDENT #40: hero SB raises AJs, flop c-bet takes it.
    assert!(find("5135071836924543030-40-1793644654").contains(&"cbet_won"));
    // TRIDENT #36: hero BB calls a river bet of 272 into 254 with A2 and loses.
    assert!(find("5135071836924543030-36-1793644577").contains(&"river_call_lost"));
}

#[test]
fn leak_finder_flags_out_of_range_stats() {
    let conn = db();
    let targets: crate::leaks::TargetsFile = toml::from_str(include_str!("../../../config/defaults/leaks/targets.toml")).unwrap();
    let rows = crate::leaks::find(&conn, &Filter::default(), &core(), &targets).unwrap();
    assert!(!rows.is_empty());
    let mtt_vpip = rows.iter().find(|r| r.stat == "vpip" && r.game_type == "mtt").unwrap();
    assert_eq!(mtt_vpip.sample, rows.iter().find(|r| r.stat == "pfr" && r.game_type == "mtt").unwrap().sample);
    assert!(mtt_vpip.ci_low.unwrap() <= mtt_vpip.value.unwrap() && mtt_vpip.value.unwrap() <= mtt_vpip.ci_high.unwrap());
    assert!(mtt_vpip.bb100_when_done.is_some());
    assert!(rows.iter().filter(|r| r.stat == "wsd").all(|r| r.bb100_when_done.is_none()));
    // Sorted: leaks first.
    let ranks: Vec<u8> = rows
        .iter()
        .map(|r| match r.status {
            crate::leaks::Status::Leak => 0,
            crate::leaks::Status::Possible => 1,
            crate::leaks::Status::Ok => 2,
            crate::leaks::Status::TooFew => 3,
        })
        .collect();
    assert!(ranks.windows(2).all(|w| w[0] <= w[1]));
}

#[test]
fn bankroll_adds_movements_cash_and_tournaments() {
    let conn = db();
    lps_db::user::add_tx(
        &conn,
        &lps_db::user::Tx { id: 0, room: "winamax".into(), at: 1_700_000_000, kind: "deposit".into(), amount_cents: 2000, note: String::new() },
    )
    .unwrap();
    let b = crate::bankroll::bankroll(&conn).unwrap();
    assert_eq!(b.deposits_cents, 2000);
    assert_eq!(b.balance_cents, b.deposits_cents - b.withdrawals_cents + b.cash_cents + b.tournaments_cents);
    assert_eq!(*b.balance_eur.last().unwrap(), b.balance_cents as f64 / 100.0);
    // Every tournament with a summary counts, even without imported hands (money really moved).
    let all: i64 = conn
        .query_row(
            "SELECT SUM(COALESCE(won_cents, 0) + COALESCE(bounty_won_cents, 0) + ticket_won_cents - (buy_in_cents + fee_cents) * entries)
             FROM tournaments WHERE has_summary = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(b.tournaments_cents, all);
    assert!(b.avg_buy_in_cents.is_some());
}

#[test]
fn advanced_filters() {
    let conn = db();
    let stats = core();
    let n = |f: Filter| query::report(&conn, &f, &stats, Group::None).unwrap().first().map(|r| r.hands).unwrap_or(0);
    let all = n(Filter::default());
    let pots: i64 = ["limped", "srp", "3bet", "4bet"].iter().map(|p| n(Filter { pot_types: vec![p.to_string()], ..Default::default() })).sum();
    assert_eq!(pots, all, "pot types partition the hands");
    let short = n(Filter { stack_max: Some(20.0), ..Default::default() });
    let deep = n(Filter { stack_min: Some(20.000001), ..Default::default() });
    assert_eq!(short + deep, all);
    let flop = n(Filter { street: Some("flop".into()), ..Default::default() });
    let river = n(Filter { street: Some("river".into()), ..Default::default() });
    assert!(flop >= river && river > 0);
    let pairs: Vec<String> = "AKQJT98765432".chars().map(|c| format!("{c}{c}")).collect();
    let pp = n(Filter { hand_classes: pairs, ..Default::default() });
    assert!(pp > 0 && pp < all);
    use lps_core_model::derived::texture::*;
    let mono = n(Filter { texture: Some(MONOTONE), ..Default::default() });
    let paired_mono = n(Filter { texture: Some(MONOTONE | PAIRED), ..Default::default() });
    assert!(mono >= paired_mono);
    let sd = n(Filter { showdown: Some(true), ..Default::default() });
    let nsd = n(Filter { showdown: Some(false), ..Default::default() });
    assert_eq!(sd + nsd, all);
    let cheap = n(Filter { buy_in_max: Some(50), ..Default::default() });
    assert!(cheap > 0 && cheap < all);
}

#[test]
fn population_and_players() {
    let conn = db();
    let stats = core();
    let pop = query::report(&conn, &Filter { who: Some("opponents".into()), ..Default::default() }, &stats, Group::None).unwrap();
    let total_rows: i64 = conn.query_row("SELECT COUNT(*) FROM hand_players WHERE is_hero = 0", [], |r| r.get(0)).unwrap();
    assert_eq!(pop[0].hands, total_rows);
    let players = query::report_limited(&conn, &Filter { who: Some("opponents".into()), ..Default::default() }, &stats, Group::Player, 30, 10).unwrap();
    assert!(!players.is_empty() && players.len() <= 10);
    assert!(players.iter().all(|p| p.hands >= 30));
    assert!(players.windows(2).all(|w| w[0].hands >= w[1].hands));
    let one = query::report(&conn, &Filter { player: Some(players[0].key.clone()), ..Default::default() }, &stats, Group::None).unwrap();
    assert_eq!(one[0].hands, players[0].hands);
}

#[test]
fn auto_notes_follow_rules_and_replace_previous_ones() {
    let mut conn = db();
    let rules: crate::autonotes::RulesFile = toml::from_str(include_str!("../../../config/defaults/notes/auto.toml")).unwrap();
    // A rule that matches everybody with enough hands, to check the mechanics.
    let mut all = rules.clone();
    all.rule.insert(
        "any".into(),
        crate::autonotes::Rule { text: defs::Text::Plain("VPIP {vpip}".into()), when: "vpip >= 0".into(), min_sample: 30 },
    );
    let notes = crate::autonotes::evaluate(&conn, "winamax", &core(), &all, "fr").unwrap();
    let any: Vec<_> = notes.iter().filter(|n| n.rule == "any").collect();
    assert!(!any.is_empty());
    assert!(any[0].text.starts_with("VPIP ") && any[0].text.ends_with('%'));
    let n = crate::autonotes::apply(&mut conn, "winamax", &notes, 1).unwrap();
    let stored: i64 = conn.query_row("SELECT COUNT(*) FROM notes WHERE auto = 1", [], |r| r.get(0)).unwrap();
    assert_eq!(stored as usize, n);
    // Manual notes survive a re-run; auto ones are replaced.
    conn.execute("INSERT INTO notes (player_id, text, auto, created_at) SELECT id, 'manuelle', 0, 1 FROM players WHERE is_hero = 0 LIMIT 1", [])
        .unwrap();
    crate::autonotes::apply(&mut conn, "winamax", &[], 2).unwrap();
    assert_eq!(conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM notes WHERE auto = 1", [], |r| r.get(0)).unwrap(), 0);
    assert_eq!(conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM notes WHERE auto = 0", [], |r| r.get(0)).unwrap(), 1);
    // The hero never gets auto-notes.
    assert!(notes.iter().all(|n| n.player != "Tantaculejen" && n.player != "GTOverBet"));
}
