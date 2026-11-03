//! A solve running in the background: progress, pause/resume, cancel, node locks.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use postflop_solver::{compute_exploitability, finalize, solve_step, PostFlopGame};
use serde::{Deserialize, Serialize};

use crate::spot::{SpotConfig, SpotError};

/// A fixed strategy at one node (node locking): the path of action/card
/// indices from the root and, for every action, one frequency per hand of the
/// player to act (`strategy[action * hands + hand]`; a hand whose frequencies
/// are all 0 is not locked).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Lock {
    pub path: Vec<usize>,
    pub strategy: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SolveParams {
    /// Stop when exploitability ≤ this % of the pot.
    pub target_pct: f64,
    pub max_iterations: u32,
    /// 0 = every core.
    pub threads: usize,
    /// 16-bit storage: half the memory, slightly less precise.
    pub compression: bool,
    /// Refuse to start above this many megabytes.
    pub memory_cap_mb: u64,
}

impl Default for SolveParams {
    fn default() -> Self {
        SolveParams {
            target_pct: 0.5,
            max_iterations: 1000,
            threads: 0,
            compression: false,
            memory_cap_mb: 8192,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Building,
    Solving,
    Paused,
    Done,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub state: State,
    pub iteration: u32,
    pub max_iterations: u32,
    /// Latest exploitability, % of the pot.
    pub exploitability_pct: f64,
    /// (iteration, exploitability %, seconds)
    pub history: Vec<(u32, f64, f64)>,
    pub elapsed_ms: u64,
    pub memory_mb: f64,
    pub error: Option<String>,
}

const RUN: u8 = 0;
const PAUSE: u8 = 1;
const CANCEL: u8 = 2;

pub struct Job {
    pub spot: SpotConfig,
    pub params: SolveParams,
    pub locks: Vec<Lock>,
    progress: Mutex<Progress>,
    control: AtomicU8,
    game: Mutex<Option<PostFlopGame>>,
}

impl Job {
    /// Starts solving on a new thread.
    pub fn start(spot: SpotConfig, params: SolveParams, locks: Vec<Lock>) -> Arc<Job> {
        let job = Arc::new(Job {
            progress: Mutex::new(Progress {
                state: State::Building,
                iteration: 0,
                max_iterations: params.max_iterations,
                exploitability_pct: f64::NAN,
                history: vec![],
                elapsed_ms: 0,
                memory_mb: 0.0,
                error: None,
            }),
            spot,
            params,
            locks,
            control: AtomicU8::new(RUN),
            game: Mutex::new(None),
        });
        let j = job.clone();
        std::thread::Builder::new()
            .name("lps-solver".into())
            .spawn(move || {
                if let Err(e) = j.run() {
                    let mut p = j.progress.lock().unwrap();
                    p.state = State::Failed;
                    p.error = Some(e.to_string());
                }
            })
            .expect("solver thread");
        job
    }

    pub fn progress(&self) -> Progress {
        self.progress.lock().unwrap().clone()
    }

    pub fn pause(&self) {
        let _ = self
            .control
            .compare_exchange(RUN, PAUSE, Ordering::SeqCst, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        let _ = self
            .control
            .compare_exchange(PAUSE, RUN, Ordering::SeqCst, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.control.store(CANCEL, Ordering::SeqCst);
    }

    /// The solved game, once done (moved out of the job).
    pub fn take_game(&self) -> Option<PostFlopGame> {
        if self.progress().state != State::Done {
            return None;
        }
        self.game.lock().unwrap().take()
    }

    fn set(&self, f: impl FnOnce(&mut Progress)) {
        f(&mut self.progress.lock().unwrap());
    }

    fn run(&self) -> Result<(), SpotError> {
        let start = Instant::now();
        let mut game = self.spot.build()?;
        let (mem, mem_c) = game.memory_usage();
        let need = if self.params.compression { mem_c } else { mem };
        let mb = need as f64 / (1024.0 * 1024.0);
        self.set(|p| p.memory_mb = mb);
        if mb > self.params.memory_cap_mb as f64 {
            return Err(SpotError::Engine(format!(
                "this spot needs {mb:.0} MB, above the {} MB cap (enable compression or simplify the tree)",
                self.params.memory_cap_mb
            )));
        }
        game.allocate_memory(self.params.compression);
        apply_locks(&mut game, &self.locks)?;
        let pot = self.spot.pot * crate::spot::SCALE;
        let threads = if self.params.threads == 0 {
            rayon::current_num_threads()
        } else {
            self.params.threads
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|e| SpotError::Engine(e.to_string()))?;
        self.set(|p| p.state = State::Solving);
        let mut cancelled = false;
        pool.install(|| {
            let mut e = compute_exploitability(&game) as f64 / pot * 100.0;
            self.set(|p| {
                p.exploitability_pct = e;
                p.history.push((0, e, 0.0));
            });
            for t in 0..self.params.max_iterations {
                if e <= self.params.target_pct {
                    break;
                }
                loop {
                    match self.control.load(Ordering::SeqCst) {
                        PAUSE => {
                            self.set(|p| p.state = State::Paused);
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        CANCEL => {
                            cancelled = true;
                            return;
                        }
                        _ => break,
                    }
                }
                self.set(|p| p.state = State::Solving);
                solve_step(&game, t);
                let last = t + 1 == self.params.max_iterations;
                if (t + 1) % 10 == 0 || last {
                    e = compute_exploitability(&game) as f64 / pot * 100.0;
                    let secs = start.elapsed().as_secs_f64();
                    self.set(|p| {
                        p.exploitability_pct = e;
                        p.history.push((t + 1, e, secs));
                    });
                }
                self.set(|p| {
                    p.iteration = t + 1;
                    p.elapsed_ms = start.elapsed().as_millis() as u64;
                });
            }
        });
        if cancelled {
            self.set(|p| p.state = State::Cancelled);
            return Ok(());
        }
        finalize(&mut game);
        *self.game.lock().unwrap() = Some(game);
        self.set(|p| {
            p.state = State::Done;
            p.elapsed_ms = start.elapsed().as_millis() as u64;
        });
        Ok(())
    }
}

/// Applies node locks to a game whose memory is allocated (before solving).
pub fn apply_locks(game: &mut PostFlopGame, locks: &[Lock]) -> Result<(), SpotError> {
    for l in locks {
        game.back_to_root();
        for &a in &l.path {
            if game.is_terminal_node() {
                return Err(SpotError::Engine(
                    "lock path goes past a terminal node".into(),
                ));
            }
            game.play(a);
        }
        if game.is_terminal_node() || game.is_chance_node() {
            return Err(SpotError::Engine(
                "a lock must be on a decision node".into(),
            ));
        }
        let n = game.available_actions().len() * game.private_cards(game.current_player()).len();
        if l.strategy.len() != n {
            return Err(SpotError::Engine(format!(
                "lock strategy needs {n} values, got {}",
                l.strategy.len()
            )));
        }
        game.lock_current_strategy(&l.strategy);
    }
    game.back_to_root();
    Ok(())
}

/// Solves to completion on the current thread (tests, batch).
pub fn solve_now(
    spot: &SpotConfig,
    params: &SolveParams,
    locks: &[Lock],
) -> Result<(PostFlopGame, f64), SpotError> {
    let mut game = spot.build()?;
    game.allocate_memory(params.compression);
    apply_locks(&mut game, locks)?;
    let pot = spot.pot * crate::spot::SCALE;
    let target = (params.target_pct / 100.0 * pot) as f32;
    let e = postflop_solver::solve(&mut game, params.max_iterations, target, false);
    Ok((game, e as f64 / pot * 100.0))
}
