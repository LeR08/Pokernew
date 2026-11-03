//! Prints the heads-up push frequency and EV gap of a few hands (convergence check).
//! cargo run --release -p lps-icm --example hu_probe
use lps_equity::combo::{class_name, NUM_CLASSES};
use lps_icm::{solve, Decision, Options, Spot};

fn main() {
    for (stack, hand) in [(4.0, "53s"), (5.0, "96o"), (6.0, "94s"), (9.0, "84s")] {
        let c = (0..NUM_CLASSES).find(|&c| class_name(c) == hand).unwrap();
        for it in [2000, 20000] {
            let s = solve(
                &Spot {
                    stacks: vec![stack, stack],
                    ..Spot::default()
                },
                &Options {
                    max_iterations: it,
                    target: 0.0,
                    locks: vec![],
                },
            )
            .unwrap();
            let n = s.node(Decision::Push { p: 0 }).unwrap();
            println!("{stack} bb {hand} it={it}: push {:.2}, EV push - fold = {:+.4} bb, exploitability {:.5}", n.freq[c], n.ev_act[c] - n.ev_fold[c], s.exploitability);
        }
    }
}
