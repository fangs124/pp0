use nalgebra::SVector;
use nnue::{INPUT_DIMENSION, InputType, SparseInputType, SparseVec};

use crate::{Castling, ChessGame, ChessPiece, Side, square::Square};

impl InputType for ChessGame {
    fn to_vector_white(&self) -> SVector<f32, INPUT_DIMENSION> {
        let mut vector = SVector::zeros();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector[index(chess_piece, square, Side::White)] = 1.0;
            }
        }
        return vector;
    }

    fn to_vector_black(&self) -> SVector<f32, INPUT_DIMENSION> {
        let mut vector = SVector::zeros();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector[index(chess_piece, square, Side::Black)] = 1.0;
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
                vector.push(index(chess_piece, square, Side::White));
            }
        }
        return vector;
    }

    fn to_sparse_vec_black(&self) -> SparseVec {
        let mut vector = SparseVec::new();
        let mailbox = self.mailbox();
        for &square in Square::iter() {
            if let Some(chess_piece) = mailbox.square_index(square) {
                vector.push(index(chess_piece, square, Side::Black));
            }
        }
        return vector;
    }
}

pub const fn index(chesspiece: ChessPiece, square: Square, side: Side) -> usize {
    let (piece_side, piece_type) = chesspiece.data();

    match side {
        Side::White => (piece_side as usize * 64 * 6) + (piece_type as usize * 64) + square.as_usize(),
        Side::Black => (piece_side.update() as usize * 64 * 6) + (piece_type as usize * 64) + square.as_usize_flipped(),
    }
}

//note: sub add sub add
pub const KINGSIDE_CASTLE_WHITE_INDICES_W: [usize; 4] = [
    index(ChessPiece::WK, Square::W_KING_SQUARE, Side::White),
    index(ChessPiece::WK, Square::W_KINGSIDE_CASTLE_SQUARE, Side::White),
    index(ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE, Side::White),
    index(ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_TARGET, Side::White),
];

pub const KINGSIDE_CASTLE_WHITE_INDICES_B: [usize; 4] = [
    index(ChessPiece::WK, Square::W_KING_SQUARE, Side::Black),
    index(ChessPiece::WK, Square::W_KINGSIDE_CASTLE_SQUARE, Side::Black),
    index(ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE, Side::Black),
    index(ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_TARGET, Side::Black),
];

pub const QUEENSIDE_CASTLE_WHITE_INDICES_W: [usize; 4] = [
    index(ChessPiece::WK, Square::W_KING_SQUARE, Side::White),
    index(ChessPiece::WK, Square::W_QUEENSIDE_CASTLE_SQUARE, Side::White),
    index(ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE, Side::White),
    index(ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_TARGET, Side::White),
];

pub const QUEENSIDE_CASTLE_WHITE_INDICES_B: [usize; 4] = [
    index(ChessPiece::WK, Square::W_KING_SQUARE, Side::Black),
    index(ChessPiece::WK, Square::W_QUEENSIDE_CASTLE_SQUARE, Side::Black),
    index(ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE, Side::Black),
    index(ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_TARGET, Side::Black),
];

pub const KINGSIDE_CASTLE_BLACK_INDICES_W: [usize; 4] = [
    index(ChessPiece::BK, Square::B_KING_SQUARE, Side::White),
    index(ChessPiece::BK, Square::B_KINGSIDE_CASTLE_SQUARE, Side::White),
    index(ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE, Side::White),
    index(ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_TARGET, Side::White),
];

pub const KINGSIDE_CASTLE_BLACK_INDICES_B: [usize; 4] = [
    index(ChessPiece::BK, Square::B_KING_SQUARE, Side::Black),
    index(ChessPiece::BK, Square::B_KINGSIDE_CASTLE_SQUARE, Side::Black),
    index(ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE, Side::Black),
    index(ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_TARGET, Side::Black),
];

pub const QUEENSIDE_CASTLE_BLACK_INDICES_W: [usize; 4] = [
    index(ChessPiece::BK, Square::B_KING_SQUARE, Side::White),
    index(ChessPiece::BK, Square::B_QUEENSIDE_CASTLE_SQUARE, Side::White),
    index(ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE, Side::White),
    index(ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_TARGET, Side::White),
];

pub const QUEENSIDE_CASTLE_BLACK_INDICES_B: [usize; 4] = [
    index(ChessPiece::BK, Square::B_KING_SQUARE, Side::Black),
    index(ChessPiece::BK, Square::B_QUEENSIDE_CASTLE_SQUARE, Side::Black),
    index(ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE, Side::Black),
    index(ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_TARGET, Side::Black),
];

pub fn castle_index(castle: Castling, side: Side) -> [usize; 4] {
    match (castle, side) {
        (Castling::Kingside(Side::White), Side::White) => KINGSIDE_CASTLE_WHITE_INDICES_W,
        (Castling::Kingside(Side::White), Side::Black) => KINGSIDE_CASTLE_WHITE_INDICES_B,
        (Castling::Queenside(Side::White), Side::White) => QUEENSIDE_CASTLE_WHITE_INDICES_W,
        (Castling::Queenside(Side::White), Side::Black) => QUEENSIDE_CASTLE_WHITE_INDICES_B,
        (Castling::Kingside(Side::Black), Side::White) => KINGSIDE_CASTLE_BLACK_INDICES_W,
        (Castling::Kingside(Side::Black), Side::Black) => KINGSIDE_CASTLE_BLACK_INDICES_B,
        (Castling::Queenside(Side::Black), Side::White) => QUEENSIDE_CASTLE_BLACK_INDICES_W,
        (Castling::Queenside(Side::Black), Side::Black) => QUEENSIDE_CASTLE_BLACK_INDICES_B,
    }
}
