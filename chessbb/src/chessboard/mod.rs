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
use crate::chessmove::MoveType;
use crate::square::Square;

mod mailbox;
mod movegen;
mod pieceboard;
mod zobrist;

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
pub(crate) struct ChessData {
    castle_bools: [bool; 4], //WK, WQ, BK, BQ castle rights
    enpassant_bb: Bitboard,  //square attackable by enemy piece
    check_bb: Bitboard,      //pieces triggering check condition
    check_mask: Bitboard,    //all the squares attacked by checking pieces
    pinned_bb: Bitboard,     //pieces that are pinned
    pinner_bb: Bitboard,     //pieces doing the pin
    side_to_move: Side,
    full_move_counter: u16,
    fifty_move_rule_counter: u16,
    zobrist_hash: ZobristHash,
    //zt
}

pub struct ChessBoardSnapshot {
    bitboards: PieceBitboard,
    mailbox: Mailbox,
    data: ChessData,
    hash: ZobristHash,
}

#[rustfmt::skip]
macro_rules! cpt {
    (P) => {Some(ChessPiece(Side::White, PieceType::Pawn  ))};
    (N) => {Some(ChessPiece(Side::White, PieceType::Knight))};
    (B) => {Some(ChessPiece(Side::White, PieceType::Bishop))};
    (R) => {Some(ChessPiece(Side::White, PieceType::Rook  ))};
    (Q) => {Some(ChessPiece(Side::White, PieceType::Queen ))};
    (K) => {Some(ChessPiece(Side::White, PieceType::King  ))};
    (p) => {Some(ChessPiece(Side::Black, PieceType::Pawn  ))};
    (n) => {Some(ChessPiece(Side::Black, PieceType::Knight))};
    (b) => {Some(ChessPiece(Side::Black, PieceType::Bishop))};
    (r) => {Some(ChessPiece(Side::Black, PieceType::Rook  ))};
    (q) => {Some(ChessPiece(Side::Black, PieceType::Queen ))};
    (k) => {Some(ChessPiece(Side::Black, PieceType::King  ))};
    (_) => {None};
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
        return ChessBoardSnapshot { bitboards, mailbox, data, hash: self.chessboard.hash() };
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

    pub fn from_fen(input: &str) -> ChessBoard {
        assert!(input.is_ascii());
        let mut input = input.split_ascii_whitespace();

        //let mut piece_board: PieceBoard = PieceBoard::EMPTY_BOARD;
        let mut bitboards: PieceBitboard = PieceBitboard::EMPTY_BOARD;
        let mut mailbox: Mailbox = Mailbox::EMPTY_MAILBOX;
        let mut castle_bools = [false, false, false, false];

        // example fen: rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1

        // parse piece placement data
        let mut square: usize = 0;
        for row in input.next().expect("from_fen error: missing pieces placement token").rsplit('/').collect::<Vec<&str>>() {
            for c in row.chars() {
                match c {
                    //TODO find a better way to do this?
                    c @ ('K' | 'Q' | 'N' | 'B' | 'R' | 'P' | 'k' | 'q' | 'n' | 'b' | 'r' | 'p') => {
                        bitboards.set_bit(chess_piece(c), Square::nth(square));
                        mailbox.set(Some(c.try_into().expect(&format!("from_fen error: invalid char {c}"))), Square::nth(square));
                    }

                    '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' => {
                        square += (c.to_digit(10).unwrap() as usize) - 1;
                    }

                    _ => panic!("from_fen error: invalid char {c}"),
                }
                square += 1;
            }
        }

        // parse active colour
        let side_to_move = match input.next().expect("from_fen error: missing active side token") {
            "w" => Side::White,
            "b" => Side::Black,
            _ => panic!("from_fen error: invalid active side token"),
        };

        // parse castling information
        for s in input.next().expect("from_fen error: missing castling rights token").chars() {
            match s {
                '-' => (),
                'K' => castle_bools[0] = true,
                'Q' => castle_bools[1] = true,
                'k' => castle_bools[2] = true,
                'q' => castle_bools[3] = true,
                _ => panic!("from_fen error: invalid castling rights token"),
            }
        }

        let mut enpassant_bb: Bitboard = Bitboard::ZERO;
        //parse en passant information
        let en_passant_token = input.next().expect("from_fen error: missing en passant token");
        if en_passant_token != "-" {
            assert!(en_passant_token.len() == 2, "from_fen error: incorrect en passant token length");
            enpassant_bb = Bitboard::nth(Square::parse_str(en_passant_token));
        }

        //parse fifty-move-rule counter
        let fifty_move_rule_counter = input.next().map_or(0, |x| x.parse::<u16>().expect("from_fen error: invalid fifty-move-rule token"));

        //parse fullmove number
        let full_move_counter = input.next().map_or(0, |x| x.parse::<u16>().expect("from_fen error: invalid move-counter token"));

        //check bitboard
        let blockers: Bitboard = bitboards.blockers();
        let enemy_side: Side = side_to_move.update();
        let king_square: Square = bitboards.piece_bitboard(ChessPiece(side_to_move, PieceType::King)).lsb_square().unwrap();
        let check_bb: Bitboard = {
            let queen_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Queen)).bit_and(&get_queen_attack(king_square, blockers));
            let knight_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Knight)).bit_and(&get_knight_attack(king_square));
            let bishop_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Bishop)).bit_and(&get_bishop_attack(king_square, blockers));
            let rook_bb: Bitboard = bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Rook)).bit_and(&get_rook_attack(king_square, blockers));
            let pawn_bb: Bitboard = match side_to_move {
                Side::White => bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Pawn)).bit_and(&get_w_pawn_attack(king_square)),
                Side::Black => bitboards.piece_bitboard(ChessPiece(enemy_side, PieceType::Pawn)).bit_and(&get_b_pawn_attack(king_square)),
            };
            queen_bb.bit_or(&knight_bb.bit_or(&bishop_bb.bit_or(&rook_bb.bit_or(&pawn_bb))))
        };

        let mut pinner_bb: Bitboard = Bitboard::ZERO;
        let mut pinned_bb: Bitboard = Bitboard::ZERO;

        let enemy_knight_piece: ChessPiece = match side_to_move {
            Side::White => ChessPiece(Side::Black, PieceType::Knight),
            Side::Black => ChessPiece(Side::White, PieceType::Knight),
        };

        let mut non_knight_check_bb: Bitboard = check_bb.bit_and(&bitboards.piece_bitboard(enemy_knight_piece).bit_not());
        let mut check_mask: Bitboard = check_bb.clone();
        while non_knight_check_bb.is_not_zero() {
            let checker_square = non_knight_check_bb.lsb_square().unwrap();
            check_mask = check_mask.bit_or(&rays(checker_square, king_square));
            non_knight_check_bb.pop_lsb();
        }

        let friends: Bitboard;
        let enemies: Bitboard;
        let diagonal_enemies: Bitboard;
        let lateral_enemies: Bitboard;

        match side_to_move {
            Side::White => {
                friends = bitboards.white_blockers();
                enemies = bitboards.black_blockers();
                diagonal_enemies = bitboards.piece_bitboard(cpt!(q).unwrap()).bit_or(&bitboards.piece_bitboard(cpt!(b).unwrap()));
                lateral_enemies = bitboards.piece_bitboard(cpt!(q).unwrap()).bit_or(&bitboards.piece_bitboard(cpt!(r).unwrap()));
            }
            Side::Black => {
                friends = bitboards.black_blockers();
                enemies = bitboards.white_blockers();
                diagonal_enemies = bitboards.piece_bitboard(cpt!(Q).unwrap()).bit_or(&bitboards.piece_bitboard(cpt!(B).unwrap()));
                lateral_enemies = bitboards.piece_bitboard(cpt!(Q).unwrap()).bit_or(&bitboards.piece_bitboard(cpt!(R).unwrap()));
            }
        }

        assert!(bitboards.piece_bitboard(ChessPiece(side_to_move, PieceType::King)).count_ones() == 1);
        let king_square = bitboards.piece_bitboard(ChessPiece(side_to_move, PieceType::King)).lsb_square().unwrap();
        let mut possible_pinners: Bitboard = (get_bishop_attack(king_square, diagonal_enemies).bit_and(&diagonal_enemies))
            .bit_or(&get_rook_attack(king_square, lateral_enemies).bit_and(&lateral_enemies));
        while possible_pinners.is_not_zero() {
            let possible_pinner = possible_pinners.lsb_square().unwrap();
            let pinner_piece: ChessPiece = mailbox.square_index(possible_pinner).unwrap();
            let attack_mask = match pinner_piece {
                ChessPiece(_, PieceType::Bishop) => get_bishop_attack(possible_pinner, enemies),
                ChessPiece(_, PieceType::Rook) => get_rook_attack(possible_pinner, enemies),
                ChessPiece(_, PieceType::Queen) => get_queen_attack(possible_pinner, enemies),
                _ => panic!(),
            };

            let relevant_mask: Bitboard = rays(king_square, possible_pinner).bit_and(&attack_mask);
            let enemy_blockers: Bitboard = relevant_mask.bit_and(&enemies);
            let possible_pinned: Bitboard = relevant_mask.bit_and(&friends);

            //NOTE: a piece is only pinned if and only if it is the only piece between the pinner and the king.
            //      enemy can also block the line of sight.
            if possible_pinned.count_ones() == 1 && enemy_blockers.count_ones() == 0 {
                pinner_bb = pinner_bb.bit_or(&possible_pinners.lsb_bitboard());
                pinned_bb = pinned_bb.bit_or(&possible_pinned);
            }

            possible_pinners.pop_lsb();
        }

        //pinned_bb = pinned_bb;
        //pinner_bb = pinner_bb;

        let zobrist_hash: ZobristHash = ZobristHash::compute_hash(side_to_move, &mailbox, castle_bools, enpassant_bb);
        //let (check_bb, check_mask) = todo!();
        let data: ChessData = ChessData {
            castle_bools,
            enpassant_bb,
            check_bb,
            check_mask,
            pinned_bb,
            pinner_bb,
            side_to_move,
            full_move_counter,
            fifty_move_rule_counter,
            zobrist_hash,
        };
        ChessBoard { bitboards, mailbox, data }
    }

    pub fn print_board_debug(&self) -> String {
        format!(
            "bitboards:\n{:?}mailbox:\n{:?}\ncheck_bb:\n{}\ncheck_mask:\n{}\npinned_bb:\n{}\npinner_bb:\n{}\nenpassant_bb\n{}\ncastle_bools:\n{:?}",
            self.bitboards,
            self.mailbox,
            self.data.check_bb,
            self.data.check_mask,
            self.data.pinned_bb,
            self.data.pinner_bb,
            self.data.enpassant_bb,
            self.data.castle_bools
        )
    }

    pub fn print_board(&self) -> String {
        let mut rows: Vec<String> = Vec::new();
        let mut r = String::new();
        for &square in Square::iter() {
            if let Some(piece) = self.mailbox.square_index(square) {
                r.push(piece.to_ascii());
            } else {
                r.push('.');
            }

            if square.to_usize() % 8 == 7 {
                r.push('\n');
                rows.push(r.clone());
                r = String::new();
            }
        }
        rows.reverse();
        return rows.join("");
    }

    pub fn perft_count_timed(&self, depth: usize, is_bulk: bool) -> (u64, Duration) {
        let now = Instant::now();
        let total_count = match is_bulk {
            true => self.perft_count_bulk(depth),
            false => self.perft_count(depth),
        };

        return (total_count, now.elapsed());
    }

    pub fn perft_count(&self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_moves();

        let mut total: u64 = 0;
        for chess_move in moves {
            let mut chessboard = *self;
            chessboard.update_state(&chess_move);
            total += chessboard.perft_count(depth - 1);
        }
        return total;
    }

    pub fn perft_count_bulk(&self, depth: usize) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = self.generate_moves();
        if depth == 1 {
            return moves.len() as u64;
        }

        let mut total: u64 = 0;
        for chess_move in moves {
            let mut chessboard = *self;
            chessboard.update_state(&chess_move);
            total += chessboard.perft_count_bulk(depth - 1);
        }
        return total;
    }

    pub(crate) fn is_castling_legal(&self, castling: Castling) -> bool {
        let blockers: Bitboard = self.bitboards.blockers();
        let (king_square, rook_square, castling_mask, castling_index) = match castling {
            Castling::Kingside(Side::White) => {
                (self.bitboards.piece_bitboard(ChessPiece::WK).lsb_square().unwrap(), Square::W_KINGSIDE_ROOK_SQ_SOURCE, W_KING_SIDE_CASTLE_MASK, 0usize)
            }
            Castling::Queenside(Side::White) => {
                (self.bitboards.piece_bitboard(ChessPiece::WK).lsb_square().unwrap(), Square::W_QUEENSIDE_ROOK_SQ_SOURCE, W_QUEEN_SIDE_CASTLE_MASK, 1usize)
            }
            Castling::Kingside(Side::Black) => {
                (self.bitboards.piece_bitboard(ChessPiece::BK).lsb_square().unwrap(), Square::B_KINGSIDE_ROOK_SQ_SOURCE, B_KING_SIDE_CASTLE_MASK, 2usize)
            }
            Castling::Queenside(Side::Black) => {
                (self.bitboards.piece_bitboard(ChessPiece::BK).lsb_square().unwrap(), Square::B_QUEENSIDE_ROOK_SQ_SOURCE, B_QUEEN_SIDE_CASTLE_MASK, 3usize)
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
        return true;
    }

    pub const fn is_square_attacked(&self, square: Square, attacker_side: Side, blockers: Bitboard) -> bool {
        let friendly_side: Side = attacker_side.update();
        let pawn_attack_bb: Bitboard = match friendly_side {
            Side::White => get_w_pawn_attack(square),
            Side::Black => get_b_pawn_attack(square),
        };
        if (pawn_attack_bb.bit_and(&self.bitboards.piece_bitboard(ChessPiece(attacker_side, PieceType::Pawn)))).is_not_zero() {
            return true;
        } else if (get_knight_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece(attacker_side, PieceType::Knight)))).is_not_zero() {
            return true;
        } else if (get_bishop_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece(attacker_side, PieceType::Bishop)))).is_not_zero() {
            return true;
        } else if (get_rook_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece(attacker_side, PieceType::Rook)))).is_not_zero() {
            return true;
        } else if (get_queen_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece(attacker_side, PieceType::Queen)))).is_not_zero() {
            return true;
        } else if (get_king_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece(attacker_side, PieceType::King)))).is_not_zero() {
            return true;
        }
        return false;
    }

    //this used to be used
    pub const fn is_square_attacked_conditional(&self, square: Square, attacker_side: Side, blockers: Bitboard) -> bool {
        match attacker_side {
            Side::White => {
                return (get_b_pawn_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece::WP))).is_not_zero()
                    || (get_rook_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece::WR))).is_not_zero()
                    || (get_bishop_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece::WB))).is_not_zero()
                    || (get_knight_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece::WN))).is_not_zero()
                    || (get_queen_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece::WQ))).is_not_zero()
                    || (get_king_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece::WK))).is_not_zero();
            }
            Side::Black => {
                return (get_w_pawn_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece::BP))).is_not_zero()
                    || (get_rook_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece::BR))).is_not_zero()
                    || (get_bishop_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece::BB))).is_not_zero()
                    || (get_knight_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece::BN))).is_not_zero()
                    || (get_queen_attack(square, blockers).bit_and(&self.bitboards.piece_bitboard(ChessPiece::BQ))).is_not_zero()
                    || (get_king_attack(square).bit_and(&self.bitboards.piece_bitboard(ChessPiece::BK))).is_not_zero();
            }
        }
    }

    //calculates all squares attacked by pinning pieces, that passes through a square
    pub(crate) const fn pin_mask(&self, square: Square) -> Bitboard {
        let mut pin_mask: Bitboard = Bitboard::ZERO;
        let mut pinners = self.data.pinner_bb;
        let side = self.data.side_to_move;
        let king_square = self.bitboards.piece_bitboard(ChessPiece(side, PieceType::King)).lsb_square().expect("King not found!");
        while pinners.is_not_zero() {
            let pinner = pinners.lsb_square().unwrap();
            let pinner_bb = pinners.lsb_bitboard();
            // check if square is between king and potential_pinner
            let ray = rays(king_square, pinner);
            if ray.nth_is_not_zero(square) {
                pin_mask = pin_mask.bit_or(&ray.bit_or(&pinner_bb));
            }
            pinners.pop_lsb();
        }
        return pin_mask;
    }

    #[inline(always)]
    const fn king_square(&self, side: Side) -> Square {
        self.bitboards.piece_bitboard(ChessPiece(side, PieceType::King)).lsb_square().expect("King not found!")
    }

    pub fn update_state(&mut self, chess_move: &ChessMove) {
        let mut enpassant_bb: Bitboard = Bitboard::ZERO;
        let mut check_bb: Bitboard = Bitboard::ZERO;
        let mut pinned_bb: Bitboard = Bitboard::ZERO;
        let mut pinner_bb: Bitboard = Bitboard::ZERO;
        let side = self.side();
        let enm_king_square: Square = self.bitboards.piece_bitboard(ChessPiece(side.update(), PieceType::King)).lsb_square().expect("King not found!");
        let source: Square = chess_move.source();
        let target: Square = chess_move.target();
        //assert!(
        //    self.mailbox.square_index(&source).expect("update_state error: source mailbox is None");.is_some(),
        //    "position:\n\r{}\n\rposition:\n\r{}\n\rchess_move:{:?}\n\rchess_move:{:?}\n\r",
        //    self,
        //    self,
        //    chess_move,
        //    chess_move
        //);
        let source_piece = self.mailbox.square_index(source).expect("update_state error: source mailbox is None");
        let target_piece = self.mailbox.square_index(target);

        //assert!(self.piece_bbs[enemy_king_index].nth_is_zero(target), "position:\n\r{}\n\rposition:\n\r{}\n\rposition:\n\r{}\n\r", self, self, self);
        let mut current_hash = self.hash();
        current_hash ^= ZobristHash::enpassant_hash(self.data.enpassant_bb);

        let mut is_counter_reset: bool = false; //fifty-move-rule counter

        /* special case bookkeeping */
        match source_piece {
            /* castling */
            ChessPiece(Side::White, PieceType::King) => {
                if self.data.castle_bools[0] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                    self.data.castle_bools[0] = false;
                }
                if self.data.castle_bools[1] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                    self.data.castle_bools[1] = false;
                }
            }

            ChessPiece(Side::Black, PieceType::King) => {
                if self.data.castle_bools[2] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                    self.data.castle_bools[2] = false;
                }
                if self.data.castle_bools[3] {
                    current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                    self.data.castle_bools[3] = false;
                }
            }

            ChessPiece(Side::White, PieceType::Rook) => {
                if source == Square::W_KINGSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[0] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                        self.data.castle_bools[0] = false;
                    }
                } else if source == Square::W_QUEENSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[1] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                        self.data.castle_bools[1] = false
                    }
                }
            }

            ChessPiece(Side::Black, PieceType::Rook) => {
                if source == Square::B_KINGSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[2] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                        self.data.castle_bools[2] = false;
                    }
                } else if source == Square::B_QUEENSIDE_ROOK_SQ_SOURCE {
                    if self.data.castle_bools[3] {
                        current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                        self.data.castle_bools[3] = false
                    }
                }
            }

            /* enpassant and fifty-move-rule */
            ChessPiece(Side::White, PieceType::Pawn) => {
                //reset 50-move rule
                self.data.fifty_move_rule_counter = 0;
                is_counter_reset = true;
                //if move is a 2-square pawn move, update enpassant bitboard
                if self.is_pawn_move_enpassant_relevant(&source, &target) {
                    //FIXME should check if enpassant is even legal for enemy
                    enpassant_bb.set_bit(Square::nth(target.to_usize() - 8));
                }
                check_bb = check_bb.bit_or(&get_b_pawn_attack(enm_king_square).bit_and(&Bitboard::nth(target)));
            }

            ChessPiece(Side::Black, PieceType::Pawn) => {
                //reset 50-move rule
                self.data.fifty_move_rule_counter = 0;
                is_counter_reset = true;
                //if move is a 2-square pawn move, update enpassant bitboard
                if self.is_pawn_move_enpassant_relevant(&source, &target) {
                    //FIXME should check if enpassant is even legal for enemy
                    enpassant_bb.set_bit(Square::nth(target.to_usize() + 8));
                }
                check_bb = check_bb.bit_or(&get_w_pawn_attack(enm_king_square).bit_and(&Bitboard::nth(target)));
            }

            ChessPiece(_, PieceType::Knight) => check_bb = check_bb.bit_or(&get_knight_attack(enm_king_square).bit_and(&Bitboard::nth(target))),
            _ => (),
        }

        //move the piece
        self.bitboards.pop_bit(source_piece, source);
        self.bitboards.set_bit(source_piece, target);
        current_hash ^= ZobristHash::piece_hash(source, source_piece);
        current_hash ^= ZobristHash::piece_hash(target, source_piece);
        self.mailbox.set(None, source);
        self.mailbox.set(Some(source_piece), target);

        //additional book keeping
        match chess_move.move_type() {
            MoveType::Normal => {
                //dealing with captures
                if let Some(target_piece) = target_piece {
                    self.bitboards.pop_bit(target_piece, target);
                    #[cfg(feature = "piececolourboard")]
                    if source_piece.1 == target_piece.1 {
                        self.bitboards.piece[target_piece.1 as usize].set_bit(target);
                    }
                    current_hash ^= ZobristHash::piece_hash(target, target_piece);

                    //reset 50-move rule
                    self.data.fifty_move_rule_counter = 0;
                    is_counter_reset = true;

                    //if capturing enemy rook, update castling rights
                    match (target_piece, target) {
                        (ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[0] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                                self.data.castle_bools[0] = false;
                            }
                        }
                        (ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[1] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                                self.data.castle_bools[1] = false;
                            }
                        }
                        (ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[2] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                                self.data.castle_bools[2] = false;
                            }
                        }
                        (ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[3] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                                self.data.castle_bools[3] = false;
                            }
                        }
                        _ => (),
                    }
                }
            }

            MoveType::Castle(castling) => {
                let (piece, rook_square_source, rook_square_target) = match castling {
                    Castling::Kingside(Side::White) => (ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE, Square::W_KINGSIDE_ROOK_SQ_TARGET),
                    Castling::Queenside(Side::White) => (ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE, Square::W_QUEENSIDE_ROOK_SQ_TARGET),
                    Castling::Kingside(Side::Black) => (ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE, Square::B_KINGSIDE_ROOK_SQ_TARGET),
                    Castling::Queenside(Side::Black) => (ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE, Square::B_QUEENSIDE_ROOK_SQ_TARGET),
                };
                assert!(self.bitboards.piece_bitboard(piece).nth_is_not_zero(rook_square_source));
                self.bitboards.pop_bit(piece, rook_square_source);
                self.bitboards.set_bit(piece, rook_square_target);
                self.mailbox.set(None, rook_square_source);
                self.mailbox.set(Some(piece), rook_square_target);

                //update hash
                current_hash ^= ZobristHash::piece_hash(rook_square_source, piece);
                current_hash ^= ZobristHash::piece_hash(rook_square_target, piece);
            }

            MoveType::EnPassant => {
                let enemy_pawn_square: Square;
                let enemy_piece: ChessPiece = ChessPiece(side.update(), PieceType::Pawn);
                match self.data.side_to_move {
                    Side::White => {
                        enemy_pawn_square = Square::nth(target.to_usize() - 8);
                    }
                    Side::Black => {
                        enemy_pawn_square = Square::nth(target.to_usize() + 8);
                    }
                }

                debug_assert!(self.bitboards.piece_bitboard(enemy_piece).nth_is_not_zero(enemy_pawn_square));
                debug_assert!(self.mailbox.square_index(enemy_pawn_square) == cpt!(p) || self.mailbox.square_index(enemy_pawn_square) == cpt!(P));
                self.bitboards.pop_bit(enemy_piece, enemy_pawn_square);
                current_hash ^= ZobristHash::piece_hash(enemy_pawn_square, enemy_piece);
                self.mailbox.set(None, enemy_pawn_square);
            }

            MoveType::Promotion(piece_type) => {
                if piece_type == PieceType::Knight {
                    check_bb = check_bb.bit_or(&get_knight_attack(enm_king_square).bit_and(&Bitboard::nth(target)));
                }

                let promoted_piece = ChessPiece(self.data.side_to_move, piece_type);

                //dealing with captures
                if let Some(target_piece) = target_piece {
                    self.bitboards.pop_bit(target_piece, target);
                    #[cfg(feature = "piececolourboard")]
                    if source_piece.1 == target_piece.1 {
                        self.bitboards.piece[target_piece.1 as usize].set_bit(target);
                    }
                    current_hash ^= ZobristHash::piece_hash(target, target_piece);

                    //reset 50-move rule
                    self.data.fifty_move_rule_counter = 0;
                    is_counter_reset = true;

                    //if capturing enemy rook, update castling rights
                    match (target_piece, target) {
                        (ChessPiece::WR, Square::W_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[0] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::White));
                            }
                            self.data.castle_bools[0] = false;
                        }
                        (ChessPiece::WR, Square::W_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[1] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::White));
                            }
                            self.data.castle_bools[1] = false;
                        }
                        (ChessPiece::BR, Square::B_KINGSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[2] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Kingside(Side::Black));
                            }
                            self.data.castle_bools[2] = false;
                        }
                        (ChessPiece::BR, Square::B_QUEENSIDE_ROOK_SQ_SOURCE) => {
                            if self.data.castle_bools[3] {
                                current_hash ^= ZobristHash::castle_hash(Castling::Queenside(Side::Black));
                            }
                            self.data.castle_bools[3] = false;
                        }
                        _ => (),
                    }
                }

                //remove the pawn piece
                self.bitboards.pop_bit(source_piece, target);
                current_hash ^= ZobristHash::piece_hash(target, source_piece);

                //add the promoted piece
                self.bitboards.set_bit(promoted_piece, target);
                current_hash ^= ZobristHash::piece_hash(target, promoted_piece);
                self.mailbox.set(Some(promoted_piece), target);
            }
        }

        //cozy-chess tech
        //note that previously check_bb contains all checking knight pieces

        // pieces: white pawn, white knight, white bishop, white rook, white queen, white king,
        //         black pawn, black knight, black bishop, black rook, black queen, black king,
        let mut check_mask: Bitboard = check_bb;
        //note that attackers can only ever be: a rook, a bishop, or a queen
        let bishops_or_queens: Bitboard;
        let rooks_or_queens: Bitboard;
        match self.side() {
            Side::White => {
                bishops_or_queens = self.bitboards.piece_bitboard(ChessPiece::WQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::WB));
                rooks_or_queens = self.bitboards.piece_bitboard(ChessPiece::WQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::WR));
            }
            Side::Black => {
                bishops_or_queens = self.bitboards.piece_bitboard(ChessPiece::BQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::BB));
                rooks_or_queens = self.bitboards.piece_bitboard(ChessPiece::BQ).bit_or(&self.bitboards.piece_bitboard(ChessPiece::BR));
            }
        }
        let bishop_ray_hits = get_bishop_ray(enm_king_square).bit_and(&bishops_or_queens);
        let rook_ray_hits = get_rook_ray(enm_king_square).bit_and(&rooks_or_queens);
        let mut attackers: Bitboard = bishop_ray_hits.bit_or(&rook_ray_hits);

        //note that attackers can only ever be: a rook, a bishop, or a queen
        while attackers.is_not_zero() {
            let attacker_square: Square = attackers.lsb_square().unwrap();
            let attacker_bb: Bitboard = attackers.lsb_bitboard();
            let ray: Bitboard = rays(attacker_square, enm_king_square);
            let pinned_pieces: Bitboard = ray.bit_and(&self.bitboards.blockers());
            match pinned_pieces.count_ones() {
                0 => {
                    check_bb = check_bb.bit_or(&attacker_bb);
                    check_mask = check_mask.bit_or(&attacker_bb.bit_or(&ray));
                }
                1 => {
                    pinned_bb = pinned_bb.bit_or(&pinned_pieces);
                    pinner_bb = pinner_bb.bit_or(&attacker_bb);
                }
                _ => (),
            }
            attackers.pop_lsb();
        }
        //compute check_bb and check_mask for knight

        if self.data.side_to_move == Side::Black {
            self.data.full_move_counter += 1;
        }

        self.data.side_to_move = self.data.side_to_move.update();
        current_hash ^= ZobristHash::side_hash();
        if is_counter_reset == false {
            self.data.fifty_move_rule_counter += 1;
        }

        self.data.enpassant_bb = enpassant_bb;
        current_hash ^= ZobristHash::enpassant_hash(enpassant_bb);

        self.data.zobrist_hash = current_hash;

        //self.compute_check_bb();
        self.data.check_bb = check_bb;
        self.data.check_mask = check_mask;

        //self.compute_pin_data();
        self.data.pinner_bb = pinner_bb;
        self.data.pinned_bb = pinned_bb;
    }

    pub(crate) const fn is_king_in_check(&self, king_side: Side) -> bool {
        let square = self.bitboards.piece_bitboard(ChessPiece(king_side, PieceType::King)).lsb_square().expect("King not found!");
        return self.is_square_attacked(square, king_side.update(), self.bitboards.blockers());
    }

    const fn side(&self) -> Side {
        self.data.side_to_move
    }

    const fn hash(&self) -> ZobristHash {
        self.data.zobrist_hash
    }

    #[inline(always)]
    fn is_pawn_move_enpassant_relevant(&self, source: &Square, target: &Square) -> bool {
        match self.side() {
            Side::White => {
                (source.to_usize() + 16 == target.to_usize())
                    && ((matches!(self.mailbox.square_index(target.right()), cpt!(p)) && (source.to_col_usize() != 7))
                        || matches!(self.mailbox.square_index(target.left()), cpt!(p)) && (source.to_col_usize() != 0))
            }
            Side::Black => {
                (source.to_usize() == target.to_usize() + 16)
                    && (matches!(self.mailbox.square_index(target.right()), cpt!(P)) && (source.to_col_usize() != 7)
                        || matches!(self.mailbox.square_index(target.left()), cpt!(P)) && (source.to_col_usize() != 0))
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
