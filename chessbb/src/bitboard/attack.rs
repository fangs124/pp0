mod init;
mod magic;

use std::ops::Index;

use crate::Side;
use crate::bitboard::Bitboard;
use crate::bitboard::attack::init::*;
use crate::bitboard::attack::magic::*;
use crate::square::Square;

impl Index<Square> for [Bitboard; 64] {
    type Output = Bitboard;

    #[inline(always)]
    fn index(&self, index: Square) -> &Self::Output {
        &self[index.to_usize()]
    }
}

const W_PAWN_ATTACKS: [Bitboard; 64] = init_pawn_attack(Side::White);
const B_PAWN_ATTACKS: [Bitboard; 64] = init_pawn_attack(Side::Black);
const KNIGHT_ATTACKS: [Bitboard; 64] = init_knight_attack();
const KING_ATTACKS: [Bitboard; 64] = init_king_attack();
const BISHOP_ATTACKS: [Bitboard; 64] = init_bishop_attack();
const ROOK_ATTACKS: [Bitboard; 64] = init_rook_attack();

#[inline(always)]
pub(crate) const fn get_pawn_attack(side: Side, square: Square) -> Bitboard {
    match side {
        Side::White => W_PAWN_ATTACKS[square.to_usize()],
        Side::Black => B_PAWN_ATTACKS[square.to_usize()],
    }
}

#[inline(always)]
pub(crate) fn get_pawn_quiet(side: Side, square: Square, blockers: &Bitboard) -> Bitboard {
    let mut quiet_moves = match side {
        Side::White => Bitboard::nth(square.up()).bit_and(&blockers.bit_not()),
        Side::Black => Bitboard::nth(square.down()).bit_and(&blockers.bit_not()),
    };

    if quiet_moves.is_not_zero() && (square.to_row_usize() == STARTING_ROWS[side as usize]) {
        quiet_moves = match side {
            Side::White => quiet_moves.bit_or(&Bitboard::nth(square.upup()).bit_and(&blockers.bit_not())),
            Side::Black => quiet_moves.bit_or(&Bitboard::nth(square.downdown()).bit_and(&blockers.bit_not())),
        };

        quiet_moves = quiet_moves.bit_and(&blockers.bit_not());
    }
    return quiet_moves;
}

const PROMOTION_ROWS: [usize; 2] = [7, 0];
#[inline(always)]
pub(crate) const fn promotion_row(side: Side) -> usize {
    PROMOTION_ROWS[side as usize]
}

const STARTING_ROWS: [usize; 2] = [1, 6];
#[inline(always)]
pub(crate) const fn starting_row(side: Side) -> usize {
    STARTING_ROWS[side as usize]
}

#[inline(always)]
pub(crate) const fn get_w_pawn_attack(square: Square) -> Bitboard {
    W_PAWN_ATTACKS[square.to_usize()]
}

#[inline(always)]
pub(crate) const fn get_b_pawn_attack(square: Square) -> Bitboard {
    B_PAWN_ATTACKS[square.to_usize()]
}

#[inline(always)]
pub(crate) const fn get_knight_attack(square: Square) -> Bitboard {
    KNIGHT_ATTACKS[square.to_usize()]
}

#[inline(always)]
pub(crate) const fn get_king_attack(square: Square) -> Bitboard {
    KING_ATTACKS[square.to_usize()]
}

#[inline(always)]
pub const fn get_bishop_ray(square: Square) -> Bitboard {
    BISHOP_ATTACKS[square.to_usize()]
}

#[inline(always)]
pub const fn get_rook_ray(square: Square) -> Bitboard {
    ROOK_ATTACKS[square.to_usize()]
}

#[inline(always)]
pub(crate) const fn get_bishop_attack(square: Square, blockers: Bitboard) -> Bitboard {
    let m = magic_index(BISHOP_MAGICS[square.to_usize()], blockers.0 & BISHOP_MBB_MASK[square.to_usize()].0, BISHOP_OCC_BITCOUNT[square.to_usize()]);
    return BISHOP_ATTACKS_MBB[square.to_usize()][m];
}

#[inline(always)]
pub(crate) const fn get_rook_attack(square: Square, blockers: Bitboard) -> Bitboard {
    let m = magic_index(ROOK_MAGICS[square.to_usize()], blockers.0 & ROOK_MBB_MASK[square.to_usize()].0, ROOK_OCC_BITCOUNT[square.to_usize()]);
    return ROOK_ATTACKS_MBB[square.to_usize()][m];
}

#[inline(always)]
pub(crate) const fn get_queen_attack(square: Square, blockers: Bitboard) -> Bitboard {
    Bitboard(get_bishop_attack(square, blockers).0 | get_rook_attack(square, blockers).0)
}

pub(crate) const fn rays(i: Square, j: Square) -> Bitboard {
    RAYS[i as usize][j as usize]
}

pub(crate) const fn long_rays(i: Square, j: Square) -> Bitboard {
    LONG_RAYS[i as usize][j as usize]
}
const RAYS: [[Bitboard; 64]; 64] = init_rays();
const LONG_RAYS: [[Bitboard; 64]; 64] = init_long_rays();

const fn init_rays() -> [[Bitboard; 64]; 64] {
    let mut rays: [[Bitboard; 64]; 64] = [[Bitboard::ZERO; 64]; 64];
    let mut i: usize = 0;
    while i < 64 {
        let i_square = Square::nth(i);
        let mut j: usize = 0;
        while j < 64 {
            let j_square = Square::nth(j);
            let squares = Bitboard((1u64 << i) | (1u64 << j));
            if i / 8 == j / 8 || i % 8 == j % 8 {
                rays[i][j].0 = get_rook_attack(i_square, squares).0 & get_rook_attack(j_square, squares).0;
            } else if is_same_ddiagonal(i, j) || is_same_adiagonal(i, j) {
                rays[i][j].0 = get_bishop_attack(i_square, squares).0 & get_bishop_attack(j_square, squares).0;
            }
            j += 1;
        }
        i += 1;
    }
    rays
}

const fn init_long_rays() -> [[Bitboard; 64]; 64] {
    let mut rays: [[Bitboard; 64]; 64] = [[Bitboard::ZERO; 64]; 64];
    let mut i: usize = 0;
    while i < 64 {
        let i_square = Square::nth(i);
        let mut j: usize = 0;
        while j < 64 {
            let j_square = Square::nth(j);
            let squares = Bitboard((1u64 << i) | (1u64 << j));
            if i / 8 == j / 8 || i % 8 == j % 8 {
                rays[i][j].0 = (get_rook_attack(i_square, Bitboard::ZERO).0 & get_rook_attack(j_square, Bitboard::ZERO).0) | squares.0;
            } else if is_same_ddiagonal(i, j) || is_same_adiagonal(i, j) {
                rays[i][j].0 = (get_bishop_attack(i_square, Bitboard::ZERO).0 & get_bishop_attack(j_square, Bitboard::ZERO).0) | squares.0;
            }
            j += 1;
        }
        i += 1;
    }
    rays
}

#[inline]
const fn is_same_ddiagonal(i: usize, j: usize) -> bool {
    //on the same ddiagonal if file distance = rank distance
    //return ((i.abs_diff(j)) % 8) == ((i.abs_diff(j)) / 8);
    return ((i % 8).abs_diff(j % 8)) == ((i / 8).abs_diff(j / 8));
}

#[inline]
const fn is_same_adiagonal(i: usize, j: usize) -> bool {
    //on the same adiagonal if file distance = -rank distance
    return ((i % 8).abs_diff(j % 8)) + ((i / 8).abs_diff(j / 8)) == 0;
}
