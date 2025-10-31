use nalgebra::SVector;
use nnue::{INPUT_DIMENSION, InputType, SparseInputType, SparseVec};

use crate::{ChessGame, ChessPiece, square::Square};

impl InputType for ChessGame {
    fn to_vector_white(&self) -> SVector<f32, INPUT_DIMENSION> {
        let mut vector = SVector::zeros();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector[index::<true>(chess_piece, square)] = 1.0;
            }
        }
        return vector;
    }

    fn to_vector_black(&self) -> SVector<f32, INPUT_DIMENSION> {
        let mut vector = SVector::zeros();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector[index::<false>(chess_piece, square)] = 1.0;
            }
        }
        return vector;
    }
}

impl SparseInputType for ChessGame {
    fn to_sparse_vec_white(&self) -> SparseVec {
        let mut vector = SparseVec::new();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector.push(index::<true>(chess_piece, square));
            }
        }
        return vector;
    }

    fn to_sparse_vec_black(&self) -> SparseVec {
        let mut vector = SparseVec::new();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector.push(index::<false>(chess_piece, square));
            }
        }
        return vector;
    }
}

fn index<const IS_STM_WHITE: bool>(chesspiece: ChessPiece, square: Square) -> usize {
    let (side, piece_type) = chesspiece.data();

    match IS_STM_WHITE {
        true => (side as usize * 64 * 6) + (piece_type as usize * 64) + square.as_usize(),
        false => (side.update() as usize * 64 * 6) + (piece_type as usize * 64) + square.as_usize_flipped(),
    }
}
