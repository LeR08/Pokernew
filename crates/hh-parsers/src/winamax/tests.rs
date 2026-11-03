use std::fmt::Write;
use std::path::PathBuf;

use lps_core_model::{cards_to_string_compat as cards, Hand, Position};

use super::*;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/winamax")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(fixtures().join(name)).unwrap()
}

const FILES: [&str; 3] = [
    "20261031_SPACE_HALLOWEEN_-_2_1209186647_real_holdem_no-limit.txt",
    "20261102_SPACE_KO1195603616_real_holdem_no-limit.txt",
    "20261102_TRIDENT_SPACE_KO1195602081_real_holdem_no-limit.txt",
];

fn parse_file(name: &str) -> Vec<Hand> {
    let text = read(name);
    let raws = Winamax.split(&text);
    raws.iter()
        .map(|r| {
            assert!(r.complete, "incomplete hand at {}", r.start);
            Winamax.parse(r).unwrap_or_else(|e| panic!("{name} @ byte {}: {e}", r.start))
        })
        .collect()
}

/// One compact line per hand: id, positions/stacks/net, board, pot.
fn render(hands: &[Hand]) -> String {
    let mut out = String::new();
    for h in hands {
        let _ = write!(out, "{} L{} {}/{}/{} btn{} pot{}", h.hand_no, h.level.unwrap_or(0), h.ante, h.small_blind, h.big_blind, h.button_seat, h.total_pot);
        if !h.board.is_empty() {
            let _ = write!(out, " [{}]", cards(&h.board));
        }
        if h.showdown {
            out.push_str(" SD");
        }
        out.push('\n');
        for p in &h.players {
            let _ = writeln!(
                out,
                "  s{} {:<4} {:<12} {:>7} net {:>+7}{}{}{}",
                p.seat,
                p.position.map(Position::as_str).unwrap_or("?"),
                p.name,
                p.stack,
                p.net_won(),
                if p.is_hero { " HERO" } else { "" },
                if p.hole_cards.is_empty() { String::new() } else { format!(" [{}]", cards(&p.hole_cards)) },
                if p.went_to_showdown { " sd" } else { "" },
            );
        }
    }
    out
}

#[test]
fn every_fixture_hand_parses_and_is_consistent() {
    let counts: Vec<usize> = FILES.iter().map(|f| parse_file(f).len()).collect();
    assert_eq!(counts, [88, 176, 24]);
}

#[test]
fn snapshots() {
    for f in FILES {
        let hands = parse_file(f);
        insta::assert_snapshot!(f.trim_end_matches(".txt"), render(&hands));
    }
}

#[test]
fn first_hand_in_detail() {
    let h = &parse_file(FILES[2])[0];
    insta::assert_debug_snapshot!("trident_first_hand", h);
}

#[test]
fn three_way_all_in_with_two_side_pots() {
    let hands = parse_file(FILES[2]);
    let h = hands.iter().find(|h| h.hand_no.ends_with("-48-1793644858")).unwrap();
    let net: Vec<(&str, i64)> = h.players.iter().map(|p| (p.name.as_str(), p.net_won())).collect();
    // Hero invested 411 + 280 + ante 8 = 699, collected 11.
    assert_eq!(net.iter().find(|(n, _)| *n == "Tantaculejen").unwrap().1, 11 - 699);
    assert_eq!(net.iter().map(|(_, x)| x).sum::<i64>(), 0);
    assert!(h.players.iter().all(|p| p.went_to_showdown));
}

#[test]
fn uncalled_raise_is_returned_through_collected() {
    let hands = parse_file(FILES[2]);
    // Player shoves 1261, the small blind folds: he collects 1329.
    let h = hands.iter().find(|h| h.hand_no.ends_with("-35-1793644560")).unwrap();
    let winner = h.players.iter().max_by_key(|p| p.net_won()).unwrap();
    assert_eq!(winner.invested, 6 + 1261);
    assert_eq!(winner.net_won(), 1329 - 1267);
    let raise = h.actions.iter().find(|a| a.kind == ActionKind::Raise).unwrap();
    assert_eq!((raise.amount, raise.raise_to, raise.is_all_in), (1211, Some(1261), true));
}

#[test]
fn dead_small_blind_positions() {
    let hands = parse_file(FILES[0]);
    let h = hands.iter().find(|h| h.hand_no.ends_with("-76-1793477188")).unwrap();
    let pos: Vec<&str> = h.players.iter().map(|p| p.position.unwrap().as_str()).collect();
    assert_eq!(pos, ["BB", "UTG", "HJ", "CO", "BTN"]);
}

#[test]
fn header_fields() {
    let h = &parse_file(FILES[1])[0];
    let t = h.tournament.as_ref().unwrap();
    assert_eq!((t.id.as_str(), t.name.as_str(), t.buy_in_cents, t.fee_cents), ("1195603616", "SPACE KO", 45, 5));
    assert_eq!(h.max_seats, 6);
    assert_eq!(h.game_type, GameType::Mtt);
    assert_eq!(h.unit, Unit::Chips);
    assert!(h.hero().is_some_and(|p| p.name == "Tantaculejen" && p.hole_cards.len() == 2));
}

#[test]
fn partial_last_hand_is_not_complete() {
    let text = read(FILES[2]);
    let cut = text.rfind("*** SUMMARY ***").unwrap() - 10;
    let raws = Winamax.split(&text[..cut]);
    assert_eq!(raws.len(), 24);
    assert!(raws[..23].iter().all(|r| r.complete));
    assert!(!raws[23].complete);
    // Cut right after the last summary line but before the blank lines.
    let body_end = text.trim_end().len();
    assert!(!Winamax.split(&text[..body_end]).last().unwrap().complete);
    assert!(Winamax.split(&text).last().unwrap().complete);
}

#[test]
fn bom_and_crlf_are_accepted() {
    let text = format!("\u{feff}{}", read(FILES[2]).replace('\n', "\r\n"));
    let raws = Winamax.split(&text);
    assert_eq!(raws.len(), 24);
    for r in &raws {
        Winamax.parse(r).unwrap();
    }
}

#[test]
fn unknown_lines_are_errors_not_skips() {
    let text = read(FILES[2]).replacen("Tantaculejen folds", "Tantaculejen dances", 1);
    let raws = Winamax.split(&text);
    let errs: Vec<_> = raws.iter().filter_map(|r| Winamax.parse(r).err()).collect();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, "Tantaculejen dances");
}

#[test]
fn cash_header_is_rejected_clearly() {
    let raw = RawHand { text: "Winamax Poker - CashGame - HandId: #1 - Holdem no limit (0.01€/0.02€)", start: 0, end: 0, complete: true };
    let e = Winamax.parse(&raw).unwrap_err();
    assert!(e.message.contains("cash games are not supported yet"), "{e}");
}

#[test]
fn sniffing() {
    assert_eq!(Winamax.sniff(read(FILES[0]).as_bytes()), Confidence::Certain);
    let summary = read("20261102_SPACE_KO1195603616_real_holdem_no-limit_summary.txt");
    assert_eq!(Winamax.sniff(summary.as_bytes()), Confidence::No);
    assert_eq!(Winamax.sniff_summary(summary.as_bytes()), Confidence::Certain);
    assert!(matches!(crate::detect(summary.as_bytes()), Some(crate::Detected::Summary(_))));
    assert!(matches!(crate::detect(read(FILES[0]).as_bytes()), Some(crate::Detected::Hands(_))));
    assert!(crate::detect(b"PokerStars Hand #1").is_none());
}

#[test]
fn summaries() {
    let ko = summary::parse(&read("20261102_SPACE_KO1195603616_real_holdem_no-limit_summary.txt")).unwrap();
    assert_eq!(ko.id, "1195603616");
    assert_eq!(ko.name, "SPACE KO");
    assert_eq!(ko.hero, "Tantaculejen");
    assert_eq!((ko.buy_in_prize_cents, ko.buy_in_bounty_cents, ko.fee_cents), (20, 25, 5));
    assert_eq!(ko.registered_players, Some(1771));
    assert_eq!(ko.prize_pool_cents, Some(46320));
    assert_eq!(ko.finish_position, Some(203));
    assert_eq!((ko.won_cents, ko.bounty_won_cents), (52, 51));
    assert_eq!(ko.duration_secs, Some(2 * 3600 + 59 * 60 + 6));
    assert_eq!(ko.speed.as_deref(), Some("normal"));

    let tri = summary::parse(&read("20261102_TRIDENT_SPACE_KO1195602081_real_holdem_no-limit_summary.txt")).unwrap();
    assert_eq!((tri.finish_position, tri.won_cents, tri.bounty_won_cents), (Some(149), 0, 0));
    assert_eq!(tri.duration_secs, Some(31 * 60 + 18));
    assert_eq!(tri.speed.as_deref(), Some("turbo"));
}
