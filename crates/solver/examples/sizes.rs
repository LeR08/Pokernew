//! Memory needed by a few bet trees on the benchmark spot.
use lps_solver::{estimate, Sizes, SpotConfig, StreetSizes, Tree};

fn main() {
    for (label, bet, raise) in [
        ("2 sizes, raise 2.5x", "33%, 75%", "2.5x"),
        ("2 sizes, raise 3x", "33%, 75%", "3x"),
        ("2 sizes, raise all-in", "33%, 75%", "a"),
        ("1 size, raise 3x", "50%", "3x"),
    ] {
        let s = StreetSizes {
            oop: Sizes {
                bet: bet.into(),
                raise: raise.into(),
            },
            ip: Sizes {
                bet: bet.into(),
                raise: raise.into(),
            },
            donk: String::new(),
        };
        let spot = SpotConfig {
            oop_range: "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".into(),
            ip_range: "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".into(),
            board: "QsJh2h".into(),
            pot: 5.5,
            stack: 97.5,
            tree: Tree {
                flop: s.clone(),
                turn: s.clone(),
                river: s,
                max_raises: std::env::args()
                    .nth(1)
                    .map(|x| x.parse().unwrap())
                    .unwrap_or(2),
                ..Tree::default()
            },
            ..SpotConfig::default()
        };
        let e = estimate(&spot).unwrap();
        println!(
            "{label}: {} nodes, {:.2} GB / {:.2} GB compressed",
            e.tree_nodes,
            e.memory as f64 / 1e9,
            e.memory_compressed as f64 / 1e9
        );
    }
}
