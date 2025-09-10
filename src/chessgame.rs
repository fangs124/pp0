use std::{
    fmt::Display,
    i16,
    sync::Arc,
    time::{Duration, Instant},
};

use chessbb::{ChessBoard, ChessBoardSnapshot, ChessMove, ChessPiece, Evaluator, GameState, NegamaxData, PieceType, Side, Square, TranspositionTable};
use nalgebra::DVector;
use nnet::{InputType, SparseInputType, SparseVec};
use rand::random_range;

use crate::AtomicTT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChessGame {
    cb: ChessBoard,
}

impl Display for ChessGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.cb.fmt(f)
    }
}

impl InputType for ChessGame {
    fn to_vector(&self) -> DVector<f32> {
        return DVector::<f32>::from_vec(self.encode().to_vec());
    }
}

impl SparseInputType for ChessGame {
    fn to_sparse_vec(&self) -> SparseVec {
        return ChessGame::encode_sparse(&self.cb);
    }
}

impl ChessGame {
    #[inline(always)]
    pub fn start_pos() -> ChessGame {
        return ChessGame { cb: ChessBoard::start_pos() };
    }

    #[inline(always)]
    pub fn from_fen(input: &str) -> ChessGame {
        return ChessGame { cb: ChessBoard::from_fen(input) };
    }

    #[inline(always)]
    pub fn parse_move(&self, move_str: &str) -> ChessMove {
        return self.cb.parse_move(move_str);
    }

    #[inline(always)]
    pub fn make_move(&mut self, move_str: &str) {
        self.update_state(&self.cb.parse_move(move_str));
    }

    #[inline(always)]
    pub fn try_generate_moves(&self) -> (Vec<ChessMove>, GameState) {
        return self.cb.try_generate_moves();
    }

    #[inline(always)]
    pub fn update_state(&mut self, chess_move: &ChessMove) {
        self.cb.update_state(chess_move);
    }

    #[inline(always)]
    pub fn negamax(&mut self, a: Option<i16>, b: Option<i16>, d: usize, ev: &mut impl Evaluator, data: &mut NegamaxData, tt: Arc<AtomicTT>) -> i16 {
        self.cb.negamax(a.unwrap_or(i16::MIN + 1), b.unwrap_or(i16::MAX - 1), d, ev, data, tt)
    }

    #[inline(always)]
    pub fn explore_state(&mut self, chess_move: &ChessMove) -> ChessBoardSnapshot {
        self.cb.explore_state(chess_move)
    }

    #[inline(always)]
    pub fn restore_state(&mut self, snapshot: ChessBoardSnapshot) {
        self.cb.restore_state(snapshot)
    }

    pub fn iterative_deepening(
        &mut self, ev: &mut impl Evaluator, node_count: &mut usize, max_depth: Option<usize>, tt: Arc<AtomicTT>, time_limit: Duration,
    ) -> (i16, ChessMove) {
        let now = Instant::now();
        let moves: Vec<ChessMove> = self.try_generate_moves().0;
        assert!(!moves.is_empty());
        let mut best_eval: i16 = i16::MIN + 1;
        let mut best_move: ChessMove = moves[0].clone();
        let max_depth = match max_depth {
            Some(x) => x,
            None => usize::MAX,
        };

        let mut d = 1;
        while now.elapsed() < time_limit && d <= max_depth {
            best_eval = i16::MIN + 1;
            for chess_move in &moves {
                let snapshot: chessbb::ChessBoardSnapshot = self.explore_state(chess_move);
                let mut data: NegamaxData = NegamaxData::new_timed(now, time_limit);
                let eval: i16 = -self.negamax(None, Some(-best_eval), d, ev, &mut data, tt.clone());
                self.restore_state(snapshot);
                *node_count += data.node_count();

                if eval >= best_eval {
                    best_eval = eval;
                    best_move = chess_move.clone();
                }
            }

            d += 1;
        }
        return (best_eval, best_move);
    }

    #[inline(always)]
    pub fn find_move(
        &mut self, ev: &mut impl Evaluator, d: usize, node_count: &mut usize, moves: Vec<ChessMove>, tt: Arc<AtomicTT>, time_limit: Option<Duration>,
    ) -> (i16, ChessMove) {
        assert!(moves.len() > 0);

        let mut best_move: ChessMove = moves[0].clone();
        let mut data: NegamaxData = match time_limit {
            Some(time_limit) => NegamaxData::new_timed(Instant::now(), time_limit),
            None => NegamaxData::new(),
        };

        let mut best_score: i16 = i16::MIN + 1;
        for chess_move in moves {
            let snapshot: chessbb::ChessBoardSnapshot = self.explore_state(&chess_move);
            let score: i16 = -self.negamax(None, Some(-best_score), d, ev, &mut data, tt.clone());
            self.restore_state(snapshot);

            if score >= best_score {
                best_move = chess_move;
                best_score = score;
            }
        }

        *node_count = data.node_count();
        return (best_score, best_move);
    }

    #[inline(always)]
    pub fn side(&self) -> Side {
        self.cb.side()
    }

    #[inline(always)]
    fn encode(&self) -> [f32; 768] {
        //position is always encoded from active side's presepctive
        return ChessGame::encode_raw(&self.cb);
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
        return moves[random_range(0..moves.len())];
    }

    #[inline(always)]
    pub(crate) fn vectorize(cb: &ChessBoard) -> DVector<f32> {
        return DVector::<f32>::from_vec(ChessGame::encode_raw(&cb).to_vec());
    }

    fn encode_raw(cb: &ChessBoard) -> [f32; 768] {
        //position is always encoded from active side's presepctive
        let mut input_data: [f32; 768] = [0.0; 768];
        for (chess_piece, i) in cb.mailbox_iterator().zip(0usize..64) {
            if let Some(chess_piece) = chess_piece {
                match cb.side() {
                    Side::White => {
                        input_data[ChessGame::index(*chess_piece, Square::nth(i))] = 1.0;
                    }
                    Side::Black => {
                        input_data[ChessGame::index_flip(*chess_piece, Square::nth(i))] = 1.0;
                    }
                }
            }
        }
        return input_data;
    }

    #[inline(always)]
    pub fn vectorize_sparse(cb: &ChessBoard) -> SparseVec {
        return ChessGame::encode_sparse(cb);
    }

    fn encode_sparse(cb: &ChessBoard) -> SparseVec {
        let mut output = SparseVec::with_capacity(32);
        //position is always encoded from active side's presepctive
        for (chess_piece, i) in cb.mailbox_iterator().zip(0usize..64) {
            if let Some(chess_piece) = chess_piece {
                match cb.side() {
                    Side::White => {
                        output.push(ChessGame::index(*chess_piece, Square::nth(i)));
                    }
                    Side::Black => {
                        output.push(ChessGame::index_flip(*chess_piece, Square::nth(i)));
                    }
                }
            }
        }
        return output;
    }
}
