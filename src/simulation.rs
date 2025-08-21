use std::sync::mpsc::Sender;

use crate::{
    FALLBACK_DEPTH,
    chessnet::{ChessGame, ChessNet},
};
use chessbb::{ChessMove, GameResult, GameState, Side};
use nalgebra::DVector;

pub struct TrainingResult {
    pub epoch: usize,
    pub result: GameResult,
    pub net_side: Side,
    pub pairs: Vec<(DVector<f32>, DVector<f32>)>, //(input,output)
}

//pub struct TrainingResultSanityTest {
//    pub epoch: usize,
//    pub result: GameResult,
//    pub net_side: Side,
//    pub pairs: Vec<(DVector<f32>, DVector<f32>)>, //(input,output)
//    pub chessmoves: Vec<ChessMove>,
//}

pub fn play(net: ChessNet, enm: Option<ChessNet>, tx: Sender<TrainingResult>, epoch: usize, is_learn: bool) {
    let is_net_white = rand::random_bool(0.5);
    let result: GameResult;
    let mut pairs: Vec<(DVector<f32>, DVector<f32>)> = Vec::new();
    match is_learn {
        true => {
            (result, pairs) = learn_game(net, enm, is_net_white);
        }
        false => {
            result = play_game(net, enm, is_net_white);
        }
    }

    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TrainingResult = TrainingResult { epoch, result, net_side, pairs };
    _ = tx.send(return_data);
}

fn play_game(mut net: ChessNet, enm: Option<ChessNet>, is_net_white: bool) -> GameResult {
    let mut chess_game = ChessGame::start_pos();
    let (mut moves, mut game_state) = chess_game.try_generate_moves();

    // play game
    match enm.is_some() {
        true => {
            let mut enm = enm.unwrap();
            while game_state == GameState::Ongoing {
                let chessmove: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax(&chess_game, FALLBACK_DEPTH, moves),
                    false => enm.negamax(&chess_game, FALLBACK_DEPTH, moves),
                };

                chess_game.update_state(chessmove);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
        false => {
            while game_state == GameState::Ongoing {
                let chessmove: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax(&chess_game, FALLBACK_DEPTH, moves),
                    false => chess_game.random_move(),
                };

                chess_game.update_state(chessmove);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
    }

    let GameState::Finished(result) = game_state else { unreachable!() };
    return result;
}

#[rustfmt::skip]
fn learn_game(mut net: ChessNet, enm: Option<ChessNet>, is_net_white: bool) -> (GameResult, Vec<(DVector<f32>, DVector<f32>)>) {
    let mut chess_game = ChessGame::start_pos();
    let (mut moves, mut game_state) = chess_game.try_generate_moves();
    let mut ins: Vec<DVector<f32>> = Vec::new();
    let mut outs: Vec<DVector<f32>> = Vec::new();
    // play game
    match enm.is_some() {
        true => {
            let mut enm = enm.unwrap();
            while game_state == GameState::Ongoing {
                let chessmove: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax_learn_epsilon(&chess_game, FALLBACK_DEPTH, &mut ins, &mut outs, moves),
                    false => enm.negamax_epsilon(&chess_game, FALLBACK_DEPTH, moves),
                };

                chess_game.update_state(chessmove);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
        false => {
            while game_state == GameState::Ongoing {
                let chessmove: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax_learn(&chess_game, FALLBACK_DEPTH, &mut ins, &mut outs, moves),
                    false => chess_game.random_move(),
                };

                chess_game.update_state(chessmove);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
    }

    let GameState::Finished(result) = game_state else { unreachable!() };
    return (result, ins.into_iter().zip(outs).collect());
}

//TODO remove this
//pub fn sanity_test(net: &mut ChessNet, tx: Sender<TrainingResultSanityTest>, epoch: usize) {
//    let mut chess_game = ChessGame::start_pos();
//    let mut rng = rand::rng();
//    let is_net_white = rng.random_bool(0.5);
//
//    let mut chessmoves: Vec<ChessMove> = Vec::new();
//    let (mut moves, mut game_state) = chess_game.try_generate_moves();
//
//    // play game
//    while game_state == GameState::Ongoing {
//        let chessmove = match is_net_white == (chess_game.side() == Side::White) {
//            true => net.negamax_sanity_test(&chess_game, FALLBACK_DEPTH),
//            false => chess_game.random_move(),
//        };
//
//        chess_game.update_state(chessmove.clone());
//        chessmoves.push(chessmove);
//    }
//
//    // game ended
//    let GameState::Finished(result) = game_state else { panic!() };
//    let net_side: Side = match is_net_white {
//        true => Side::White,
//        false => Side::Black,
//    };
//
//    let return_data: TrainingResultSanityTest =
//        TrainingResultSanityTest { epoch, result, net_side, pairs: Vec::new(), chessmoves };
//    _ = tx.send(return_data);
//}
