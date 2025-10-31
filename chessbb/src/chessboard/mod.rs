use std::fmt::Debug;
use std::fmt::Display;
use std::time::Duration;
use std::time::Instant;

#[cfg(feature = "arrayvec")]
use arrayvec::ArrayVec;

#[cfg(feature = "smallvec")]
use smallvec::SmallVec;

#[cfg(not(feature = "piececolourboard"))]
pub(crate) use crate::chessboard::pieceboard::PieceBoard;

#[cfg(feature = "piececolourboard")]
pub(crate) use crate::chessboard::pieceboard::PieceColourBoard;

use crate::Bitboard;
use crate::ChessPiece;
use crate::PieceType;
use crate::Side;
use crate::bitboard::attack::*;
use crate::chessboard::mailbox::Mailbox;
use crate::chessboard::zobrist::{ZobristHash, ZobristTable};
use crate::chessmove::Castling;
use crate::chessmove::ChessMove;
use crate::square::Square;

mod fen;
mod mailbox;
mod movegen;
mod perft;
mod pieceboard;
mod updatestate;
pub mod zobrist;

#[cfg(feature = "arrayvec")]
pub type MoveList = ArrayVec<ChessMove, SIZE>;

#[cfg(feature = "smallvec")]
pub type MoveList = SmallVec<[ChessMove; 64]>;

#[cfg(not(any(feature = "arrayvec", feature = "smallvec")))]
pub type MoveList = Vec<ChessMove>;

#[cfg(not(feature = "piececolourboard"))]
pub type PieceBitboard = PieceBoard;

#[cfg(feature = "piececolourboard")]
pub type PieceBitboard = PieceColourBoard;

pub(crate) const SIZE: usize = 218; //256 looks nicer.. but apparently this is the upperbound of moves in classical chess rule

//const foo: usize = size_of::<PieceBoard>();
//const bar: usize = size_of::<PieceColourBoard>();
//const baz: usize = size_of::<Mailbox>();
//const faz: usize = size_of::<ChessBoard>();

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ChessBoard {
    bitboards: PieceBitboard,
    mailbox: Mailbox,
    pub(crate) data: ChessData, //data to clone that are annoying to undo
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChessGame {
    chessboard: ChessBoard,
    zobrist_table: ZobristTable,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GameState {
    Finished(GameResult),
    Ongoing,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GameResult {
    Win(Side),
    Draw,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ChessData {
    castle_bools: [bool; 4], //WK, WQ, BK, BQ castle rights
    enpassant_bb: Bitboard,  //square attackable by enemy piece
    check_bb: Bitboard,      //pieces triggering check condition
    check_mask: Bitboard,    //all the squares attacked by checking pieces
    pinned_bb: Bitboard,     //pieces that are pinned
    pinner_bb: Bitboard,     //pieces doing the pin
    side_to_move: Side,
    full_move_counter: u16, //engine games can go over 400 moves, u8::MAX is 255
    fifty_move_rule_counter: u8,
    zobrist_hash: ZobristHash,
    //zt
}

pub struct ChessBoardSnapshot {
    bitboards: PieceBitboard,
    mailbox: Mailbox,
    data: ChessData,
    hash: ZobristHash,
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut rows: Vec<String> = Vec::new();
        let mut r = String::new();
        for &square in Square::iter() {
            if let Some(piece) = self.mailbox.square_index(square) {
                r.push(piece.to_ascii());
            } else {
                r.push('.');
            }

            if square.as_usize() % 8 == 7 {
                r.push('\n');
                rows.push(r.clone());
                r = String::new();
            }
        }
        rows.reverse();
        write!(f, "{}", rows.join(""))
    }
}
impl Debug for ChessBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        s.push_str(&format!("bitboards:\n{:?}", self.bitboards));
        s.push_str(&format!("mailbox:\n{:?}", self.mailbox));
        s.push_str(&format!("check_bb:\n{:?}", self.data.check_bb));
        s.push_str(&format!("check_mask:\n{:?}", self.data.check_mask));
        s.push_str(&format!("pinned_bb:\n{:?}", self.data.pinned_bb));
        s.push_str(&format!("pinner_bb:\n{:?}", self.data.pinner_bb));
        s.push_str(&format!("enpassant_bb:\n{:?}", self.data.enpassant_bb));
        s.push_str(&format!("castle_bool:\n{:?}", self.data.castle_bools));
        write!(f, "{s}")
    }
}

#[rustfmt::skip]
pub const fn cp_index(data: ChessPiece) -> usize {
    match data {
        ChessPiece(Side::White, PieceType::Pawn  ) => 00,
        ChessPiece(Side::White, PieceType::Knight) => 01,
        ChessPiece(Side::White, PieceType::Bishop) => 02,
        ChessPiece(Side::White, PieceType::Rook  ) => 03,
        ChessPiece(Side::White, PieceType::Queen ) => 04,
        ChessPiece(Side::White, PieceType::King  ) => 05,
        ChessPiece(Side::Black, PieceType::Pawn  ) => 06,
        ChessPiece(Side::Black, PieceType::Knight) => 07,
        ChessPiece(Side::Black, PieceType::Bishop) => 08,
        ChessPiece(Side::Black, PieceType::Rook  ) => 09,
        ChessPiece(Side::Black, PieceType::Queen ) => 10,
        ChessPiece(Side::Black, PieceType::King  ) => 11,
    }
}

pub const fn sym_index(c: char) -> usize {
    match c {
        'K' => 0,
        'Q' => 1,
        'N' => 2,
        'B' => 3,
        'R' => 4,
        'P' => 5,
        'k' => 6,
        'q' => 7,
        'n' => 8,
        'b' => 9,
        'r' => 10,
        'p' => 11,
        _ => panic!("sym_index error: invalid char!"),
    }
}

//Pawn, Knight, Bishop, Rook, Queen, King
//White, Black
pub const COLOUR_PIECE_SYMBOLS: [char; 12] = ['P', 'N', 'B', 'R', 'Q', 'K', 'p', 'n', 'b', 'r', 'q', 'k'];
pub const PIECE_LABELS: [&str; 6] = ["pawn", "knight", "bihop", "rook", "queen", "king"];
pub const COLOUR_LABELS: [&str; 2] = ["white", "black"];

pub const fn chess_piece(c: char) -> ChessPiece {
    match c {
        'K' => ChessPiece(Side::White, PieceType::King),
        'Q' => ChessPiece(Side::White, PieceType::Queen),
        'N' => ChessPiece(Side::White, PieceType::Knight),
        'B' => ChessPiece(Side::White, PieceType::Bishop),
        'R' => ChessPiece(Side::White, PieceType::Rook),
        'P' => ChessPiece(Side::White, PieceType::Pawn),
        'k' => ChessPiece(Side::Black, PieceType::King),
        'q' => ChessPiece(Side::Black, PieceType::Queen),
        'n' => ChessPiece(Side::Black, PieceType::Knight),
        'b' => ChessPiece(Side::Black, PieceType::Bishop),
        'r' => ChessPiece(Side::Black, PieceType::Rook),
        'p' => ChessPiece(Side::Black, PieceType::Pawn),
        _ => panic!("chess_piece error: invalid char!"),
    }
}

impl ChessGame {
    pub const fn start_pos() -> ChessGame {
        ChessGame { chessboard: ChessBoard::start_pos(), zobrist_table: ZobristTable::initial_table() }
    }

    #[inline(always)]
    const fn king_square(&self, side: Side) -> Square {
        self.chessboard.king_square(side)
    }

    #[inline(always)]
    pub fn side(&self) -> Side {
        self.chessboard.side()
    }

    #[inline(always)]
    pub const fn hash(&self) -> ZobristHash {
        self.chessboard.hash()
    }

    #[inline(always)]
    pub fn mailbox(&self) -> Mailbox {
        self.chessboard.mailbox()
    }

    #[inline(always)]
    pub fn piece_bitboard(&self, chess_piece: ChessPiece) -> Bitboard {
        self.chessboard.bitboards.piece_bitboard(chess_piece)
    }

    #[inline(always)]
    pub const fn square_index(&self, square: Square) -> Option<ChessPiece> {
        self.chessboard.mailbox.square_index(square)
    }

    #[inline(always)]
    pub fn repetition(&self) -> usize {
        self.zobrist_table.count_hash(self.hash())
    }

    pub fn try_generate_moves(&self) -> (MoveList, GameState) {
        if self.repetition() >= 3 || self.chessboard.is_fifty_move_rule() {
            return (MoveList::new(), GameState::Finished(GameResult::Draw));
        }
        let side = self.side();
        let moves = self.chessboard.generate_moves();
        if moves.len() != 0 {
            return (moves, GameState::Ongoing);
        } else if self.chessboard.is_king_in_check(side) {
            return (moves, GameState::Finished(GameResult::Win(side.update())));
        } else {
            return (moves, GameState::Finished(GameResult::Draw));
        }
    }

    pub fn from_fen(input: &str) -> ChessGame {
        let chessboard: ChessBoard = ChessBoard::from_fen(input);
        let zobrist_table: ZobristTable = ZobristTable::new(chessboard.hash());
        ChessGame { chessboard, zobrist_table }
    }

    #[inline(always)]
    pub fn explore_state(&mut self, chess_move: &ChessMove) -> ChessBoardSnapshot {
        let bitboards = self.chessboard.bitboards.clone();
        let mailbox = self.chessboard.mailbox.clone();
        let data = self.chessboard.data.clone();
        self.update_state(chess_move);
        ChessBoardSnapshot { bitboards, mailbox, data, hash: self.chessboard.hash() }
    }

    #[inline(always)]
    pub fn restore_state(&mut self, snapshot: ChessBoardSnapshot) {
        self.chessboard.bitboards = snapshot.bitboards;
        self.chessboard.mailbox = snapshot.mailbox;
        self.chessboard.data = snapshot.data;
        self.zobrist_table.remove_last(snapshot.hash);
    }

    pub fn update_state(&mut self, chess_move: &ChessMove) {
        self.chessboard.update_state(chess_move);
        self.zobrist_table.push(self.chessboard.hash());
    }
}

impl ChessBoard {
    pub const fn start_pos() -> ChessBoard {
        ChessBoard { bitboards: PieceBitboard::START_BOARD, mailbox: Mailbox::START_MAILBOX, data: ChessData::start_pos() }
    }

    #[inline(always)]
    const fn king_square(&self, side: Side) -> Square {
        self.piece_bitboard(ChessPiece(side, PieceType::King)).lsb_square().expect("King not found!")
    }

    pub(crate) const fn is_king_in_check(&self, king_side: Side) -> bool {
        let square = self.piece_bitboard(ChessPiece(king_side, PieceType::King)).lsb_square().expect("King not found!");
        self.is_square_attacked(square, king_side.update(), self.bitboards.blockers())
    }

    #[inline(always)]
    pub const fn side(&self) -> Side {
        self.data.side_to_move
    }

    #[inline(always)]
    pub const fn hash(&self) -> ZobristHash {
        self.data.zobrist_hash
    }

    #[inline(always)]
    pub fn mailbox(&self) -> Mailbox {
        self.mailbox
    }

    #[inline(always)]
    pub const fn piece_bitboard(&self, chess_piece: ChessPiece) -> Bitboard {
        self.bitboards.piece_bitboard(chess_piece)
    }

    #[inline(always)]
    pub const fn square_index(&self, square: Square) -> Option<ChessPiece> {
        self.mailbox.square_index(square)
    }

    #[inline(always)]
    pub fn is_fifty_move_rule(&self) -> bool {
        self.data.fifty_move_rule_counter >= 100
    }

    pub(crate) fn is_castling_legal(&self, castling: Castling) -> bool {
        let blockers: Bitboard = self.bitboards.blockers();
        let (king_square, rook_square, castling_mask, castling_index) = match castling {
            Castling::Kingside(Side::White) => {
                (self.piece_bitboard(ChessPiece::WK).lsb_square().unwrap(), Square::W_KINGSIDE_ROOK_SQ_SOURCE, W_KING_SIDE_CASTLE_MASK, 0usize)
            }
            Castling::Queenside(Side::White) => {
                (self.piece_bitboard(ChessPiece::WK).lsb_square().unwrap(), Square::W_QUEENSIDE_ROOK_SQ_SOURCE, W_QUEEN_SIDE_CASTLE_MASK, 1usize)
            }
            Castling::Kingside(Side::Black) => {
                (self.piece_bitboard(ChessPiece::BK).lsb_square().unwrap(), Square::B_KINGSIDE_ROOK_SQ_SOURCE, B_KING_SIDE_CASTLE_MASK, 2usize)
            }
            Castling::Queenside(Side::Black) => {
                (self.piece_bitboard(ChessPiece::BK).lsb_square().unwrap(), Square::B_QUEENSIDE_ROOK_SQ_SOURCE, B_QUEEN_SIDE_CASTLE_MASK, 3usize)
            }
        };

        // check if friendly side can still castle, and if there are blockers in relevant squares
        if (self.data.castle_bools[castling_index] == false) || (blockers.bit_and(&castling_mask).is_not_zero()) {
            return false;
        }

        // check if squares between rook and king are empty
        if rays(king_square, rook_square).bit_and(&blockers).is_not_zero() {
            return false;
        }

        let mut squares = castling_mask;
        while squares.is_not_zero() {
            let square = squares.lsb_square().unwrap();
            if self.is_square_attacked(square, self.side().update(), self.bitboards.blockers()) {
                return false;
            }
            squares.pop_bit(square);
        }

        true
    }

    pub const fn is_square_attacked(&self, square: Square, attacker_side: Side, blockers: Bitboard) -> bool {
        let friendly_side: Side = attacker_side.update();
        let pawn_attack_bb: Bitboard = match friendly_side {
            Side::White => get_w_pawn_attack(square),
            Side::Black => get_b_pawn_attack(square),
        };
        if (pawn_attack_bb.bit_and(&self.piece_bitboard(ChessPiece(attacker_side, PieceType::Pawn)))).is_not_zero() {
            return true;
        } else if (get_knight_attack(square).bit_and(&self.piece_bitboard(ChessPiece(attacker_side, PieceType::Knight)))).is_not_zero() {
            return true;
        } else if (get_bishop_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece(attacker_side, PieceType::Bishop)))).is_not_zero() {
            return true;
        } else if (get_rook_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece(attacker_side, PieceType::Rook)))).is_not_zero() {
            return true;
        } else if (get_queen_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece(attacker_side, PieceType::Queen)))).is_not_zero() {
            return true;
        } else if (get_king_attack(square).bit_and(&self.piece_bitboard(ChessPiece(attacker_side, PieceType::King)))).is_not_zero() {
            return true;
        }

        false
    }

    //this used to be used
    pub const fn is_square_attacked_conditional(&self, square: Square, attacker_side: Side, blockers: Bitboard) -> bool {
        match attacker_side {
            Side::White => {
                (get_b_pawn_attack(square).bit_and(&self.piece_bitboard(ChessPiece::WP))).is_not_zero()
                    || (get_rook_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece::WR))).is_not_zero()
                    || (get_bishop_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece::WB))).is_not_zero()
                    || (get_knight_attack(square).bit_and(&self.piece_bitboard(ChessPiece::WN))).is_not_zero()
                    || (get_queen_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece::WQ))).is_not_zero()
                    || (get_king_attack(square).bit_and(&self.piece_bitboard(ChessPiece::WK))).is_not_zero()
            }
            Side::Black => {
                (get_w_pawn_attack(square).bit_and(&self.piece_bitboard(ChessPiece::BP))).is_not_zero()
                    || (get_rook_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece::BR))).is_not_zero()
                    || (get_bishop_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece::BB))).is_not_zero()
                    || (get_knight_attack(square).bit_and(&self.piece_bitboard(ChessPiece::BN))).is_not_zero()
                    || (get_queen_attack(square, blockers).bit_and(&self.piece_bitboard(ChessPiece::BQ))).is_not_zero()
                    || (get_king_attack(square).bit_and(&self.piece_bitboard(ChessPiece::BK))).is_not_zero()
            }
        }
    }
}

impl ChessData {
    const fn start_pos() -> ChessData {
        ChessData {
            castle_bools: [true; 4],
            enpassant_bb: Bitboard::ZERO,
            check_bb: Bitboard::ZERO,
            check_mask: Bitboard::ZERO,
            pinned_bb: Bitboard::ZERO,
            pinner_bb: Bitboard::ZERO,
            side_to_move: Side::White,
            full_move_counter: 0,
            fifty_move_rule_counter: 0,
            zobrist_hash: ZobristHash::initial_hash(),
        }
    }
}

#[rustfmt::skip]
const W_KING_SIDE_CASTLE_MASK:  Bitboard = Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01100000);
const W_QUEEN_SIDE_CASTLE_MASK: Bitboard = Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00001100);

#[rustfmt::skip]
const B_KING_SIDE_CASTLE_MASK:  Bitboard = Bitboard::new(0b01100000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
const B_QUEEN_SIDE_CASTLE_MASK: Bitboard = Bitboard::new(0b00001100_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
