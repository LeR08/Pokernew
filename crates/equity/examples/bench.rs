//! Performance targets of the spec (run in release):
//! cargo run --release -p lps-equity --example bench
use std::time::Instant;

use lps_equity::{equity, Mode, Options, Range};

fn run(label: &str, ranges: &[&str], mode: Mode) {
    let ranges: Vec<Range> = ranges.iter().map(|r| Range::parse(r).unwrap()).collect();
    let opts = Options {
        mode,
        ..Options::default()
    };
    let t = Instant::now();
    let r = equity(&ranges, &[], &[], &opts, None).unwrap();
    println!(
        "{label}: {:.3} % in {} ms ({:?}, {} samples, ±{:.3} %, {} threads)",
        r.players[0].equity * 100.0,
        t.elapsed().as_millis(),
        r.method,
        r.samples,
        r.margin * 100.0,
        rayon::current_num_threads()
    );
}

fn main() {
    run(
        "AsAh vs KsKh preflop exhaustive",
        &["AsAh", "KsKh"],
        Mode::Exhaustive,
    );
    run(
        "AKs vs 22 preflop exhaustive",
        &["AcKc", "2d2h"],
        Mode::Exhaustive,
    );
    run(
        "top 15 % vs top 30 % preflop ±0.1 %",
        &["15%", "30%"],
        Mode::MonteCarlo,
    );
    run(
        "QQ+,AK vs random preflop ±0.1 %",
        &["QQ+,AK", "random"],
        Mode::MonteCarlo,
    );
}
