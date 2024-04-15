#![allow(long_running_const_eval)]
mod chessmove;
//mod constdata;
mod constvec;
mod hashvec;
mod movegen;

use self::chessmove::*;
//use self::constdata::*;
use self::hashvec::*;
//use self::movegen::*;

use bitboard::*;
use std::fmt::Display;

// note: castle_bools[] = [white-king  side castle,
//                         white-queen side castle,
//                         black-king  side castle,
//                         black-queen side castle]
//
// pieces: white king, white queen, white knight, white bishop, white rook, white pawn,
//         black king, black queen, black knight, black bishop, black rook, black pawn,

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChessBoard {
    pub piece_bbs: [BitBoard; 12],
    pub mailbox: [Option<ChessPiece>; 64],
    pub castle_bools: [bool; 4],
    pub state: GameState,
    pub enpassant_bb: BitBoard,
    pub check_bb: BitBoard, //piece locations causing the check
    pub side_to_move: Side,
    pub half_move_clock: u16,
    pub full_move_counter: u16,
    pub hash_vec: HashVec,
    pub moverule_counter: u16,
    pub moves: Option<(MoveVec, ZorbistHash)>, //pub history: [([BitBoard; 12], u16); 8], //HashArray has maximum capacity of (1<<11), u16 should be more than enough, u16::MAX is null
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GameState {
    Ongoing,
    WhiteWins,
    BlackWins,
    Draw,
}

type ChessPiece = (Side, PieceType);

impl Default for ChessBoard {
    fn default() -> Self {
        Self {
            piece_bbs: INITIAL_CHESS_POS,
            mailbox: INITIAL_MAILBOX,
            castle_bools: [true; 4],
            state: GameState::Ongoing,
            enpassant_bb: BitBoard::ZERO,
            check_bb: BitBoard::ZERO,
            side_to_move: Side::White,
            half_move_clock: 0,
            full_move_counter: 0,
            hash_vec: HashVec::new_from(ZorbistHash::new(0x157014a493d64d76)),
            moverule_counter: 0, //50 move rule counter
            moves: None,
        }
    }
}

impl Display for ChessBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();

        // get empty_squares
        let mut empty_squares = BitBoard::ZERO;
        for piece_bb in self.piece_bbs {
            empty_squares = piece_bb | empty_squares;
        }
        empty_squares = !empty_squares;

        // append characters according to piece
        for i in 1..=64usize {
            if !empty_squares.nth_is_zero(64 - i) {
                s.push('.');
            } else {
                let mut j = 0usize;
                while j < self.piece_bbs.len() {
                    let piece_bb: BitBoard = self.piece_bbs[j];
                    if !piece_bb.nth_is_zero(64 - i) {
                        s.push(UNICODE_SYM[j]);
                    }
                    j += 1;
                }
            }

            if i % 8 == 0 {
                s.push('\n');
            }
        }
        write!(f, "{}", s)
    }
}

impl ChessBoard {
    pub const fn start_pos() -> Self {
        Self {
            piece_bbs: INITIAL_CHESS_POS,
            mailbox: INITIAL_MAILBOX,
            castle_bools: [true; 4],
            state: GameState::Ongoing,
            enpassant_bb: BitBoard::ZERO,
            check_bb: BitBoard::ZERO,
            side_to_move: Side::White,
            half_move_clock: 0,
            full_move_counter: 0,
            hash_vec: HashVec::new_from(ZorbistHash::new(0x157014a493d64d76)),
            moverule_counter: 0, //50 move rule counter
            moves: None,
        }
    }

    //todo:
    pub fn from_fen(input: &str) -> ChessBoard {
        assert!(input.is_ascii());
        let input_vec: Vec<&str> = input.split_ascii_whitespace().collect();
        assert!(input_vec.len() == 6);
        let mut chessboard = ChessBoard::start_pos();

        // parse piece placement data
        let mut i: usize = 0;
        while i < 8 {
            let mut j = 0usize;
            while j < 8 {
                let square: usize = 8 * i + j;
                let mut k: usize = 0;
                while k < input_vec[0].len() {
                    let s = match input_vec[0].chars().nth(k) {
                        Some(x) => x,
                        None => unreachable!(),
                    };
                    if s.is_ascii_alphabetic() {
                        chessboard.piece_bbs[sym_index(s)].set_bit(square);
                        let piece_data = match s {
                            'K' => (Side::White, PieceType::King),
                            'Q' => (Side::White, PieceType::Queen),
                            'N' => (Side::White, PieceType::Knight),
                            'B' => (Side::White, PieceType::Bishop),
                            'R' => (Side::White, PieceType::Rook),
                            'P' => (Side::White, PieceType::Pawn),
                            'k' => (Side::Black, PieceType::King),
                            'q' => (Side::Black, PieceType::Queen),
                            'n' => (Side::Black, PieceType::Knight),
                            'b' => (Side::Black, PieceType::Bishop),
                            'r' => (Side::Black, PieceType::Rook),
                            'p' => (Side::Black, PieceType::Pawn),
                            _ => panic!("from_fen error!: invalidcharacter!"),
                        };
                        chessboard.mailbox[square] = Some(piece_data);
                    } else if s.is_ascii_digit() {
                        j += (s.to_digit(10).unwrap() as usize) - 1;
                        //break
                    } else {
                        panic!("from_fen error: invalid char in piece placement portion!")
                    }
                    k += 1;
                }
                j += 1;
            }
            i += 1
        }
        // parse active colour
        chessboard.side_to_move = match input_vec[1] {
            "w" => Side::White,
            "b" => Side::Black,
            _ => panic!("from_fen error: invalid active side!"),
        };

        i = 0;
        // parse castling information
        while i < input_vec[2].len() {
            let s = match input_vec[2].chars().nth(i) {
                Some(x) => x,
                None => unreachable!(),
            };

            match s {
                '-' => continue,
                'K' => chessboard.castle_bools[0] = true,
                'Q' => chessboard.castle_bools[1] = true,
                'k' => chessboard.castle_bools[2] = true,
                'q' => chessboard.castle_bools[3] = true,
                _ => panic!("from_fen error: invalid castling information!"),
            }
            i += 1;
        }

        // parse en passant information
        if input_vec[3] != "-" {
            chessboard.enpassant_bb.set_bit(square_index(input_vec[3]));
        }
        //parse halfmove clock
        //assert!(input_vec[4].is_ascii_digit()); doesnt work for &str
        chessboard.half_move_clock = input_vec[4].parse::<u16>().unwrap();
        //parse fullmove number
        //assert!(input_vec[5].is_ascii_digit()); doesnt work for &str
        chessboard.full_move_counter = input_vec[5].parse::<u16>().unwrap();

        //calculate king_is_in_check information.
        assert!(chessboard.piece_bbs[0].count_ones() == 1);
        assert!(chessboard.piece_bbs[6].count_ones() == 1);
        let side = chessboard.side_to_move;
        if chessboard.king_is_in_check(side) {
            match side {
                Side::White => {
                    let blockers = chessboard.blockers();
                    if let Some(king_pos) = chessboard.piece_bbs[0].lsb_index() {
                        let mut check_bitboard = BitBoard::ZERO;
                        //q
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[07] & get_queen_attack(king_pos, blockers));
                        //n
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[08] & get_knight_attack(king_pos));
                        //b
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[09] & get_bishop_attack(king_pos, blockers));
                        //r
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[10] & get_rook_attack(king_pos, blockers));
                        //p
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[11] & get_w_pawn_attack(king_pos));
                        chessboard.check_bb = check_bitboard;
                    } else {
                        unreachable!();
                    }
                }

                Side::Black => {
                    let blockers = chessboard.blockers();
                    if let Some(king_pos) = chessboard.piece_bbs[6].lsb_index() {
                        let mut check_bitboard = BitBoard::ZERO;
                        //Q
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[01] & get_queen_attack(king_pos, blockers));
                        //N
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[02] & get_knight_attack(king_pos));
                        //B
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[03] & get_bishop_attack(king_pos, blockers));
                        //R
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[04] & get_rook_attack(king_pos, blockers));
                        //P
                        check_bitboard = check_bitboard | (chessboard.piece_bbs[05] & get_b_pawn_attack(king_pos));
                        chessboard.check_bb = check_bitboard;
                    } else {
                        unreachable!();
                    }
                }
            }
        }
        chessboard.get_state();
        chessboard.hash_vec.set(0, ZorbistHash::hash(&chessboard));
        return chessboard;
    }

    pub const fn blockers(&self) -> BitBoard {
        let mut i = 0;
        let mut bitboard: BitBoard = BitBoard::ZERO;
        while i < 12 {
            bitboard = bitboard.bit_or(&self.piece_bbs[i]);
            i += 1;
        }
        return bitboard;
    }

    pub const fn white_blockers(&self) -> BitBoard {
        let mut i = 0;
        let mut bitboard: BitBoard = BitBoard::ZERO;
        while i < 6 {
            bitboard = bitboard.bit_or(&self.piece_bbs[i]);
            i += 1;
        }
        return bitboard;
    }

    pub const fn black_blockers(&self) -> BitBoard {
        let mut i = 6;
        let mut bitboard: BitBoard = BitBoard::ZERO;
        while i < self.piece_bbs.len() {
            bitboard = bitboard.bit_or(&self.piece_bbs[i]);
            i += 1;
        }
        return bitboard;
    }

    pub const fn is_square_attacked(&self, square: usize, attacker_side: Side) -> bool {
        assert!(square < 64);
        let blockers = self.blockers();
        match attacker_side {
            Side::White => {
                return (get_b_pawn_attack(square).bit_and(&self.piece_bbs[5])).is_not_zero()
                    || (get_rook_attack(square, blockers).bit_and(&self.piece_bbs[4])).is_not_zero()
                    || (get_bishop_attack(square, blockers).bit_and(&self.piece_bbs[3])).is_not_zero()
                    || (get_knight_attack(square).bit_and(&self.piece_bbs[2])).is_not_zero()
                    || (get_queen_attack(square, blockers).bit_and(&self.piece_bbs[1])).is_not_zero()
                    || (get_king_attack(square).bit_and(&self.piece_bbs[0])).is_not_zero()
            }
            Side::Black => {
                return (get_b_pawn_attack(square).bit_and(&self.piece_bbs[11])).is_not_zero()
                    || (get_rook_attack(square, blockers).bit_and(&self.piece_bbs[10])).is_not_zero()
                    || (get_bishop_attack(square, blockers).bit_and(&self.piece_bbs[9])).is_not_zero()
                    || (get_knight_attack(square).bit_and(&self.piece_bbs[8])).is_not_zero()
                    || (get_queen_attack(square, blockers).bit_and(&self.piece_bbs[7])).is_not_zero()
                    || (get_king_attack(square).bit_and(&self.piece_bbs[6])).is_not_zero()
            }
        }
    }

    pub const fn is_piece_pinned(&self, square: usize) -> bool {
        assert!(square < 64);
        let mut chessboard = self.const_clone();
        let piece = match self.mailbox[square] {
            Some(p) => p,
            None => {
                panic!("is_piece_pinned error: square is empty!");
            }
        };
        let index = cp_index(piece);
        chessboard.piece_bbs[index] = chessboard.piece_bbs[index].bit_and(&BitBoard::nth(square).bit_not());
        chessboard.mailbox[index] = None;
        let side = self.side_to_move;

        // assertion hack
        match piece {
            cpt!(K) | cpt!(k) => panic!("is_piece_pinned error: invalid piece to check!"),
            _ => {}
        }
        // if king is not in check, test if removing piece causes king to be in check
        if !self.king_is_in_check(side) {
            return chessboard.king_is_in_check(side);
        } else {
            //note: THIS BIT IS SLOW!!!
            let (q_index, b_index, r_index) = match side {
                Side::White => (07, 09, 10),
                Side::Black => (01, 03, 04),
            };

            let diagonals = self.piece_bbs[q_index].bit_or(&self.piece_bbs[b_index]);
            let laterals = self.piece_bbs[q_index].bit_or(&self.piece_bbs[r_index]);

            let enemies = match side {
                Side::White => self.black_blockers(),
                Side::Black => self.white_blockers(),
            };

            assert!(self.piece_bbs[0].count_ones() == 1 && self.piece_bbs[6].count_ones() == 1);

            let king_pos: usize = match side {
                Side::White => match self.piece_bbs[0].lsb_index() {
                    Some(x) => x,
                    None => unreachable!(),
                },
                Side::Black => match self.piece_bbs[6].lsb_index() {
                    Some(x) => x,
                    None => unreachable!(),
                },
            };

            let removed_blockers = self.blockers().bit_and(&BitBoard::nth(square).bit_not());
            let bb1 = get_bishop_attack(king_pos, removed_blockers).bit_and(&diagonals);
            let bb2 = get_rook_attack(king_pos, removed_blockers).bit_and(&laterals);
            let mut potential_pinners = enemies.bit_and(&bb1.bit_or(&bb2));
            while potential_pinners.is_not_zero() {
                let potential_pinner = match potential_pinners.lsb_index() {
                    Some(x) => x,
                    None => unreachable!(),
                };
                // check if piece is between king and potential_pinner
                if RAYS[king_pos][potential_pinner].nth_is_not_zero(square) {
                    return true;
                }
                potential_pinners = potential_pinners.pop_bit(potential_pinner);
            }
        }
        return false;
    }

    pub const fn king_is_in_check(&self, king_side: Side) -> bool {
        let i = match king_side {
            Side::White => 0,
            Side::Black => 6,
        };

        let square = match self.piece_bbs[i].lsb_index() {
            Some(x) => x,
            None => panic!("king_is_in_check error: king not found!"),
        };

        self.is_square_attacked(square, self.side_to_move.update())
    }

    pub const fn const_clone(&self) -> ChessBoard {
        ChessBoard {
            piece_bbs: self.piece_bbs,
            mailbox: self.mailbox,
            castle_bools: self.castle_bools,
            state: self.state,
            enpassant_bb: self.enpassant_bb,
            side_to_move: self.side_to_move,
            half_move_clock: self.half_move_clock,
            full_move_counter: self.full_move_counter,
            check_bb: self.check_bb,
            hash_vec: self.hash_vec,
            moverule_counter: self.moverule_counter,
            moves: self.moves,
        }
    }

    pub const fn get_state(&self) -> GameState {
        assert!(self.moves.is_some());
        let (m, h) = match self.moves {
            Some(x) => x,
            None => unreachable!(),
        };
        assert!(self.get_hash().val == h.val);
        if m.len() != 0 {
            return GameState::Ongoing;
        } else if self.hash_count(self.get_hash()) == 3 || self.moverule_counter > 100 {
            return GameState::Draw;
        } else if self.king_is_in_check(Side::White) {
            return GameState::BlackWins;
        } else if self.king_is_in_check(Side::Black) {
            return GameState::WhiteWins;
        } else {
            return GameState::Draw;
        }
    }

    #[inline(always)]
    pub const fn get_hash(&self) -> ZorbistHash {
        self.hash_vec.tail()
    }

    pub const fn hash_count(&self, hash: ZorbistHash) -> u16 {
        let mut count: u16 = 0;
        let mut i = 0;
        assert!(self.hash_vec.len() > 0);
        while i < self.hash_vec.len() {
            if self.hash_vec.nth(i).val == hash.val {
                count += 1;
            }
            i += 1;
        }
        return count;
    }

    #[inline(always)]
    pub const fn rep_count(&self) -> u16 {
        self.hash_count(self.get_hash())
    }

    #[inline(always)]
    pub const fn update_state(&self, chess_move: ChessMove) -> ChessBoard {
        movegen::update_state(self, chess_move)
    }

    #[inline(always)]
    pub const fn generate_moves(&self) -> MoveVec {
        movegen::generate_moves(self)
    }
}
/* ================ constants and supporting functions ================ */
type BB = BitBoard;

const ASCII_SYM: [char; 12] = ['K', 'Q', 'N', 'B', 'R', 'P', 'k', 'q', 'n', 'b', 'r', 'p'];
const UNICODE_SYM: [char; 12] = ['♚', '♛', '♞', '♝', '♜', '♟', '♔', '♕', '♘', '♗', '♖', '♙'];

const W_KING_SIDE_CASTLE_MASK: BB = BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000110);
const W_QUEEN_SIDE_CASTLE_MASK: BB = BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01110000);
const B_KING_SIDE_CASTLE_MASK: BB = BB::new(0b00000110_00000000_00000000_00000000_00000000_00000000_00000000_00000000);
const B_QUEEN_SIDE_CASTLE_MASK: BB = BB::new(0b01110000_00000000_00000000_00000000_00000000_00000000_00000000_00000000);

#[rustfmt::skip]
pub const INITIAL_CHESS_POS: [BB; 12] = [
    BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00001000), // ♔
    BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00010000), // ♕
    BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01000010), // ♘
    BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00100100), // ♗
    BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_10000001), // ♖
    BB::new(0b00000000_00000000_00000000_00000000_00000000_00000000_11111111_00000000), // ♙
    BB::new(0b00001000_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♚
    BB::new(0b00010000_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♛
    BB::new(0b01000010_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♞
    BB::new(0b00100100_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♝
    BB::new(0b10000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♜
    BB::new(0b00000000_11111111_00000000_00000000_00000000_00000000_00000000_00000000), // ♟
];

#[rustfmt::skip]
#[macro_export] 
macro_rules! opt_cpt {
    (K) => {Some((Side::White, PieceType::King  ))};
    (Q) => {Some((Side::White, PieceType::Queen ))};
    (N) => {Some((Side::White, PieceType::Knight))};
    (B) => {Some((Side::White, PieceType::Bishop))};
    (R) => {Some((Side::White, PieceType::Rook  ))};
    (P) => {Some((Side::White, PieceType::Pawn  ))};
    (k) => {Some((Side::Black, PieceType::King  ))};
    (q) => {Some((Side::Black, PieceType::Queen ))};
    (n) => {Some((Side::Black, PieceType::Knight))};
    (b) => {Some((Side::Black, PieceType::Bishop))};
    (r) => {Some((Side::Black, PieceType::Rook  ))};
    (p) => {Some((Side::Black, PieceType::Pawn  ))};
    (_) => {None};
}

#[rustfmt::skip]
#[macro_export]
macro_rules! cpt {
    (K) => {(Side::White, PieceType::King  )};
    (Q) => {(Side::White, PieceType::Queen )};
    (N) => {(Side::White, PieceType::Knight)};
    (B) => {(Side::White, PieceType::Bishop)};
    (R) => {(Side::White, PieceType::Rook  )};
    (P) => {(Side::White, PieceType::Pawn  )};
    (k) => {(Side::Black, PieceType::King  )};
    (q) => {(Side::Black, PieceType::Queen )};
    (n) => {(Side::Black, PieceType::Knight)};
    (b) => {(Side::Black, PieceType::Bishop)};
    (r) => {(Side::Black, PieceType::Rook  )};
    (p) => {(Side::Black, PieceType::Pawn  )};
}

#[rustfmt::skip]
pub const INITIAL_MAILBOX: [Option<ChessPiece>; 64] = [
    opt_cpt!(R), opt_cpt!(N), opt_cpt!(B), opt_cpt!(K), opt_cpt!(Q), opt_cpt!(B), opt_cpt!(N), opt_cpt!(R),
    opt_cpt!(P), opt_cpt!(P), opt_cpt!(P), opt_cpt!(P), opt_cpt!(P), opt_cpt!(P), opt_cpt!(P), opt_cpt!(P),
    opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_),
    opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_),
    opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_),
    opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_), opt_cpt!(_),
    opt_cpt!(p), opt_cpt!(p), opt_cpt!(p), opt_cpt!(p), opt_cpt!(p), opt_cpt!(p), opt_cpt!(p), opt_cpt!(p),
    opt_cpt!(r), opt_cpt!(n), opt_cpt!(b), opt_cpt!(k), opt_cpt!(q), opt_cpt!(b), opt_cpt!(n), opt_cpt!(r),
];

#[rustfmt::skip]
pub const fn cp_index(data: ChessPiece) -> usize {
    match data {
        cpt!(K) => 00,
        cpt!(Q) => 01,
        cpt!(N) => 02,
        cpt!(B) => 03,
        cpt!(R) => 04,
        cpt!(P) => 05,
        cpt!(k) => 06,
        cpt!(q) => 07,
        cpt!(n) => 08,
        cpt!(b) => 09,
        cpt!(r) => 10,
        cpt!(p) => 11,
    }
}

#[rustfmt::skip]
pub const fn sym_index(c: char) -> usize {
    match c {
        'K' =>  0,
        'Q' =>  1,
        'N' =>  2,
        'B' =>  3,
        'R' =>  4,
        'P' =>  5,
        'k' =>  6,
        'q' =>  7,
        'n' =>  8,
        'b' =>  9,
        'r' => 10,
        'p' => 11,
        _   => panic!("sym_index error: invalid char!"),
    }
}

pub const fn square_index(square_name: &str) -> usize {
    let mut i: usize = 0;
    while i < 64 {
        let mut j: usize = 0;
        let mut is_match = SQUARE_SYM[i].as_bytes().len() == square_name.as_bytes().len();
        while j < SQUARE_SYM[i].as_bytes().len() {
            if SQUARE_SYM[i].as_bytes()[j] != square_name.as_bytes()[j] {
                is_match = false;
            }
            j += 1;
        }
        if is_match {
            return i;
        }
        i += 1
    }
    panic!("square_index error: invalid square!");
}
