use std::sync::mpsc::Sender;

use crate::{ChessGame, ChessNet, FALLBACK_DEPTH};
use chessbb::{ChessMove, GameResult, GameState, Side, TranspositionTable};
use nalgebra::DVector;

pub struct TrainingResult {
    pub epoch: usize,
    pub result: GameResult,
    pub net_side: Side,
    //pub history: Option<Vec<ChessMove>>,
    pub pairs: Vec<(DVector<f32>, i16)>, //(input,output)
}

type TR = TrainingResult;

pub struct PlayParameter {
    epoch: usize,
    is_learn: bool,
    is_history: bool,
    enm: Option<ChessNet>,
    fen: Option<String>,
}

pub fn play(net: ChessNet, enm: Option<ChessNet>, fen: Option<&str>, tx: Sender<TR>, epoch: usize, is_learn: bool) {
    let is_net_white = rand::random_bool(0.5);
    let result: GameResult;
    let mut pairs: Vec<(DVector<f32>, i16)> = Vec::new();
    match is_learn {
        true => {
            (result, pairs) = learn_game(net, enm, is_net_white, fen);
        }
        false => {
            result = play_game(net, enm, is_net_white, fen);
        }
    }

    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    //let return_data: TrainingResult = match is_learn {
    //    true => TrainingResult { epoch, result, net_side, history: None, pairs },
    //    false => TrainingResult { epoch, result, net_side, history: Some(history), pairs },
    //};

    let return_data: TR = match is_learn {
        true => TR { epoch, result, net_side, pairs },
        false => TR { epoch, result, net_side, pairs },
    };
    _ = tx.send(return_data);
}

fn play_game(mut net: ChessNet, enm: Option<ChessNet>, is_net_white: bool, fen: Option<&str>) -> GameResult {
    let mut chess_game: ChessGame = match fen {
        Some(fen) => ChessGame::from_fen(fen),
        None => ChessGame::start_pos(),
    };
    let (mut moves, mut game_state) = chess_game.try_generate_moves();
    let mut tt_net = TranspositionTable::new();
    let mut tt_enm = TranspositionTable::new();

    // play game
    match enm.is_some() {
        true => {
            let mut enm = enm.unwrap();
            while game_state == GameState::Ongoing {
                let chess_move: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax(&mut chess_game, FALLBACK_DEPTH, &moves, &mut tt_net),
                    false => enm.negamax(&mut chess_game, FALLBACK_DEPTH - 1, &moves, &mut tt_enm),
                };

                chess_game.update_state(chess_move);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
        false => {
            while game_state == GameState::Ongoing {
                let chess_move: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax(&mut chess_game, FALLBACK_DEPTH, &moves, &mut tt_net),
                    false => chess_game.find_move_hce(FALLBACK_DEPTH - 1, &moves, &mut tt_enm),
                };

                chess_game.update_state(chess_move);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
    }

    let GameState::Finished(result) = game_state else { unreachable!() };
    return result;
}

fn learn_game(
    mut net: ChessNet,
    enm: Option<ChessNet>,
    is_net_white: bool,
    fen: Option<&str>,
) -> (GameResult, Vec<(DVector<f32>, i16)>) {
    let mut chess_game: ChessGame = match fen {
        Some(fen) => ChessGame::from_fen(fen),
        None => ChessGame::start_pos(),
    };
    let (mut moves, mut game_state) = chess_game.try_generate_moves();
    let mut ins: Vec<DVector<f32>> = Vec::new();
    let mut outs: Vec<i16> = Vec::new();
    let mut tt_net = TranspositionTable::new();
    let mut tt_enm = TranspositionTable::new();
    // play game
    match enm.is_some() {
        true => {
            let mut enm = enm.unwrap();
            while game_state == GameState::Ongoing {
                let chess_move = match is_net_white == (chess_game.side() == Side::White) {
                    true => {
                        net.negamax_learn(&mut chess_game, FALLBACK_DEPTH, &mut ins, &mut outs, &moves, &mut tt_net)
                    }
                    false => enm.negamax_epsilon(&mut chess_game, FALLBACK_DEPTH - 1, &moves, &mut tt_enm),
                };

                chess_game.update_state(chess_move);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
        false => {
            while game_state == GameState::Ongoing {
                let chess_move = match is_net_white == (chess_game.side() == Side::White) {
                    true => {
                        net.negamax_learn(&mut chess_game, FALLBACK_DEPTH, &mut ins, &mut outs, &moves, &mut tt_net)
                    }
                    false => chess_game.find_move_hce_epsilon(FALLBACK_DEPTH - 1, &moves, &mut tt_enm),
                };

                chess_game.update_state(chess_move);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
    }

    let GameState::Finished(result) = game_state else { unreachable!() };
    return (result, ins.into_iter().zip(outs).collect());
}

fn play_game_history(mut net: ChessNet, enm: Option<ChessNet>, is_net_white: bool) -> (GameResult, Vec<ChessMove>) {
    let mut chess_game = ChessGame::start_pos();
    let mut history = Vec::new();
    let (mut moves, mut game_state) = chess_game.try_generate_moves();
    let mut tt_net = TranspositionTable::new();
    let mut tt_enm = TranspositionTable::new();

    // play game
    match enm {
        Some(mut enm) => {
            while game_state == GameState::Ongoing {
                let chess_move: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax(&mut chess_game, FALLBACK_DEPTH, &moves, &mut tt_net),
                    false => enm.negamax(&mut chess_game, FALLBACK_DEPTH - 1, &moves, &mut tt_enm),
                };

                history.push(chess_move);
                chess_game.update_state(chess_move);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
        None => {
            while game_state == GameState::Ongoing {
                let chess_move: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
                    true => net.negamax(&mut chess_game, FALLBACK_DEPTH, &moves, &mut tt_net),
                    false => chess_game.find_move_hce(FALLBACK_DEPTH - 1, &moves, &mut tt_enm),
                };

                history.push(chess_move);
                chess_game.update_state(chess_move);
                (moves, game_state) = chess_game.try_generate_moves();
            }
        }
    }

    let GameState::Finished(result) = game_state else { unreachable!() };
    return (result, history);
}

//TODO remove this

pub struct TrainingResultSanityTest {
    pub epoch: usize,
    pub result: GameResult,
    pub net_side: Side,
    pub pairs: Vec<(DVector<f32>, DVector<f32>)>, //(input,output)
    pub history: Vec<ChessMove>,
}

pub fn sanity_test(net: &mut ChessNet, tx: Sender<TrainingResultSanityTest>, epoch: usize) {
    let mut chess_game = ChessGame::start_pos();
    let is_net_white = rand::random_bool(0.5);
    let mut tt = TranspositionTable::new();

    let mut history: Vec<ChessMove> = Vec::new();
    let (mut moves, mut game_state) = chess_game.try_generate_moves();

    // play game
    while game_state == GameState::Ongoing {
        let chessmove = match is_net_white == (chess_game.side() == Side::White) {
            true => chess_game.find_move_hce(FALLBACK_DEPTH, &moves, &mut tt),
            false => chess_game.random_move(),
        };

        chess_game.update_state(chessmove.clone());
        history.push(chessmove);
        (moves, game_state) = chess_game.try_generate_moves();
    }

    // game ended
    let GameState::Finished(result) = game_state else { panic!() };
    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TrainingResultSanityTest =
        TrainingResultSanityTest { epoch, result, net_side, pairs: Vec::new(), history };
    _ = tx.send(return_data);
}
