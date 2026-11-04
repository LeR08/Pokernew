//! The simplified preflop game tree.
//!
//! Players are listed in preflop order: `[first to act, …, SB, BB]`
//! (heads-up: `[SB, BB]`). Everyone posts the ante and the blinds, then:
//! * **open**: each player in turn, while everybody before has folded, folds,
//!   raises to `open` bb, moves all-in (if allowed), or — the small blind
//!   only — limps (if allowed);
//! * **responses**: the players behind the raiser answer in turn: fold, call,
//!   3-bet (× `three_bet`) or all-in. The **first** one who does not fold is
//!   the only opponent left (the others fold): this is the multi-player
//!   approximation; heads-up it is exact;
//! * then the raiser faces the 3-bet: fold, call, 4-bet (× `four_bet`) or all-in;
//!   the 3-bettor faces the 4-bet: fold, call or all-in; facing an all-in: fold
//!   or call;
//! * after a limp the big blind checks, raises (to `open`) or moves all-in.
//!
//! A raise that would put in at least the effective stack becomes the all-in.
//! Ends: everybody folds, all-in and call (showdown), or a call that is not
//! all-in (the flop is seen: each player realises a share of their equity).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// Stacks in bb, preflop order (SB and BB last).
    pub stacks: Vec<f64>,
    pub sb: f64,
    pub bb: f64,
    /// Ante per player, bb.
    pub ante: f64,
    /// Raise-to size of the first raise, bb.
    pub open: f64,
    /// 3-bet size, times the raise it faces.
    pub three_bet: f64,
    /// 4-bet size, times the 3-bet.
    pub four_bet: f64,
    pub allow_limp: bool,
    /// All-in available at every decision (otherwise only when a raise reaches the stack).
    pub allow_allin: bool,
    /// Share of their equity a player realises when the flop is seen (in / out of position).
    pub realization_ip: f64,
    pub realization_oop: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            stacks: vec![100.0, 100.0],
            sb: 0.5,
            bb: 1.0,
            ante: 0.0,
            open: 2.5,
            three_bet: 3.0,
            four_bet: 2.2,
            allow_limp: true,
            allow_allin: true,
            realization_ip: 1.0,
            realization_oop: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Fold,
    Check,
    Call,
    Limp,
    Raise,
    AllIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Act {
    pub kind: Kind,
    /// Total put in by the actor this hand after the action (blind included, ante excluded), bb.
    pub to: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum End {
    /// Everybody else folded.
    Fold { winner: usize },
    /// Two players all-in (or one all-in and called).
    Showdown { p: usize, q: usize },
    /// The flop is seen by two players.
    Flop { p: usize, q: usize },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub player: Option<usize>,
    pub actions: Vec<Act>,
    /// Child node of each action.
    pub children: Vec<usize>,
    pub end: Option<End>,
    /// Chips put in by each player (blinds and bets), bb; antes in `Tree::antes`.
    pub bets: Vec<f64>,
    /// Players still in the hand.
    pub active: Vec<usize>,
}

pub struct Tree {
    pub config: Config,
    pub nodes: Vec<Node>,
    pub antes: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GameError {
    #[error("between 2 and 6 players are needed")]
    Players,
    #[error("stacks, blinds and sizes must be positive, the open bigger than the big blind")]
    Amounts,
}

#[derive(Clone)]
struct St {
    bets: Vec<f64>,
    active: Vec<usize>,
}

impl Tree {
    pub fn build(config: &Config) -> Result<Tree, GameError> {
        let n = config.stacks.len();
        if !(2..=6).contains(&n) {
            return Err(GameError::Players);
        }
        let ok = config
            .stacks
            .iter()
            .all(|&s| s.is_finite() && s > config.bb + config.ante)
            && config.sb > 0.0
            && config.bb >= config.sb
            && config.ante >= 0.0
            && config.open > config.bb
            && config.three_bet > 1.0
            && config.four_bet > 1.0
            && (0.0..=2.0).contains(&config.realization_ip)
            && (0.0..=2.0).contains(&config.realization_oop);
        if !ok {
            return Err(GameError::Amounts);
        }
        let antes = vec![config.ante; n];
        let mut bets = vec![0.0; n];
        bets[n - 1] = config.bb;
        bets[n - 2] = config.sb;
        let mut t = Tree {
            config: config.clone(),
            nodes: vec![],
            antes,
        };
        let st = St {
            bets,
            active: (0..n).collect(),
        };
        t.open(st, 0);
        Ok(t)
    }

    /// Stack left behind the chips already in, for player i (antes paid).
    fn behind(&self, i: usize, st: &St) -> f64 {
        self.config.stacks[i] - self.antes[i] - st.bets[i]
    }

    fn cap(&self, i: usize) -> f64 {
        self.config.stacks[i] - self.antes[i]
    }

    fn push(&mut self, node: Node) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn end(&mut self, end: End, st: &St) -> usize {
        self.push(Node {
            player: None,
            actions: vec![],
            children: vec![],
            end: Some(end),
            bets: st.bets.clone(),
            active: st.active.clone(),
        })
    }

    fn decision(
        &mut self,
        player: usize,
        st: &St,
        options: Vec<(Act, Box<dyn FnOnce(&mut Tree) -> usize + '_>)>,
    ) -> usize {
        let id = self.push(Node {
            player: Some(player),
            actions: vec![],
            children: vec![],
            end: None,
            bets: st.bets.clone(),
            active: st.active.clone(),
        });
        let mut actions = vec![];
        let mut children = vec![];
        for (a, build) in options {
            actions.push(a);
            children.push(build(self));
        }
        self.nodes[id].actions = actions;
        self.nodes[id].children = children;
        id
    }

    /// Player k to open, everybody before folded.
    fn open(&mut self, st: St, k: usize) -> usize {
        let n = self.config.stacks.len();
        if k == n - 1 {
            return self.end(End::Fold { winner: k }, &st);
        }
        let c = self.config.clone();
        let mut opts: Vec<(Act, Box<dyn FnOnce(&mut Tree) -> usize + '_>)> = vec![];
        let mut fold = st.clone();
        fold.active.retain(|&x| x != k);
        opts.push((
            Act {
                kind: Kind::Fold,
                to: st.bets[k],
            },
            Box::new(move |t: &mut Tree| t.open(fold, k + 1)),
        ));
        // effective stack against the deepest remaining player
        let deepest = st
            .active
            .iter()
            .filter(|&&x| x > k)
            .map(|&x| self.cap(x))
            .fold(0.0, f64::max);
        let eff = self.cap(k).min(deepest);
        if k == n - 2 && c.allow_limp {
            let mut s = st.clone();
            s.bets[k] = c.bb;
            s.active = vec![n - 2, n - 1];
            opts.push((
                Act {
                    kind: Kind::Limp,
                    to: c.bb,
                },
                Box::new(move |t: &mut Tree| t.after_limp(s)),
            ));
        }
        if c.open < eff {
            let mut s = st.clone();
            s.bets[k] = c.open;
            opts.push((
                Act {
                    kind: Kind::Raise,
                    to: c.open,
                },
                Box::new(move |t: &mut Tree| t.respond(s, k, k + 1)),
            ));
        }
        if c.allow_allin || c.open >= eff {
            let mut s = st.clone();
            s.bets[k] = self.cap(k);
            let to = s.bets[k];
            opts.push((
                Act {
                    kind: Kind::AllIn,
                    to,
                },
                Box::new(move |t: &mut Tree| t.respond(s, k, k + 1)),
            ));
        }
        self.decision(k, &st, opts)
    }

    /// Player q answers p's raise (players between p and q folded).
    fn respond(&mut self, st: St, p: usize, q: usize) -> usize {
        let n = self.config.stacks.len();
        if q == n {
            let mut s = st.clone();
            s.active = vec![p];
            return self.end(End::Fold { winner: p }, &s);
        }
        let c = self.config.clone();
        let level = st.bets[p];
        let p_allin = (level - self.cap(p)).abs() < 1e-9;
        let eff = self.cap(p).min(self.cap(q));
        let mut opts: Vec<(Act, Box<dyn FnOnce(&mut Tree) -> usize + '_>)> = vec![];
        let mut fold = st.clone();
        fold.active.retain(|&x| x != q);
        opts.push((
            Act {
                kind: Kind::Fold,
                to: st.bets[q],
            },
            Box::new(move |t: &mut Tree| t.respond(fold, p, q + 1)),
        ));
        let mut call = st.clone();
        call.active = vec![p, q];
        call.bets[q] = level.min(self.cap(q));
        let call_to = call.bets[q];
        let allin_call = p_allin || call_to >= self.cap(q) - 1e-9;
        opts.push((
            Act {
                kind: Kind::Call,
                to: call_to,
            },
            Box::new(move |t: &mut Tree| t.call(call, p, q, allin_call)),
        ));
        if !p_allin && eff > level {
            let size = level * c.three_bet;
            if size < eff {
                let mut s = st.clone();
                s.active = vec![p, q];
                s.bets[q] = size;
                opts.push((
                    Act {
                        kind: Kind::Raise,
                        to: size,
                    },
                    Box::new(move |t: &mut Tree| t.face_three_bet(s, p, q)),
                ));
            }
            if c.allow_allin || size >= eff {
                let mut s = st.clone();
                s.active = vec![p, q];
                s.bets[q] = self.cap(q);
                let to = s.bets[q];
                opts.push((
                    Act {
                        kind: Kind::AllIn,
                        to,
                    },
                    Box::new(move |t: &mut Tree| t.face_allin(s, p, q)),
                ));
            }
        }
        self.decision(q, &st, opts)
    }

    /// A call: showdown when someone is all-in, otherwise the flop.
    fn call(&mut self, mut st: St, a: usize, b: usize, allin: bool) -> usize {
        // equalise: the bigger bet gets back what the other could not match
        let m = st.bets[a].min(st.bets[b]);
        st.bets[a] = m;
        st.bets[b] = m;
        if allin {
            self.end(End::Showdown { p: a, q: b }, &st)
        } else {
            self.end(End::Flop { p: a, q: b }, &st)
        }
    }

    /// `victim` faces an all-in by `jammer`: fold or call.
    fn face_allin(&mut self, st: St, victim: usize, jammer: usize) -> usize {
        let mut fold = st.clone();
        fold.active = vec![jammer];
        let mut call = st.clone();
        call.bets[victim] = st.bets[jammer].min(self.cap(victim));
        let to = call.bets[victim];
        self.decision(
            victim,
            &st,
            vec![
                (
                    Act {
                        kind: Kind::Fold,
                        to: st.bets[victim],
                    },
                    Box::new(move |t: &mut Tree| t.end(End::Fold { winner: jammer }, &fold)),
                ),
                (
                    Act {
                        kind: Kind::Call,
                        to,
                    },
                    Box::new(move |t: &mut Tree| t.call(call, victim, jammer, true)),
                ),
            ],
        )
    }

    /// Raiser p faces q's 3-bet.
    fn face_three_bet(&mut self, st: St, p: usize, q: usize) -> usize {
        let c = self.config.clone();
        let level = st.bets[q];
        let eff = self.cap(p).min(self.cap(q));
        let mut opts: Vec<(Act, Box<dyn FnOnce(&mut Tree) -> usize + '_>)> = vec![];
        let mut fold = st.clone();
        fold.active = vec![q];
        opts.push((
            Act {
                kind: Kind::Fold,
                to: st.bets[p],
            },
            Box::new(move |t: &mut Tree| t.end(End::Fold { winner: q }, &fold)),
        ));
        let mut call = st.clone();
        call.bets[p] = level;
        opts.push((
            Act {
                kind: Kind::Call,
                to: level,
            },
            Box::new(move |t: &mut Tree| t.call(call, p, q, false)),
        ));
        let size = level * c.four_bet;
        if size < eff {
            let mut s = st.clone();
            s.bets[p] = size;
            opts.push((
                Act {
                    kind: Kind::Raise,
                    to: size,
                },
                Box::new(move |t: &mut Tree| t.face_four_bet(s, q, p)),
            ));
        }
        if c.allow_allin || size >= eff {
            let mut s = st.clone();
            s.bets[p] = self.cap(p);
            let to = s.bets[p];
            opts.push((
                Act {
                    kind: Kind::AllIn,
                    to,
                },
                Box::new(move |t: &mut Tree| t.face_allin(s, q, p)),
            ));
        }
        self.decision(p, &st, opts)
    }

    /// 3-bettor q faces p's 4-bet: fold, call, all-in.
    fn face_four_bet(&mut self, st: St, q: usize, p: usize) -> usize {
        let level = st.bets[p];
        let mut fold = st.clone();
        fold.active = vec![p];
        let mut call = st.clone();
        call.bets[q] = level;
        let mut jam = st.clone();
        jam.bets[q] = self.cap(q);
        let to = jam.bets[q];
        self.decision(
            q,
            &st,
            vec![
                (
                    Act {
                        kind: Kind::Fold,
                        to: st.bets[q],
                    },
                    Box::new(move |t: &mut Tree| t.end(End::Fold { winner: p }, &fold)),
                ),
                (
                    Act {
                        kind: Kind::Call,
                        to: level,
                    },
                    Box::new(move |t: &mut Tree| t.call(call, q, p, false)),
                ),
                (
                    Act {
                        kind: Kind::AllIn,
                        to,
                    },
                    Box::new(move |t: &mut Tree| t.face_allin(jam, p, q)),
                ),
            ],
        )
    }

    /// The SB limped: the BB checks, raises or moves all-in.
    fn after_limp(&mut self, st: St) -> usize {
        let n = self.config.stacks.len();
        let (sb, bb) = (n - 2, n - 1);
        let c = self.config.clone();
        let eff = self.cap(sb).min(self.cap(bb));
        let mut opts: Vec<(Act, Box<dyn FnOnce(&mut Tree) -> usize + '_>)> = vec![];
        let check = st.clone();
        opts.push((
            Act {
                kind: Kind::Check,
                to: st.bets[bb],
            },
            Box::new(move |t: &mut Tree| t.call(check, sb, bb, false)),
        ));
        if c.open < eff {
            let mut s = st.clone();
            s.bets[bb] = c.open;
            opts.push((
                Act {
                    kind: Kind::Raise,
                    to: c.open,
                },
                Box::new(move |t: &mut Tree| t.limper_faces_raise(s, sb, bb)),
            ));
        }
        if c.allow_allin || c.open >= eff {
            let mut s = st.clone();
            s.bets[bb] = self.cap(bb);
            let to = s.bets[bb];
            opts.push((
                Act {
                    kind: Kind::AllIn,
                    to,
                },
                Box::new(move |t: &mut Tree| t.face_allin(s, sb, bb)),
            ));
        }
        self.decision(bb, &st, opts)
    }

    fn limper_faces_raise(&mut self, st: St, sb: usize, bb: usize) -> usize {
        let level = st.bets[bb];
        let mut fold = st.clone();
        fold.active = vec![bb];
        let mut call = st.clone();
        call.bets[sb] = level;
        let mut jam = st.clone();
        jam.bets[sb] = self.cap(sb);
        let to = jam.bets[sb];
        let mut opts: Vec<(Act, Box<dyn FnOnce(&mut Tree) -> usize + '_>)> = vec![
            (
                Act {
                    kind: Kind::Fold,
                    to: st.bets[sb],
                },
                Box::new(move |t: &mut Tree| t.end(End::Fold { winner: bb }, &fold)),
            ),
            (
                Act {
                    kind: Kind::Call,
                    to: level,
                },
                Box::new(move |t: &mut Tree| t.call(call, sb, bb, false)),
            ),
        ];
        if self.config.allow_allin {
            opts.push((
                Act {
                    kind: Kind::AllIn,
                    to,
                },
                Box::new(move |t: &mut Tree| t.face_allin(jam, bb, sb)),
            ));
        }
        self.decision(sb, &st, opts)
    }

    /// Chips each player has put in at a node (antes included).
    pub fn invested(&self, node: usize, i: usize) -> f64 {
        self.antes[i] + self.nodes[node].bets[i]
    }

    pub fn pot(&self, node: usize) -> f64 {
        (0..self.config.stacks.len())
            .map(|i| self.invested(node, i))
            .sum()
    }

    /// Postflop order: SB, BB, then the others in preflop order. True when `a` acts after `b` postflop.
    pub fn in_position(&self, a: usize, b: usize) -> bool {
        let n = self.config.stacks.len();
        let rank = |i: usize| {
            if i == n - 2 {
                0
            } else if i == n - 1 {
                1
            } else {
                2 + i
            }
        };
        rank(a) > rank(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_up_tree() {
        let t = Tree::build(&Config::default()).unwrap();
        let root = &t.nodes[0];
        assert_eq!(root.player, Some(0));
        let kinds: Vec<Kind> = root.actions.iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            vec![Kind::Fold, Kind::Limp, Kind::Raise, Kind::AllIn]
        );
        // every node ends with chips that make sense
        for (i, n) in t.nodes.iter().enumerate() {
            assert!(t.pot(i) > 0.0);
            assert!(n
                .bets
                .iter()
                .zip(&t.config.stacks)
                .all(|(b, s)| *b <= *s + 1e-9));
            if n.end.is_none() {
                assert_eq!(n.actions.len(), n.children.len());
            }
        }
    }

    #[test]
    fn jam_only_is_push_fold() {
        let c = Config {
            stacks: vec![10.0, 10.0],
            open: 20.0,
            allow_limp: false,
            ..Config::default()
        };
        let t = Tree::build(&c).unwrap();
        let kinds: Vec<Kind> = t.nodes[0].actions.iter().map(|a| a.kind).collect();
        assert_eq!(kinds, vec![Kind::Fold, Kind::AllIn]);
        let bb = &t.nodes[t.nodes[0].children[1]];
        let kinds: Vec<Kind> = bb.actions.iter().map(|a| a.kind).collect();
        assert_eq!(kinds, vec![Kind::Fold, Kind::Call]);
    }

    #[test]
    fn six_max_tree() {
        let t = Tree::build(&Config {
            stacks: vec![100.0; 6],
            allow_limp: false,
            ..Config::default()
        })
        .unwrap();
        assert!(t.nodes.len() > 50);
        assert!(t.in_position(3, 4)); // BTN acts after SB postflop
        assert!(!t.in_position(4, 0)); // SB is out of position against UTG
        assert!(Tree::build(&Config {
            stacks: vec![100.0],
            ..Config::default()
        })
        .is_err());
    }
}
