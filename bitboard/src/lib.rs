#![allow(long_running_const_eval)]
pub mod chessbb;
mod constdata;
pub mod init;
use chessbb::*;
use std::fmt::Display;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitBoard {
    data: u64,
}

impl Display for BitBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for i in 0..8u64 {
            s.push_str(&format!("{:08b}", (self.data & (0xFFu64 << 8 * (7 - i))) >> 8 * (7 - i)));
            s.push('\n');
        }
        write!(f, "{}", s)
    }
}

impl BitAnd for BitBoard {
    type Output = BitBoard;
    fn bitand(self, rhs: BitBoard) -> Self::Output {
        BitBoard { data: self.data & rhs.data }
    }
}

impl BitAndAssign for BitBoard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.data &= rhs.data;
    }
}

impl BitOr for BitBoard {
    type Output = BitBoard;
    fn bitor(self, rhs: BitBoard) -> Self::Output {
        BitBoard { data: self.data | rhs.data }
    }
}

impl BitOrAssign for BitBoard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.data |= rhs.data;
    }
}

impl BitXor for BitBoard {
    type Output = BitBoard;
    fn bitxor(self, rhs: BitBoard) -> Self::Output {
        BitBoard { data: self.data ^ rhs.data }
    }
}
impl BitXorAssign for BitBoard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.data ^= rhs.data;
    }
}

impl Not for BitBoard {
    type Output = BitBoard;
    fn not(self) -> Self::Output {
        BitBoard { data: !self.data }
    }
}

/* some u64 bit manipulation support */
impl BitBoard {
    #[inline(always)]
    pub const fn new(data: u64) -> Self {
        Self { data }
    }

    //pub const MOVES: u16 = 64 * 64 * 5; //{64-from square index} x {64-to square index} x {promotion data}
    pub const ZERO: BitBoard = BitBoard { data: 0u64 };
    pub const ONES: BitBoard = BitBoard { data: u64::MAX };

    //creates a bitboard bitboard with a single non-zero bit
    #[inline(always)]
    pub const fn nth(n: usize) -> Self {
        Self { data: 1u64 << n }
    }

    #[inline(always)]
    pub const fn count_ones(&self) -> u32 {
        self.data.count_ones()
    }

    #[inline(always)]
    pub const fn bit_and(&self, other: &BitBoard) -> BitBoard {
        BitBoard { data: self.data & other.data }
    }
    #[inline(always)]
    pub const fn bit_or(&self, other: &BitBoard) -> BitBoard {
        BitBoard { data: self.data | other.data }
    }

    #[inline(always)]
    pub const fn bit_xor(&self, other: &BitBoard) -> BitBoard {
        BitBoard { data: self.data ^ other.data }
    }

    #[inline(always)]
    pub const fn bit_not(&self) -> BitBoard {
        BitBoard { data: !self.data }
    }

    #[inline(always)]
    pub fn set_bit(&mut self, i: usize) {
        self.data = self.data | 1u64 << i;
    }

    #[inline(always)]
    pub const fn lsb_index(&self) -> Option<usize> {
        if self.data == 0u64 {
            return None;
        } else {
            return Some(self.data.trailing_zeros() as usize);
        }
    }

    #[inline(always)]
    pub const fn get_bit(&self, i: usize) -> BitBoard {
        BitBoard { data: self.data & (1u64 << i) }
    }

    #[inline(always)]
    pub const fn pop_bit(&self, i: usize) -> BitBoard {
        BitBoard { data: self.data & !(1u64 << i) }
    }

    // why did I separated the return value into two cases?
    //pub const fn pop_bit(&self, i: usize) -> BitBoard {
    //    BitBoard {
    //        data: match self.get_bit(i).data {
    //            0u64 => 0,
    //            _ => self.data & !(1u64 << i),
    //        },
    //    }
    //}

    #[inline(always)]
    pub const fn nth_is_zero(&self, index: usize) -> bool {
        match self.data & (1u64 << index) {
            0 => true,
            _ => false,
        }
    }

    #[inline(always)]
    pub const fn nth_is_not_zero(&self, index: usize) -> bool {
        match self.data & (1u64 << index) {
            0 => false,
            _ => true,
        }
    }

    #[inline(always)]
    pub const fn is_zero(&self) -> bool {
        self.data == 0u64
    }

    #[inline(always)]
    pub const fn is_not_zero(&self) -> bool {
        self.data != 0u64
    }

    //reflects the bitboard across a horizontal line.
    pub const fn refl(&self) -> BitBoard {
        let mut data: u64 = 0;
        let mut i = 0;
        let mask: u64 = 0b11111111;
        while i < 8 {
            data = data | (self.data & (mask << (8 * 7 - i)));
            i += 1
        }
        return BitBoard { data };
    }
}

/* ================================ constants ================================ */

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

pub const SQUARE_SYM_REV: [&str; 64] = [
    "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8", //
    "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7", //
    "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6", //
    "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5", //
    "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4", //
    "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3", //
    "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2", //
    "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1", //
];

pub const SQUARE_SYM: [&str; 64] = [
    "h1", "g1", "f1", "e1", "d1", "c1", "b1", "a1", //
    "h2", "g2", "f2", "e2", "d2", "c2", "b2", "a2", //
    "h3", "g3", "f3", "e3", "d3", "c3", "b3", "a3", //
    "h4", "g4", "f4", "e4", "d4", "c4", "b4", "a4", //
    "h5", "g5", "f5", "e5", "d5", "c5", "b5", "a5", //
    "h6", "g6", "f6", "e6", "d6", "c6", "b6", "a6", //
    "h7", "g7", "f7", "e7", "d7", "c7", "b7", "a7", //
    "h8", "g8", "f8", "e8", "d8", "c8", "b8", "a8", //
];

pub const RANK_CHAR: [char; 64] = [
    '1', '1', '1', '1', '1', '1', '1', '1', //
    '2', '2', '2', '2', '2', '2', '2', '2', //
    '3', '3', '3', '3', '3', '3', '3', '3', //
    '4', '4', '4', '4', '4', '4', '4', '4', //
    '5', '5', '5', '5', '5', '5', '5', '5', //
    '6', '6', '6', '6', '6', '6', '6', '6', //
    '7', '7', '7', '7', '7', '7', '7', '7', //
    '8', '8', '8', '8', '8', '8', '8', '8', //
];

pub const FILE_CHAR: [char; 64] = [
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
    'h', 'g', 'f', 'e', 'd', 'c', 'b', 'a', //
];

pub const ROWS: [usize; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0, //
    1, 1, 1, 1, 1, 1, 1, 1, //
    2, 2, 2, 2, 2, 2, 2, 2, //
    3, 3, 3, 3, 3, 3, 3, 3, //
    4, 4, 4, 4, 4, 4, 4, 4, //
    5, 5, 5, 5, 5, 5, 5, 5, //
    6, 6, 6, 6, 6, 6, 6, 6, //
    7, 7, 7, 7, 7, 7, 7, 7, //
];

pub const COLS: [usize; 64] = [
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
    0, 1, 2, 3, 4, 5, 6, 7, //
];

pub const DDIAG: [usize; 64] = [
    07, 08, 09, 10, 11, 12, 13, 14, //
    06, 07, 08, 09, 10, 11, 12, 13, //
    05, 06, 07, 08, 09, 10, 11, 12, //
    04, 05, 06, 07, 08, 09, 10, 11, //
    03, 04, 05, 06, 07, 08, 09, 10, //
    02, 03, 04, 05, 06, 07, 08, 09, //
    01, 02, 03, 04, 05, 06, 07, 08, //
    00, 01, 02, 03, 04, 05, 06, 07, //
];

pub const ADIAG: [usize; 64] = [
    00, 01, 02, 03, 04, 05, 06, 07, //
    01, 02, 03, 04, 05, 06, 07, 08, //
    02, 03, 04, 05, 06, 07, 08, 09, //
    03, 04, 05, 06, 07, 08, 09, 10, //
    04, 05, 06, 07, 08, 09, 10, 11, //
    05, 06, 07, 08, 09, 10, 11, 12, //
    06, 07, 08, 09, 10, 11, 12, 13, //
    07, 08, 09, 10, 11, 12, 13, 14, //
];

pub const RAYS: [[BitBoard; 64]; 64] = init::rays();

/* chessboard specific bitboard functions and definitions*/
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub const fn to_char(&self) -> char {
        match self {
            PieceType::Pawn => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub const fn update(&self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
}

#[inline(always)]
pub fn get_pawn_attack(square: usize, side: Side) -> BitBoard {
    match side {
        Side::White => W_PAWN_ATTACKS[square],
        Side::Black => B_PAWN_ATTACKS[square],
    }
}

#[inline(always)]
pub const fn get_w_pawn_attack(square: usize) -> BitBoard {
    W_PAWN_ATTACKS[square]
}

#[inline(always)]
pub const fn get_b_pawn_attack(square: usize) -> BitBoard {
    B_PAWN_ATTACKS[square]
}

#[inline(always)]
pub const fn get_knight_attack(square: usize) -> BitBoard {
    KNIGHT_ATTACKS[square]
}

#[inline(always)]
pub const fn get_king_attack(square: usize) -> BitBoard {
    KING_ATTACKS[square]
}

pub const fn get_bishop_attack(square: usize, blockers: BitBoard) -> BitBoard {
    let data = blockers.data & BISHOP_MBB_MASK[square].data;
    let m = magic_index(BISHOP_MAGICS[square], BitBoard { data }, BISHOP_OCC_BITCOUNT[square]);
    return BISHOP_ATTACKS_MBB[square][m];
}

pub const fn get_rook_attack(square: usize, blockers: BitBoard) -> BitBoard {
    let data = blockers.data & ROOK_MBB_MASK[square].data;
    let m = magic_index(ROOK_MAGICS[square], BitBoard { data }, ROOK_OCC_BITCOUNT[square]);
    return ROOK_ATTACKS_MBB[square][m];
}

pub const fn get_queen_attack(square: usize, blockers: BitBoard) -> BitBoard {
    BitBoard { data: get_bishop_attack(square, blockers).data | get_rook_attack(square, blockers).data }
}
