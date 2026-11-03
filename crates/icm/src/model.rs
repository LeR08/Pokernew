//! Tournament equity models: Malmuth-Harville ICM and chip EV.
//!
//! # Malmuth-Harville
//! Each player finishes first with probability `stack / total`. Given the
//! players already placed, the next place goes to each remaining player with
//! probability `stack / remaining total`. A player's equity is the sum over
//! places of P(place) × payout.
//!
//! Computed exactly by dynamic programming over the set of players already
//! placed (2ⁿ sets, n ≤ 16): `prob[S]` is the probability that the players of
//! S take the first |S| places (in any order). Players with an empty stack
//! take the last places and share their payouts equally.
//!
//! The equities always add up to the sum of the payouts that can be reached
//! (the first `players` places).

/// Maximum number of players with chips for the exact computation.
pub const MAX_PLAYERS: usize = 16;

/// ICM equity of each player. `payouts[k]` is the prize for place k+1 (missing = 0).
pub fn icm(stacks: &[f64], payouts: &[f64]) -> Vec<f64> {
    let n = stacks.len();
    let mut eq = vec![0.0; n];
    let live: Vec<usize> = (0..n).filter(|&i| stacks[i] > 0.0).collect();
    let m = live.len();
    assert!(
        m <= MAX_PLAYERS,
        "ICM is limited to {MAX_PLAYERS} players with chips"
    );
    let pay = |k: usize| payouts.get(k).copied().unwrap_or(0.0);
    // Busted players share the last places.
    let dead: Vec<usize> = (0..n).filter(|&i| stacks[i] <= 0.0).collect();
    if !dead.is_empty() {
        let share = (m..n).map(pay).sum::<f64>() / dead.len() as f64;
        for &i in &dead {
            eq[i] = share;
        }
    }
    if m == 0 {
        return eq;
    }
    let s: Vec<f64> = live.iter().map(|&i| stacks[i]).collect();
    let total: f64 = s.iter().sum();
    let depth = m.min(payouts.len());
    let size = 1usize << m;
    let mut prob = vec![0.0f64; size];
    let mut sums = vec![0.0f64; size];
    for mask in 1..size {
        let low = mask.trailing_zeros() as usize;
        sums[mask] = sums[mask & (mask - 1)] + s[low];
    }
    prob[0] = 1.0;
    for mask in 0..size {
        let p = prob[mask];
        if p == 0.0 {
            continue;
        }
        let k = mask.count_ones() as usize;
        if k >= depth {
            continue;
        }
        let rest = total - sums[mask];
        for (j, &sj) in s.iter().enumerate() {
            if mask & (1 << j) != 0 {
                continue;
            }
            let q = p * sj / rest;
            eq[live[j]] += q * pay(k);
            prob[mask | (1 << j)] += q;
        }
    }
    eq
}

/// Chip EV in money: each player's share of the chips times the prize pool.
pub fn chip_ev(stacks: &[f64], payouts: &[f64]) -> Vec<f64> {
    let total: f64 = stacks.iter().sum();
    let pool: f64 = payouts.iter().take(stacks.len()).sum();
    stacks
        .iter()
        .map(|s| if total > 0.0 { s / total * pool } else { 0.0 })
        .collect()
}

/// Bubble factor of player `i` against player `j`: what `i` loses by losing an
/// all-in against `j` (for the smaller of the two stacks), divided by what `i`
/// wins by winning it. 1 means chips and money have the same value.
pub fn bubble_factor(stacks: &[f64], payouts: &[f64], i: usize, j: usize) -> f64 {
    let x = stacks[i].min(stacks[j]);
    if i == j || x <= 0.0 {
        return 1.0;
    }
    let now = icm(stacks, payouts)[i];
    let mut win = stacks.to_vec();
    win[i] += x;
    win[j] -= x;
    let mut lose = stacks.to_vec();
    lose[i] -= x;
    lose[j] += x;
    let gain = icm(&win, payouts)[i] - now;
    let loss = now - icm(&lose, payouts)[i];
    if gain <= 0.0 {
        f64::INFINITY
    } else {
        loss / gain
    }
}

/// Risk premium: the equity `i` needs to call an all-in against `j` with no dead
/// money, minus the 50 % that chip EV would need.
pub fn risk_premium(bf: f64) -> f64 {
    if bf.is_infinite() {
        0.5
    } else {
        bf / (1.0 + bf) - 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn sums_to_the_prize_pool() {
        // deterministic pseudo-random cases, 2 to 10 players
        let mut seed = 7u64;
        let mut rnd = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as f64 / (1u64 << 31) as f64
        };
        for n in 2..=10 {
            for _ in 0..20 {
                let stacks: Vec<f64> = (0..n).map(|_| 100.0 + rnd() * 10_000.0).collect();
                let places = 1 + (rnd() * n as f64) as usize;
                let payouts: Vec<f64> = (0..places).map(|k| 1000.0 / (k + 1) as f64).collect();
                let eq = icm(&stacks, &payouts);
                let pool: f64 = payouts.iter().take(n).sum();
                assert!((eq.iter().sum::<f64>() - pool).abs() < 1e-6, "n={n}");
            }
        }
    }

    #[test]
    fn hand_computed() {
        // Two players: winner-takes-all is chip EV.
        let e = icm(&[3000.0, 1000.0], &[100.0]);
        assert!(close(e[0], 75.0) && close(e[1], 25.0));
        // Three players 50/30/20 with stacks 5000/3000/2000 (computed by hand):
        // P1(1st)=.5, P2(1st)=.3, P3(1st)=.2
        // P1 2nd = .3*5/7 + .2*5/8 = .339285714...  → 1st .5, 2nd .3392857, 3rd .1607143
        let e = icm(&[5000.0, 3000.0, 2000.0], &[50.0, 30.0, 20.0]);
        let p1 = 0.5 * 50.0
            + (0.3 * 5.0 / 7.0 + 0.2 * 5.0 / 8.0) * 30.0
            + (1.0 - 0.5 - (0.3 * 5.0 / 7.0 + 0.2 * 5.0 / 8.0)) * 20.0;
        assert!(close(e[0], p1), "{} vs {p1}", e[0]);
        assert!(close(e.iter().sum::<f64>(), 100.0));
        // Equal stacks: equal equity.
        let e = icm(&[1.0; 6], &[40.0, 25.0, 15.0]);
        assert!(e.iter().all(|&x| close(x, 80.0 / 6.0)));
    }

    #[test]
    fn busted_players_take_the_last_places() {
        let e = icm(&[1000.0, 0.0, 1000.0, 0.0], &[50.0, 30.0, 20.0, 10.0]);
        assert!(close(e[1], 15.0) && close(e[3], 15.0));
        assert!(close(e[0], 40.0) && close(e[2], 40.0));
    }

    #[test]
    fn bubble() {
        // On the bubble (3 left, 2 paid) the short stack's calls are expensive for the big stacks.
        let s = [5000.0, 4000.0, 1000.0];
        let p = [65.0, 35.0];
        let bf = bubble_factor(&s, &p, 1, 0);
        assert!(bf > 1.0, "{bf}");
        assert!(risk_premium(bf) > 0.0);
        // Winner-takes-all: no bubble factor.
        assert!((bubble_factor(&s, &[100.0], 1, 0) - 1.0).abs() < 1e-9);
        let c = chip_ev(&s, &p);
        assert!(close(c[0], 50.0));
    }
}
