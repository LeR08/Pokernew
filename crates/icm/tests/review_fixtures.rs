//! The hand review runs on the real Winamax fixture hands: every hero
//! push/fold decision at 20 bb or less is found, rebuilt and valued.
use lps_icm::review::{review, Choice, ReviewError, ReviewModel};

#[test]
fn reviews_the_winamax_fixture_hands() {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../hh-parsers/fixtures/winamax"
    );
    let parser = lps_hh_parsers::parser_for_room("winamax").unwrap();
    let mut hands = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.to_string_lossy().ends_with(".txt")
                && !p.to_string_lossy().contains("summary")
            {
                let text = std::fs::read_to_string(&p).unwrap();
                for raw in parser.split(&text) {
                    if let Ok(h) = parser.parse(&raw) {
                        hands.push(h);
                    }
                }
            }
        }
    }
    assert!(hands.len() > 100);
    let model = ReviewModel {
        payouts: vec![50.0, 30.0, 20.0],
        ..ReviewModel::default()
    };
    // Hero's hands of the KO fixtures (all of them would take long in debug builds).
    let sample: Vec<_> = hands
        .into_iter()
        .filter(|h| h.tournament.as_ref().is_some_and(|t| t.name.contains("KO")))
        .take(400)
        .collect();
    let mut found = [0usize; 3];
    for h in &sample {
        match review(h, &model) {
            Ok(r) => {
                assert!(r.loss_chips >= 0.0 && r.loss_money.unwrap() >= 0.0);
                assert!(r.effective_bb <= 20.0);
                assert_eq!(r.spot.stacks.len(), r.names.len());
                assert!(r.spot.stacks.len() >= 2);
                found[match r.played {
                    Choice::Push => 0,
                    Choice::Call => 1,
                    Choice::Fold => 2,
                }] += 1;
            }
            Err(ReviewError::Solve(e)) => panic!("{e}"),
            Err(_) => {}
        }
    }
    eprintln!(
        "reviewed: push {}, call {}, fold {}",
        found[0], found[1], found[2]
    );
    assert!(found[0] > 0, "no hero shove found in the fixtures");
    assert!(found[2] > 0, "no hero fold found in the fixtures");
}
