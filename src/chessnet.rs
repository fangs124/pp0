use chessbb::{ChessBoard, ChessBoardCore, ChessMove, ChessPiece, Evaluator, GameResult, GameState, PieceType, Side, Square, MATERIAL_EVAL};
use nalgebra::DVector;
use rand::random_range;
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

impl Evaluator for ChessNet {
    fn eval(&mut self, cb: &ChessBoard) -> i32 {
        self.net.forward_prop(&ChessGame {cb: cb.clone()});
        return (self.phi_z()[0] * 1000.0) as i32;
    }
}

impl ChessGame {
    #[inline(always)]
    pub const fn start_pos() -> ChessGame {
        return ChessGame { cb: ChessBoard::start_pos() };
    }

    #[inline(always)]
    pub fn from_fen(input: &str) -> ChessGame {
        return ChessGame { cb: ChessBoard::from_fen(input) };
    }

    #[inline(always)]
    pub fn try_generate_moves(&self) -> (Vec<ChessMove>, GameState) {
        return self.cb.try_generate_moves();
    }

    //#[inline(always)]
    //pub fn state(&self) -> GameState {
    //    return self.cb.state();
    //}
    
    #[inline(always)]
    pub fn update_state(&mut self, chess_move: ChessMove) {
        self.cb.update_state(chess_move);
    }
    
    #[rustfmt::skip]
    #[inline(always)]
    pub fn negamax(&mut self, a: i32, b: i32, d: usize, net: &mut ChessNet) -> (i32, Option<ChessMove>) {
        return self.cb.negamax(a, b, d, 0,net);
    }

    #[inline(always)]
    pub fn side(&self) -> Side {
        self.cb.side()
    }

    fn encode(&self) -> [f32; 768] {
        //position is always encoded from active side's presepctive
        let mut input_data: [f32; 768] = [0.0; 768];
        for (chesspiece, i) in self.cb.mailbox_iterator().zip(0usize..64) {
            if let Some(chesspiece) = chesspiece {
                match self.side() {
                    Side::White => {
                        input_data[ChessGame::index(*chesspiece, Square::nth(i))] = 1.0;
                    },
                    Side::Black => {
                        input_data[ChessGame::index_flip(*chesspiece,Square::nth(i))] = 1.0;
                    },
                }
                
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

    fn index_flip(chesspiece: ChessPiece, square: Square) -> usize {
        let side = match chesspiece.0 {
            Side::White => 1,
            Side::Black => 0,
        };
        let piece_type = match chesspiece.1 {
            PieceType::King => 0,
            PieceType::Queen => 1,
            PieceType::Knight => 2,
            PieceType::Bishop => 3,
            PieceType::Rook => 4,
            PieceType::Pawn => 5,
        };
        return (side * 64 * 6) + (piece_type * 64) + Square::nth_flipped(square.to_usize()).to_usize();
    }

    #[inline(always)]
    pub fn random_move(&self) -> ChessMove {
        let moves = self.cb.try_generate_moves().0;
        assert!(moves.len() > 0);
        return moves[random_range(0..moves.len())]
    }

}

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
    
    pub fn negamax_learn(&mut self, cg: &ChessGame, d: usize, ins: &mut Vec<DVector<f32>>,  outs: &mut Vec<DVector<f32>>, moves: Vec<ChessMove>) -> ChessMove {
        ins.push(cg.to_vector());
        outs.push(self.eval(&cg));
        return self.negamax(cg, d, moves);
    }
   
 

    pub fn negamax_cold(&mut self, cg: &ChessGame, d: usize) -> ChessMove {
        //TODO safety of the return result?
        cg.clone().cb.negamax(i32::MIN + 1, i32::MAX - 1, d, 0,self).1.unwrap()
    }

    //TODO remove this
    //pub fn negamax_sanity_test(&mut self, cg: &ChessGame, d: usize) -> ChessMove {
    //    cg.clone().cb.negamax(i32::MIN + 1, i32::MAX - 1, d, 0,&mut MATERIAL_EVAL).1.unwrap()
    //}

    pub fn negamax(&mut self, cg: &ChessGame, d: usize, moves: Vec<ChessMove>) ->ChessMove {
        assert!(!moves.is_empty());
        let mut alpha:i32 = i32::MIN + 1;
        let beta:i32 = i32::MAX - 1;
        let mut best_move: ChessMove = moves[0].clone(); 
        let mut chess_game = cg.clone();
        for chess_move in moves {
            //let old_core: ChessBoardCore = chess_game.cb.core.clone();
            let snapshot = chess_game.cb.explore_state(chess_move);
            let (neg_score, next_move) = chess_game.cb.negamax(-beta, -alpha, d-1, 1,self);
            let score = -neg_score;
            chess_game.cb.restore_state(snapshot);

            if score > alpha {
                alpha = score;
                best_move = chess_move;
            }
        }
        return best_move;
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
            let target_output = DVector::from_element(1, reward);
            
            let grad = self.back_prop_vector(input, target_output, scaled_reward);
            self.update(grad, -LEARNING_RATE);
            ith_move += 2;
        }
    }

    pub fn update(&mut self, grad: Gradient, r: f32) {
        self.net.update(grad, r);
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