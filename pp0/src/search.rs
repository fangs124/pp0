use core::time;
use std::{
    num::NonZero,
    sync::Arc,
    time::{Duration, Instant},
};

use chessbb::{ChessBoardSnapshot, ChessGame, ChessMove, GameResult, GameState, MoveList};
use nnue::{SparseInputType, SparseVec};

use crate::{evaluator::Evaluator, transposition::TranspositionTable};

pub const LOSE_SCORE: i16 = (i16::MIN + 2) / 2;
pub const WIN_SCORE: i16 = -LOSE_SCORE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchLimit {
    Depth(NonZero<usize>),
    Nodes(NodeLimit),
    Time(TimeLimit),
}
//let mut hard_time_limit: Duration = match chess_game.side() {
//    Side::White => (wtime / HARD_BASE_COEFF) + (winc / HARD_INCREMENT_COEFF),
//    Side::Black => (btime / HARD_BASE_COEFF) + (binc / HARD_INCREMENT_COEFF),
//};
//hard_time_limit = hard_time_limit.checked_sub(MILLIS_MARGIN).unwrap_or(hard_time_limit);//
//let mut soft_time_limit: Duration = match chess_game.side() {
//    Side::White => (wtime / SOFT_BASE_COEFF) + (winc / SOFT_INCREMENT_COEFF),
//    Side::Black => (btime / SOFT_BASE_COEFF) + (binc / SOFT_INCREMENT_COEFF),
//};
//const HARD_BASE_COEFF: u32 = 10;
//const HARD_INCREMENT_COEFF: u32 = 2;
//const SOFT_BASE_COEFF: u32 = 20;
//const SOFT_INCREMENT_COEFF: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeLimit {
    soft_limit: Duration,
    hard_limit: Duration,
}

impl TimeLimit {
    pub fn new(soft_limit: Duration, hard_limit: Duration) -> TimeLimit {
        TimeLimit { soft_limit, hard_limit }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeLimit {
    soft_limit: NonZero<usize>,
    hard_limit: NonZero<usize>,
}

impl NodeLimit {
    pub fn new(soft_limit: usize, hard_limit: usize) -> NodeLimit {
        let soft_limit = NonZero::new(soft_limit).expect("node soft_limit cannot be zero!");
        let hard_limit = NonZero::new(hard_limit).expect("node soft_limit cannot be zero!");
        NodeLimit { soft_limit, hard_limit }
    }
}

impl SearchLimit {
    pub fn depth(depth: usize) -> SearchLimit {
        SearchLimit::Depth(NonZero::new(depth).expect("depth cannot be zero!"))
    }

    pub fn node(soft_limit: usize, hard_limit: usize) -> SearchLimit {
        SearchLimit::Nodes(NodeLimit::new(soft_limit, hard_limit))
    }

    pub fn time(soft_limit: Duration, hard_limit: Duration) -> SearchLimit {
        SearchLimit::Time(TimeLimit::new(soft_limit, hard_limit))
    }
}

pub struct SearchData {
    ply: u16,
    max_depth: usize,
    node_count: usize,
    collect_pairs: bool,
    pairs: Option<((SparseVec, SparseVec), i16)>,
    is_aborted: bool,
}

type TT = TranspositionTable;

const NODE_COUNT_CHECK_LIMIT: usize = 1024;

impl SearchData {
    pub const fn new() -> SearchData {
        SearchData { ply: 0, max_depth: 0, node_count: 0, collect_pairs: false, pairs: None, is_aborted: false }
    }

    pub const fn new_collect_pairs() -> SearchData {
        SearchData { ply: 0, max_depth: 0, node_count: 0, collect_pairs: true, pairs: None, is_aborted: false }
    }

    pub const fn ply(&self) -> u16 {
        self.ply
    }

    pub const fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn pairs(self) -> Option<((SparseVec, SparseVec), i16)> {
        self.pairs
    }

    pub fn find_move(&mut self, chessgame: &mut ChessGame, ev: &mut impl Evaluator, tt: Arc<TT>, limit: &SearchLimit, moves: MoveList) -> ChessMove {
        match limit {
            SearchLimit::Depth(depth) => self.search(chessgame, depth, ev, tt, moves),
            SearchLimit::Nodes(node_limit) => self.search_node_limited(chessgame, node_limit, ev, tt, moves),
            SearchLimit::Time(time_limit) => self.search_time_limited(chessgame, time_limit, ev, tt, moves),
        }
    }

    fn search_node_limited(&mut self, chessgame: &mut ChessGame, node_limit: &NodeLimit, ev: &mut impl Evaluator, tt: Arc<TT>, moves: MoveList) -> ChessMove {
        assert!(!moves.is_empty());
        if moves.len() == 1 {
            if self.collect_pairs {
                let input = match chessgame.side() {
                    chessbb::Side::White => (chessgame.to_sparse_vec_white(), chessgame.to_sparse_vec_black()),
                    chessbb::Side::Black => (chessgame.to_sparse_vec_black(), chessgame.to_sparse_vec_white()),
                };
                self.pairs = Some((input, ev.eval(&chessgame)));
            }
            return moves[0];
        }

        let mut best_move: ChessMove = moves[0].clone();
        let mut best_d: usize = 0;
        let mut best_eval: i16 = i16::MIN + 1;
        //let hard_node_limit: NonZero<usize> = NonZero::new(node_limit.get() * 10).unwrap();

        let mut d: usize = 1;
        let max_d: usize = u8::MAX as usize;
        'search: while self.node_count <= node_limit.soft_limit.get() && d < max_d {
            best_eval = i16::MIN + 1;
            //search previous best_move
            ev.update(&chessgame, &best_move);
            let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&best_move);
            let eval: i16 = -self.negamax::<false, true>(chessgame, best_eval, i16::MAX - 1, d, ev, tt.clone(), None, Some(node_limit.hard_limit));
            chessgame.restore_state(snapshot);
            ev.revert(&chessgame, &best_move);
            if self.is_aborted {
                break 'search;
            }

            if eval > best_eval || d > best_d {
                best_eval = eval;
                best_d = d;
            }

            if self.node_count > node_limit.soft_limit.get() {
                break 'search;
            }

            for &chessmove in &moves {
                if chessmove == best_move {
                    continue;
                }

                ev.update(&chessgame, &chessmove);
                let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&chessmove);
                let eval: i16 = -self.negamax::<false, true>(chessgame, best_eval, i16::MAX - 1, d, ev, tt.clone(), None, Some(node_limit.hard_limit));
                chessgame.restore_state(snapshot);
                ev.revert(&chessgame, &chessmove);
                if self.is_aborted {
                    break 'search;
                }
                //let mating_ply: i16 = ((eval_data.signum() * WIN_SCORE - eval_data) / 2) + 1;
                //if mating_ply.abs() < 32 && eval_data != 0 {
                //}
                if eval > best_eval || d > best_d {
                    best_eval = eval;
                    best_move = chessmove.clone();
                    best_d = d;
                }

                if self.node_count > node_limit.soft_limit.get() {
                    break 'search;
                }
            }

            d += 1;
        }

        if self.collect_pairs {
            let input = match chessgame.side() {
                chessbb::Side::White => (chessgame.to_sparse_vec_white(), chessgame.to_sparse_vec_black()),
                chessbb::Side::Black => (chessgame.to_sparse_vec_black(), chessgame.to_sparse_vec_white()),
            };
            self.pairs = Some((input, best_eval));
        }
        best_move
    }

    fn search_time_limited(&mut self, chessgame: &mut ChessGame, time_limit: &TimeLimit, ev: &mut impl Evaluator, tt: Arc<TT>, moves: MoveList) -> ChessMove {
        assert!(!moves.is_empty());
        if moves.len() == 1 {
            if self.collect_pairs {
                let input = match chessgame.side() {
                    chessbb::Side::White => (chessgame.to_sparse_vec_white(), chessgame.to_sparse_vec_black()),
                    chessbb::Side::Black => (chessgame.to_sparse_vec_black(), chessgame.to_sparse_vec_white()),
                };
                self.pairs = Some((input, ev.eval(&chessgame)));
            }
            return moves[0];
        }

        let start = Instant::now();
        let mut best_move: ChessMove = moves[0].clone();
        let mut best_d: usize = 0;
        let mut best_eval: i16 = i16::MIN + 1;

        let mut d: usize = 1;
        let max_d: usize = u8::MAX as usize;
        let soft_time_limit = time_limit.soft_limit;
        'search: while start.elapsed() < soft_time_limit && d < max_d {
            best_eval = i16::MIN + 1;
            //search previous best_move
            ev.update(&chessgame, &best_move);
            let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&best_move);
            let eval: i16 = -self.negamax::<true, false>(chessgame, best_eval, i16::MAX - 1, d, ev, tt.clone(), Some((start, time_limit.hard_limit)), None);
            chessgame.restore_state(snapshot);
            ev.revert(&chessgame, &best_move);

            if self.is_aborted {
                break 'search;
            }

            if eval > best_eval || d > best_d {
                best_eval = eval;
                best_d = d;
            }

            if start.elapsed() >= soft_time_limit {
                break 'search;
            }

            for &chessmove in &moves {
                if chessmove == best_move {
                    continue;
                }

                ev.update(&chessgame, &chessmove);
                let snapshot: chessbb::ChessBoardSnapshot = chessgame.explore_state(&chessmove);
                let eval: i16 = -self.negamax::<true, false>(chessgame, best_eval, i16::MAX - 1, d, ev, tt.clone(), Some((start, time_limit.hard_limit)), None);
                chessgame.restore_state(snapshot);
                ev.revert(&chessgame, &chessmove);

                if self.is_aborted {
                    break 'search;
                }
                //let mating_ply: i16 = ((eval_data.signum() * WIN_SCORE - eval_data) / 2) + 1;
                //if mating_ply.abs() < 32 && eval_data != 0 {
                //}
                if eval > best_eval || d > best_d {
                    best_eval = eval;
                    best_move = chessmove.clone();
                    best_d = d;
                }

                if start.elapsed() >= soft_time_limit {
                    break 'search;
                }
            }

            d += 1;
        }

        if self.collect_pairs {
            let input = match chessgame.side() {
                chessbb::Side::White => (chessgame.to_sparse_vec_white(), chessgame.to_sparse_vec_black()),
                chessbb::Side::Black => (chessgame.to_sparse_vec_black(), chessgame.to_sparse_vec_white()),
            };
            self.pairs = Some((input, best_eval));
        }
        best_move
    }

    fn search(&mut self, chessgame: &mut ChessGame, d: &NonZero<usize>, ev: &mut impl Evaluator, tt: Arc<TT>, moves: MoveList) -> ChessMove {
        assert!(!moves.is_empty());
        if moves.len() == 1 {
            if self.collect_pairs {
                let input = match chessgame.side() {
                    chessbb::Side::White => (chessgame.to_sparse_vec_white(), chessgame.to_sparse_vec_black()),
                    chessbb::Side::Black => (chessgame.to_sparse_vec_black(), chessgame.to_sparse_vec_white()),
                };
                self.pairs = Some((input, ev.eval(&chessgame)));
            }
            return moves[0];
        }

        let mut best_eval: i16 = i16::MIN + 1;
        let mut best_move: ChessMove = moves[0].clone();
        for chess_move in moves {
            let snapshot = chessgame.explore_state(&chess_move);
            let eval: i16 = -self.negamax::<false, false>(chessgame, best_eval, i16::MAX - 1, d.get() - 1, ev, tt.clone(), None, None);
            chessgame.restore_state(snapshot);

            if eval > best_eval {
                best_move = chess_move;
                best_eval = eval;
            }
        }

        if self.collect_pairs {
            let input = match chessgame.side() {
                chessbb::Side::White => (chessgame.to_sparse_vec_white(), chessgame.to_sparse_vec_black()),
                chessbb::Side::Black => (chessgame.to_sparse_vec_black(), chessgame.to_sparse_vec_white()),
            };
            self.pairs = Some((input, best_eval));
        }
        best_move
    }

    pub fn negamax<const IS_TIME_LIMITED: bool, const IS_NODE_LIMITED: bool>(
        &mut self, chessgame: &mut ChessGame, a: i16, b: i16, d: usize, ev: &mut impl Evaluator, tt: Arc<TT>, time_limit: Option<(Instant, Duration)>,
        node_limit: Option<NonZero<usize>>,
    ) -> i16 {
        if d == 0 {
            return ev.eval(&chessgame);
        }

        let (chessmoves, gamestate) = chessgame.try_generate_moves();

        if let GameState::Finished(state) = gamestate {
            match state {
                GameResult::Win(_) => {
                    return LOSE_SCORE + (self.ply as i16); //TODO determine if +d or -d or something else should be used here.
                }
                GameResult::Draw => return 0,
            }
        }

        let mut alpha: i16 = a;
        let mut best_value: i16 = i16::MIN + 1;
        //let mut best_move: Option<ChessMove> = None;
        for chessmove in chessmoves {
            //chef: only check every 1024 node
            if IS_NODE_LIMITED {
                let node_limit = unsafe { node_limit.unwrap_unchecked() };
                if node_limit.get() > self.node_count {
                    self.is_aborted = true;
                    return match best_value >= b {
                        true => best_value,
                        false => i16::MIN + 1,
                    };
                }
            }

            if IS_TIME_LIMITED && self.node_count % NODE_COUNT_CHECK_LIMIT == 0 {
                let time_limit = unsafe { time_limit.unwrap_unchecked() };
                if time_limit.0.elapsed() >= time_limit.1 {
                    self.is_aborted = true;
                    return match best_value >= b {
                        true => best_value,
                        false => i16::MIN + 1,
                    };
                }
            }

            self.node_count += 1; //apparently this is the accepted way to count nps

            ev.update(&chessgame, &chessmove);
            let snapshot: ChessBoardSnapshot = chessgame.explore_state(&chessmove);
            self.ply += 1;
            let value: i16 = -self.negamax::<IS_TIME_LIMITED, IS_NODE_LIMITED>(chessgame, -b, -alpha, d - 1, ev, tt.clone(), time_limit, node_limit);
            self.ply -= 1;
            ev.revert(&chessgame, &chessmove);
            chessgame.restore_state(snapshot);

            if value > best_value {
                best_value = value;
                //best_move = Some(chess_move);
            }

            if value > alpha {
                alpha = value;
            }

            if alpha >= b {
                break;
            }
        }

        best_value
    }
}
