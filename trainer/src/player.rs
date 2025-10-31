use std::{
    f32::consts::E,
    num::NonZero,
    time::{Duration, Instant},
};

use chessbb::ChessBoard;
use nnue::Network;
use pp0::{Evaluator, MaterialEvaluator, SearchLimit, StaticEvaluator};

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub evaluator: PlayerEvaluator,
    pub search_limit: SearchLimit,
}

impl Player {
    pub fn new(evaluator: PlayerEvaluator, search_limit: SearchLimit) -> Player {
        Player { evaluator, search_limit }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvaluator {
    Network(Network),
    StaticEval(StaticEvaluator),
    MaterialEvaluator(MaterialEvaluator),
}

impl Evaluator for PlayerEvaluator {
    fn eval(&mut self, chessgame: &chessbb::ChessGame) -> i16 {
        match self {
            PlayerEvaluator::Network(network) => <Network as Evaluator>::eval(network, chessgame),
            PlayerEvaluator::StaticEval(static_evaluator) => static_evaluator.eval(chessgame),
            PlayerEvaluator::MaterialEvaluator(material_evaluator) => material_evaluator.eval(chessgame),
        }
    }

    fn update(&mut self, chessgame: &chessbb::ChessGame, chessmove: &chessbb::ChessMove) {
        match self {
            PlayerEvaluator::Network(network) => <Network as Evaluator>::update(network, chessgame, chessmove),
            PlayerEvaluator::StaticEval(static_evaluator) => static_evaluator.update(chessgame, chessmove),
            PlayerEvaluator::MaterialEvaluator(material_evaluator) => material_evaluator.update(chessgame, chessmove),
        }
    }

    fn revert(&mut self, chessgame: &chessbb::ChessGame, chessmove: &chessbb::ChessMove) {
        match self {
            PlayerEvaluator::Network(network) => <Network as Evaluator>::revert(network, chessgame, chessmove),
            PlayerEvaluator::StaticEval(static_evaluator) => static_evaluator.revert(chessgame, chessmove),
            PlayerEvaluator::MaterialEvaluator(material_evaluator) => material_evaluator.revert(chessgame, chessmove),
        }
    }

    fn initialize(&mut self, chessgame: &chessbb::ChessGame) {
        match self {
            PlayerEvaluator::Network(network) => <Network as Evaluator>::initialize(network, chessgame),
            PlayerEvaluator::StaticEval(static_evaluator) => static_evaluator.initialize(chessgame),
            PlayerEvaluator::MaterialEvaluator(material_evaluator) => material_evaluator.initialize(chessgame),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Epoch(pub u16);
