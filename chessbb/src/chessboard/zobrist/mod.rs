//use bytemuck::NoUninit;

//use crate::bitboard::*;
//use crate::chessmove::Castling;
use crate::{
    Bitboard, ChessPiece, Side,
    chessboard::{ChessBoard, Mailbox, cp_index},
    chessmove::Castling,
    square::Square,
};

pub mod bit_ops;

include!("data/data.rs");

//#[derive(Debug, Copy, Clone, PartialEq, Eq, NoUninit)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct ZobristHash(u64);

const DEFAULT_SIZE: usize = 1 << 10;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ZobristTable {
    data: [ZobristHash; DEFAULT_SIZE],
    index: usize,
}

impl ZobristTable {
    pub(super) const fn new(hash: ZobristHash) -> ZobristTable {
        let mut data: [ZobristHash; DEFAULT_SIZE] = [ZobristHash(0); DEFAULT_SIZE];
        data[0] = hash;
        return ZobristTable { data, index: 0 };
    }

    #[inline(always)]
    pub(super) const fn initial_table() -> ZobristTable {
        ZobristTable::new(ZobristHash::initial_hash())
    }

    #[inline(always)]
    pub(super) const fn push(&mut self, hash: ZobristHash) {
        debug_assert!(self.index < (DEFAULT_SIZE));
        self.index += 1;
        self.data[self.index] = hash;
    }

    #[inline(always)]
    pub const fn remove_last(&mut self, hash: ZobristHash) {
        debug_assert!(self.data[self.index].0 == hash.0);
        self.index -= 1;
    }

    #[inline(always)]
    pub const unsafe fn remove_last_unchecked(&mut self) {
        self.index -= 1;
    }
}

impl ZobristHash {
    pub(super) const ZERO: ZobristHash = ZobristHash(0);

    #[inline(always)]
    pub(super) const fn to_usize(&self) -> usize {
        self.0 as usize
    }

    #[inline(always)]
    pub(super) const fn to_u64(&self) -> u64 {
        self.0
    }

    #[inline(always)]
    pub(super) const fn new(value: u64) -> ZobristHash {
        ZobristHash(value)
    }

    pub(super) const fn initial_hash() -> ZobristHash {
        let mut value: u64 = 0;

        //starting side is white, no hash
        //no en-passant in starting position either

        //piece hash
        let mut i: usize = 0;
        while i < 64 {
            if let Some(piece_data) = Mailbox::START_MAILBOX.square_index(Square::nth(i)) {
                value ^= PIECE_HASH[i][piece_data.to_index()];
            }
            i += 1;
        }

        //castle hash
        i = 0;
        while i < 4 {
            value ^= CASTLE_HASH[i];
            i += 1;
        }

        return ZobristHash(value);
    }

    pub(super) const fn compute_hash(side: Side, mb: &Mailbox, castle: [bool; 4], enpassant: Bitboard) -> ZobristHash {
        //side hash
        let mut value = match side {
            Side::White => 0u64,
            Side::Black => SIDE_HASH[0],
        };

        //piece hash
        let mut i: usize = 0;
        while i < 64 {
            if let Some(piece_data) = mb.square_index(Square::nth(i)) {
                value ^= PIECE_HASH[i][cp_index(piece_data)];
            }
            i += 1;
        }

        //castle hash
        i = 0;
        while i < 4 {
            if castle[i] {
                value ^= CASTLE_HASH[i]
            }
            i += 1;
        }

        //en-passant hash
        let mut enpassant_bb = enpassant;
        while enpassant_bb.is_not_zero() {
            let square = enpassant_bb.lsb_square().unwrap();
            value ^= ENPASSANT_FILE_HASH[square.to_col_usize()];
            enpassant_bb.pop_bit(square);
        }

        return ZobristHash(value);
    }

    pub(super) const fn compute_castle_hash(chessboard: &ChessBoard) -> ZobristHash {
        let mut value = 0u64;

        let mut i: usize = 0;
        while i < 4 {
            if chessboard.data.castle_bools[i] {
                value ^= CASTLE_HASH[i]
            }
            i += 1;
        }
        return ZobristHash(value);
    }

    #[inline(always)]
    pub(crate) const fn castle_hash(castling: Castling) -> ZobristHash {
        match castling {
            Castling::Kingside(Side::White) => ZobristHash(CASTLE_HASH[0]),
            Castling::Queenside(Side::White) => ZobristHash(CASTLE_HASH[1]),
            Castling::Kingside(Side::Black) => ZobristHash(CASTLE_HASH[2]),
            Castling::Queenside(Side::Black) => ZobristHash(CASTLE_HASH[3]),
        }
    }

    #[inline(always)]
    pub(crate) const fn piece_hash(square: Square, chesspiece: ChessPiece) -> ZobristHash {
        ZobristHash(PIECE_HASH[square.to_usize()][cp_index(chesspiece)])
    }

    pub(super) const fn enpassant_hash(enpassant_bb: Bitboard) -> ZobristHash {
        //this function assumes there is only at most one non-zero bit in enpassant_bb
        debug_assert!(enpassant_bb.count_ones() == 1);
        return ZobristHash(match enpassant_bb.lsb_square() {
            Some(square) => ENPASSANT_COL_HASH[square.to_col_usize()],
            None => 0,
        });
    }

    #[inline(always)]
    pub(crate) const fn side_hash() -> ZobristHash {
        ZobristHash(SIDE_HASH[0])
    }
}
