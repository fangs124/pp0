use std::{
    sync::{Arc, mpsc::Sender},
    time::{Duration, Instant},
};

use crate::{AtomicTT, BASE_COEFF, BASE_TIME, ChessGame, ChessNet, FALLBACK_DEPTH, FIXED_NODE_LIMIT, INCREMENT_COEFF, INCREMENT_TIME, STUNTED_FALLBACK_DEPTH};
use chessbb::{AtomicTranspositionTable, ChessMove, GameResult, GameState, MATERIAL_EVAL, Side, TranspositionTable};
use nnet::SparseVec;
use rand::{random_bool, random_range, seq::SliceRandom};

pub const EPS: f64 = 0.1;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainingResult {
    pub epoch: usize,
    pub result: GameResult,
    pub net_side: Side,
    pub node_count: usize,
    //pub history: Option<Vec<ChessMove>>,
    pub pairs: Vec<(SparseVec, i16)>, //(input,output)
}

type TR = TrainingResult;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeControl {
    base: Duration,
    increment: Duration,
}

impl TimeControl {
    pub const fn new(base: Duration, increment: Duration) -> TimeControl {
        TimeControl { base, increment }
    }
}

impl Default for TimeControl {
    fn default() -> Self {
        Self { base: BASE_TIME, increment: INCREMENT_TIME }
    }
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
    let tt_net: Arc<AtomicTT> = Arc::new(AtomicTranspositionTable::new());
    let tt_enm: Arc<AtomicTT> = Arc::new(AtomicTranspositionTable::new());
    let mut node_count_net: usize = 0;
    let mut node_count_enm: usize = 0;
    let (find_move_net, find_move_enm) = parse_param(enm.is_some(), param);

    let mut white_tc: Option<Duration> = match &param.tc {
        Some(tc) => Some(tc.base),
        None => None,
    };

    let mut black_tc: Option<Duration> = match &param.tc {
        Some(tc) => Some(tc.base),
        None => None,
    };

    // play game
    let result = loop {
        if let GameState::Finished(result) = game_state {
            break result;
        }
        let side = chess_game.side();

        let time_limit: Option<Duration> = match &param.tc {
            None => None,
            Some(tc) => match side {
                Side::White => Some((white_tc.unwrap() / BASE_COEFF) + (tc.increment / INCREMENT_COEFF)),
                Side::Black => Some((black_tc.unwrap() / BASE_COEFF) + (tc.increment / INCREMENT_COEFF)),
            },
        };

        let now = Instant::now();
        let chess_move: ChessMove = match is_net_white == (side == Side::White) {
            true => find_move_net(&mut net, &mut chess_game, &mut node_count_net, &mut ins, &mut outs, moves, tt_net.clone(), time_limit),
            false => find_move_enm(&mut enm, &mut chess_game, &mut node_count_enm, moves, tt_enm.clone(), time_limit),
        };

        if let Some(tc) = &param.tc {
            match side {
                Side::White => white_tc = Some((white_tc.unwrap() + tc.increment).checked_sub(now.elapsed()).unwrap_or(Duration::ZERO)),
                Side::Black => black_tc = Some((black_tc.unwrap() + tc.increment).checked_sub(now.elapsed()).unwrap_or(Duration::ZERO)),
            }
        }

        chess_game.update_state(&chess_move);
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
    fn(&mut ChessNet, &mut ChessGame, node_count: &mut usize, &mut Vec<SparseVec>, &mut Vec<i16>, Vec<ChessMove>, Arc<AtomicTT>, Option<Duration>) -> ChessMove;

type EnmFindMove = fn(&mut Option<ChessNet>, &mut ChessGame, node_count: &mut usize, Vec<ChessMove>, Arc<AtomicTT>, Option<Duration>) -> ChessMove;

fn parse_param(enm_is_some: bool, param: &PlayParameter) -> (NetFindMove, EnmFindMove) {
    let find_move_net: fn(
        &mut ChessNet,
        &mut ChessGame,
        &mut usize,
        &mut Vec<SparseVec>,
        &mut Vec<i16>,
        Vec<ChessMove>,
        Arc<AtomicTT>,
        Option<Duration>,
    ) -> ChessMove = match param.is_learn {
        true => |net: &mut ChessNet,
                 chess_game: &mut ChessGame,
                 node_count: &mut usize,
                 ins: &mut Vec<SparseVec>,
                 outs: &mut Vec<i16>,
                 moves: Vec<ChessMove>,
                 tt_net: Arc<AtomicTT>,
                 time_limit: Option<Duration>| {
            return net.learn(chess_game, FALLBACK_DEPTH, node_count, ins, outs, moves, tt_net, time_limit);
        },
        false => |net: &mut ChessNet,
                  chess_game: &mut ChessGame,
                  node_count: &mut usize,
                  _ins: &mut Vec<SparseVec>,
                  _outs: &mut Vec<i16>,
                  moves: Vec<ChessMove>,
                  tt_net: Arc<AtomicTT>,
                  time_limit: Option<Duration>| {
            return chess_game.iterative_deepening(net, node_count, Some(moves), tt_net, time_limit, Some(FIXED_NODE_LIMIT)).1;
        },
    };

    let find_move_enm: fn(&mut Option<ChessNet>, &mut ChessGame, &mut usize, Vec<ChessMove>, Arc<AtomicTT>, Option<Duration>) -> ChessMove =
        match (enm_is_some, param.is_learn) {
            (true, _) => |enm: &mut Option<ChessNet>,
                          chess_game: &mut ChessGame,
                          node_count: &mut usize,
                          moves: Vec<ChessMove>,
                          tt_enm: Arc<AtomicTT>,
                          time_limit: Option<Duration>| {
                epsilon(EPS, moves, |moves| chess_game.find_move(enm.as_mut().unwrap(), STUNTED_FALLBACK_DEPTH, node_count, moves, tt_enm, time_limit).1)
            },

            (false, true) => |_enm: &mut Option<ChessNet>,
                              chess_game: &mut ChessGame,
                              node_count: &mut usize,
                              mut moves: Vec<ChessMove>,
                              tt_enm: Arc<AtomicTT>,
                              _time_limit: Option<Duration>| {
                moves.shuffle(&mut rand::rng());
                epsilon(EPS, moves, |moves| chess_game.find_move(&mut MATERIAL_EVAL, STUNTED_FALLBACK_DEPTH, node_count, moves, tt_enm, None).1)
            },

            (false, false) => |_enm: &mut Option<ChessNet>,
                               chess_game: &mut ChessGame,
                               node_count: &mut usize,
                               mut moves: Vec<ChessMove>,
                               tt_enm: Arc<AtomicTT>,
                               _time_limit: Option<Duration>| {
                moves.shuffle(&mut rand::rng());
                chess_game.find_move(&mut MATERIAL_EVAL, STUNTED_FALLBACK_DEPTH, node_count, moves, tt_enm, None).1
            },
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
