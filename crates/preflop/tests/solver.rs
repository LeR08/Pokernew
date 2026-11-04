use lps_equity::combo::{class_name, NUM_CLASSES};
use lps_preflop::{solve, Config, Kind};

fn cell(name: &str) -> usize {
    (0..NUM_CLASSES).find(|&c| class_name(c) == name).unwrap()
}

/// Acceptance test of phase 7: the heads-up solver converges below the target.
#[test]
fn heads_up_converges_below_target() {
    let target = 0.005; // bb per hand
    let sol = solve(&Config::default(), target, 3000, |_| true).unwrap();
    eprintln!(
        "heads-up 100 bb: exploitability {:.4} bb after {} iterations",
        sol.exploitability, sol.iterations
    );
    assert!(
        sol.exploitability <= target,
        "{} after {} iterations",
        sol.exploitability,
        sol.iterations
    );
    // zero-sum (chip EV, no rake)
    assert!((sol.values[0] + sol.values[1]).abs() < 1e-6);
    // exploitability goes down
    let h = &sol.history;
    assert!(h.last().unwrap().1 < h[0].1 / 10.0);
    // sane strategies: AA never folds at the root, 72o folds or limps more than it raises
    let root = &sol.nodes[0];
    let fold = root
        .actions
        .iter()
        .position(|a| a.kind == Kind::Fold)
        .unwrap();
    assert!(root.freq[fold][cell("AA")] < 0.01);
    for n in sol.nodes.iter().filter(|n| n.player.is_some()) {
        for c in 0..NUM_CLASSES {
            let s: f32 = n.freq.iter().map(|f| f[c]).sum();
            assert!((s - 1.0).abs() < 1e-4);
        }
    }
}

/// Cross-check: with only all-in raises the game is push/fold, already solved
/// (and checked against published Nash charts) by the ICM module.
#[test]
fn jam_only_matches_the_push_fold_solver() {
    let stack = 8.0;
    let cfg = Config {
        stacks: vec![stack, stack],
        open: 100.0,
        allow_limp: false,
        ..Config::default()
    };
    let sol = solve(&cfg, 0.0002, 4000, |_| true).unwrap();
    let pf = lps_icm::solve(
        &lps_icm::Spot {
            stacks: vec![stack, stack],
            ..lps_icm::Spot::default()
        },
        &lps_icm::Options {
            max_iterations: 4000,
            target: 0.0002,
            locks: vec![],
        },
    )
    .unwrap();
    let push = pf.node(lps_icm::Decision::Push { p: 0 }).unwrap();
    let call = pf.node(lps_icm::Decision::Call { p: 0, q: 1 }).unwrap();
    let root = &sol.nodes[0];
    let jam = root
        .actions
        .iter()
        .position(|a| a.kind == Kind::AllIn)
        .unwrap();
    let bb = &sol.nodes[root.children[jam]];
    let callk = bb
        .actions
        .iter()
        .position(|a| a.kind == Kind::Call)
        .unwrap();
    let mut differ = vec![];
    for c in 0..NUM_CLASSES {
        for (ours, theirs, node) in [
            (root.freq[jam][c], push.freq[c], push),
            (bb.freq[callk][c], call.freq[c], call),
        ] {
            if (ours >= 0.5) != (theirs >= 0.5) && (node.ev_act[c] - node.ev_fold[c]).abs() > 0.01 {
                differ.push(format!("{}: {ours:.2} vs {theirs:.2}", class_name(c)));
            }
        }
    }
    assert!(differ.is_empty(), "{differ:?}");
}

#[test]
fn six_max_approximation_converges() {
    let cfg = Config {
        stacks: vec![100.0; 6],
        allow_limp: false,
        ..Config::default()
    };
    let sol = solve(&cfg, 0.0, 300, |_| true).unwrap();
    let h = &sol.history;
    eprintln!(
        "6-max: exploitability {:.4} → {:.4} bb in {} iterations",
        h[0].1,
        h.last().unwrap().1,
        sol.iterations
    );
    assert!(h.last().unwrap().1 < h[0].1 / 3.0, "{h:?}");
    assert!(sol.values.iter().sum::<f64>().abs() < 0.05);
    // UTG opens tighter than the button
    let raise_share = |node: usize| {
        let n = &sol.nodes[node];
        let k = n
            .actions
            .iter()
            .position(|a| a.kind == Kind::Raise)
            .unwrap();
        (0..NUM_CLASSES)
            .map(|c| n.freq[k][c] as f64 * lps_equity::combo::class_size(c) as f64)
            .sum::<f64>()
            / 1326.0
    };
    let utg = raise_share(0);
    // follow folds to the BTN (player 3)
    let mut node = 0;
    for _ in 0..3 {
        let n = &sol.nodes[node];
        node = n.children[n.actions.iter().position(|a| a.kind == Kind::Fold).unwrap()];
    }
    assert_eq!(sol.nodes[node].player, Some(3));
    let btn = raise_share(node);
    assert!(utg < btn, "UTG {utg:.3} vs BTN {btn:.3}");
}
