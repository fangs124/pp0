use chessbb::{ChessBoard, ChessMove, ChessPiece, Evaluator, GameState, PieceType, Side, Square, TranspositionTable};
use nalgebra::DVector;
use nnet::{InputType, SparseInputType, SparseVec};
use rand::random_range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChessGame {
    pub cb: ChessBoard,
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
        self.update_state(self.cb.parse_move(move_str));
    }

    #[inline(always)]
    pub fn try_generate_moves(&self) -> (Vec<ChessMove>, GameState) {
        return self.cb.try_generate_moves();
    }

    #[inline(always)]
    pub fn update_state(&mut self, chess_move: ChessMove) {
        self.cb.update_state(chess_move);
    }

    #[inline(always)]
    pub fn negamax(
        &mut self, d: u16, ev: &mut impl Evaluator, tt: &mut TranspositionTable, node_count: &mut usize, pair: Option<(Vec<ChessMove>, GameState)>,
    ) -> (i16, Option<ChessMove>) {
        self.cb.negamax(i16::MIN + 1, i16::MAX - 1, d, 0, ev, tt, node_count, pair).unwrap()
    }

    #[inline(always)]
    pub fn find_move(&mut self, d: u16, ev: &mut impl Evaluator, node_count: &mut usize, moves: Vec<ChessMove>, tt: &mut TranspositionTable) -> ChessMove {
        assert!(moves.len() > 0);
        let chess_move: ChessMove = moves[0].clone();
        return self.negamax(d, ev, tt, node_count, Some((moves, GameState::Ongoing))).1.unwrap_or(chess_move);
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

#[inline(always)]
fn negate(pair: (i16, Option<ChessMove>)) -> (i16, Option<ChessMove>) {
    return (-pair.0, pair.1);
}
