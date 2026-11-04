//! Preflop solver: a simplified preflop tree (open, limp, 3-bet, 4-bet, all-in)
//! solved by CFR+ over the 169 hand classes, plus range bundles for the
//! range library. See `docs/preflop.md`.

pub mod bundle;
pub mod cfr;
pub mod game;

use lps_equity::combo::NUM_CLASSES;
use serde::Serialize;

pub use cfr::{Progress, Solver};
pub use game::{Act, Config, End, GameError, Kind, Tree};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeOut {
    pub id: usize,
    pub player: Option<usize>,
    pub actions: Vec<Act>,
    pub children: Vec<usize>,
    pub end: Option<End>,
    pub pot: f64,
    /// Average strategy: [action][class].
    pub freq: Vec<Vec<f32>>,
    /// How often the acting player arrives here with each class (own actions only, 0–1).
    pub reach: Vec<f32>,
    /// Probability that the hand gets here at all.
    pub prob: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    pub config: Config,
    pub nodes: Vec<NodeOut>,
    /// Expected value of each player, bb per hand.
    pub values: Vec<f64>,
    pub exploitability: f64,
    pub iterations: usize,
    pub history: Vec<(usize, f64)>,
}

/// Builds and solves; `progress` returns false to stop early.
pub fn solve(
    config: &Config,
    target: f64,
    max_iterations: usize,
    mut progress: impl FnMut(&Progress) -> bool,
) -> Result<Solution, GameError> {
    let tree = Tree::build(config)?;
    let mut s = Solver::new(tree);
    let mut history = vec![];
    let every = 10;
    let e = s.solve(target, max_iterations, every, |p| {
        history.push((p.iterations, p.exploitability));
        progress(p)
    });
    let values = s.values();
    let reaches = s.reaches();
    let prior: Vec<f64> = (0..NUM_CLASSES)
        .map(|c| lps_equity::combo::class_size(c) as f64 / 1326.0)
        .collect();
    let nodes = s
        .tree
        .nodes
        .iter()
        .enumerate()
        .map(|(id, n)| {
            let r = &reaches[id];
            let prob: f64 = r
                .iter()
                .map(|rp| (0..NUM_CLASSES).map(|c| prior[c] * rp[c]).sum::<f64>())
                .product();
            NodeOut {
                id,
                player: n.player,
                actions: n.actions.clone(),
                children: n.children.clone(),
                end: n.end.clone(),
                pot: s.tree.pot(id),
                freq: if n.player.is_some() {
                    s.average(id)
                        .iter()
                        .map(|v| v.iter().map(|&x| x as f32).collect())
                        .collect()
                } else {
                    vec![]
                },
                reach: n
                    .player
                    .map(|p| r[p].iter().map(|&x| x as f32).collect())
                    .unwrap_or_default(),
                prob,
            }
        })
        .collect();
    Ok(Solution {
        config: config.clone(),
        nodes,
        values,
        exploitability: e,
        iterations: s.iterations,
        history,
    })
}
