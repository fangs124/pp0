use chessbb::{
    ChessBoard, ChessMove, ChessPiece, GameState, MATERIAL_EVAL, PieceType, Side, Square, TranspositionTable,
};
use nalgebra::DVector;
use nnet::InputType;
use rand::random_range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChessGame {
    pub(crate) cb: ChessBoard,
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

    //#[rustfmt::skip]
    #[inline(always)]
    pub fn find_move_sanity_test(&mut self, d: usize, moves: Vec<ChessMove>, tt: &mut TranspositionTable) -> ChessMove {
        assert!(!moves.is_empty() && d > 0);
        let mut alpha: i16 = i16::MIN + 1;
        let b: i16 = i16::MAX - 1;
        let mut best_move: ChessMove = moves[0].clone();
        let mut action_values: Vec<(ChessMove, i16)> = moves.iter().map(|&x| (x, i16::MIN + 1)).collect();
        for depth in d..=d {
            for (chess_move, old_value) in action_values.iter_mut() {
                let snapshot = self.cb.explore_state(*chess_move);
                //NOTE: depth instead of depth-1 here so that call to ChessNet::negamax() has implicit depth >= 1.
                let (value, _next_move) = negate(self.cb.negamax(-b, -alpha, depth - 1, 1, &mut MATERIAL_EVAL, tt));
                self.cb.restore_state(snapshot);

                if value > alpha {
                    alpha = value;
                    best_move = *chess_move;
                }

                *old_value = value;
            }

            //action_values.sort_by(|(_, av), (_, bv)| av.cmp(bv));
        }

        return best_move;
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

    pub(crate) fn vectorize(cb: &ChessBoard) -> DVector<f32> {
        return DVector::<f32>::from_vec(ChessGame::encode_raw(&cb).to_vec());
    }

    fn encode_raw(cb: &ChessBoard) -> [f32; 768] {
        //position is always encoded from active side's presepctive
        let mut input_data: [f32; 768] = [0.0; 768];
        for (chesspiece, i) in cb.mailbox_iterator().zip(0usize..64) {
            if let Some(chesspiece) = chesspiece {
                match cb.side() {
                    Side::White => {
                        input_data[ChessGame::index(*chesspiece, Square::nth(i))] = 1.0;
                    }
                    Side::Black => {
                        input_data[ChessGame::index_flip(*chesspiece, Square::nth(i))] = 1.0;
                    }
                }
            }
        }
        return input_data;
    }
}

#[inline(always)]
fn negate(pair: (i16, Option<ChessMove>)) -> (i16, Option<ChessMove>) {
    return (-pair.0, pair.1);
}
