//! Acceptance test of phase 4: 20 reference cases computed exhaustively with
//! eval7 (tools/reference_equity.py) must match within ±0.1 %.
use lps_equity::{equity, Mode, Options, Range};

#[test]
fn twenty_reference_cases_match_eval7() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("reference_cases.json")).unwrap();
    assert_eq!(cases.len(), 20);
    let mut methods = std::collections::BTreeMap::new();
    let mut worst = 0.0f64;
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let ranges: Vec<Range> = case["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Range::parse(p.as_str().unwrap()).unwrap())
            .collect();
        let board = lps_evaluator::parse_cards(case["board"].as_str().unwrap()).unwrap();
        // Monte Carlo cases are run to ±0.05 % so that the ±0.1 % check is not luck.
        let opts = Options {
            mode: Mode::Auto,
            precision: 0.0005,
            ..Options::default()
        };
        let res = equity(&ranges, &board, &[], &opts, None).unwrap();
        *methods.entry(format!("{:?}", res.method)).or_insert(0) += 1;
        for (p, want) in case["equity"].as_array().unwrap().iter().enumerate() {
            let want = want.as_f64().unwrap();
            let got = res.players[p].equity;
            worst = worst.max((got - want).abs());
            assert!(
                (got - want).abs() <= 0.001,
                "{name}, player {}: {got:.5} vs eval7 {want:.5}",
                p + 1
            );
        }
    }
    eprintln!(
        "methods used: {methods:?}; largest difference: {:.4} %",
        worst * 100.0
    );
}
