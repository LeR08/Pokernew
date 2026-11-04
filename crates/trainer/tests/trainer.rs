use lps_solver::job::{solve_now, SolveParams};
use lps_solver::{Sizes, SpotConfig, StreetSizes, Tree};
use lps_trainer::analysis::{grade_line, prepare};
use lps_trainer::drill::{answer, deal, next, Mode};
use lps_trainer::Rng;

fn simple_tree() -> Tree {
    let s = StreetSizes {
        oop: Sizes {
            bet: "50%".into(),
            raise: "a".into(),
        },
        ip: Sizes {
            bet: "50%".into(),
            raise: "a".into(),
        },
        donk: String::new(),
    };
    Tree {
        flop: s.clone(),
        turn: s.clone(),
        river: s,
        max_raises: 1,
        ..Tree::default()
    }
}

#[test]
fn drills_are_graded_against_the_solution() {
    let spot = SpotConfig {
        oop_range: "66+,A8s+,AJo+,K9s+,KQo,QTs+,JTs".into(),
        ip_range: "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,QJs,JTs,T9s".into(),
        board: "Td9d6hQc".into(),
        pot: 20.0,
        stack: 80.0,
        tree: simple_tree(),
        ..SpotConfig::default()
    };
    let (mut game, _) = solve_now(
        &spot,
        &SolveParams {
            target_pct: 0.5,
            max_iterations: 400,
            ..SolveParams::default()
        },
        &[],
    )
    .unwrap();
    let mut rng = Rng(42);
    for mode in [Mode::Node, Mode::Street, Mode::Hand] {
        for _ in 0..10 {
            let q = deal(&mut game, mode, None, None, &mut rng).expect("question");
            assert!(!q.actions.is_empty() && q.combo.len() == 4);
            let mut best_seen = false;
            for a in 0..q.actions.len() {
                let ans = answer(&mut game, &q, a).unwrap();
                assert!(ans.loss_bb >= 0.0);
                assert!((ans.freqs.iter().sum::<f32>() - 1.0).abs() < 1e-3);
                if a == ans.best {
                    assert_eq!(ans.loss_bb, 0.0);
                    best_seen = true;
                }
            }
            assert!(best_seen);
            if mode == Mode::Hand {
                // play the hand to the end following the best action
                let mut q = q;
                for _ in 0..10 {
                    let best = answer(&mut game, &q, 0).unwrap().best;
                    match next(&mut game, &q, best, &mut rng) {
                        Some(n) => {
                            assert_eq!(n.player, q.player);
                            assert_eq!(n.combo, q.combo);
                            q = n;
                        }
                        None => break,
                    }
                }
            }
        }
    }
    // weak spots: a key with a big past loss is preferred
    let mut weak = std::collections::HashMap::new();
    weak.insert("turn · IP · first".to_string(), 50.0);
    let picks = (0..10)
        .filter(|_| {
            deal(&mut game, Mode::Node, None, Some(&weak), &mut rng)
                .unwrap()
                .node_key
                == "turn · IP · first"
        })
        .count();
    assert!(picks >= 8, "{picks}");
}

/// Acceptance test of phase 8: a real hand of the fixtures is analysed and the
/// EV loss of each of hero's postflop decisions is reported.
#[test]
fn analyses_a_real_hand() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hh-parsers/fixtures/winamax"
    );
    let parser = lps_hh_parsers::parser_for_room("winamax").unwrap();
    let mut hands = vec![];
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.to_string_lossy().ends_with(".txt")
                && !p.to_string_lossy().contains("summary")
            {
                let text = std::fs::read_to_string(&p).unwrap();
                hands.extend(
                    parser
                        .split(&text)
                        .iter()
                        .filter_map(|r| parser.parse(r).ok()),
                );
            }
        }
    }
    let tree = simple_tree();
    let mut done = 0;
    for h in &hands {
        // narrow ranges holding hero's cards, so that the test solves fast
        let hero = h.players.iter().find(|p| p.is_hero);
        let Some(hero) = hero else { continue };
        if hero.hole_cards.len() != 2 {
            continue;
        }
        let hc: String = hero.hole_cards.iter().map(|c| format!("{c}")).collect();
        let charts = |_: &str| Some(format!("{hc},QQ,JJ,AKs,T9s"));
        let Ok(prep) = prepare(h, "test", &tree, &charts) else {
            continue;
        };
        let Ok((mut game, _)) = solve_now(
            &prep.spot,
            &SolveParams {
                target_pct: 1.0,
                max_iterations: 200,
                ..SolveParams::default()
            },
            &[],
        ) else {
            continue;
        };
        let a = grade_line(&mut game, h, &prep).unwrap();
        assert!(a.decisions.iter().all(|d| d.loss_bb >= 0.0));
        if !a.decisions.is_empty() {
            assert!(
                (a.total_loss_bb - a.decisions.iter().map(|d| d.loss_bb).sum::<f64>()).abs() < 1e-9
            );
            done += 1;
            if done >= 2 {
                break;
            }
        }
    }
    assert!(done >= 1, "no analysable hand in the fixtures");
}
