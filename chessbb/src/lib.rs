#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod bitboard;
mod chessboard;
mod chessmove;
mod chesspiece;
mod square;

/* re-export */
//pub use crate::bitboard::{ChessPiece, PieceType, Side};
//pub use crate::chessmove::{ChessMove, LexiOrd};
//pub use crate::search::{Evaluator, MATERIAL_EVAL, MaterialEvaluator, NegamaxData};
//pub use crate::square::Square;
//pub use crate::transposition::{
//    AtomicTranspositionTable, NodeType, PositionData, TranspositionTable,
//};

pub use crate::bitboard::Bitboard;
pub use crate::chessboard::{ChessBoard, ChessGame};
pub use crate::chessmove::{ChessMove, LexiOrd};
pub use crate::chesspiece::{ChessPiece, PieceType, Side};
