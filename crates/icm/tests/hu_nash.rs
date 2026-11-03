//! Acceptance test of phase 5: our heads-up push/fold equilibrium against the
//! published HoldemResources Nash tables (tests/hune_hu_noante.txt).
//!
//! At each tested stack, a hand is in the published push (call) range when the
//! stack is within one of its intervals. Hands whose published boundary is
//! within 0.15 bb of the stack are skipped: the table is rounded to 0.1 bb and
//! hands right at a boundary play a mixed strategy.
use lps_equity::combo::{class_name, NUM_CLASSES};
use lps_icm::{solve, Decision, Options, Spot};

struct Row {
    push: Vec<(f64, f64)>,
    call: Vec<(f64, f64)>,
}

fn intervals(s: &str) -> Vec<(f64, f64)> {
    if let Some(x) = s.strip_suffix('+') {
        let v: f64 = x.parse().unwrap();
        return vec![(0.0, v.max(1000.0))];
    }
    s.split(',')
        .map(|part| match part.split_once('-') {
            Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()),
            None => (0.0, part.parse().unwrap()),
        })
        .collect()
}

fn table() -> Vec<(String, Row)> {
    include_str!("hune_hu_noante.txt")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (
                f[0].to_string(),
                Row {
                    push: intervals(f[1]),
                    call: intervals(f[2]),
                },
            )
        })
        .collect()
}

fn inside(iv: &[(f64, f64)], s: f64) -> Option<bool> {
    // None when the stack is too close to a boundary to decide
    for &(a, b) in iv {
        for edge in [a, b] {
            if edge > 0.0 && edge < 1000.0 && (s - edge).abs() < 0.15 {
                return None;
            }
        }
    }
    Some(iv.iter().any(|&(a, b)| s > a && s <= b))
}

#[test]
fn heads_up_matches_published_nash_tables() {
    let t = table();
    assert_eq!(t.len(), 169);
    let cell = |name: &str| (0..NUM_CLASSES).find(|&c| class_name(c) == name).unwrap();
    let mut checked = 0;
    let mut wrong = Vec::new();
    for stack in [
        2.0, 3.0, 4.0, 5.0, 6.0, 7.5, 9.0, 11.0, 13.0, 15.0, 17.0, 19.5,
    ] {
        let spot = Spot {
            stacks: vec![stack, stack],
            ..Spot::default()
        };
        let sol = solve(
            &spot,
            &Options {
                max_iterations: 4000,
                target: 0.0002,
                locks: vec![],
            },
        )
        .unwrap();
        let push = sol.node(Decision::Push { p: 0 }).unwrap();
        let call = sol.node(Decision::Call { p: 0, q: 1 }).unwrap();
        for (hand, row) in &t {
            let c = cell(hand);
            for (kind, iv, freq) in [
                ("push", &row.push, push.freq[c]),
                ("call", &row.call, call.freq[c]),
            ] {
                let Some(want) = inside(iv, stack) else {
                    continue;
                };
                checked += 1;
                if (freq >= 0.5) != want {
                    wrong.push(format!(
                        "{stack} bb {kind} {hand}: ours {freq:.2}, published {want}"
                    ));
                }
            }
        }
    }
    eprintln!("{checked} decisions checked, {} differ", wrong.len());
    assert!(wrong.is_empty(), "{wrong:#?}");
}
