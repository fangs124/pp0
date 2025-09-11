use std::{
    i16,
    num::NonZero,
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use chessbb::{AtomicTranspositionTable, ChessBoard, ChessMove, Evaluator, GameResult, GameState, NegamaxData, Side, TranspositionTable};
use nalgebra::DVector;
use rand::random_range;
use serde::{Deserialize, Serialize};

use crate::{AtomicTT, ChessGame, LEARNING_RATE, nnet::*, simulation::TrainingResult};

const MAX_SEARCH_INSTANCE: usize = 16;
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

    pub fn learn(
        &mut self, cg: &mut ChessGame, node_count: &mut usize, ins: &mut Vec<SparseVec>, outs: &mut Vec<i16>, moves: Vec<ChessMove>, tt: Arc<AtomicTT>,
        node_limit: Option<NonZero<usize>>, time_limit: Option<Duration>,
    ) -> ChessMove {
        ins.push(cg.to_sparse_vec());
        let (best_eval, best_move) = cg.iterative_deepening(self, node_count, Some(moves), tt, time_limit, node_limit);
        outs.push(best_eval);
        return best_move;
    }

    pub fn process_training_result(&mut self, data: TrainingResult) {
        let total_moves = data.pairs.len();
        let reward: f32 = match (data.net_side, data.result) {
            (Side::White, GameResult::WhiteWins) | (Side::Black, GameResult::BlackWins) => 1.0,
            (Side::White, GameResult::BlackWins) | (Side::Black, GameResult::WhiteWins) => -1.0,
            (_, GameResult::Draw) => 0.0,
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
