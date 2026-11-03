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
            Winamax
                .parse(r)
                .unwrap_or_else(|e| panic!("{name} @ byte {}: {e}", r.start))
        })
        .collect()
}

/// One compact line per hand: id, positions/stacks/net, board, pot.
fn render(hands: &[Hand]) -> String {
    let mut out = String::new();
    for h in hands {
        let _ = write!(
            out,
            "{} L{} {}/{}/{} btn{} pot{}",
            h.hand_no,
            h.level.unwrap_or(0),
            h.ante,
            h.small_blind,
            h.big_blind,
            h.button_seat,
            h.total_pot
        );
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
                if p.hole_cards.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", cards(&p.hole_cards))
                },
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
    let h = hands
        .iter()
        .find(|h| h.hand_no.ends_with("-48-1793644858"))
        .unwrap();
    let net: Vec<(&str, i64)> = h
        .players
        .iter()
        .map(|p| (p.name.as_str(), p.net_won()))
        .collect();
    // Hero invested 411 + 280 + ante 8 = 699, collected 11.
    assert_eq!(
        net.iter().find(|(n, _)| *n == "Tantaculejen").unwrap().1,
        11 - 699
    );
    assert_eq!(net.iter().map(|(_, x)| x).sum::<i64>(), 0);
    assert!(h.players.iter().all(|p| p.went_to_showdown));
}

#[test]
fn uncalled_raise_is_returned_through_collected() {
    let hands = parse_file(FILES[2]);
    // Player shoves 1261, the small blind folds: he collects 1329.
    let h = hands
        .iter()
        .find(|h| h.hand_no.ends_with("-35-1793644560"))
        .unwrap();
    let winner = h.players.iter().max_by_key(|p| p.net_won()).unwrap();
    assert_eq!(winner.invested, 6 + 1261);
    assert_eq!(winner.net_won(), 1329 - 1267);
    let raise = h
        .actions
        .iter()
        .find(|a| a.kind == ActionKind::Raise)
        .unwrap();
    assert_eq!(
        (raise.amount, raise.raise_to, raise.is_all_in),
        (1211, Some(1261), true)
    );
}

#[test]
fn dead_small_blind_positions() {
    let hands = parse_file(FILES[0]);
    let h = hands
        .iter()
        .find(|h| h.hand_no.ends_with("-76-1793477188"))
        .unwrap();
    let pos: Vec<&str> = h
        .players
        .iter()
        .map(|p| p.position.unwrap().as_str())
        .collect();
    assert_eq!(pos, ["BB", "UTG", "HJ", "CO", "BTN"]);
}

#[test]
fn header_fields() {
    let h = &parse_file(FILES[1])[0];
    let t = h.tournament.as_ref().unwrap();
    assert_eq!(
        (t.id.as_str(), t.name.as_str(), t.buy_in_cents, t.fee_cents),
        ("1195603616", "SPACE KO", 45, 5)
    );
    assert_eq!(h.max_seats, 6);
    assert_eq!(h.game_type, GameType::Mtt);
    assert_eq!(h.unit, Unit::Chips);
    assert!(h
        .hero()
        .is_some_and(|p| p.name == "Tantaculejen" && p.hole_cards.len() == 2));
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

/// Every hand that failed on the user's PC on 3 Nov. 2026 (exported from the
/// app's error log, opponents anonymised) now parses and is consistent.
#[test]
fn user_error_export_of_2026_11_03_parses() {
    let dir = fixtures().join("user-2026-11-03");
    let (mut hands, mut summaries) = (0, 0);
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let text = std::fs::read_to_string(e.path()).unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with("summary.txt") {
            summaries += summary::parse_all(&text)
                .unwrap_or_else(|err| panic!("{name}: {err}"))
                .len();
            continue;
        }
        for r in Winamax.split(&text) {
            assert!(r.complete);
            Winamax
                .parse(&r)
                .unwrap_or_else(|err| panic!("{name} @ {}: {err}", r.start));
            hands += 1;
        }
    }
    assert_eq!((hands, summaries), (208, 2));
}

fn parse_user(name: &str) -> Vec<Hand> {
    let text = std::fs::read_to_string(fixtures().join("user-2026-11-03").join(name)).unwrap();
    Winamax
        .split(&text)
        .iter()
        .map(|r| Winamax.parse(r).unwrap())
        .collect()
}

#[test]
fn cash_game_hands() {
    let hands = parse_user("20250103_Aalen 29_real_holdem_no-limit.txt");
    insta::assert_snapshot!("cash_aalen_29", render(&hands));
    let h = &hands[0];
    assert_eq!(
        (h.game_type, h.unit, h.small_blind, h.big_blind),
        (GameType::Cash, Unit::Cents, 1, 2)
    );
    assert!(h.tournament.is_none());
    assert_eq!(h.table_name, "Aalen 29");
    // Raked hand: pot 0.74€ after a 0.04€ rake.
    let r = hands
        .iter()
        .find(|h| h.hand_no == "20478746-125-1735914461")
        .unwrap();
    assert_eq!((r.total_pot, r.rake), (78, 4));
    assert_eq!(r.players.iter().map(|p| p.net_won()).sum::<i64>(), -4);
    // A newcomer posting the big blind out of position is not the BB.
    let oop = hands
        .iter()
        .find(|h| h.hand_no == "20478746-131-1735914696")
        .unwrap();
    let bb: Vec<&str> = oop
        .players
        .iter()
        .filter(|p| p.position == Some(Position::Bb))
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(bb.len(), 1);
    assert_ne!(
        bb[0],
        oop.players.iter().find(|p| p.seat == 2).unwrap().name
    );
}

#[test]
fn expresso_without_ante_is_a_spin() {
    let hands = parse_user("20250327_Expresso Nitro(921620907)_real_holdem_no-limit.txt");
    let h = &hands[0];
    assert_eq!(
        (h.game_type, h.ante, h.small_blind, h.big_blind, h.max_seats),
        (GameType::Spin, 0, 10, 20, 3)
    );
    assert!(h.players.iter().all(|p| p.bounty_cents.is_none()));
}

#[test]
fn negative_level_is_unknown() {
    let h = &parse_user("20250405_ROTATION(912308558)_real_holdem_no-limit.txt")[0];
    assert_eq!(h.level, None);
    assert_eq!(h.big_blind, 200);
}

#[test]
fn voluntary_show_without_showdown() {
    let hands = parse_user("20250406_ROTATION(913224798)_real_holdem_no-limit.txt");
    let h = hands
        .iter()
        .find(|h| {
            !h.showdown
                && h.players
                    .iter()
                    .any(|p| !p.is_hero && p.hole_cards.len() == 2)
        })
        .unwrap();
    let shower = h
        .players
        .iter()
        .find(|p| !p.is_hero && p.hole_cards.len() == 2)
        .unwrap();
    assert!(!shower.went_to_showdown);
    assert!(shower.net_won() > 0);
}

#[test]
fn satellite_summary_with_ticket() {
    let text = std::fs::read_to_string(
        fixtures().join("user-2026-11-03/20250401_PLANETE POK - ROAD TO WINAMAX SERIES 20 €(910389948)_real_holdem_no-limit_summary.txt"),
    )
    .unwrap();
    let all = summary::parse_all(&text).unwrap();
    let last = all.last().unwrap();
    assert_eq!(last.name, "PLANETE POK - ROAD TO WINAMAX SERIES 20 €");
    assert_eq!(
        (last.won_cents, last.ticket_won_cents, last.bounty_won_cents),
        (19776, 2000, 20419)
    );
}

#[test]
fn sniffing() {
    assert_eq!(
        Winamax.sniff(read(FILES[0]).as_bytes()),
        Confidence::Certain
    );
    let summary = read("20261102_SPACE_KO1195603616_real_holdem_no-limit_summary.txt");
    assert_eq!(Winamax.sniff(summary.as_bytes()), Confidence::No);
    assert_eq!(
        Winamax.sniff_summary(summary.as_bytes()),
        Confidence::Certain
    );
    assert!(matches!(
        crate::detect(summary.as_bytes()),
        Some(crate::Detected::Summary(_))
    ));
    assert!(matches!(
        crate::detect(read(FILES[0]).as_bytes()),
        Some(crate::Detected::Hands(_))
    ));
    assert!(crate::detect(b"PokerStars Hand #1").is_none());
}

#[test]
fn summaries() {
    let ko = summary::parse(&read(
        "20261102_SPACE_KO1195603616_real_holdem_no-limit_summary.txt",
    ))
    .unwrap();
    assert_eq!(ko.id, "1195603616");
    assert_eq!(ko.name, "SPACE KO");
    assert_eq!(ko.hero, "Tantaculejen");
    assert_eq!(
        (ko.buy_in_prize_cents, ko.buy_in_bounty_cents, ko.fee_cents),
        (20, 25, 5)
    );
    assert_eq!(ko.registered_players, Some(1771));
    assert_eq!(ko.prize_pool_cents, Some(46320));
    assert_eq!(ko.finish_position, Some(203));
    assert_eq!((ko.won_cents, ko.bounty_won_cents), (52, 51));
    assert_eq!(ko.duration_secs, Some(2 * 3600 + 59 * 60 + 6));
    assert_eq!(ko.speed.as_deref(), Some("normal"));

    let tri = summary::parse(&read(
        "20261102_TRIDENT_SPACE_KO1195602081_real_holdem_no-limit_summary.txt",
    ))
    .unwrap();
    assert_eq!(
        (tri.finish_position, tri.won_cents, tri.bounty_won_cents),
        (Some(149), 0, 0)
    );
    assert_eq!(tri.duration_secs, Some(31 * 60 + 18));
    assert_eq!(tri.speed.as_deref(), Some("turbo"));
}

#[test]
fn re_entry_summary_with_late_registration_and_bounty_only_win() {
    let all = summary::parse_all(&read(
        "20261015_TRIDENT SPACE KO(1185497705)_real_holdem_no-limit_summary.txt",
    ))
    .unwrap();
    assert_eq!(all.len(), 2);
    assert!(all[0].late_registration && !all[1].late_registration);
    assert_eq!(
        (
            all[0].finish_position,
            all[0].won_cents,
            all[0].bounty_won_cents
        ),
        (Some(646), 0, 0)
    );
    assert_eq!(
        (
            all[1].finish_position,
            all[1].won_cents,
            all[1].bounty_won_cents
        ),
        (Some(241), 0, 107)
    );
    assert_eq!(all[0].duration_secs, Some(142));
    assert_eq!(
        (
            all[1].buy_in_prize_cents,
            all[1].buy_in_bounty_cents,
            all[1].fee_cents
        ),
        (40, 50, 10)
    );
    assert_eq!(all[1].name, "TRIDENT SPACE KO");
}

#[test]
fn won_line_variants() {
    let base = read("20261102_SPACE_KO1195603616_real_holdem_no-limit_summary.txt");
    for (line, won, bounty) in [
        ("You won 0.52€ + Bounty 0.51€", 52, 51),
        ("You won 3€", 300, 0),
        ("You won Bounty 1.07€", 0, 107),
    ] {
        let text = base.replace("You won 0.52€ + Bounty 0.51€", line);
        let s = summary::parse(&text).unwrap();
        assert_eq!((s.won_cents, s.bounty_won_cents), (won, bounty), "{line}");
    }
    assert!(summary::parse(&base.replace("You won 0.52€ + Bounty 0.51€", "You won")).is_err());
}

#[test]
fn windows_1252_file_parses_like_utf8() {
    let utf8 = read(FILES[2]);
    let bytes: Vec<u8> = utf8
        .chars()
        .map(|c| if c == '€' { 0x80 } else { c as u8 })
        .collect();
    assert!(utf8.chars().all(|c| c == '€' || c.is_ascii()));
    let d = crate::util::Decoded::new(bytes.clone());
    assert!(d.was_reencoded());
    let raws = Winamax.split(&d.text);
    assert_eq!(raws.len(), 24);
    for r in &raws {
        Winamax.parse(r).unwrap();
    }
    assert_eq!(d.byte_offset(raws.last().unwrap().end), bytes.len());
}
