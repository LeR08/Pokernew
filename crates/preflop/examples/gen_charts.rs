//! Generates the default preflop charts (config/defaults/ranges/<format>/...) with
//! our own preflop solver: one solve per format, then every situation of the
//! Charts tab is read from the solution.
//!
//! cargo run --release -p lps-preflop --example gen_charts [out_dir]
//!
//! A situation's range gives each hand class its weight: how often the player
//! holds it here (own earlier actions) times how often they take the action.
use std::path::{Path, PathBuf};

use lps_equity::combo::{class_name, NUM_CLASSES};
use lps_preflop::{solve, Config, Kind, NodeOut, Solution};

struct Format {
    id: String,
    positions: Vec<&'static str>,
    config: Config,
}

fn formats() -> Vec<Format> {
    let six = vec!["UTG", "HJ", "CO", "BTN", "SB", "BB"];
    // Calibration (docs/preflop.md): cash with a 5 % rake capped at 3 bb and playability 1.5;
    // tournaments without rake, playability 2.5 and 80 % realisation out of position.
    let cash = Config {
        allow_limp: true,
        allow_allin: true,
        realization_ip: 1.0,
        realization_oop: 0.85,
        realization_curve: 1.5,
        rake: 0.05,
        rake_cap: 3.0,
        ..Config::default()
    };
    let base = Config {
        allow_limp: true,
        allow_allin: true,
        realization_ip: 1.0,
        realization_oop: 0.8,
        realization_curve: 2.5,
        ..Config::default()
    };
    let mut out = vec![Format {
        id: "cash 6max 100bb".into(),
        positions: six.clone(),
        config: Config {
            stacks: vec![100.0; 6],
            open: 2.5,
            three_bet: 3.0,
            four_bet: 2.2,
            ..cash
        },
    }];
    for d in [10.0, 15.0, 20.0, 25.0, 40.0, 60.0, 100.0] {
        let open = if d <= 20.0 {
            2.0
        } else if d <= 40.0 {
            2.2
        } else {
            2.3
        };
        out.push(Format {
            id: format!("MTT {d}bb"),
            positions: six.clone(),
            config: Config {
                stacks: vec![d; 6],
                ante: 0.12,
                open,
                three_bet: 3.0,
                four_bet: 2.2,
                ..base.clone()
            },
        });
    }
    out.push(Format {
        id: "Spin 3max 25bb".into(),
        positions: vec!["BTN", "SB", "BB"],
        config: Config {
            stacks: vec![25.0; 3],
            open: 2.0,
            three_bet: 3.0,
            four_bet: 2.2,
            ..base
        },
    });
    out
}

/// Child of `node` reached by the first action of one of `kinds`.
fn child(sol: &Solution, node: usize, kinds: &[Kind]) -> Option<usize> {
    let n = &sol.nodes[node];
    kinds
        .iter()
        .find_map(|k| n.actions.iter().position(|a| a.kind == *k))
        .map(|i| n.children[i])
}

/// Folds until `player` is to act.
fn fold_to(sol: &Solution, mut node: usize, player: usize) -> Option<usize> {
    loop {
        let n = &sol.nodes[node];
        match n.player {
            Some(p) if p == player => return Some(node),
            Some(_) => node = child(sol, node, &[Kind::Fold])?,
            None => return None,
        }
    }
}

/// Weight of each class for the given actions at a node: reach × frequency.
fn weights(n: &NodeOut, kinds: &[Kind]) -> Vec<f64> {
    (0..NUM_CLASSES)
        .map(|c| {
            let f: f32 = n
                .actions
                .iter()
                .enumerate()
                .filter(|(_, a)| kinds.contains(&a.kind))
                .map(|(k, _)| n.freq[k][c])
                .sum();
            (n.reach[c] * f) as f64
        })
        .collect()
}

/// Range notation: whole classes, weights in percent, below 2 % dropped.
fn notation(w: &[f64]) -> String {
    let mut toks = vec![];
    for (c, &x) in w.iter().enumerate() {
        let p = (x * 100.0).round();
        if p >= 2.0 {
            toks.push(if p >= 99.0 {
                class_name(c)
            } else {
                format!("{}:{}%", class_name(c), p)
            });
        }
    }
    toks.join(",")
}

fn situations(sol: &Solution, positions: &[&str]) -> Vec<(String, Vec<f64>)> {
    let mut out = vec![];
    let n = positions.len();
    let raise = [Kind::Raise, Kind::AllIn];
    for (i, p) in positions.iter().enumerate() {
        // open
        if i < n - 1 {
            if let Some(node) = fold_to(sol, 0, i) {
                out.push((format!("{p}/open"), weights(&sol.nodes[node], &raise)));
            }
        }
        // against an earlier position's open: 3-bet or call
        for (v, vp) in positions.iter().enumerate().take(i) {
            let resp = fold_to(sol, 0, v)
                .and_then(|o| child(sol, o, &[Kind::Raise, Kind::AllIn]))
                .and_then(|r| fold_to(sol, r, i));
            if let Some(node) = resp {
                let nd = &sol.nodes[node];
                out.push((format!("{p} vs {vp}/3bet"), weights(nd, &raise)));
                out.push((format!("{p} vs {vp}/call"), weights(nd, &[Kind::Call])));
            }
        }
        // against a 3-bet after opening: average over who 3-bets, weighted by how often it happens
        if i < n - 1 {
            let mut four = vec![0.0; NUM_CLASSES];
            let mut call = vec![0.0; NUM_CLASSES];
            let mut total = 0.0;
            if let Some(open) = fold_to(sol, 0, i).and_then(|o| child(sol, o, &[Kind::Raise])) {
                for q in i + 1..n {
                    if let Some(node) =
                        fold_to(sol, open, q).and_then(|r| child(sol, r, &[Kind::Raise]))
                    {
                        let nd = &sol.nodes[node];
                        if nd.player != Some(i) || nd.prob <= 0.0 {
                            continue;
                        }
                        for (acc, kinds) in
                            [(&mut four, &raise[..]), (&mut call, &[Kind::Call][..])]
                        {
                            for (a, w) in acc.iter_mut().zip(weights(nd, kinds)) {
                                *a += nd.prob * w;
                            }
                        }
                        total += nd.prob;
                    }
                }
            }
            if total > 0.0 {
                out.push((
                    format!("{p} vs 3bet/4bet"),
                    four.iter().map(|x| x / total).collect(),
                ));
                out.push((
                    format!("{p} vs 3bet/call"),
                    call.iter().map(|x| x / total).collect(),
                ));
            }
        }
    }
    // BB against the SB's limp
    if let Some(node) = fold_to(sol, 0, n - 2).and_then(|sb| child(sol, sb, &[Kind::Limp])) {
        out.push(("BB vs limp/raise".into(), weights(&sol.nodes[node], &raise)));
    }
    out
}

fn main() {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/defaults/ranges")
        });
    for f in formats() {
        let t = std::time::Instant::now();
        let sol = solve(&f.config, 0.003, 1500, |_| true).unwrap();
        let sits = situations(&sol, &f.positions);
        for (name, w) in &sits {
            let path = dir.join(&f.id).join(format!("{name}.txt"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("{}\n", notation(w))).unwrap();
        }
        println!(
            "{}: {} situations, exploitability {:.4} bb after {} iterations ({:.1} s)",
            f.id,
            sits.len(),
            sol.exploitability,
            sol.iterations,
            t.elapsed().as_secs_f64()
        );
    }
}
