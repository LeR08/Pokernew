//! Push/fold equilibrium for 2 to 10 players (see `docs/icm.md`).
//!
//! # Game
//! Players are listed in preflop order: `[first to act, …, SB, BB]` (heads-up:
//! `[SB, BB]`). Everyone posts the ante (or the BB posts the whole "BB ante"),
//! then the blinds. Each player in turn, while everybody before has folded,
//! either folds or moves all-in. Facing an all-in, the players behind call or
//! fold in turn; the **first caller** goes to showdown (once someone has called,
//! the others fold — overcalls are rare at these stack sizes and are not
//! modelled). If everybody folds to the BB, the BB wins the blinds.
//!
//! # Values
//! Every way the hand can end gives a final stack for each player. Its value is
//! the chips (chip EV), the ICM money (Malmuth-Harville, with the other tables'
//! stacks if given) plus the bounties won, or, with FGS, the value of the
//! following hands played push/fold with the blinds moving round (see [`Fgs`]).
//!
//! # Hands and card removal
//! Hands are the 169 classes; showdown equities are exact (table from
//! `lps_equity::preflop`). The opponent's hand is drawn knowing ours (card
//! removal between the two players at showdown); the folding probabilities of
//! the other players ignore card removal.
//!
//! # Solving
//! Fictitious play: at every iteration each decision gets its best response to
//! the others' *average* strategies, and the average moves towards it with step
//! 1/(t+2). The averages converge to an equilibrium; we stop when the
//! exploitability (what a player could still win by changing their strategy,
//! summed over their decisions and weighted by how often they are reached) is
//! small enough, or after `max_iterations`.

// The maths reads best with explicit indices (players p, q and hand classes a, b).
#![allow(clippy::needless_range_loop)]

use std::collections::HashMap;

use lps_equity::combo::{class_size, NUM_CLASSES};
use lps_equity::preflop::matrix;
use serde::{Deserialize, Serialize};

use crate::model::icm;

const H: usize = NUM_CLASSES;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Spot {
    /// Chips of each player at the start of the hand, in preflop order (SB and BB last).
    pub stacks: Vec<f64>,
    pub sb: f64,
    pub bb: f64,
    /// Ante per player; with `bb_ante`, the amount the big blind posts for everyone.
    pub ante: f64,
    pub bb_ante: bool,
    /// Stacks of the players at other tables (ICM only).
    pub others: Vec<f64>,
    /// Prize of each place (money). Empty: chip EV.
    pub payouts: Vec<f64>,
    /// Money won for knocking out each player (PKO: the cash part of their bounty). Empty: none.
    pub bounties: Vec<f64>,
    /// Future Game Simulation depth (0 = plain ICM, 1 to 3 hands).
    pub fgs_depth: u8,
}

impl Default for Spot {
    fn default() -> Self {
        Spot {
            stacks: vec![],
            sb: 0.5,
            bb: 1.0,
            ante: 0.0,
            bb_ante: false,
            others: vec![],
            payouts: vec![],
            bounties: vec![],
            fgs_depth: 0,
        }
    }
}

/// A decision: player `p` moving all-in first, or player `q` calling `p`'s all-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Decision {
    Push { p: usize },
    Call { p: usize, q: usize },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    pub max_iterations: usize,
    /// Stop when the exploitability is below this, in big blinds (chip EV) or money (ICM).
    pub target: f64,
    /// Fixed strategies (exploitative mode): 169 frequencies per locked decision.
    pub locks: Vec<(Decision, Vec<f64>)>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SolveError {
    #[error("between 2 and 10 players are needed")]
    PlayerCount,
    #[error("player {0} has no chips left after posting the blinds and antes")]
    AllInPreflop(usize),
    #[error("ICM is limited to 16 players with chips")]
    TooManyForIcm,
    #[error("invalid amounts")]
    Invalid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub decision: Decision,
    /// Frequency of push (or call) for each of the 169 classes.
    pub freq: Vec<f64>,
    /// Value of pushing (calling) and of folding, for each class.
    pub ev_act: Vec<f64>,
    pub ev_fold: Vec<f64>,
    /// Probability that this decision is reached.
    pub reach: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    pub nodes: Vec<Node>,
    /// Value of the hand for each player (chips or money) before the cards are dealt.
    pub values: Vec<f64>,
    pub exploitability: f64,
    pub iterations: usize,
    /// "chips" or "money".
    pub unit: &'static str,
}

impl Solution {
    pub fn node(&self, d: Decision) -> Option<&Node> {
        self.nodes.iter().find(|n| n.decision == d)
    }
}

/// How a hand ends.
#[derive(Clone, Copy)]
enum End {
    Walk,
    Steal(usize),
    /// pusher, caller, pusher wins
    Showdown(usize, usize, bool),
}

struct Posted {
    ante: Vec<f64>,
    blind: Vec<f64>,
}

fn posts(spot: &Spot) -> Result<Posted, SolveError> {
    let n = spot.stacks.len();
    let mut ante = vec![0.0; n];
    let mut blind = vec![0.0; n];
    for i in 0..n {
        let s = spot.stacks[i];
        if !(s.is_finite() && s > 0.0) {
            return Err(SolveError::Invalid);
        }
        ante[i] = if spot.bb_ante {
            if i == n - 1 {
                spot.ante.min(s)
            } else {
                0.0
            }
        } else {
            spot.ante.min(s)
        };
        let rest = s - ante[i];
        blind[i] = if i == n - 1 {
            spot.bb.min(rest)
        } else if i == n - 2 {
            spot.sb.min(rest)
        } else {
            0.0
        };
        if s - ante[i] - blind[i] <= 0.0 {
            return Err(SolveError::AllInPreflop(i));
        }
    }
    Ok(Posted { ante, blind })
}

fn final_stacks(spot: &Spot, ps: &Posted, end: End) -> Vec<f64> {
    let n = spot.stacks.len();
    let sa: Vec<f64> = (0..n).map(|i| spot.stacks[i] - ps.ante[i]).collect();
    let antes: f64 = ps.ante.iter().sum();
    let blinds: f64 = ps.blind.iter().sum();
    let mut out: Vec<f64> = (0..n).map(|i| sa[i] - ps.blind[i]).collect();
    match end {
        End::Walk => out[n - 1] += antes + blinds,
        End::Steal(p) => out[p] += antes + blinds,
        End::Showdown(p, q, p_wins) => {
            let c = sa[p].min(sa[q]);
            let dead = antes + blinds - ps.blind[p] - ps.blind[q];
            let (w, l) = if p_wins { (p, q) } else { (q, p) };
            out[w] = sa[w] - c + 2.0 * c + dead;
            out[l] = sa[l] - c;
        }
    }
    out
}

/// Value of a set of final stacks for every table player.
trait Valuer {
    fn value(&mut self, stacks: &[f64], spot: &Spot) -> Vec<f64>;
}

struct Plain;

impl Valuer for Plain {
    fn value(&mut self, stacks: &[f64], spot: &Spot) -> Vec<f64> {
        let n = stacks.len();
        let mut v = if spot.payouts.is_empty() {
            stacks.to_vec()
        } else {
            let all: Vec<f64> = stacks.iter().chain(&spot.others).copied().collect();
            icm(&all, &spot.payouts)[..n].to_vec()
        };
        add_bounties(&mut v, stacks, spot);
        v
    }
}

fn add_bounties(v: &mut [f64], stacks: &[f64], spot: &Spot) {
    if spot.bounties.is_empty() {
        return;
    }
    // Whoever gained chips from a busted player collects the bounty (exactly one winner per showdown).
    let busted: Vec<usize> = (0..stacks.len()).filter(|&i| stacks[i] <= 0.0).collect();
    if busted.is_empty() {
        return;
    }
    let winner = (0..stacks.len())
        .max_by(|&a, &b| (stacks[a] - spot.stacks[a]).total_cmp(&(stacks[b] - spot.stacks[b])));
    if let Some(w) = winner {
        for b in busted {
            v[w] += spot.bounties.get(b).copied().unwrap_or(0.0);
        }
    }
}

/// Future Game Simulation: the final stacks are valued by playing `depth` more
/// hands push/fold (equilibrium of each), the blinds moving one seat each hand,
/// and applying ICM after the last one. Results are cached by stacks.
pub struct Fgs {
    cache: HashMap<(Vec<i64>, u8), Vec<f64>>,
}

impl Valuer for Fgs {
    fn value(&mut self, stacks: &[f64], spot: &Spot) -> Vec<f64> {
        let mut v = self.future(stacks, spot, spot.fgs_depth);
        add_bounties(&mut v, stacks, spot);
        v
    }
}

impl Fgs {
    fn future(&mut self, stacks: &[f64], spot: &Spot, depth: u8) -> Vec<f64> {
        let n = stacks.len();
        let plain = |st: &[f64]| {
            let all: Vec<f64> = st.iter().chain(&spot.others).copied().collect();
            icm(&all, &spot.payouts)[..n].to_vec()
        };
        let live: Vec<usize> = (0..n).filter(|&i| stacks[i] > 0.0).collect();
        if depth == 0 || live.len() < 2 || spot.payouts.is_empty() {
            return plain(stacks);
        }
        let key = (
            stacks
                .iter()
                .map(|s| (s * 100.0).round() as i64)
                .collect::<Vec<_>>(),
            depth,
        );
        if let Some(v) = self.cache.get(&key) {
            return v.clone();
        }
        // Next hand: the button moves one seat, so the order rotates left by one (busted players leave).
        let order: Vec<usize> = live
            .iter()
            .copied()
            .cycle()
            .skip(1)
            .take(live.len())
            .collect();
        let next = Spot {
            stacks: order.iter().map(|&i| stacks[i]).collect(),
            bounties: vec![],
            fgs_depth: depth - 1,
            ..spot.clone()
        };
        let out = match posts(&next) {
            Err(_) => plain(stacks), // someone would be all-in from the blinds: stop simulating
            Ok(_) => {
                let opts = Options {
                    max_iterations: 300,
                    target: 0.0,
                    locks: vec![],
                };
                let sol = if depth - 1 == 0 {
                    solve_with(&next, &opts, &mut Plain)
                } else {
                    let mut inner = Fgs {
                        cache: HashMap::new(),
                    };
                    solve_with(&next, &opts, &mut inner)
                };
                let mut v = plain(stacks);
                if let Ok(sol) = sol {
                    for (j, &i) in order.iter().enumerate() {
                        v[i] = sol.values[j];
                    }
                }
                v
            }
        };
        self.cache.insert(key, out.clone());
        out
    }
}

/// Solves the push/fold equilibrium of a spot.
pub fn solve(spot: &Spot, opts: &Options) -> Result<Solution, SolveError> {
    if spot.fgs_depth > 0 && !spot.payouts.is_empty() {
        solve_with(
            spot,
            opts,
            &mut Fgs {
                cache: HashMap::new(),
            },
        )
    } else {
        solve_with(spot, opts, &mut Plain)
    }
}

fn solve_with(
    spot: &Spot,
    opts: &Options,
    valuer: &mut impl Valuer,
) -> Result<Solution, SolveError> {
    let n = spot.stacks.len();
    if !(2..=10).contains(&n) {
        return Err(SolveError::PlayerCount);
    }
    if !spot.payouts.is_empty() && n + spot.others.len() > crate::model::MAX_PLAYERS {
        return Err(SolveError::TooManyForIcm);
    }
    let ps = posts(spot)?;
    let m = matrix();
    let eq = |a: usize, b: usize| m.equity(a, b);
    // prior and conditional hand distributions
    let prior: Vec<f64> = (0..H).map(|c| class_size(c) as f64 / 1326.0).collect();
    let cond: Vec<Vec<f64>> = (0..H)
        .map(|a| {
            let row: Vec<f64> = (0..H).map(|b| m.pairs(a, b)).collect();
            let s: f64 = row.iter().sum();
            row.into_iter().map(|x| x / s).collect()
        })
        .collect();

    // values of every way the hand can end
    let walk = valuer.value(&final_stacks(spot, &ps, End::Walk), spot);
    let steal: Vec<Vec<f64>> = (0..n)
        .map(|p| valuer.value(&final_stacks(spot, &ps, End::Steal(p)), spot))
        .collect();
    let mut win = vec![vec![vec![]; n]; n];
    let mut lose = vec![vec![vec![]; n]; n];
    for p in 0..n - 1 {
        for q in p + 1..n {
            win[p][q] = valuer.value(&final_stacks(spot, &ps, End::Showdown(p, q, true)), spot);
            lose[p][q] = valuer.value(&final_stacks(spot, &ps, End::Showdown(p, q, false)), spot);
        }
    }

    // strategies
    let mut push = vec![vec![0.5; H]; n - 1];
    let mut call = vec![vec![vec![0.5; H]; n]; n - 1];
    let mut locked: HashMap<Decision, ()> = HashMap::new();
    for (d, f) in &opts.locks {
        if f.len() != H {
            return Err(SolveError::Invalid);
        }
        match *d {
            Decision::Push { p } if p < n - 1 => push[p] = f.clone(),
            Decision::Call { p, q } if p < q && q < n => call[p][q] = f.clone(),
            _ => return Err(SolveError::Invalid),
        }
        locked.insert(*d, ());
    }

    let unit = if spot.payouts.is_empty() {
        "chips"
    } else {
        "money"
    };
    let scale = if spot.payouts.is_empty() {
        spot.bb
    } else {
        1.0
    };
    let max_it = if opts.max_iterations == 0 {
        3000
    } else {
        opts.max_iterations
    };
    let mut it = 0;
    loop {
        let ev = evaluate(
            n, &prior, &cond, &eq, &push, &call, &walk, &steal, &win, &lose,
        );
        let expl = ev.exploitability / scale;
        let done = it >= max_it || (it > 50 && expl <= opts.target);
        if done {
            let nodes = ev.nodes(n, &push, &call);
            return Ok(Solution {
                nodes,
                values: ev.values,
                exploitability: expl,
                iterations: it,
                unit,
            });
        }
        let step = 1.0 / (it as f64 + 2.0);
        for p in 0..n - 1 {
            if !locked.contains_key(&Decision::Push { p }) {
                for a in 0..H {
                    let br = if ev.push_ev[p][a] > ev.fold_ev[p] {
                        1.0
                    } else {
                        0.0
                    };
                    push[p][a] += (br - push[p][a]) * step;
                }
            }
            for q in p + 1..n {
                if locked.contains_key(&Decision::Call { p, q }) {
                    continue;
                }
                for b in 0..H {
                    let br = if ev.call_ev[p][q][b] > ev.callfold_ev[p][q][b] {
                        1.0
                    } else {
                        0.0
                    };
                    call[p][q][b] += (br - call[p][q][b]) * step;
                }
            }
        }
        it += 1;
    }
}

struct Evaluation {
    push_ev: Vec<Vec<f64>>,
    fold_ev: Vec<f64>,
    call_ev: Vec<Vec<Vec<f64>>>,
    callfold_ev: Vec<Vec<Vec<f64>>>,
    push_reach: Vec<f64>,
    call_reach: Vec<Vec<f64>>,
    values: Vec<f64>,
    exploitability: f64,
}

impl Evaluation {
    fn nodes(&self, n: usize, push: &[Vec<f64>], call: &[Vec<Vec<f64>>]) -> Vec<Node> {
        let mut out = Vec::new();
        for p in 0..n - 1 {
            out.push(Node {
                decision: Decision::Push { p },
                freq: push[p].clone(),
                ev_act: self.push_ev[p].clone(),
                ev_fold: vec![self.fold_ev[p]; H],
                reach: self.push_reach[p],
            });
            for q in p + 1..n {
                out.push(Node {
                    decision: Decision::Call { p, q },
                    freq: call[p][q].clone(),
                    ev_act: self.call_ev[p][q].clone(),
                    ev_fold: self.callfold_ev[p][q].clone(),
                    reach: self.call_reach[p][q],
                });
            }
        }
        out
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate(
    n: usize,
    prior: &[f64],
    cond: &[Vec<f64>],
    eq: &impl Fn(usize, usize) -> f64,
    push: &[Vec<f64>],
    call: &[Vec<Vec<f64>>],
    walk: &[f64],
    steal: &[Vec<f64>],
    win: &[Vec<Vec<f64>>],
    lose: &[Vec<Vec<f64>>],
) -> Evaluation {
    // For each pusher p and caller q: K[p][q][a] = P(q calls | p holds a), S = P(q calls and p wins | a).
    let mut k = vec![vec![vec![0.0; H]; n]; n];
    let mut s = vec![vec![vec![0.0; H]; n]; n];
    for p in 0..n - 1 {
        for q in p + 1..n {
            for a in 0..H {
                let (mut kk, mut ss) = (0.0, 0.0);
                for b in 0..H {
                    let w = cond[a][b] * call[p][q][b];
                    kk += w;
                    ss += w * eq(a, b);
                }
                k[p][q][a] = kk;
                s[p][q][a] = ss;
            }
        }
    }
    // Expected value vector (all players) when p pushes hand a, from caller index `from` on.
    let after_push = |p: usize, a: usize, from: usize, who: usize| -> f64 {
        let mut pre = 1.0;
        let mut v = 0.0;
        for q in from..n {
            let kq = k[p][q][a];
            let sq = s[p][q][a];
            v += pre * (sq * win[p][q][who] + (kq - sq) * lose[p][q][who]);
            pre *= 1.0 - kq;
        }
        v + pre * steal[p][who]
    };
    let mut push_ev = vec![vec![0.0; H]; n - 1];
    // V[k] = value vector when the action reaches k with everyone before folded.
    let mut vnext: Vec<f64> = walk.to_vec();
    let mut fold_ev = vec![0.0; n - 1];
    let mut node_value = vec![vec![0.0; n]; n];
    for p in (0..n - 1).rev() {
        fold_ev[p] = vnext[p];
        let mut v = vec![0.0; n];
        for a in 0..H {
            let f = push[p][a];
            for (who, vw) in v.iter_mut().enumerate() {
                let e = after_push(p, a, p + 1, who);
                if who == p {
                    push_ev[p][a] = e;
                }
                *vw += prior[a] * (f * e + (1.0 - f) * vnext[who]);
            }
        }
        node_value[p] = v.clone();
        vnext = v;
    }
    let values = vnext.clone();
    // reach probabilities of the push decisions
    let mut push_reach = vec![0.0; n - 1];
    let mut r = 1.0;
    for p in 0..n - 1 {
        push_reach[p] = r;
        let pf: f64 = (0..H).map(|a| prior[a] * push[p][a]).sum();
        r *= 1.0 - pf;
    }
    // callers
    let mut call_ev = vec![vec![vec![0.0; H]; n]; n - 1];
    let mut callfold_ev = vec![vec![vec![0.0; H]; n]; n - 1];
    let mut call_reach = vec![vec![0.0; n]; n - 1];
    for p in 0..n - 1 {
        let pf: f64 = (0..H).map(|a| prior[a] * push[p][a]).sum();
        // G[q][a]: value for q of the rest of the hand when q folds, p holds a.
        for q in p + 1..n {
            // reach: p pushed, players between folded (no card removal)
            let mut between = 1.0;
            for r2 in p + 1..q {
                between *= 1.0 - (0..H).map(|b| prior[b] * call[p][r2][b]).sum::<f64>();
            }
            call_reach[p][q] = push_reach[p] * pf * between;
            let g: Vec<f64> = (0..H).map(|a| after_push(p, a, q + 1, q)).collect();
            for b in 0..H {
                let (mut nn, mut ce, mut fe) = (0.0, 0.0, 0.0);
                for a in 0..H {
                    let w = cond[b][a] * push[p][a];
                    if w == 0.0 {
                        continue;
                    }
                    nn += w;
                    let e = eq(b, a);
                    ce += w * (e * lose[p][q][q] + (1.0 - e) * win[p][q][q]);
                    fe += w * g[a];
                }
                if nn > 0.0 {
                    call_ev[p][q][b] = ce / nn;
                    callfold_ev[p][q][b] = fe / nn;
                } else {
                    call_ev[p][q][b] = 0.0;
                    callfold_ev[p][q][b] = 0.0;
                }
            }
        }
    }
    // exploitability: per player, reach-weighted gain of the best response at each decision
    let mut gain = vec![0.0; n];
    for p in 0..n - 1 {
        for a in 0..H {
            let cur = push[p][a] * push_ev[p][a] + (1.0 - push[p][a]) * fold_ev[p];
            gain[p] += push_reach[p] * prior[a] * (push_ev[p][a].max(fold_ev[p]) - cur);
        }
        for q in p + 1..n {
            for b in 0..H {
                let c = call[p][q][b];
                let cur = c * call_ev[p][q][b] + (1.0 - c) * callfold_ev[p][q][b];
                gain[q] += call_reach[p][q]
                    * prior[b]
                    * (call_ev[p][q][b].max(callfold_ev[p][q][b]) - cur);
            }
        }
    }
    let _ = node_value;
    Evaluation {
        push_ev,
        fold_ev,
        call_ev,
        callfold_ev,
        push_reach,
        call_reach,
        values,
        exploitability: gain.iter().copied().fold(0.0, f64::max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lps_equity::combo::class_name;

    pub(crate) fn hu(stack: f64) -> Solution {
        let spot = Spot {
            stacks: vec![stack, stack],
            ..Spot::default()
        };
        solve(
            &spot,
            &Options {
                max_iterations: 3000,
                target: 0.0005,
                locks: vec![],
            },
        )
        .unwrap()
    }

    fn cell(name: &str) -> usize {
        (0..H).find(|&c| class_name(c) == name).unwrap()
    }

    #[test]
    fn chip_ev_is_zero_sum_and_converges() {
        let sol = hu(10.0);
        assert!((sol.values[0] + sol.values[1] - 20.0).abs() < 1e-6);
        assert!(sol.exploitability < 0.01, "{}", sol.exploitability);
        let push = sol.node(Decision::Push { p: 0 }).unwrap();
        assert!(push.freq[cell("AA")] > 0.99);
        assert!(push.freq[cell("72o")] < 0.01);
    }

    #[test]
    fn icm_values_sum_to_the_prize_pool() {
        let spot = Spot {
            stacks: vec![2000.0, 3000.0, 1500.0],
            sb: 50.0,
            bb: 100.0,
            ante: 10.0,
            payouts: vec![50.0, 30.0, 20.0],
            ..Spot::default()
        };
        let sol = solve(
            &spot,
            &Options {
                max_iterations: 800,
                ..Options::default()
            },
        )
        .unwrap();
        assert!((sol.values.iter().sum::<f64>() - 100.0).abs() < 1e-6);
        assert_eq!(sol.unit, "money");
    }

    #[test]
    fn locked_villain_gives_a_max_ev_push() {
        // BB calls only with AA: SB should push everything that beats folding.
        let mut aa = vec![0.0; H];
        aa[cell("AA")] = 1.0;
        let spot = Spot {
            stacks: vec![10.0, 10.0],
            ..Spot::default()
        };
        let sol = solve(
            &spot,
            &Options {
                max_iterations: 200,
                target: 0.0,
                locks: vec![(Decision::Call { p: 0, q: 1 }, aa)],
            },
        )
        .unwrap();
        let push = sol.node(Decision::Push { p: 0 }).unwrap();
        assert!(push.freq[cell("72o")] > 0.99);
    }

    #[test]
    fn rejects_bad_spots() {
        let e = solve(
            &Spot {
                stacks: vec![10.0],
                ..Spot::default()
            },
            &Options::default(),
        )
        .unwrap_err();
        assert_eq!(e, SolveError::PlayerCount);
        let e = solve(
            &Spot {
                stacks: vec![10.0, 1.0],
                ..Spot::default()
            },
            &Options::default(),
        )
        .unwrap_err();
        assert_eq!(e, SolveError::AllInPreflop(1));
    }
}
