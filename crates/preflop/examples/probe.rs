//! Open % per position for a few realisation factors (calibration).
use lps_equity::combo::{class_size, NUM_CLASSES};
use lps_preflop::{solve, Config, Kind};

fn main() {
    let args: Vec<f64> = std::env::args()
        .skip(1)
        .map(|x| x.parse().unwrap())
        .collect();
    let (stack, ip, oop, ante, open) = (args[0], args[1], args[2], args[3], args[4]);
    let rake = args.get(5).copied().unwrap_or(0.0);
    let curve = args.get(6).copied().unwrap_or(1.0);
    let cfg = Config {
        stacks: vec![stack; 6],
        ante,
        open,
        realization_ip: ip,
        realization_oop: oop,
        allow_limp: true,
        rake,
        rake_cap: 3.0,
        realization_curve: curve,
        ..Config::default()
    };
    let sol = solve(&cfg, 0.003, 1500, |_| true).unwrap();
    let mut node = 0;
    let mut line = String::new();
    for p in 0..5 {
        let n = &sol.nodes[node];
        let share = |kinds: &[Kind]| -> f64 {
            (0..NUM_CLASSES)
                .map(|c| {
                    n.actions
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| kinds.contains(&a.kind))
                        .map(|(k, _)| n.freq[k][c] as f64)
                        .sum::<f64>()
                        * class_size(c) as f64
                })
                .sum::<f64>()
                / 1326.0
                * 100.0
        };
        line.push_str(&format!(
            "p{p}: raise {:.0}% jam {:.0}% limp {:.0}% | ",
            share(&[Kind::Raise]),
            share(&[Kind::AllIn]),
            share(&[Kind::Limp])
        ));
        node = n.children[n.actions.iter().position(|a| a.kind == Kind::Fold).unwrap()];
    }
    println!(
        "stack {stack} ip {ip} oop {oop} curve {curve}: {line} expl {:.4}",
        sol.exploitability
    );
}
