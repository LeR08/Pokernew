//! Performance target of the spec: 100 bb single-raised pot on the flop, 2 bet
//! sizes per street, solved to 0.5 % of the pot.
//! cargo run --release -p lps-solver --example bench
use std::time::Instant;

use lps_solver::job::{solve_now, SolveParams};
use lps_solver::{estimate, Sizes, SpotConfig, StreetSizes, Tree};

fn main() {
    // Ranges from the engine's own example (BTN open vs BB call style), pot 5.5 bb, 97.5 bb behind.
    let spot = SpotConfig {
        oop_range: "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".into(),
        ip_range: "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".into(),
        board: std::env::args().nth(1).unwrap_or_else(|| "QsJh2h".into()),
        pot: 5.5,
        stack: 97.5,
        tree: srp_two_sizes(),
        ..SpotConfig::default()
    };
    let est = estimate(&spot).unwrap();
    println!(
        "{} decision nodes per runout path, {} vs {} hands, memory {:.2} GB ({:.2} GB compressed)",
        est.tree_nodes,
        est.oop_hands,
        est.ip_hands,
        est.memory as f64 / 1e9,
        est.memory_compressed as f64 / 1e9
    );
    let t = Instant::now();
    let compression = std::env::var("COMPRESS").is_ok();
    let (_, e) = solve_now(
        &spot,
        &SolveParams {
            target_pct: 0.5,
            max_iterations: 1000,
            compression,
            ..SolveParams::default()
        },
        &[],
    )
    .unwrap();
    println!(
        "exploitability {e:.3} % of the pot in {:.1} s on {} threads",
        t.elapsed().as_secs_f64(),
        rayon::current_num_threads()
    );
}

/// The "SRP 100bb · 2 sizes" preset: bets 33 % and 75 % on every street, raise 3x, one raise per street
/// (plus all-in when the biggest bet is at most 1.5 pots).
fn srp_two_sizes() -> Tree {
    let s = StreetSizes {
        oop: Sizes {
            bet: "33%, 75%".into(),
            raise: "3x".into(),
        },
        ip: Sizes {
            bet: "33%, 75%".into(),
            raise: "3x".into(),
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
