//! CFR+ on the preflop tree, with the 169 hand classes.
//!
//! # Values
//! At each end of the tree the money is known: a fold ends with the pot for the
//! winner; a showdown gives each player pot × equity (exact preflop equity
//! table, card removal included); when the flop is seen, the equities are
//! scaled by each player's realisation factor (in / out of position) and
//! renormalised so that the two shares still add up to the pot. Values are in
//! bb relative to the starting stacks, so they add up to zero.
//!
//! # Counterfactual values
//! For player `i` holding class `a`, the value of a node is the sum over the
//! other players' hands of their probability of having played to that node,
//! times `i`'s payoff. The opponent still in at the end is drawn knowing `a`
//! (card removal); the players who folded only count through their total
//! probability of folding (no card removal).
//!
//! # CFR+
//! Regret matching on positive cumulative regrets (floored at 0 after each
//! update), alternating updates, linear averaging of the strategy (iteration t
//! weighs t). The average strategy converges to an equilibrium (heads-up:
//! a Nash equilibrium of this game).
//!
//! # Exploitability
//! Each player's best response to the others' average strategies is computed
//! exactly (max over actions for every class at each of their nodes).
//! Exploitability = mean over players of (best-response value − actual value),
//! in bb per hand.

// The maths reads best with explicit class indices (a, b) and action indices (k).
#![allow(clippy::needless_range_loop)]

use lps_equity::combo::{class_size, NUM_CLASSES};
use lps_equity::preflop::matrix;
use serde::Serialize;

use crate::game::{End, Tree};

const H: usize = NUM_CLASSES;

pub struct Solver {
    pub tree: Tree,
    prior: Vec<f64>,
    cond: Vec<Vec<f64>>,
    /// Payoff matrices of two-player ends: [node] -> (p, q, U_p[a*H+b]) with U_q = pot - inv - ... computed from U_p.
    pay: Vec<Option<Pay>>,
    /// Cumulative regrets and strategy sums per decision node: [action][class].
    regret: Vec<Vec<Vec<f64>>>,
    strat_sum: Vec<Vec<Vec<f64>>>,
    pub iterations: usize,
}

struct Pay {
    p: usize,
    q: usize,
    /// Share of the pot won by p for (a, b).
    share: Vec<f64>,
    /// What the winner takes: the pot minus the rake.
    pot: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub iterations: usize,
    pub exploitability: f64,
}

/// Pot minus the rake (only pots that see a flop or a showdown are raked).
fn raked(tree: &Tree, node: usize) -> f64 {
    let pot = tree.pot(node);
    let cap = if tree.config.rake_cap > 0.0 {
        tree.config.rake_cap
    } else {
        f64::INFINITY
    };
    pot - (pot * tree.config.rake).min(cap)
}

impl Solver {
    pub fn new(tree: Tree) -> Solver {
        let m = matrix();
        let prior: Vec<f64> = (0..H).map(|c| class_size(c) as f64 / 1326.0).collect();
        let cond: Vec<Vec<f64>> = (0..H)
            .map(|a| {
                let row: Vec<f64> = (0..H).map(|b| m.pairs(a, b)).collect();
                let s: f64 = row.iter().sum();
                row.into_iter().map(|x| x / s).collect()
            })
            .collect();
        let pay = tree
            .nodes
            .iter()
            .enumerate()
            .map(|(id, n)| match n.end {
                Some(End::Showdown { p, q }) => Some(Pay {
                    p,
                    q,
                    share: (0..H * H).map(|k| m.equity(k / H, k % H)).collect(),
                    pot: raked(&tree, id),
                }),
                Some(End::Flop { p, q }) => {
                    let (rp, rq) = if tree.in_position(p, q) {
                        (tree.config.realization_ip, tree.config.realization_oop)
                    } else {
                        (tree.config.realization_oop, tree.config.realization_ip)
                    };
                    let share = (0..H * H)
                        .map(|k| {
                            let e = m.equity(k / H, k % H);
                            let c = tree.config.realization_curve;
                            let (x, y) = (e.powf(c) * rp, (1.0 - e).powf(c) * rq);
                            if x + y > 0.0 {
                                x / (x + y)
                            } else {
                                0.5
                            }
                        })
                        .collect();
                    Some(Pay {
                        p,
                        q,
                        share,
                        pot: raked(&tree, id),
                    })
                }
                _ => None,
            })
            .collect();
        let regret = tree
            .nodes
            .iter()
            .map(|n| vec![vec![0.0; H]; n.actions.len()])
            .collect();
        let strat_sum = tree
            .nodes
            .iter()
            .map(|n| vec![vec![0.0; H]; n.actions.len()])
            .collect();
        Solver {
            tree,
            prior,
            cond,
            pay,
            regret,
            strat_sum,
            iterations: 0,
        }
    }

    fn players(&self) -> usize {
        self.tree.config.stacks.len()
    }

    fn current(&self, node: usize) -> Vec<Vec<f64>> {
        let r = &self.regret[node];
        let na = r.len();
        let mut s = vec![vec![0.0; H]; na];
        for c in 0..H {
            let sum: f64 = (0..na).map(|k| r[k][c].max(0.0)).sum();
            for k in 0..na {
                s[k][c] = if sum > 0.0 {
                    r[k][c].max(0.0) / sum
                } else {
                    1.0 / na as f64
                };
            }
        }
        s
    }

    /// Average strategy at a node: [action][class].
    pub fn average(&self, node: usize) -> Vec<Vec<f64>> {
        let ss = &self.strat_sum[node];
        let na = ss.len();
        let mut s = vec![vec![0.0; H]; na];
        for c in 0..H {
            let sum: f64 = (0..na).map(|k| ss[k][c]).sum();
            for k in 0..na {
                s[k][c] = if sum > 0.0 {
                    ss[k][c] / sum
                } else {
                    1.0 / na as f64
                };
            }
        }
        s
    }

    /// Value for player i at an end, given the others' reach probabilities.
    fn end_value(&self, node: usize, i: usize, reach: &[Vec<f64>]) -> Vec<f64> {
        let n = self.players();
        let nd = &self.tree.nodes[node];
        let mass = |k: usize| -> f64 { (0..H).map(|c| self.prior[c] * reach[k][c]).sum() };
        let inv_i = self.tree.invested(node, i);
        if let Some(pay) = &self.pay[node] {
            if i == pay.p || i == pay.q {
                let j = if i == pay.p { pay.q } else { pay.p };
                let others: f64 = (0..n).filter(|&k| k != i && k != j).map(mass).product();
                let mut v = vec![0.0; H];
                for (a, va) in v.iter_mut().enumerate() {
                    let mut s = 0.0;
                    for b in 0..H {
                        let w = self.cond[a][b] * reach[j][b];
                        if w == 0.0 {
                            continue;
                        }
                        let share = if i == pay.p {
                            pay.share[a * H + b]
                        } else {
                            1.0 - pay.share[b * H + a]
                        };
                        s += w * (pay.pot * share - inv_i);
                    }
                    *va = s * others;
                }
                return v;
            }
        }
        let u = match nd.end {
            Some(End::Fold { winner }) if winner == i => self.tree.pot(node) - inv_i,
            _ => -inv_i,
        };
        let others: f64 = (0..n).filter(|&k| k != i).map(mass).product();
        vec![u * others; H]
    }

    /// Counterfactual values of player i; `mode`: 0 = CFR update, 1 = average strategy, 2 = best response.
    fn walk(
        &mut self,
        node: usize,
        i: usize,
        reach: &mut [Vec<f64>],
        mode: u8,
        weight: f64,
    ) -> Vec<f64> {
        let nd = &self.tree.nodes[node];
        let Some(p) = nd.player else {
            return self.end_value(node, i, reach);
        };
        let children = nd.children.clone();
        let strat = if mode == 0 {
            self.current(node)
        } else {
            self.average(node)
        };
        if p == i {
            let vals: Vec<Vec<f64>> = children
                .iter()
                .map(|&c| self.walk(c, i, reach, mode, weight))
                .collect();
            let mut v = vec![0.0; H];
            for a in 0..H {
                v[a] = if mode == 2 {
                    vals.iter().map(|x| x[a]).fold(f64::NEG_INFINITY, f64::max)
                } else {
                    (0..vals.len()).map(|k| strat[k][a] * vals[k][a]).sum()
                };
            }
            if mode == 0 {
                for k in 0..vals.len() {
                    for a in 0..H {
                        let r = &mut self.regret[node][k][a];
                        *r = (*r + vals[k][a] - v[a]).max(0.0);
                        self.strat_sum[node][k][a] += weight * reach[i][a] * strat[k][a];
                    }
                }
            }
            v
        } else {
            let mut v = vec![0.0; H];
            let saved = reach[p].clone();
            for (k, &c) in children.iter().enumerate() {
                for a in 0..H {
                    reach[p][a] = saved[a] * strat[k][a];
                }
                let x = self.walk(c, i, reach, mode, weight);
                for a in 0..H {
                    v[a] += x[a];
                }
            }
            reach[p] = saved;
            v
        }
    }

    fn root_value(&mut self, i: usize, mode: u8) -> f64 {
        let n = self.players();
        let mut reach = vec![vec![1.0; H]; n];
        let v = self.walk(0, i, &mut reach, mode, 0.0);
        (0..H).map(|a| self.prior[a] * v[a]).sum()
    }

    /// One CFR+ iteration (every player in turn).
    pub fn iterate(&mut self) {
        self.iterations += 1;
        let w = self.iterations as f64;
        let n = self.players();
        for i in 0..n {
            let mut reach = vec![vec![1.0; H]; n];
            self.walk(0, i, &mut reach, 0, w);
        }
    }

    /// Expected value of each player (bb) with the average strategies.
    pub fn values(&mut self) -> Vec<f64> {
        (0..self.players()).map(|i| self.root_value(i, 1)).collect()
    }

    /// Mean over players of what a best response would add, bb per hand.
    pub fn exploitability(&mut self) -> f64 {
        let n = self.players();
        let mut s = 0.0;
        for i in 0..n {
            s += self.root_value(i, 2) - self.root_value(i, 1);
        }
        s / n as f64
    }

    /// Iterates until the exploitability is at most `target` (bb) or `max_iterations`;
    /// calls `progress` every `every` iterations.
    pub fn solve(
        &mut self,
        target: f64,
        max_iterations: usize,
        every: usize,
        mut progress: impl FnMut(&Progress) -> bool,
    ) -> f64 {
        let mut e = f64::INFINITY;
        while self.iterations < max_iterations {
            self.iterate();
            if self.iterations.is_multiple_of(every) || self.iterations == max_iterations {
                e = self.exploitability();
                if !progress(&Progress {
                    iterations: self.iterations,
                    exploitability: e,
                }) || e <= target
                {
                    break;
                }
            }
        }
        if !e.is_finite() {
            e = self.exploitability();
        }
        e
    }

    /// Probability that each player's actions reach `node` (average strategies), per class: [player][class].
    pub fn reaches(&self) -> Vec<Vec<Vec<f64>>> {
        let n = self.players();
        let mut out = vec![vec![vec![0.0; H]; n]; self.tree.nodes.len()];
        fn rec(s: &Solver, node: usize, reach: Vec<Vec<f64>>, out: &mut Vec<Vec<Vec<f64>>>) {
            let nd = &s.tree.nodes[node];
            out[node] = reach.clone();
            if let Some(p) = nd.player {
                let strat = s.average(node);
                for (k, &c) in nd.children.iter().enumerate() {
                    let mut r = reach.clone();
                    for a in 0..H {
                        r[p][a] *= strat[k][a];
                    }
                    rec(s, c, r, out);
                }
            }
        }
        rec(self, 0, vec![vec![1.0; H]; n], &mut out);
        out
    }
}
