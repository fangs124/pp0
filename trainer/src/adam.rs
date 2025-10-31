use std::f32::EPSILON;

use chessbb::{GameResult, Side};
use nalgebra::SVector;
use nnue::{Gradient, Network};

use crate::{LAMBDA, LEARNING_RATE, simulation::MatchResult};

pub fn adam(net: &mut Network, results: Vec<MatchResult>, beta1: f32, beta2: f32, m: &mut Gradient, v: &mut Gradient) {
    let mut m = m.clone();
    let mut v = v.clone();
    let total_results = results.len();
    let mut i: usize = 0;
    for result in results {
        let grad = game_gradient(net, result);

        m = beta1 * m + (1.0 - beta1) * grad.clone();
        v = beta2 * v + (1.0 - beta2) * grad.component_square();

        let adam = Gradient::adam(beta1, beta2, i, &m, &v);
        net.update(Gradient::adam(beta1, beta2, i, &m, &v), -LEARNING_RATE);
        let reg = net.regularization_term(LAMBDA);
        net.update(reg, -LEARNING_RATE);

        i += 1;
    }
}

pub fn game_gradient(net: &mut Network, data: MatchResult) -> Gradient {
    let pairs = data.pairs.unwrap();
    let total_moves = pairs.len();
    let reward: f32 = match (data.p1_side, data.result) {
        (Side::White, GameResult::Win(Side::White)) | (Side::Black, GameResult::Win(Side::Black)) => 1.0,
        (Side::White, GameResult::Win(Side::Black)) | (Side::Black, GameResult::Win(Side::White)) => -1.0,
        (_, GameResult::Draw) => 0.0,
    };

    let mut total_grad: Gradient = Gradient::zeros();
    let mut ith_move: usize = 0;

    for ((in_stm, in_ntm), eval) in pairs {
        let t: f32 = ith_move as f32 / total_moves as f32;
        let lerp = (1.0 - t.powi(4)).max(0.0) * (eval.min(2000).max(-2000) as f32 / 2000.0) + t.powi(4).min(1.0) * reward;
        let target: SVector<f32, 1> = SVector::from_element(lerp);
        let grad = net.backward_prop_sparse(in_stm, in_ntm, target, 1.0);
        ith_move += 1;
        total_grad = total_grad + grad;
    }

    return (1.0 / (total_moves as f32)) * total_grad;
}
