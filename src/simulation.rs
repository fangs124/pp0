use std::sync::mpsc::Sender;

use chessbb::{ChessMove, GameResult, GameState, Side};
use itertools::interleave;
use nalgebra::DVector;
use nnet::{Gradient, InputType};
use rand::Rng;

use crate::{
    FALLBACK_DEPTH,
    chessnet::{ChessGame, ChessNet},
};

pub struct TrainingResult {
    pub epoch: usize,
    pub result: GameResult,
    pub net_side: Side,
    pub pairs: Vec<(DVector<f32>, DVector<f32>)>, //(input,output)
}

pub fn play(net: &mut ChessNet, enm: &mut ChessNet, tx: Sender<TrainingResult>, epoch: usize) {
    let mut chessgame = ChessGame::start_pos();
    let mut rng = rand::rng();
    let is_net_white = rng.random_bool(0.5);

    let mut input_vec: Vec<DVector<f32>> = Vec::new();
    let mut output_vec: Vec<DVector<f32>> = Vec::new();
    //let mut chessmove_vec: Vec<ChessMove> = Vec::new();

    // play game
    while chessgame.state() == GameState::Ongoing {
        let active_side = chessgame.side();
        let chessmove: ChessMove;
        chessmove = match is_net_white == (active_side == Side::White) {
            true => net.negamax_learn(&chessgame, FALLBACK_DEPTH, &mut input_vec, &mut output_vec),
            false => enm.negamax(&chessgame, FALLBACK_DEPTH),
        };

        chessgame.update_state(chessmove);
    }

    // game ended
    let GameState::Finished(result) = chessgame.state() else { unreachable!() };
    let pairs: Vec<(DVector<f32>, DVector<f32>)> = input_vec.into_iter().zip(output_vec).collect();
    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TrainingResult = TrainingResult { epoch, result, net_side, pairs };
    _ = tx.send(return_data);
}

pub fn play_rand(net: &mut ChessNet, tx: Sender<TrainingResult>, epoch: usize) {
    let mut chessgame = ChessGame::start_pos();
    let mut rng = rand::rng();
    let is_net_white = rng.random_bool(0.5);

    let mut input_vec: Vec<DVector<f32>> = Vec::new();
    let mut output_vec: Vec<DVector<f32>> = Vec::new();
    //let mut chessmove_vec: Vec<ChessMove> = Vec::new();

    // play game
    while chessgame.state() == GameState::Ongoing {
        let active_side = chessgame.side();
        let chessmove: ChessMove;
        chessmove = match is_net_white == (active_side == Side::White) {
            true => net.negamax_learn(&chessgame, FALLBACK_DEPTH, &mut input_vec, &mut output_vec),
            false => chessgame.random_move(),
        };

        chessgame.update_state(chessmove);
    }

    // game ended
    let GameState::Finished(result) = chessgame.state() else { unreachable!() };
    let pairs: Vec<(DVector<f32>, DVector<f32>)> = input_vec.into_iter().zip(output_vec).collect();
    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TrainingResult = TrainingResult { epoch, result, net_side, pairs };
    _ = tx.send(return_data);
}

pub fn review_play(net: &mut ChessNet, enm: &mut ChessNet, tx: Sender<TrainingResult>, epoch: usize) {
    let mut chessgame = ChessGame::start_pos();
    let mut rng = rand::rng();
    let is_net_white = rng.random_bool(0.5);

    // play game
    while chessgame.state() == GameState::Ongoing {
        let active_side = chessgame.side();
        let chessmove: ChessMove;
        chessmove = match is_net_white == (active_side == Side::White) {
            true => net.negamax(&chessgame, FALLBACK_DEPTH),
            false => enm.negamax(&chessgame, FALLBACK_DEPTH),
        };

        chessgame.update_state(chessmove);
    }

    // game ended
    let GameState::Finished(result) = chessgame.state() else { unreachable!() };
    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TrainingResult = TrainingResult { epoch, result, net_side, pairs: Vec::new() };
    _ = tx.send(return_data);
}

pub fn review_play_rand(net: &mut ChessNet, tx: Sender<TrainingResult>, epoch: usize) {
    let mut chessgame = ChessGame::start_pos();
    let mut rng = rand::rng();
    let is_net_white = rng.random_bool(0.5);

    // play game
    while chessgame.state() == GameState::Ongoing {
        let active_side = chessgame.side();
        let chessmove: ChessMove;
        chessmove = match is_net_white == (active_side == Side::White) {
            true => net.negamax(&chessgame, FALLBACK_DEPTH),
            false => chessgame.random_move(),
        };

        chessgame.update_state(chessmove);
    }

    // game ended
    let GameState::Finished(result) = chessgame.state() else { unreachable!() };
    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TrainingResult = TrainingResult { epoch, result, net_side, pairs: Vec::new() };
    _ = tx.send(return_data);
}
