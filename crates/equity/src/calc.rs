//! Equity of 2 to 10 hands or ranges on a board of 0 to 5 cards.
//!
//! # What is computed
//! Each player holds one combo of their range. A *deal* is one combo per player
//! (no shared card) plus the missing board cards. Deals are weighted by the
//! product of the combos' weights; every board completion is equally likely.
//! A player's equity is their expected share of the pot (1 if they win alone,
//! 1/k when k players split).
//!
//! # Exhaustive enumeration
//! When the number of deals is small (`Options::exhaustive_limit`), every deal
//! is visited: exact result. The work is split across threads by player 1's
//! combo and by the first board card.
//!
//! # Monte Carlo
//! Otherwise deals are sampled: one combo per player drawn by weight, the whole
//! deal is redrawn when two players share a card (rejection sampling, which
//! gives exactly the weighting above), then the board is completed uniformly.
//! Batches run on all cores until the 95 % confidence half-width of every
//! player's equity (1.96 × standard error) is below `Options::precision`, or the
//! time / sample limit is reached.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use lps_evaluator::{evaluate, Card, Hand};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::combo::Combo;
use crate::range::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Auto,
    Exhaustive,
    MonteCarlo,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    /// Monte Carlo stops when the 95 % half-width is below this (0.001 = ±0.1 %).
    pub precision: f64,
    pub max_ms: u64,
    pub max_samples: u64,
    pub seed: u64,
    /// Auto mode enumerates when `deals × players` is at most this.
    pub exhaustive_limit: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Auto,
            precision: 0.001,
            max_ms: 30_000,
            max_samples: 2_000_000_000,
            seed: 0x5eed_1234_abcd,
            exhaustive_limit: 1.2e8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CalcError {
    #[error("between 2 and 10 players are needed")]
    PlayerCount,
    #[error("the board must have 0 to 5 different cards, not shared with dead cards")]
    BadBoard,
    #[error("player {0}'s range is empty (after removing the board and dead cards)")]
    EmptyRange(usize),
    #[error("no deal is possible: the hands or ranges share all their cards")]
    NoValidDeal,
    #[error("not enough cards left in the deck")]
    DeckTooSmall,
    #[error("cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComboEquity {
    pub combo: u16,
    pub weight: f64,
    pub equity: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerResult {
    pub equity: f64,
    pub win: f64,
    pub tie: f64,
    /// Weighted number of live combos.
    pub combos: f64,
    /// Equity of each combo of the range (for the distribution graph).
    pub per_combo: Vec<ComboEquity>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EquityResult {
    pub players: Vec<PlayerResult>,
    pub method: Mode,
    /// Boards evaluated (exhaustive) or deals sampled (Monte Carlo).
    pub samples: u64,
    pub elapsed_ms: u64,
    /// 95 % half-width of the least precise equity (0 when exhaustive).
    pub margin: f64,
}

struct Player {
    combos: Vec<(Combo, u64, f64)>,
    /// Cumulative weights, for sampling.
    cum: Vec<f64>,
}

#[derive(Clone)]
struct Acc {
    total: f64,
    win: Vec<f64>,
    tie: Vec<f64>,
    eq: Vec<f64>,
    eq2: Vec<f64>,
    combo_eq: Vec<Vec<f64>>,
    combo_w: Vec<Vec<f64>>,
    samples: u64,
    rejected: u64,
}

impl Acc {
    fn new(players: &[Player]) -> Acc {
        let n = players.len();
        Acc {
            total: 0.0,
            win: vec![0.0; n],
            tie: vec![0.0; n],
            eq: vec![0.0; n],
            eq2: vec![0.0; n],
            combo_eq: players.iter().map(|p| vec![0.0; p.combos.len()]).collect(),
            combo_w: players.iter().map(|p| vec![0.0; p.combos.len()]).collect(),
            samples: 0,
            rejected: 0,
        }
    }

    fn merge(mut self, o: Acc) -> Acc {
        self.total += o.total;
        self.samples += o.samples;
        self.rejected += o.rejected;
        for p in 0..self.win.len() {
            self.win[p] += o.win[p];
            self.tie[p] += o.tie[p];
            self.eq[p] += o.eq[p];
            self.eq2[p] += o.eq2[p];
            for (a, b) in self.combo_eq[p].iter_mut().zip(&o.combo_eq[p]) {
                *a += b;
            }
            for (a, b) in self.combo_w[p].iter_mut().zip(&o.combo_w[p]) {
                *a += b;
            }
        }
        self
    }

    /// 95 % half-width of the least precise player (Monte Carlo).
    fn margin(&self) -> f64 {
        let n = self.total;
        if n < 2.0 {
            return f64::INFINITY;
        }
        (0..self.eq.len())
            .map(|p| {
                let m = self.eq[p] / n;
                let var = (self.eq2[p] / n - m * m).max(0.0);
                1.96 * (var / n).sqrt()
            })
            .fold(0.0, f64::max)
    }
}

/// Showdown of one deal: adds each player's result, weighted by `w`.
#[inline]
fn showdown(masks: &[u64], board: u64, vals: &mut [u32], out: &mut [(f64, f64, f64)], w: f64) {
    let mut best = 0;
    let mut winners = 0u32;
    for (i, &m) in masks.iter().enumerate() {
        let v = evaluate(Hand(m | board));
        vals[i] = v;
        if v > best {
            best = v;
            winners = 1;
        } else if v == best {
            winners += 1;
        }
    }
    let share = w / winners as f64;
    for (i, &v) in vals.iter().enumerate() {
        if v == best {
            if winners == 1 {
                out[i].0 += w;
            } else {
                out[i].1 += w;
            }
            out[i].2 += share;
        }
    }
}

fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.0;
    }
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// Computes the equity of every player.
pub fn equity(
    ranges: &[Range],
    board: &[Card],
    dead: &[Card],
    opts: &Options,
    cancel: Option<&AtomicBool>,
) -> Result<EquityResult, CalcError> {
    let start = Instant::now();
    let n = ranges.len();
    if !(2..=10).contains(&n) {
        return Err(CalcError::PlayerCount);
    }
    let board_mask = Hand::from_cards(board);
    let dead_mask = Hand::from_cards(dead);
    if board.len() > 5
        || board_mask.len() as usize != board.len()
        || board_mask.0 & dead_mask.0 != 0
    {
        return Err(CalcError::BadBoard);
    }
    let blocked = board_mask.union(dead_mask);
    let players: Vec<Player> = ranges
        .iter()
        .map(|r| {
            let combos: Vec<(Combo, u64, f64)> =
                r.live(blocked).map(|(c, w)| (c, c.mask().0, w)).collect();
            let mut acc = 0.0;
            let cum = combos.iter().map(|c| {
                acc += c.2;
                acc
            });
            Player {
                cum: cum.collect(),
                combos,
            }
        })
        .collect();
    if let Some(i) = players.iter().position(|p| p.combos.is_empty()) {
        return Err(CalcError::EmptyRange(i));
    }
    let avail: Vec<Card> = (0..52u8).filter(|&c| !blocked.contains(c)).collect();
    let k = 5 - board.len();
    if avail.len() < 2 * n + k {
        return Err(CalcError::DeckTooSmall);
    }
    if !feasible(&players) {
        return Err(CalcError::NoValidDeal);
    }

    let deals: f64 = players
        .iter()
        .map(|p| p.combos.len() as f64)
        .product::<f64>()
        * binomial(avail.len() - 2 * n, k);
    let mode = match opts.mode {
        Mode::Auto if deals * n as f64 <= opts.exhaustive_limit => Mode::Exhaustive,
        Mode::Auto => Mode::MonteCarlo,
        m => m,
    };
    let acc = if mode == Mode::Exhaustive {
        exhaustive(&players, board_mask.0, &avail, k, cancel)
    } else {
        monte_carlo(&players, board_mask.0, &avail, k, opts, cancel, start)?
    };
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        return Err(CalcError::Cancelled);
    }
    if acc.total <= 0.0 {
        return Err(CalcError::NoValidDeal);
    }
    let margin = if mode == Mode::Exhaustive {
        0.0
    } else {
        acc.margin()
    };
    let results = players
        .iter()
        .enumerate()
        .map(|(p, pl)| PlayerResult {
            equity: acc.eq[p] / acc.total,
            win: acc.win[p] / acc.total,
            tie: acc.tie[p] / acc.total,
            combos: pl.combos.iter().map(|c| c.2).sum(),
            per_combo: pl
                .combos
                .iter()
                .enumerate()
                .filter(|(j, _)| acc.combo_w[p][*j] > 0.0)
                .map(|(j, c)| ComboEquity {
                    combo: c.0.index() as u16,
                    weight: c.2,
                    equity: acc.combo_eq[p][j] / acc.combo_w[p][j],
                })
                .collect(),
        })
        .collect();
    Ok(EquityResult {
        players: results,
        method: mode,
        samples: acc.samples,
        elapsed_ms: start.elapsed().as_millis() as u64,
        margin,
    })
}

/// Is there at least one deal with no shared card? (bounded search)
fn feasible(players: &[Player]) -> bool {
    fn rec(players: &[Player], p: usize, used: u64, budget: &mut u32) -> bool {
        if p == players.len() {
            return true;
        }
        for c in &players[p].combos {
            if *budget == 0 {
                return true; // give up proving it: Monte Carlo will tell
            }
            *budget -= 1;
            if c.1 & used == 0 && rec(players, p + 1, used | c.1, budget) {
                return true;
            }
        }
        false
    }
    rec(players, 0, 0, &mut 2_000_000)
}

/// Calls `f(mask)` for every set of `k` cards of `avail[start..]` avoiding `used`.
fn boards(avail: &[Card], start: usize, k: usize, used: u64, cur: u64, f: &mut impl FnMut(u64)) {
    if k == 0 {
        f(cur);
        return;
    }
    for i in start..=avail.len().saturating_sub(k) {
        let b = Hand::bit(avail[i]);
        if b & used == 0 {
            boards(avail, i + 1, k - 1, used, cur | b, f);
        }
    }
}

fn exhaustive(
    players: &[Player],
    board: u64,
    avail: &[Card],
    k: usize,
    cancel: Option<&AtomicBool>,
) -> Acc {
    let n = players.len();
    // Work items: (player 1's combo, first board card).
    let items: Vec<(usize, Option<usize>)> = (0..players[0].combos.len())
        .flat_map(|i| {
            let firsts: Vec<Option<usize>> = if k == 0 {
                vec![None]
            } else {
                (0..=avail.len() - k).map(Some).collect()
            };
            firsts.into_iter().map(move |f| (i, f))
        })
        .collect();
    items
        .par_iter()
        .fold(
            || Acc::new(players),
            |mut acc, &(i, first)| {
                if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                    return acc;
                }
                let (_, m0, w0) = players[0].combos[i];
                if let Some(a) = first {
                    if Hand::bit(avail[a]) & m0 != 0 {
                        return acc;
                    }
                }
                let mut sel = vec![0usize; n];
                let mut masks = vec![0u64; n];
                sel[0] = i;
                masks[0] = m0;
                let mut vals = vec![0u32; n];
                let mut out = vec![(0.0, 0.0, 0.0); n];
                deal(
                    players,
                    1,
                    m0,
                    w0,
                    &mut sel,
                    &mut masks,
                    &mut |sel, masks, w| {
                        let used: u64 = masks.iter().fold(0, |a, m| a | m);
                        out.iter_mut().for_each(|o| *o = (0.0, 0.0, 0.0));
                        let mut nb = 0u64;
                        let mut visit = |b: u64| {
                            nb += 1;
                            showdown(masks, board | b, &mut vals, &mut out, 1.0);
                        };
                        match first {
                            None => visit(0),
                            Some(a) => {
                                let fb = Hand::bit(avail[a]);
                                if fb & used != 0 {
                                    return;
                                }
                                boards(avail, a + 1, k - 1, used, fb, &mut visit);
                            }
                        }
                        let nbf = nb as f64;
                        acc.total += w * nbf;
                        acc.samples += nb;
                        for p in 0..n {
                            acc.win[p] += w * out[p].0;
                            acc.tie[p] += w * out[p].1;
                            acc.eq[p] += w * out[p].2;
                            acc.combo_eq[p][sel[p]] += w * out[p].2;
                            acc.combo_w[p][sel[p]] += w * nbf;
                        }
                    },
                );
                acc
            },
        )
        .reduce(|| Acc::new(players), Acc::merge)
}

/// Visits every combination of combos for players `p..` that shares no card with `used`.
fn deal(
    players: &[Player],
    p: usize,
    used: u64,
    w: f64,
    sel: &mut [usize],
    masks: &mut [u64],
    f: &mut impl FnMut(&[usize], &[u64], f64),
) {
    if p == players.len() {
        f(sel, masks, w);
        return;
    }
    for (j, &(_, m, wj)) in players[p].combos.iter().enumerate() {
        if m & used == 0 {
            sel[p] = j;
            masks[p] = m;
            deal(players, p + 1, used | m, w * wj, sel, masks, f);
        }
    }
}

/// Small, fast, seedable generator (SplitMix64).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n`.
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next_u64() as u128 * n as u128) >> 64) as usize
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn mc_batch(
    players: &[Player],
    board: u64,
    avail: &[Card],
    k: usize,
    trials: u64,
    rng: &mut Rng,
) -> Acc {
    let n = players.len();
    let mut acc = Acc::new(players);
    let mut sel = vec![0usize; n];
    let mut masks = vec![0u64; n];
    let mut vals = vec![0u32; n];
    let mut out = vec![(0.0, 0.0, 0.0); n];
    let mut done = 0;
    while done < trials {
        let mut used = 0u64;
        let mut ok = true;
        for (p, pl) in players.iter().enumerate() {
            let j = if pl.combos.len() == 1 {
                0
            } else {
                let r = rng.unit() * pl.cum[pl.cum.len() - 1];
                pl.cum.partition_point(|&c| c <= r).min(pl.combos.len() - 1)
            };
            let m = pl.combos[j].1;
            if m & used != 0 {
                ok = false;
                break;
            }
            used |= m;
            sel[p] = j;
            masks[p] = m;
        }
        if !ok {
            acc.rejected += 1;
            if acc.rejected > 1000 * (done + 1000) {
                break; // almost no valid deal: give up this batch
            }
            continue;
        }
        let mut b = 0u64;
        let mut cnt = 0;
        while cnt < k {
            let bit = Hand::bit(avail[rng.below(avail.len())]);
            if (used | b) & bit == 0 {
                b |= bit;
                cnt += 1;
            }
        }
        out.iter_mut().for_each(|o| *o = (0.0, 0.0, 0.0));
        showdown(&masks, board | b, &mut vals, &mut out, 1.0);
        acc.total += 1.0;
        acc.samples += 1;
        for p in 0..n {
            let s = out[p].2;
            acc.win[p] += out[p].0;
            acc.tie[p] += out[p].1;
            acc.eq[p] += s;
            acc.eq2[p] += s * s;
            acc.combo_eq[p][sel[p]] += s;
            acc.combo_w[p][sel[p]] += 1.0;
        }
        done += 1;
    }
    acc
}

fn splitmix(x: u64) -> u64 {
    Rng::new(x).next_u64()
}

fn monte_carlo(
    players: &[Player],
    board: u64,
    avail: &[Card],
    k: usize,
    opts: &Options,
    cancel: Option<&AtomicBool>,
    start: Instant,
) -> Result<Acc, CalcError> {
    let threads = rayon::current_num_threads().max(1) as u64;
    let mut acc = Acc::new(players);
    let mut batch = 20_000u64;
    for round in 0u64.. {
        let parts: Vec<Acc> = (0..threads)
            .into_par_iter()
            .map(|t| {
                let mut rng = Rng::new(splitmix(opts.seed ^ (round * 1024 + t)));
                mc_batch(players, board, avail, k, batch, &mut rng)
            })
            .collect();
        for p in parts {
            acc = acc.merge(p);
        }
        if acc.samples == 0 && acc.rejected > 0 {
            return Err(CalcError::NoValidDeal);
        }
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return Err(CalcError::Cancelled);
        }
        let precise = acc.samples >= 50_000 && acc.margin() <= opts.precision;
        if precise
            || acc.samples >= opts.max_samples
            || start.elapsed().as_millis() as u64 >= opts.max_ms
        {
            break;
        }
        batch = (batch * 2).min(400_000);
    }
    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lps_evaluator::parse_cards;

    fn r(s: &str) -> Range {
        Range::parse(s).unwrap()
    }

    fn eq(ranges: &[&str], board: &str, mode: Mode) -> EquityResult {
        let ranges: Vec<Range> = ranges.iter().map(|s| r(s)).collect();
        let opts = Options {
            mode,
            ..Options::default()
        };
        equity(&ranges, &parse_cards(board).unwrap(), &[], &opts, None).unwrap()
    }

    #[test]
    fn river_is_deterministic() {
        // Royal flush on board: split.
        let res = eq(&["2c2d", "3c3d"], "AsKsQsJsTs", Mode::Auto);
        assert_eq!(res.method, Mode::Exhaustive);
        assert_eq!(res.players[0].equity, 0.5);
        assert_eq!(res.players[0].tie, 1.0);
        let res = eq(&["AhAd", "KhKd"], "2c7d9sJhQc", Mode::Auto);
        assert_eq!(res.players[0].equity, 1.0);
        assert_eq!(res.samples, 1);
    }

    #[test]
    fn turn_outs() {
        // KK has a set; AA only wins with the two other aces (44 river cards).
        let res = eq(&["AhAd", "KhKd"], "Kc7d2s3h", Mode::Auto);
        assert!((res.players[0].equity - 2.0 / 44.0).abs() < 1e-12);
    }

    #[test]
    fn exhaustive_and_monte_carlo_agree() {
        let ex = eq(&["AKs", "QQ,JJ"], "Th5h2c", Mode::Exhaustive);
        let mc = eq(&["AKs", "QQ,JJ"], "Th5h2c", Mode::MonteCarlo);
        assert!(mc.margin <= 0.001);
        assert!(
            (ex.players[0].equity - mc.players[0].equity).abs() < 0.0015,
            "{} vs {}",
            ex.players[0].equity,
            mc.players[0].equity
        );
        let sum: f64 = ex.players.iter().map(|p| p.equity).sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn errors() {
        let o = Options::default();
        assert_eq!(
            equity(&[r("AA")], &[], &[], &o, None).unwrap_err(),
            CalcError::PlayerCount
        );
        assert_eq!(
            equity(&[r("AhKh"), r("AhKh")], &[], &[], &o, None).unwrap_err(),
            CalcError::NoValidDeal
        );
        let b = parse_cards("AhAdAs").unwrap();
        assert_eq!(
            equity(&[r("AA"), r("KK")], &[], &b, &o, None).unwrap_err(),
            CalcError::EmptyRange(0)
        );
    }
}
