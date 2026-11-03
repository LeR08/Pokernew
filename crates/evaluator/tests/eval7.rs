//! Our evaluator must pick the same winner as eval7 (tools/reference_evaluator.py).
use lps_evaluator::{evaluate_cards, parse_cards};

#[test]
fn same_winner_as_eval7() {
    let text = include_str!("eval7_pairs.txt");
    let mut n = 0;
    for line in text.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = line.split(' ').collect();
        let a = evaluate_cards(&parse_cards(f[0]).unwrap());
        let b = evaluate_cards(&parse_cards(f[1]).unwrap());
        let want: i32 = f[2].parse().unwrap();
        assert_eq!(a.cmp(&b) as i32, want, "{line}");
        n += 1;
    }
    assert_eq!(n, 20_000);
}
