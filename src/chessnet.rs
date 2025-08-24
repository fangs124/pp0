use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use chessbb::{
    ChessBoard, ChessBoardCore, ChessMove, ChessPiece, Evaluator, GameResult, GameState, MATERIAL_EVAL, PieceType,
    Side, Square, TranspositionTable,
};
use nalgebra::DVector;
use rand::{random_bool, random_range};
use serde::{Deserialize, Serialize};

use crate::{ChessGame, LEARNING_RATE, nnet::*, simulation::TrainingResult};

const TABLE_SIZE: usize = 1 << 22;

const MAX_SEARCH_INSTANCE: usize = 1;
static SEARCH_INSTANCE_COUNT: AtomicUsize = AtomicUsize::new(0_usize);

//alpha-beta nets
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChessNet {
    pub net: Network<ChessGame>,
    pub version: u32,
}

impl Evaluator for ChessNet {
    //TODO fix this so that its not horridly expensive
    fn eval(&mut self, cb: &ChessBoard) -> i16 {
        self.net.forward_prop_vector(ChessGame::vectorize(cb));
        return (self.phi_z()[0] * 1000.0) as i16;
    }
}

const EPSILON: f64 = 0.4;

type DVf32 = DVector<f32>;

impl ChessNet {
    #[inline(always)]
    pub fn new(node_counts: Vec<usize>) -> Self {
        let input_dim = ChessGame::start_pos().to_vector().len();
        ChessNet { net: Network::new(input_dim, node_counts), version: 0 }
    }

    #[inline(always)]
    pub fn eval(&mut self, cg: &ChessGame) -> DVector<f32> {
        self.net.forward_prop(cg);
        return self.net.phi_z_vector();
    }

    #[inline(always)]
    pub fn back_prop(&mut self, cg: &ChessGame, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop(cg, target, r)
    }

    #[inline(always)]
    pub fn back_prop_vector(&mut self, input: DVector<f32>, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop_vector(input, target, r)
    }

    #[inline(always)]
    pub fn phi_z(&self) -> Vec<f32> {
        self.net.phi_z()
    }

    pub fn negamax_cold(&mut self, cg: &ChessGame, d: usize, tt: &mut TranspositionTable) -> ChessMove {
        //TODO safety of the return result?
        cg.clone().cb.negamax(i16::MIN + 1, i16::MAX - 1, d, 0, self, tt).1.unwrap()
    }

    pub fn iterative_deepening_hot(
        &mut self,
        cg: &ChessGame,
        moves: Vec<ChessMove>,
        tt: &mut TranspositionTable,
        time_limit: Duration,
    ) -> ChessMove {
        assert!(!moves.is_empty());
        let now = Instant::now();
        let mut best_move = moves[0].clone();
        let mut d: usize = 1;
        while now.elapsed() < time_limit {
            best_move = self.negamax(cg, d, &moves, tt);
            d += 1;
        }
        return best_move;
    }

    pub fn iterative_deepening(
        &mut self,
        cg: &ChessGame,
        max_depth: Option<usize>,
        tt: &mut TranspositionTable,
        time_limit: Duration,
    ) -> ChessMove {
        let now = Instant::now();
        let moves: Vec<ChessMove> = cg.cb.try_generate_moves().0;
        assert!(!moves.is_empty());
        let mut best_move = moves[0].clone();
        let mut d: usize = 1;
        let (tx, rx) = mpsc::channel::<ChessMove>();
        let max_depth = match max_depth {
            Some(x) => x,
            None => usize::MAX,
        };

        while now.elapsed() < time_limit && d <= max_depth {
            if SEARCH_INSTANCE_COUNT.load(Ordering::SeqCst) <= MAX_SEARCH_INSTANCE {
                SEARCH_INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
                let mut net = self.clone();
                let tx_new = tx.clone();
                let cg_new = cg.clone();
                let mut tt_new = tt.clone();
                let moves_new = moves.clone();
                rayon::spawn(move || {
                    _ = tx_new.send(net.negamax(&cg_new, d, &moves_new, &mut tt_new));
                    SEARCH_INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
                });
            }

            while let Ok(data) = rx.try_recv() {
                best_move = data;
                d += 1;
            }
        }

        return best_move;
    }
    pub fn negamax(
        &mut self,
        cg: &ChessGame,
        d: usize,
        moves: &Vec<ChessMove>,
        tt: &mut TranspositionTable,
    ) -> ChessMove {
        assert!(!moves.is_empty() && d > 0);
        let mut alpha: i16 = i16::MIN + 1;
        let beta: i16 = i16::MAX - 1;
        let mut best_move: ChessMove = moves[0].clone();
        let mut chess_game: ChessGame = cg.clone();

        for chess_move in moves {
            //let old_core: ChessBoardCore = chess_game.cb.core.clone();
            let snapshot = chess_game.cb.explore_state(*chess_move);
            //depth instead of depth-1 here so that call to ChessNet::negamax() has implicit depth >= 1.
            let (value, _next_move) = negate(chess_game.cb.negamax(-beta, -alpha, d - 1, 1, self, tt));
            chess_game.cb.restore_state(snapshot);

            if value > alpha {
                alpha = value;
                best_move = *chess_move;
            }
        }

        //let mut action_values: Vec<(ChessMove, i16)> = moves.iter().map(|&x| (x, i16::MIN + 1)).collect();
        //for depth in d..=d {
        //    for (chess_move, old_value) in action_values.iter_mut() {
        //        let snapshot = chess_game.cb.explore_state(*chess_move);
        //        //NOTE: depth instead of depth-1 here so that call to ChessNet::negamax() has implicit depth >= 1.
        //        let (value, next_move) = negate(chess_game.cb.negamax(-beta, -alpha, depth - 1, 1, self, tt));
        //        chess_game.cb.restore_state(snapshot);
        //
        //        if value > alpha {
        //            alpha = value;
        //            best_move = *chess_move;
        //        }
        //
        //        *old_value = value;
        //    }
        //
        //    //action_values.sort_by(|(_, av), (_, bv)| av.cmp(bv));
        //}
        return best_move;
    }

    pub fn negamax_learn(
        &mut self,
        cg: &ChessGame,
        d: usize,
        ins: &mut Vec<DVf32>,
        outs: &mut Vec<DVf32>,
        moves: &Vec<ChessMove>,
        tt: &mut TranspositionTable,
    ) -> ChessMove {
        ins.push(cg.to_vector());
        outs.push(self.eval(&cg));
        return self.negamax(cg, d, moves, tt);
    }

    pub fn negamax_epsilon(
        &mut self,
        cg: &ChessGame,
        d: usize,
        moves: &Vec<ChessMove>,
        tt: &mut TranspositionTable,
    ) -> ChessMove {
        assert!(!moves.is_empty());
        if random_bool(EPSILON) {
            return moves[random_range(0..moves.len())];
        }
        return self.negamax(cg, d, moves, tt);
    }

    pub fn negamax_learn_epsilon(
        &mut self,
        cg: &ChessGame,
        d: usize,
        ins: &mut Vec<DVf32>,
        outs: &mut Vec<DVf32>,
        moves: &Vec<ChessMove>,
        tt: &mut TranspositionTable,
    ) -> ChessMove {
        ins.push(cg.to_vector());
        outs.push(self.eval(&cg));
        return self.negamax_epsilon(cg, d, moves, tt);
    }

    pub fn process_training_result(&mut self, data: TrainingResult) {
        let total_moves = data.pairs.len();
        let reward: f32 = match (data.net_side, data.result) {
            (Side::White, GameResult::WhiteWins) | (Side::Black, GameResult::BlackWins) => 1.0,
            (Side::White, GameResult::BlackWins) | (Side::Black, GameResult::WhiteWins) => -1.0,
            (_, GameResult::Draw) => -0.01,
        };

        let mut ith_move: usize = match data.net_side {
            Side::White => 0,
            Side::Black => 1,
        };

        /* maybe isolate this? */
        for (input, output) in data.pairs {
            let scaled_reward = reward * compute_scalar(ith_move, total_moves);
            let target_output = DVector::from_element(1, reward);

            let grad = self.back_prop_vector(input, target_output, scaled_reward);
            self.update(grad, -LEARNING_RATE);
            ith_move += 2;
        }
    }

    pub fn update(&mut self, grad: Gradient, r: f32) {
        self.net.update(grad, r);
    }

    //greedy
    //pub fn get_move(&mut self, gb: &GameBoard) -> BitBoard {
    //    self.net.forward_prop(gb);
    //    let output = get_index(&self.pi(), gb);
    //    return BitBoard::MOVES[output];
    //}
    //
    ////epsilon-greedy
    //pub fn get_move_eps(&mut self, gb: &GameBoard) -> BitBoard {
    //    let is_play_random = random_bool(EPS);
    //    if !is_play_random {
    //        self.net.forward_prop(gb);
    //        let output = get_index(&self.pi(), gb);
    //        return BitBoard::MOVES[output];
    //    }
    //
    //    let valid_moves: Vec<BitBoard> =
    //        BitBoard::MOVES.to_vec().into_iter().filter(|bb| gb.is_valid_move(bb)).collect();
    //    let i: usize = random_range(0..valid_moves.len());
    //    return valid_moves[i];
    //}
    //
    ////random
    //pub fn get_move_rand(&mut self, gb: &GameBoard) -> BitBoard {
    //    let valid_moves: Vec<BitBoard> =
    //        BitBoard::MOVES.to_vec().into_iter().filter(|bb| gb.is_valid_move(bb)).collect();
    //    let i: usize = random_range(0..valid_moves.len());
    //    return valid_moves[i];
    //}
}

#[inline(always)]
fn compute_scalar(index: usize, total: usize) -> f32 {
    0.5 + (0.5 * (((index) as f32) / (total as f32)))
}

#[inline(always)]
fn negate(pair: (i16, Option<ChessMove>)) -> (i16, Option<ChessMove>) {
    return (-pair.0, pair.1);
}
