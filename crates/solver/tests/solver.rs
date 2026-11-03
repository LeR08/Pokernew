use std::time::{Duration, Instant};

use lps_solver::job::{solve_now, Job, Lock, SolveParams, State};
use lps_solver::{cache, estimate, view, Sizes, SpotConfig, StreetSizes, Tree};

fn river_spot() -> SpotConfig {
    let allin = StreetSizes {
        oop: Sizes {
            bet: "a".into(),
            raise: String::new(),
        },
        ip: Sizes {
            bet: "a".into(),
            raise: String::new(),
        },
        donk: String::new(),
    };
    SpotConfig {
        oop_range: "AsAh,QsQh".into(),
        ip_range: "KsKh".into(),
        board: "2s3h4d6c7c".into(),
        pot: 20.0,
        stack: 10.0,
        tree: Tree {
            flop: allin.clone(),
            turn: allin.clone(),
            river: allin,
            ..Tree::default()
        },
        ..SpotConfig::default()
    }
}

fn turn_spot() -> SpotConfig {
    SpotConfig {
        oop_range: "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s".into(),
        ip_range: "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+".into(),
        board: "Td9d6hQc".into(),
        pot: 20.0,
        stack: 90.0,
        ..SpotConfig::default()
    }
}

#[test]
fn solves_below_target_and_values_add_up_to_the_pot() {
    let spot = turn_spot();
    let est = estimate(&spot).unwrap();
    assert!(est.memory > 0 && est.tree_nodes > 10 && est.oop_hands > 100);
    let (mut game, e) = solve_now(
        &spot,
        &SolveParams {
            target_pct: 0.5,
            max_iterations: 500,
            ..SolveParams::default()
        },
        &[],
    )
    .unwrap();
    assert!(e <= 0.5, "{e}");
    let v = view(&mut game, &[]).unwrap();
    assert_eq!(v.kind, "action");
    assert_eq!(v.player, Some(0));
    assert!((v.totals.iter().sum::<f32>() - 1.0).abs() < 1e-3);
    // No rake: what OOP and IP expect to get adds up to the pot (within the exploitability).
    let sum = v.players[0].ev + v.players[1].ev;
    assert!(
        (sum as f64 - spot.pot).abs() < spot.pot * 0.02,
        "{sum} vs {}",
        spot.pot
    );
    // Navigate: bet, then the river card.
    let bet = v.actions.iter().position(|a| a.kind == "bet").unwrap();
    let v2 = view(&mut game, &[bet]).unwrap();
    assert_eq!(v2.player, Some(1));
    let call = v2.actions.iter().position(|a| a.kind == "call").unwrap();
    let v3 = view(&mut game, &[bet, call]).unwrap();
    assert_eq!(v3.kind, "chance");
    assert_eq!(v3.cards.len(), 48);
    let card = lps_evaluator::parse_card(&v3.cards[0]).unwrap() as usize;
    let v4 = view(&mut game, &[bet, call, card]).unwrap();
    assert_eq!(v4.board.len(), 5);
    assert!(!lps_solver::view::to_csv(&v4).is_empty());
}

#[test]
fn node_locking_changes_the_strategy() {
    // IP (KK) locked to fold 100 % against the all-in: OOP shoves everything (also QQ).
    let spot = river_spot();
    let params = SolveParams {
        target_pct: 0.01,
        max_iterations: 1000,
        ..SolveParams::default()
    };
    let lock = Lock {
        path: vec![1],
        strategy: vec![1.0, 0.0],
    }; // [fold, call] for KsKh
    let (mut game, _) = solve_now(&spot, &params, &[lock]).unwrap();
    let v = view(&mut game, &[]).unwrap();
    let qq = v.players[0]
        .hands
        .iter()
        .find(|h| h.cards == "QsQh")
        .unwrap();
    assert!(qq.strategy[1] > 0.99, "{:?}", qq.strategy);
    let v1 = view(&mut game, &[1]).unwrap();
    assert!(v1.locked);
    // Without the lock, QQ only bluffs part of the time (KK calls at equilibrium).
    let (mut free, _) = solve_now(&spot, &params, &[]).unwrap();
    let v = view(&mut free, &[]).unwrap();
    let qq = v.players[0]
        .hands
        .iter()
        .find(|h| h.cards == "QsQh")
        .unwrap();
    assert!(qq.strategy[1] < 0.5, "{:?}", qq.strategy);
}

#[test]
fn background_job_pause_resume_cancel() {
    let job = Job::start(
        turn_spot(),
        SolveParams {
            target_pct: 0.0,
            max_iterations: 100_000,
            ..SolveParams::default()
        },
        vec![],
    );
    let wait = |f: &dyn Fn() -> bool| {
        let t = Instant::now();
        while !f() && t.elapsed() < Duration::from_secs(30) {
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    wait(&|| job.progress().iteration >= 3);
    job.pause();
    wait(&|| job.progress().state == State::Paused);
    let it = job.progress().iteration;
    std::thread::sleep(Duration::from_millis(300));
    assert!(job.progress().iteration <= it + 1);
    job.resume();
    wait(&|| job.progress().iteration > it + 2);
    job.cancel();
    wait(&|| job.progress().state == State::Cancelled);
    assert_eq!(job.progress().state, State::Cancelled);
    assert!(job.take_game().is_none());
}

#[test]
fn memory_cap_refuses_to_start() {
    let job = Job::start(
        turn_spot(),
        SolveParams {
            memory_cap_mb: 0,
            ..SolveParams::default()
        },
        vec![],
    );
    let t = Instant::now();
    while job.progress().state == State::Building && t.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(20));
    }
    let p = job.progress();
    assert_eq!(p.state, State::Failed);
    assert!(p.error.unwrap().contains("MB"));
}

#[test]
fn cache_round_trip_is_instant() {
    let dir = tempfile::tempdir().unwrap();
    let spot = turn_spot();
    let (mut game, e) = solve_now(
        &spot,
        &SolveParams {
            target_pct: 1.0,
            max_iterations: 300,
            ..SolveParams::default()
        },
        &[],
    )
    .unwrap();
    let before = view(&mut game, &[0]).unwrap();
    let entry = cache::save(dir.path(), "test", &spot, &[], &game, e, 300, 1).unwrap();
    assert_eq!(cache::list(dir.path()).len(), 1);
    let t = Instant::now();
    let (mut loaded, s2, locks) = cache::load(dir.path(), &entry.hash).unwrap();
    assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
    assert_eq!(s2, spot);
    assert!(locks.is_empty());
    let after = view(&mut loaded, &[0]).unwrap();
    assert_eq!(before.totals, after.totals);
    assert_eq!(
        before.players[1].hands[0].strategy,
        after.players[1].hands[0].strategy
    );
    cache::delete(dir.path(), &entry.hash).unwrap();
    assert!(cache::list(dir.path()).is_empty());
}

#[test]
fn bad_spots_are_rejected() {
    let mut s = turn_spot();
    s.board = "AsAs7d".into();
    assert!(s.build().is_err());
    let mut s = turn_spot();
    s.oop_range = "XYZ".into();
    assert!(s.build().is_err());
    let mut s = turn_spot();
    s.tree.flop.oop.bet = "abc".into();
    assert!(s.build().is_err());
    assert_ne!(turn_spot().hash(), river_spot().hash());
    assert_eq!(turn_spot().hash(), turn_spot().hash());
}
