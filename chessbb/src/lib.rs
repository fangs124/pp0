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

#[cfg(feature = "nnue")]
mod nnue;

/* re-export */
//pub use crate::bitboard::{ChessPiece, PieceType, Side};
//pub use crate::chessmove::{ChessMove, LexiOrd};
//pub use crate::search::{Evaluator, MATERIAL_EVAL, MaterialEvaluator, NegamaxData};
//pub use crate::square::Square;
//pub use crate::transposition::{
//    AtomicTranspositionTable, NodeType, PositionData, TranspositionTable,
//};

pub use crate::bitboard::Bitboard;
pub use crate::chessboard::zobrist::ZobristHash;
pub use crate::chessboard::{ChessBoard, ChessBoardSnapshot, ChessGame, GameResult, GameState, MoveList};

pub use crate::chessmove::{Castling, ChessMove, LexiOrd, MoveType};
pub use crate::chesspiece::{ChessPiece, PieceType, Side};

#[cfg(feature = "nnue")]
pub use crate::nnue::{castle_index, index};
