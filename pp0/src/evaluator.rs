use chessbb::{ChessBoard, ChessGame, ChessMove, ChessPiece, MoveType, PieceType, Side, Square, castle_index, index_pair, index};
use nnue::Network;

pub trait Evaluator {
    //i16 is used here as a fixed-precision evaluation out of 2000
    fn eval(&mut self, chessgame: &ChessGame) -> i16;
    fn update(&mut self, chessgame: &ChessGame, chessmove: &ChessMove);
    fn revert(&mut self, chessgame: &ChessGame, chessmove: &ChessMove);
    fn initialize(&mut self, chessgame: &ChessGame);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialEvaluator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticEvaluator;

pub const MATERIAL_EVAL: MaterialEvaluator = MaterialEvaluator;
pub const STATIC_EVAL: StaticEvaluator = StaticEvaluator;


impl Evaluator for Network {
    fn eval(&mut self, chessgame: &ChessGame) -> i16 {
        match chessgame.side() {
            Side::White => (self.eval::<true>() * 2000.0 ) as i16,
            Side::Black => (self.eval::<false>() * 2000.0 ) as i16,
        }
        
    }
     fn update(&mut self, chessgame: &ChessGame, chessmove: &ChessMove) {  
        let side = chessgame.side();
        let source_square = chessmove.source();
        let source_piece = chessgame.square_index(source_square).unwrap();
        //lift source piece
        let (s_w, s_b) = index_pair(source_piece, source_square);
        match chessmove.move_type() {
            MoveType::Normal => {
                //place source piece
                let target_square = chessmove.target();
                let (a_w, a_b) = index_pair(source_piece, target_square);
                match chessgame.square_index(chessmove.target()) {
                    Some(target_piece) => {
                        //remove captured piece
                        let (s2_w, s2_b) = index_pair(target_piece, target_square);
                        self.accumulators_addsubsub(a_w, s_w, s2_w, a_b, s_b, s2_b);
                    }
                    None => {
                        self.accumulators_addsub(a_w, s_w, a_b, s_b);
                    }
                }
            },

            MoveType::Castle(castling) => {
                let [k_sub_w, k_add_w, r_sub_w, r_add_w] = castle_index(castling, Side::White);
                let [k_sub_b, k_add_b, r_sub_b, r_add_b] = castle_index(castling, Side::Black);
                
                //move king
                self.accumulators_addsub(k_add_w, k_sub_w, k_add_b, k_sub_b);
                //move rook
                self.accumulators_addsub(r_add_w, r_sub_w, r_add_b, r_sub_b);
            },

            MoveType::EnPassant => {
                //place source piece
                let target_square = chessmove.target();
                let (a_w, a_b) = index_pair(source_piece, target_square);
                //remove captured piece
                let victim_square: Square = match side {
                    Side::White => chessmove.target().down(),
                    Side::Black => chessmove.target().up(),
                };
                let victim_piece = chessgame.square_index(victim_square).unwrap();
                let (s2_w, s2_b) = index_pair(victim_piece, victim_square);
                self.accumulators_addsubsub(a_w, s_w, s2_w, a_b, s_b, s2_b);
            },

            MoveType::Promotion(piece_type) => {
                //place promoted piece
                let promotion_piece = ChessPiece::new(side, piece_type);
                let target_square = chessmove.target();
                let (a_w, a_b) = index_pair(promotion_piece, target_square);
                match chessgame.square_index(chessmove.target()) {
                    Some(target_piece) => {
                        //remove captured piece
                        let (s2_w, s2_b) = index_pair(target_piece, target_square);
                        self.accumulators_addsubsub(a_w, s_w, s2_w, a_b, s_b, s2_b);
                    }
                    None => {
                        self.accumulators_addsub(a_w, s_w, a_b, s_b);
                    }
                }
            },
        }
    }

    
    fn revert(&mut self, chessgame: &ChessGame, chessmove: &ChessMove) {
        let side = chessgame.side();
        //lift source piece
        let source_square = chessmove.source();
        let source_piece = chessgame.square_index(source_square).unwrap();
        let (s_w, s_b) = index_pair(source_piece, source_square);
        match chessmove.move_type() {
            MoveType::Normal => {
                //place source piece
                let target_square = chessmove.target();
                let (a_w, a_b) = index_pair(source_piece, target_square);
                match chessgame.square_index(chessmove.target()) {
                    Some(target_piece) => {
                        //remove captured piece
                        let (s2_w, s2_b) = index_pair(target_piece, target_square);
                        self.accumulators_addaddsub( s_w, s2_w, a_w, s_b, s2_b, a_b);
                    }
                    None => {
                        self.accumulators_addsub(s_w, a_w, s_b, a_b);
                    }
                }
            },

            MoveType::Castle(castling) => {
                let [k_sub_w, k_add_w, r_sub_w, r_add_w] = castle_index(castling, Side::White);
                let [k_sub_b, k_add_b, r_sub_b, r_add_b] = castle_index(castling, Side::Black);
                
                //move king
                self.accumulators_addsub(k_sub_w, k_add_w, k_sub_b, k_add_b);
                //move rook
                self.accumulators_addsub(r_sub_w, r_add_w, r_sub_b, r_add_b);
            },

            MoveType::EnPassant => {
                //place source piece
                let target_square = chessmove.target();
                let (a_w, a_b) = index_pair(source_piece, target_square);
                //remove captured piece
                 let victim_square: Square = match side {
                    Side::White => chessmove.target().down(),
                    Side::Black => chessmove.target().up(),
                };
                let victim_piece = chessgame.square_index(victim_square).unwrap();
                let (s2_w, s2_b) = index_pair(victim_piece, victim_square);
                self.accumulators_addaddsub( s_w, s2_w, a_w, s_b, s2_b, a_b);
            },

            MoveType::Promotion(piece_type) => {
                //place promoted piece
                let promotion_piece = ChessPiece::new(side, piece_type);
                let target_square = chessmove.target();
                let (a_w, a_b) = index_pair(promotion_piece, target_square);
                match chessgame.square_index(chessmove.target()) {
                    Some(target_piece) => {
                        //remove captured piece
                        let (s2_w, s2_b) = index_pair(target_piece, target_square);
                        self.accumulators_addaddsub( s_w, s2_w, a_w, s_b, s2_b, a_b);
                    }
                    None => {
                        self.accumulators_addsub( s_w, a_w, s_b, a_b);
                    }
                }
            },
        }
    }

    fn initialize(&mut self, chessgame: &ChessGame) {
        self.refresh_accumulator_sparse(chessgame);
    }
    
  
}

impl Evaluator for MaterialEvaluator {
    fn eval(&mut self, cg: &ChessGame) -> i16 {
        let mut total: i16 = 0;
        for piece in cg.mailbox().iter() {
            if let Some(chesspiece) = piece {
                total += match chesspiece {
                    ChessPiece(Side::White, PieceType::King) => 20000,
                    ChessPiece(Side::White, PieceType::Queen) => 0900,
                    ChessPiece(Side::White, PieceType::Knight) => 0320,
                    ChessPiece(Side::White, PieceType::Bishop) => 0330,
                    ChessPiece(Side::White, PieceType::Rook) => 0500,
                    ChessPiece(Side::White, PieceType::Pawn) => 0100,
                    ChessPiece(Side::Black, PieceType::King) => -20000,
                    ChessPiece(Side::Black, PieceType::Queen) => -0900,
                    ChessPiece(Side::Black, PieceType::Knight) => -0320,
                    ChessPiece(Side::Black, PieceType::Bishop) => -0330,
                    ChessPiece(Side::Black, PieceType::Rook) => -0500,
                    ChessPiece(Side::Black, PieceType::Pawn) => -0100,
                }
            }
        }

        return match cg.side() {
            Side::White => total,
            Side::Black => -total,
        };
    }
    
    fn update(&mut self, chessgame: &ChessGame, chessmove: &ChessMove) {}
    fn revert(&mut self, chessgame: &ChessGame, chessmove: &ChessMove) {}
    fn initialize(&mut self, chessgame: &ChessGame) {}
}

impl Evaluator for StaticEvaluator {
    #[rustfmt::skip]
    fn eval(&mut self, cg: &ChessGame) -> i16 {
        let mut total: i16 = 0;
        let mut index: usize = 0;
        //both sides have no queens or
        //every side which has a queen has additionally no other pieces or one minorpiece maximum.
        let is_white_in_endgame: bool =
            (cg.piece_bitboard(ChessPiece::WQ).is_zero()) || (cg.piece_bitboard(ChessPiece::WN).bit_and(&cg.piece_bitboard(ChessPiece::WB).bit_and(&cg.piece_bitboard(ChessPiece::WR))).count_ones() == 1);
        let is_black_in_endgame: bool =
            (cg.piece_bitboard(ChessPiece::BQ).is_zero()) || (cg.piece_bitboard(ChessPiece::BN).bit_and(&cg.piece_bitboard(ChessPiece::BB).bit_and(&cg.piece_bitboard(ChessPiece::BR))).count_ones() == 1);
        let (w_king_array, b_king_array) = match is_white_in_endgame && is_black_in_endgame {
            true => (StaticEvaluator::W_KING_END_GAME_PSQT, StaticEvaluator::B_KING_END_GAME_PSQT),
            false => (StaticEvaluator::W_KING_MIDDLE_GAME_PSQT, StaticEvaluator::B_KING_MIDDLE_GAME_PSQT),
        };
        
        for piece in cg.mailbox().iter() {
            if let Some(chesspiece) = piece {
                
                total += match chesspiece {
                    ChessPiece(Side::White, PieceType::King)   => 20000 + w_king_array[index],
                    ChessPiece(Side::White, PieceType::Queen)  =>  0900 + StaticEvaluator::W_QUEEN_PSQT[index],
                    ChessPiece(Side::White, PieceType::Knight) =>  0320 + StaticEvaluator::W_KNIGHT_PSQT[index],
                    ChessPiece(Side::White, PieceType::Bishop) =>  0330 + StaticEvaluator::W_BISHOP_PSQT[index],
                    ChessPiece(Side::White, PieceType::Rook)   =>  0500 + StaticEvaluator::W_ROOK_PSQT[index],
                    ChessPiece(Side::White, PieceType::Pawn)   =>  0100 + StaticEvaluator::W_PAWN_PSQT[index],
                    ChessPiece(Side::Black, PieceType::King)   =>-20000 - b_king_array[index],
                    ChessPiece(Side::Black, PieceType::Queen)  => -0900 - StaticEvaluator::B_QUEEN_PSQT[index],
                    ChessPiece(Side::Black, PieceType::Knight) => -0320 - StaticEvaluator::B_KNIGHT_PSQT[index],
                    ChessPiece(Side::Black, PieceType::Bishop) => -0330 - StaticEvaluator::B_BISHOP_PSQT[index],
                    ChessPiece(Side::Black, PieceType::Rook)   => -0500 - StaticEvaluator::B_ROOK_PSQT[index],
                    ChessPiece(Side::Black, PieceType::Pawn)   => -0100 - StaticEvaluator::B_PAWN_PSQT[index],
                }
            }
            index += 1;
        }

        return match cg.side() {
            Side::White => total,
            Side::Black => -total,
        };
    }
    
    fn update(&mut self, chessgame: &ChessGame, chessmove: &ChessMove) {}
    fn revert(&mut self, chessgame: &ChessGame, chessmove: &ChessMove) {}
    fn initialize(&mut self, chessgame: &ChessGame) {}
}

impl StaticEvaluator {
    /* ==== labels ==== */

    /* indexing the 64-squares:
       -----------------------
    8 |63 62 61 60 59 58 57 56|
    7 |55 54 53 52 51 50 49 48|
    6 |47 46 45 44 43 42 41 40|
    5 |39 38 37 36 35 34 33 32|
    4 |31 30 29 28 27 26 25 24|
    3 |23 22 21 20 19 18 17 16|
    2 |15 14 13 12 11 10  9  8|
    1 | 7  6  5  4  3  2  1  0|
       -----------------------
        A  B  C  D  E  F  G  H */

    //these are from https://www.chessprogramming.org/Simplified_Evaluation_Function
    #[rustfmt::skip]
    const B_PAWN_PSQT: [i16; 64] = [
        00, 00, 00, 00, 00, 00, 00, 00, //A
        50, 50, 50, 50, 50, 50, 50, 50, //B
        10, 10, 20, 30, 30, 20, 10, 10, //C
        05, 05, 10, 25, 25, 10, 05, 05, //D
        00, 00, 00, 20, 20, 00, 00, 00, //E
        05,-05,-10, 00, 00,-10,-05, 05, //F
        05, 10, 10,-20,-20, 10, 10, 05, //G
        00, 00, 00, 00, 00, 00, 00, 00, //H
    ];

    #[rustfmt::skip]
    const W_PAWN_PSQT: [i16; 64] = [
        00, 00, 00, 00, 00, 00, 00, 00, //H
        05, 10, 10,-20,-20, 10, 10, 05, //G
        05,-05,-10, 00, 00,-10,-05, 05, //F
        05, 05, 10, 25, 25, 10, 05, 05, //D
        10, 10, 20, 30, 30, 20, 10, 10, //C
        00, 00, 00, 20, 20, 00, 00, 00, //E
        50, 50, 50, 50, 50, 50, 50, 50, //B
        00, 00, 00, 00, 00, 00, 00, 00, //A
    ];

    #[rustfmt::skip]
    const B_KNIGHT_PSQT: [i16; 64] = [
        -50,-40,-30,-30,-30,-30,-40,-50, //A
        -40,-20,  0,  0,  0,  0,-20,-40, //B
        -30,  0, 10, 15, 15, 10,  0,-30, //C
        -30,  5, 15, 20, 20, 15,  5,-30, //D
        -30,  0, 15, 20, 20, 15,  0,-30, //E
        -30,  5, 10, 15, 15, 10,  5,-30, //F
        -40,-20,  0,  5,  5,  0,-20,-40, //G
        -50,-40,-30,-30,-30,-30,-40,-50, //H
    ];

    #[rustfmt::skip]
    const W_KNIGHT_PSQT: [i16; 64] = [
        -50,-40,-30,-30,-30,-30,-40,-50, //H
        -40,-20,  0,  5,  5,  0,-20,-40, //G
        -30,  5, 10, 15, 15, 10,  5,-30, //F
        -30,  0, 15, 20, 20, 15,  0,-30, //E
        -30,  5, 15, 20, 20, 15,  5,-30, //D
        -30,  0, 10, 15, 15, 10,  0,-30, //C
        -40,-20,  0,  0,  0,  0,-20,-40, //B
        -50,-40,-30,-30,-30,-30,-40,-50, //A
    ];

    #[rustfmt::skip]
    const B_BISHOP_PSQT: [i16; 64] = [
        -20,-10,-10,-10,-10,-10,-10,-20, //A
        -10, 00, 00, 00, 00, 00, 00,-10, //B
        -10, 00, 05, 10, 10, 05, 00,-10, //C
        -10, 05, 05, 10, 10, 05, 05,-10, //D
        -10, 00, 10, 10, 10, 10, 00,-10, //E
        -10, 10, 10, 10, 10, 10, 10,-10, //F
        -10, 05, 00, 00, 00, 00, 05,-10, //G
        -20,-10,-10,-10,-10,-10,-10,-20, //H
    ];

    #[rustfmt::skip]
    const W_BISHOP_PSQT: [i16; 64] = [
        -20,-10,-10,-10,-10,-10,-10,-20, //H
        -10, 05, 00, 00, 00, 00, 05,-10, //G
        -10, 10, 10, 10, 10, 10, 10,-10, //F
        -10, 00, 10, 10, 10, 10, 00,-10, //E
        -10, 05, 05, 10, 10, 05, 05,-10, //D
        -10, 00, 05, 10, 10, 05, 00,-10, //C
        -10, 00, 00, 00, 00, 00, 00,-10, //B
        -20,-10,-10,-10,-10,-10,-10,-20, //A
    ];

    #[rustfmt::skip]
    const B_ROOK_PSQT: [i16; 64] = [
          0,  0,  0,  0,  0,  0,  0,  0, //A
          5, 10, 10, 10, 10, 10, 10,  5, //B
         -5,  0,  0,  0,  0,  0,  0, -5, //C
         -5,  0,  0,  0,  0,  0,  0, -5, //D
         -5,  0,  0,  0,  0,  0,  0, -5, //E
         -5,  0,  0,  0,  0,  0,  0, -5, //F
         -5,  0,  0,  0,  0,  0,  0, -5, //G
          0,  0,  0,  5,  5,  0,  0,  0, //H
    ];

    #[rustfmt::skip]
    const W_ROOK_PSQT: [i16; 64] = [
        00, 00,  0,  5,  5,  0,  0,  0, //H
       -05, 00,  0,  0,  0,  0,  0, -5, //G
       -05, 00,  0,  0,  0,  0,  0, -5, //F
       -05, 00,  0,  0,  0,  0,  0, -5, //E
       -05, 00,  0,  0,  0,  0,  0, -5, //D
       -05, 00,  0,  0,  0,  0,  0, -5, //C
        05, 10, 10, 10, 10, 10, 10,  5, //B
        00, 00, 00, 00, 00, 00, 00, 00, //A
    ];


    #[rustfmt::skip]
    const B_QUEEN_PSQT: [i16; 64] = [
       -20,-10,-10, -5, -5,-10,-10,-20, //A
       -10,  0,  0,  0,  0,  0,  0,-10, //B
       -10,  0,  5,  5,  5,  5,  0,-10, //C
       -05,  0,  5,  5,  5,  5,  0, -5, //D
       -05,  0,  5,  5,  5,  5,  0, 00, //E
       -10,  0,  5,  5,  5,  5,  5,-10, //F
       -10,  0,  0,  0,  0,  5,  0,-10, //G
       -20,-10,-10, -5, -5,-10,-10,-20, //H
    ];

    #[rustfmt::skip]
    const W_QUEEN_PSQT: [i16; 64] = [
       -20,-10,-10, -5, -5,-10,-10,-20, //H
       -10,  0,  0,  0,  0,  5,  0,-10, //G
       -10,  0,  5,  5,  5,  5,  5,-10, //F
       -05,  0,  5,  5,  5,  5,  0, 00, //E
       -05,  0,  5,  5,  5,  5,  0, -5, //D
       -10,  0,  5,  5,  5,  5,  0,-10, //C
       -10,  0,  0,  0,  0,  0,  0,-10, //B
       -20,-10,-10, -5, -5,-10,-10,-20, //A
    ];

    #[rustfmt::skip]
    const B_KING_MIDDLE_GAME_PSQT: [i16; 64] = [
        -30,-40,-40,-50,-50,-40,-40,-30, //A
        -30,-40,-40,-50,-50,-40,-40,-30, //B
        -30,-40,-40,-50,-50,-40,-40,-30, //C
        -30,-40,-40,-50,-50,-40,-40,-30, //D
        -20,-30,-30,-40,-40,-30,-30,-20, //E
        -10,-20,-20,-20,-20,-20,-20,-10, //F
         20, 20,  0,  0,  0,  0, 20, 20, //G
         20, 30, 10,  0,  0, 10, 30, 20, //H
    ];

    #[rustfmt::skip]
    const W_KING_MIDDLE_GAME_PSQT: [i16; 64] = [
        20, 30, 10,  0,  0, 10, 30, 20, //H
        20, 20,  0,  0,  0,  0, 20, 20, //G
       -10,-20,-20,-20,-20,-20,-20,-10, //F
       -20,-30,-30,-40,-40,-30,-30,-20, //E
       -30,-40,-40,-50,-50,-40,-40,-30, //D
       -30,-40,-40,-50,-50,-40,-40,-30, //C
       -30,-40,-40,-50,-50,-40,-40,-30, //B
       -30,-40,-40,-50,-50,-40,-40,-30, //A
    ];

     #[rustfmt::skip]
    const B_KING_END_GAME_PSQT: [i16; 64] = [
        -50,-40,-30,-20,-20,-30,-40,-50, //A
        -30,-20,-10,  0,  0,-10,-20,-30, //B
        -30,-10, 20, 30, 30, 20,-10,-30, //C
        -30,-10, 30, 40, 40, 30,-10,-30, //D
        -30,-10, 30, 40, 40, 30,-10,-30, //E
        -30,-10, 20, 30, 30, 20,-10,-30, //F
        -30,-30,  0,  0,  0,  0,-30,-30, //G
        -50,-30,-30,-30,-30,-30,-30,-50, //H
    ];

    #[rustfmt::skip]
    const W_KING_END_GAME_PSQT: [i16; 64] = [
        -50,-30,-30,-30,-30,-30,-30,-50, //H
        -30,-30,  0,  0,  0,  0,-30,-30, //G
        -30,-10, 20, 30, 30, 20,-10,-30, //F
        -30,-10, 30, 40, 40, 30,-10,-30, //E
        -30,-10, 30, 40, 40, 30,-10,-30, //D
        -30,-10, 20, 30, 30, 20,-10,-30, //C
        -30,-20,-10,  0,  0,-10,-20,-30, //B
        -50,-40,-30,-20,-20,-30,-40,-50, //A
    ];
}