//! Writes data/preflop_ranking.json: the 169 classes ordered by all-in equity
//! against one random hand (Monte Carlo, ±0.02 % at 95 %).
//!
//! cargo run --release -p lps-equity --example gen_ranking
use lps_equity::combo::{all_combos, class_name, NUM_CLASSES};
use lps_equity::{equity, Mode, Options, Range};

fn main() {
    let random = Range::parse("random").unwrap();
    let mut out: Vec<(String, f64)> = (0..NUM_CLASSES)
        .map(|cell| {
            // one representative combo per class (equity vs random does not depend on suits)
            let c = all_combos().iter().find(|c| c.class() == cell).unwrap();
            let opts = Options {
                mode: Mode::MonteCarlo,
                precision: 0.0002,
                max_ms: 600_000,
                seed: cell as u64 + 1,
                ..Options::default()
            };
            let r = equity(&[Range::single(*c), random.clone()], &[], &[], &opts, None).unwrap();
            eprintln!(
                "{} {:.4} ({} samples)",
                class_name(cell),
                r.players[0].equity,
                r.samples
            );
            (class_name(cell), r.players[0].equity)
        })
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let lines: Vec<String> = out
        .iter()
        .map(|(h, e)| format!("  {{ \"hand\": \"{h}\", \"equity\": {e:.5} }}"))
        .collect();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/preflop_ranking.json");
    std::fs::write(path, format!("[\n{}\n]\n", lines.join(",\n"))).unwrap();
    println!("wrote {path}");
}
