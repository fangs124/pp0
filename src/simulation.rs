use std::{sync::mpsc::Sender, time::Duration};

use crate::{ChessGame, ChessNet, FALLBACK_DEPTH, STUNTED_FALLBACK_DEPTH};
use chessbb::{ChessMove, GameResult, GameState, MATERIAL_EVAL, Side, TranspositionTable};
use nnet::SparseVec;
use rand::{random_bool, random_range, seq::SliceRandom};

pub const EPS: f64 = 0.2;
pub struct TrainingResult {
    pub epoch: usize,
    pub result: GameResult,
    pub net_side: Side,
    pub node_count: usize,
    //pub history: Option<Vec<ChessMove>>,
    pub pairs: Vec<(SparseVec, i16)>, //(input,output)
}

type TR = TrainingResult;

pub struct PlayParameter {
    epoch: usize,
    is_learn: bool,
    fen: Option<String>,
    tc: Option<TimeControl>,
}

impl PlayParameter {
    #[rustfmt::skip]
    pub fn new(epoch: usize, is_learn: bool, fen: Option<String>, tc: Option<TimeControl>) -> Self{
        PlayParameter { epoch, is_learn, fen, tc, }
    }
}

pub struct TimeControl {
    base: Duration,
    increment: Duration,
}

pub fn play(mut net: ChessNet, mut enm: Option<ChessNet>, tx: Sender<TR>, param: &PlayParameter) {
    let is_net_white = rand::random_bool(0.5);
    let mut chess_game: ChessGame = match &param.fen {
        Some(fen) => ChessGame::from_fen(fen),
        None => ChessGame::start_pos(),
    };
    let (mut moves, mut game_state) = chess_game.try_generate_moves();
    assert!(!moves.is_empty());

    let mut ins: Vec<SparseVec> = Vec::new();
    let mut outs: Vec<i16> = Vec::new();
    let mut tt_net: TranspositionTable = TranspositionTable::new();
    let mut tt_enm: TranspositionTable = TranspositionTable::new();
    let mut node_count_net: usize = 0;
    let mut node_count_enm: usize = 0;
    let (find_move_net, find_move_enm) = parse_param(enm.is_some(), param);

    // play game
    let result = loop {
        if let GameState::Finished(result) = game_state {
            break result;
        }

        let chess_move: ChessMove = match is_net_white == (chess_game.side() == Side::White) {
            true => find_move_net(&mut net, &mut chess_game, &mut node_count_net, &mut ins, &mut outs, moves, &mut tt_net),
            false => find_move_enm(&mut enm, &mut chess_game, &mut node_count_enm, moves, &mut tt_enm),
        };

        chess_game.update_state(chess_move);
        (moves, game_state) = chess_game.try_generate_moves();
    };

    let pairs: Vec<(SparseVec, i16)> = ins.into_iter().zip(outs).collect();
    let net_side: Side = match is_net_white {
        true => Side::White,
        false => Side::Black,
    };

    let return_data: TR = TR { epoch: param.epoch, result, net_side, node_count: node_count_net, pairs };
    _ = tx.send(return_data);
}

type NetFindMove =
    fn(&mut ChessNet, &mut ChessGame, node_count: &mut usize, &mut Vec<SparseVec>, &mut Vec<i16>, Vec<ChessMove>, &mut TranspositionTable) -> ChessMove;

type EnmFindMove = fn(&mut Option<ChessNet>, &mut ChessGame, node_count: &mut usize, Vec<ChessMove>, &mut TranspositionTable) -> ChessMove;

fn parse_param(enm_is_some: bool, param: &PlayParameter) -> (NetFindMove, EnmFindMove) {
    let find_move_net: fn(&mut ChessNet, &mut ChessGame, &mut usize, &mut Vec<SparseVec>, &mut Vec<i16>, Vec<ChessMove>, &mut TranspositionTable) -> ChessMove =
        match param.is_learn {
            true => |net: &mut ChessNet,
                     chess_game: &mut ChessGame,
                     node_count: &mut usize,
                     ins: &mut Vec<SparseVec>,
                     outs: &mut Vec<i16>,
                     moves: Vec<ChessMove>,
                     tt_net: &mut TranspositionTable| {
                return net.learn(chess_game, FALLBACK_DEPTH, node_count, ins, outs, moves, tt_net);
            },

            false => |net: &mut ChessNet,
                      chess_game: &mut ChessGame,
                      node_count: &mut usize,
                      _ins: &mut Vec<SparseVec>,
                      _outs: &mut Vec<i16>,
                      moves: Vec<ChessMove>,
                      tt_net: &mut TranspositionTable| {
                return net.find_move(chess_game, FALLBACK_DEPTH, node_count, moves, tt_net);
            },
        };

    let find_move_enm: fn(&mut Option<ChessNet>, &mut ChessGame, &mut usize, Vec<ChessMove>, &mut TranspositionTable) -> ChessMove =
        match (enm_is_some, param.is_learn) {
            (true, _) => {
                |enm: &mut Option<ChessNet>, chess_game: &mut ChessGame, node_count: &mut usize, moves: Vec<ChessMove>, tt_enm: &mut TranspositionTable| {
                    epsilon(EPS, moves, |moves| ChessNet::find_move(enm.as_mut().unwrap(), chess_game, STUNTED_FALLBACK_DEPTH, node_count, moves, tt_enm))
                }
            }

            (false, true) => {
                |_enm: &mut Option<ChessNet>, chess_game: &mut ChessGame, node_count: &mut usize, mut moves: Vec<ChessMove>, tt_enm: &mut TranspositionTable| {
                    moves.shuffle(&mut rand::rng());
                    epsilon(EPS, moves, |moves| chess_game.find_move(STUNTED_FALLBACK_DEPTH, &mut MATERIAL_EVAL, node_count, moves, tt_enm))
                }
            }

            (false, false) => {
                |_enm: &mut Option<ChessNet>, chess_game: &mut ChessGame, node_count: &mut usize, mut moves: Vec<ChessMove>, tt_enm: &mut TranspositionTable| {
                    moves.shuffle(&mut rand::rng());
                    chess_game.find_move(STUNTED_FALLBACK_DEPTH, &mut MATERIAL_EVAL, node_count, moves, tt_enm)
                }
            }
        };
    return (find_move_net, find_move_enm);
}

pub fn epsilon(p: f64, moves: Vec<ChessMove>, f: impl FnOnce(Vec<ChessMove>) -> ChessMove) -> ChessMove {
    assert!(!moves.is_empty());
    match random_bool(p) {
        true => moves[random_range(0..moves.len())],
        false => f(moves),
    }
}
