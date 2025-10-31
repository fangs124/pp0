use std::{
    sync::{
        Arc,
        mpsc::{SendError, Sender},
    },
    time::Duration,
};

use chessbb::{ChessGame, ChessMove, GameResult, GameState, Side};
use nnue::SparseVec;
use pp0::{Evaluator, SearchData, TranspositionTable};

use crate::player::{Epoch, Player};

pub const EPS: f64 = 0.005;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub result: GameResult,
    pub p1_side: Side,
    pub node_count: usize,
    //pub history: Option<ArrayVec<ChessMove,SIZE>>,
    pub pairs: Option<Vec<((SparseVec, SparseVec), i16)>>, //(input,output) oldest first, last element is the (net's) last move of the game
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairResult {
    pub result1: MatchResult,
    pub result2: MatchResult,
    pub epoch: Epoch,
}

const BASE_TIME: Duration = Duration::from_secs(5);
const INCREMENT_TIME: Duration = Duration::from_millis(5);

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

pub fn play<const COLLECT_PAIRS: bool>(player1: &mut Player, player2: &mut Player, fen: Option<&str>, p1_white: bool) -> MatchResult {
    //net is white
    let mut chessgame: ChessGame = match fen {
        Some(fen) => ChessGame::from_fen(&fen),
        None => ChessGame::start_pos(),
    };

    player1.evaluator.initialize(&chessgame);
    player2.evaluator.initialize(&chessgame);

    let (mut moves, mut game_state) = chessgame.try_generate_moves();
    assert!(!moves.is_empty());

    let mut pairs: Option<Vec<((SparseVec, SparseVec), i16)>> = match COLLECT_PAIRS {
        true => Some(Vec::new()),
        false => None,
    };

    let tt1: Arc<TranspositionTable> = Arc::new(TranspositionTable::new());
    let tt2: Arc<TranspositionTable> = Arc::new(TranspositionTable::new());

    let mut node_count_p1_total: usize = 0;

    // play game
    let result: GameResult = loop {
        if let GameState::Finished(result) = game_state {
            break result;
        }

        let side: Side = chessgame.side();

        let mut search_data: SearchData = match COLLECT_PAIRS {
            true => SearchData::new_collect_pairs(),
            false => SearchData::new(),
        };

        let chess_move: ChessMove = match p1_white == (side == Side::White) {
            true => {
                let chess_move = search_data.find_move(&mut chessgame, &mut player1.evaluator, tt1.clone(), &player1.search_limit, moves);
                node_count_p1_total += search_data.node_count();
                if COLLECT_PAIRS {
                    if let Some(pairs) = &mut pairs {
                        pairs.push(search_data.pairs().unwrap());
                    }
                }
                chess_move
            }
            false => {
                let chess_move = search_data.find_move(&mut chessgame, &mut player2.evaluator, tt2.clone(), &player2.search_limit, moves);
                chess_move
            }
        };

        chessgame.update_state(&chess_move);
        (moves, game_state) = chessgame.try_generate_moves();
    };

    let net_side: Side = match p1_white {
        true => Side::White,
        false => Side::Black,
    };

    MatchResult { result, p1_side: net_side, node_count: node_count_p1_total, pairs }
}
