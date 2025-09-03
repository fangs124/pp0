use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

use chessbb::{ChessBoard, ChessMove, Evaluator, GameResult, GameState, Side, TranspositionTable};
use nalgebra::DVector;
use rand::random_range;
use serde::{Deserialize, Serialize};

use crate::{ChessGame, LEARNING_RATE, nnet::*, simulation::TrainingResult};

const MAX_SEARCH_INSTANCE: usize = 1;
static SEARCH_INSTANCE_COUNT: AtomicUsize = AtomicUsize::new(0_usize);

//alpha-beta nets
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChessNet {
    pub net: Network<ChessGame>,
    pub version: u32,
}

impl Evaluator for ChessNet {
    fn eval(&mut self, cb: &ChessBoard) -> i16 {
        self.net.forward_prop_sparse_vec(ChessGame::vectorize_sparse(cb));
        return (self.phi_z()[0] * 1000.0) as i16;
    }
}
//old DVector input
//impl Evaluator for ChessNet {
//    //TODO fix this so that its not horridly expensive
//    fn eval(&mut self, cb: &ChessBoard) -> i16 {
//        self.net.forward_prop_vector(ChessGame::vectorize(cb));
//        return (self.phi_z()[0] * 1000.0) as i16;
//    }
//}

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
    pub fn eval_sparse(&mut self, cg: &ChessGame) -> DVector<f32> {
        self.net.forward_prop_sparse(cg);
        return self.net.phi_z_vector();
    }

    #[inline(always)]
    pub fn back_prop(&mut self, cg: &ChessGame, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop(cg, target, r)
    }

    #[inline(always)]
    pub fn back_prop_sparse(&mut self, cg: &ChessGame, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop_sparse(cg, target, r)
    }

    #[inline(always)]
    pub fn back_prop_vector(&mut self, input: DVector<f32>, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop_vector(input, target, r)
    }

    #[inline(always)]
    pub fn back_prop_sparse_vec(&mut self, input: SparseVec, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop_sparse_vec(input, target, r)
    }

    #[inline(always)]
    pub fn phi_z(&self) -> Vec<f32> {
        self.net.phi_z()
    }

    pub fn iterative_deepening(&mut self, cg: &mut ChessGame, max_depth: Option<usize>, tt: &mut TranspositionTable, time_limit: Duration) -> ChessMove {
        let now = Instant::now();
        let moves: Vec<ChessMove> = cg.cb.try_generate_moves().0;
        assert!(!moves.is_empty());
        let mut best_move = moves[0].clone();
        let (tx, rx) = mpsc::channel::<ChessMove>();
        let max_depth = match max_depth {
            Some(x) => x,
            None => usize::MAX,
        };
        let mut d = 1;
        while now.elapsed() < time_limit && d <= max_depth {
            if SEARCH_INSTANCE_COUNT.load(Ordering::SeqCst) <= MAX_SEARCH_INSTANCE {
                SEARCH_INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
                let mut net = self.clone();
                let tx_new = tx.clone();
                let mut cg_new = cg.clone();
                let mut tt_new = tt.clone();
                let moves_new = moves.clone();
                rayon::spawn(move || {
                    let mut node_count: usize = 0;
                    _ = tx_new.send(net.find_move(&mut cg_new, d, &mut node_count, moves_new, &mut tt_new));
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

    pub fn iterative_deepening_no_tt(
        &mut self, cg: &mut ChessGame, max_depth: Option<usize>, time_limit: Duration, tt: Arc<Mutex<TranspositionTable>>,
    ) -> (usize, usize, i16, Duration, ChessMove) {
        let moves: Vec<ChessMove> = cg.cb.try_generate_moves().0;
        SEARCH_INSTANCE_COUNT.store(0, Ordering::SeqCst);
        assert!(!moves.is_empty());
        let mut best_move = moves[0].clone();
        let (tx, rx) = mpsc::channel::<(ChessMove, i16, usize)>();
        let max_depth = match max_depth {
            Some(x) => x,
            None => usize::MAX,
        };
        let mut d: usize = 1;
        let mut node_count_total: usize = 0;
        let mut eval: i16 = 0;
        let now = Instant::now();
        while now.elapsed() < time_limit && d <= max_depth {
            if SEARCH_INSTANCE_COUNT.load(Ordering::SeqCst) <= MAX_SEARCH_INSTANCE {
                SEARCH_INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst);
                let mut net = self.clone();
                let tx_new = tx.clone();
                let mut cg_new = cg.clone();
                let moves_new = moves.clone();
                //let tt_new = tt.clone();
                rayon::spawn(move || {
                    //let mut tt_new = tt_new.lock().unwrap();
                    let mut tt_new = TranspositionTable::new();
                    let mut node_count: usize = 0;
                    let (eval, chess_move) = cg_new.negamax(d, &mut net, &mut tt_new, &mut node_count, Some((moves_new, GameState::Ongoing)));
                    _ = tx_new.send((chess_move.unwrap(), eval, node_count));
                    SEARCH_INSTANCE_COUNT.fetch_sub(1_usize, Ordering::SeqCst);
                });
            }

            while let Ok((chess_move_data, eval_data, node_count_data)) = rx.try_recv() {
                best_move = chess_move_data;
                eval = eval_data;
                node_count_total += node_count_data;
                d += 1;
            }
        }
        SEARCH_INSTANCE_COUNT.store(0, Ordering::SeqCst);
        return (d - 1, node_count_total, eval, now.elapsed(), best_move);
    }

    pub fn find_move(&mut self, cg: &mut ChessGame, d: usize, node_count: &mut usize, moves: Vec<ChessMove>, tt: &mut TranspositionTable) -> ChessMove {
        return cg.find_move(d, self, node_count, moves, tt);
    }

    pub fn learn(
        &mut self, cg: &mut ChessGame, d: usize, node_count: &mut usize, ins: &mut Vec<SparseVec>, outs: &mut Vec<i16>, moves: Vec<ChessMove>,
        tt: &mut TranspositionTable,
    ) -> ChessMove {
        ins.push(cg.to_sparse_vec());
        assert!(!moves.is_empty() && d > 0);
        let chess_move = moves[0].clone();
        let (eval, best_move) = cg.negamax(d, self, tt, node_count, Some((moves, GameState::Ongoing)));
        outs.push(eval);
        return best_move.unwrap_or(chess_move);
    }

    pub fn process_training_result(&mut self, data: TrainingResult) {
        let total_moves = data.pairs.len();
        let reward: f32 = match (data.net_side, data.result) {
            (Side::White, GameResult::WhiteWins) | (Side::Black, GameResult::BlackWins) => 1.0,
            (Side::White, GameResult::BlackWins) | (Side::Black, GameResult::WhiteWins) => -1.0,
            (_, GameResult::Draw) => -0.1,
        };

        let mut ith_move: usize = 0;
        /* maybe isolate this? */
        for (input, eval) in data.pairs {
            //old reward scheme
            let scaled_reward = compute_scalar(ith_move, total_moves);
            //let target_output = DVector::from_element(1, reward);
            let t: f32 = ith_move as f32 / total_moves as f32;
            let lerp = (1.0 - t.powi(8)) * (eval.min(1000).max(-1000) as f32 / 1000.0) + t.powi(8) * reward;
            let target_output = DVector::from_element(1, lerp);

            let grad = self.back_prop_sparse_vec(input, target_output, scaled_reward);
            self.update(grad, -LEARNING_RATE);
            ith_move += 1;
        }
    }

    pub fn update(&mut self, grad: Gradient, r: f32) {
        self.net.update(grad, r);
    }
}

#[inline(always)]
fn compute_scalar(index: usize, total: usize) -> f32 {
    0.4 + (0.6 * (((index) as f32) / (total as f32)))
}

#[inline(always)]
fn get_move_rand(_: &mut ChessNet, _: &mut ChessGame, _: usize, moves: &Vec<ChessMove>, _: &mut TranspositionTable) -> ChessMove {
    moves[random_range(0..moves.len())]
}
