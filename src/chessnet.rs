use chessbb::{ChessBoard, ChessMove, ChessPiece, GameResult, GameState, PieceType, Side, Square};
use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use crate::{nnet::*, simulation::TrainingResult, LEARNING_RATE};
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChessGame {
    cb: ChessBoard,
}

//alpha-beta nets
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChessNet {
    pub net: Network<ChessGame>,
    pub version: u32,
}

impl InputType for ChessGame {
    fn to_vector(&self) -> DVector<f32> {
        return DVector::<f32>::from_vec(self.encode().to_vec());
    }
}

impl ChessGame {
    #[inline(always)]
    pub fn start_pos() -> ChessGame {
        return ChessGame { cb: ChessBoard::start_pos() };
    }

    #[inline(always)]
    pub fn new() -> ChessGame {
        return ChessGame { cb: ChessBoard::start_pos() };
    }

    #[inline(always)]
    pub fn try_generate_moves(&self) -> (Vec<ChessMove>, GameState) {
        return self.cb.try_generate_moves();
    }

    #[inline(always)]
    pub fn state(&self) -> GameState {
        return self.cb.state();
    }
    
    #[inline(always)]
    pub fn update_state(&mut self, chessmove: ChessMove) {
        self.cb.update_state(chessmove);
    }
    
    #[rustfmt::skip]
    #[inline(always)]
    pub fn negamax(&mut self, a: i32, b: i32, d: usize, eval: impl FnMut(&ChessBoard) -> i32) -> (i32, Option<ChessMove>) {//
        return self.cb.negamax(a, b, d, eval);
    }

    #[inline(always)]
    pub fn side(&self) -> Side {
        self.cb.side()
    }

    fn encode(&self) -> [f32; 768] {
        //position is always encoded from active side's presepctive
        let mut input_data: [f32; 768] = [0.0; 768];
        for (chesspiece, i) in self.cb.mailbox_iterator().zip(0u8..64) {
            if let Some(chesspiece) = chesspiece {
                input_data[ChessGame::index(*chesspiece, Square::new(i))] = 1.0;
            }
        }
        return input_data;
    }

    fn index(chesspiece: ChessPiece, square: Square) -> usize {
        let side = match chesspiece.0 {
            Side::White => 0,
            Side::Black => 1,
        };
        let piece_type = match chesspiece.1 {
            PieceType::King => 0,
            PieceType::Queen => 1,
            PieceType::Knight => 2,
            PieceType::Bishop => 3,
            PieceType::Rook => 4,
            PieceType::Pawn => 5,
        };
        return (side * 64 * 6) + (piece_type * 64) + square.to_usize();
    }
}

impl ChessNet {
    #[inline(always)]
    pub fn new(node_counts: Vec<usize>) -> Self {
        let input_dim = ChessGame::new().to_vector().len();
        ChessNet { net: Network::new(input_dim, node_counts), version: 0 }
    }

    #[inline(always)]
    pub fn eval(&mut self, cg: &ChessGame) -> DVector<f32> {
        self.net.forward_prop(cg);
        return self.net.phi_z_vector();
    }

    #[inline(always)]
    pub fn back_prop(&mut self, cg: &ChessGame, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop(cg, target, r)
    }

    #[inline(always)]
    pub fn back_prop_vector(&mut self,  input: DVector<f32>, target: DVector<f32>, r: f32) -> Gradient {
        self.net.backward_prop_vector(input, target, r)
    }


    #[inline(always)]
    pub fn phi_z(&self) -> Vec<f32> {
        self.net.phi_z()
    }
    
    pub fn negamax_learn(&mut self, cg: &ChessGame, d: usize, ins: &mut Vec<DVector<f32>>,  outs: &mut Vec<DVector<f32>>) -> ChessMove {
        //TODO determine if clone is necessary here
        //TODO safety of the return result?
        ins.push(cg.to_vector());
        outs.push(self.eval(&cg));
        cg.clone().negamax(i32::MIN + 1, i32::MAX - 1, d, self.get_eval_fn()).1.unwrap()
    }
   

    pub fn negamax(&mut self, cg: &ChessGame, d: usize) -> ChessMove {
        //TODO determine if clone is necessary here
        //TODO safety of the return result?
        cg.clone().negamax(i32::MIN + 1, i32::MAX - 1, d, self.get_eval_fn()).1.unwrap()
    }

     pub fn get_eval_fn(&self) -> impl FnMut(&ChessBoard) -> i32 + use<> {
        let mut net = self.net.clone();
        move |cb: &ChessBoard| {
            net.forward_prop(&ChessGame { cb: *cb });
            let output = net.phi_z();
            return (output[0] * 1000.0) as i32;
        }
    }

     pub fn process_training_result(&mut self, data: TrainingResult) {
        let total_moves = data.pairs.len();
        let reward: f32 = match (data.net_side, data.result) {
            (Side::White, GameResult::WhiteWins) | (Side::Black, GameResult::BlackWins) => 1.0,
            (Side::White, GameResult::BlackWins) | (Side::Black, GameResult::WhiteWins) => -1.0,
            (_, GameResult::Draw) => 0.1,
        };

        let mut ith_move: usize = match data.net_side {
            Side::White => 0,
            Side::Black => 1,
        };
        
        /* maybe isolate this? */
        for (input, output) in data.pairs {
            let scaled_reward = reward * compute_scalar(ith_move, total_moves);
            let target_output = DVector::from_element(1, 1.0);
            
            let grad = self.back_prop_vector(input, target_output, scaled_reward);
            self.update(grad, -LEARNING_RATE);
            ith_move += 2;
        }
    }


    //pub fn process_training_result(&mut self, data: TrainingResult) {
    //    let total_moves = data.pairs.len();
    //    let reward: f32 = match (data.net_side, data.result) {
    //        (Side::X, GameResult::XWin) | (Side::O, GameResult::OWin) => 1.0,
    //        (Side::X, GameResult::OWin) | (Side::O, GameResult::XWin) => -1.0,
    //        (_, GameResult::Tie) => 0.1,
    //    };
    //
    //    let mut ith_move: usize = match data.net_side {
    //        Side::X => 0,
    //        Side::O => 1,
    //    };
    //    /* maybe isolate this? */
    //    for (game_move, state) in data.pairs {
    //        let scaled_reward = reward * compute_scalar(ith_move, total_moves);
    //        let mut target_probability = DVector::from_element(9, 0.0);
    //        let square_index = BitBoard::index(&game_move);
    //        if scaled_reward >= 0.0 {
    //            target_probability[square_index] = 1.0;
    //        } else {
    //            let other_prob = 1.0 / ((9 - ith_move - 1) as f32);
    //            for test_move in BitBoard::MOVES {
    //                if state.is_valid_move(&test_move) && (test_move != game_move) {
    //                    target_probability[BitBoard::index(&test_move)] = other_prob;
    //                }
    //            }
    //        }
    //        //net.dumb_backward_prop(&state, target_probability, scaled_reward);
    //        let grad = self.back_prop_pi(&state, target_probability, scaled_reward);
    //        self.update(grad, -LEARNING_RATE);
    //        ith_move += 2;
    //    }
    //}

    pub fn update(&mut self, grad: Gradient, r: f32) {
        self.net.update(grad, r);
    }

    pub fn update_sum(&mut self, pairs: &mut Vec<(Gradient, f32)>) {
        self.net.update_sum(pairs);
    }

    //greedy
    //pub fn get_move(&mut self, gb: &GameBoard) -> BitBoard {
    //    self.net.forward_prop(gb);
    //    let output = get_index(&self.pi(), gb);
    //    return BitBoard::MOVES[output];
    //}
    //
    ////epsilon-greedy
    //pub fn get_move_eps(&mut self, gb: &GameBoard) -> BitBoard {
    //    let is_play_random = random_bool(EPS);
    //    if !is_play_random {
    //        self.net.forward_prop(gb);
    //        let output = get_index(&self.pi(), gb);
    //        return BitBoard::MOVES[output];
    //    }
    //
    //    let valid_moves: Vec<BitBoard> =
    //        BitBoard::MOVES.to_vec().into_iter().filter(|bb| gb.is_valid_move(bb)).collect();
    //    let i: usize = random_range(0..valid_moves.len());
    //    return valid_moves[i];
    //}
    //
    ////random
    //pub fn get_move_rand(&mut self, gb: &GameBoard) -> BitBoard {
    //    let valid_moves: Vec<BitBoard> =
    //        BitBoard::MOVES.to_vec().into_iter().filter(|bb| gb.is_valid_move(bb)).collect();
    //    let i: usize = random_range(0..valid_moves.len());
    //    return valid_moves[i];
    //}
}

fn compute_scalar(index: usize, total: usize) -> f32 {
    0.5 + (0.5 * (((index) as f32) / (total as f32)))
}