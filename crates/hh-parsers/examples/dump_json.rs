//! Prints the hands of a hand-history file as JSON (dev tool).
//! `cargo run -p lps-hh-parsers --example dump_json -- <file> [max]`
use lps_hh_parsers::{detect, Detected};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: dump_json <file> [max]");
    let max: usize = args
        .next()
        .and_then(|m| m.parse().ok())
        .unwrap_or(usize::MAX);
    let text = std::fs::read_to_string(&path).expect("read file");
    let Some(Detected::Hands(p)) = detect(text.as_bytes()) else {
        panic!("not a hand-history file")
    };
    let hands: Vec<_> = p
        .split(&text)
        .iter()
        .take(max)
        .map(|r| p.parse(r).expect("parse"))
        .collect();
    println!("{}", serde_json::to_string_pretty(&hands).unwrap());
}
