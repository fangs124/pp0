use chessbb::{ChessBoard, ChessGame, ChessPiece, PieceType, Side, Square};
use nalgebra::DVector;

use crate::nnets::*;

//alpha-beta nets
pub struct ChessNet {
    net: Network<ChessGame>,
    chessgame: ChessGame,
}
impl ChessNet {
    fn encode(&self) -> [f32; 768] {
        //position is always encoded from active side's presepctive
        let mut input_data: [f32; 768] = [0.0; 768];
        for (chesspiece, i) in self.chessgame.chessboard.mailbox_iterator().zip(0u8..64) {
            if let Some(chesspiece) = chesspiece {
                input_data[index(*chesspiece, Square::new(i))] = 1.0;
            }
        }
        return input_data;
    }
    //TODO
    //pub fn new() -> Self
}

impl InputType for ChessNet {
    fn to_vector(&self) -> DVector<f32> {
        return DVector::<f32>::from_vec(self.encode().to_vec());
    }
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
