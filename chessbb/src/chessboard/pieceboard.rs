use std::{fmt::Debug, ops::Index};

use crate::{
    Bitboard, ChessPiece, Side,
    chessboard::{COLOUR_LABELS, PIECE_LABELS},
    square::Square,
};

//Pawn, Knight, Bishop Rook, Queen, King
//White, Black
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PieceColourBoard {
    pub(crate) piece: [Bitboard; 6],
    pub(crate) colour: [Bitboard; 2],
}

impl Debug for PieceColourBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut i: usize = 0;
        while i < 6 {
            write!(f, "(piece[{}]: {})\n", i, PIECE_LABELS[i])?;
            write!(f, "{}\n", self.piece[i])?;
            i += 1;
        }
        i = 0;
        while i < 2 {
            write!(f, "(colour[{}]: {})\n", i, COLOUR_LABELS[i])?;
            write!(f, "{}\n", self.colour[i])?;
            i += 1;
        }
        Ok(())
    }
}

impl PieceColourBoard {
    pub(crate) const EMPTY_BOARD: PieceColourBoard = PieceColourBoard { colour: [Bitboard::ZERO; 2], piece: [Bitboard::ZERO; 6] };
    pub(crate) const START_BOARD: PieceColourBoard = PieceColourBoard {
        piece: [
            Bitboard::new(0b00000000_11111111_00000000_00000000_00000000_00000000_11111111_00000000), // ♟♙
            Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01000010), // ♞♘
            Bitboard::new(0b00100100_00000000_00000000_00000000_00000000_00000000_00000000_00100100), // ♝♗
            Bitboard::new(0b10000001_00000000_00000000_00000000_00000000_00000000_00000000_10000001), // ♜♖
            Bitboard::new(0b00010000_00000000_00000000_00000000_00000000_00000000_00000000_00010000), // ♛♕
            Bitboard::new(0b00001000_00000000_00000000_00000000_00000000_00000000_00000000_00001000), // ♚♔
        ],

        colour: [
            Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_11111111_11111111), // White
            Bitboard::new(0b11111111_11111111_00000000_00000000_00000000_00000000_00000000_00000000), // Black
        ],
    };

    pub const fn piece_bitboard(&self, chess_piece: ChessPiece) -> Bitboard {
        self.colour[chess_piece.0 as usize].bit_and(&self.piece[chess_piece.1 as usize])
    }

    pub(crate) const fn white_blockers(&self) -> Bitboard {
        self.colour[0]
    }

    pub(crate) const fn black_blockers(&self) -> Bitboard {
        self.colour[1]
    }

    pub(crate) const fn colour_blockers(&self, side: Side) -> Bitboard {
        self.colour[side as usize]
    }

    pub(crate) const fn blockers(&self) -> Bitboard {
        self.colour[0].bit_or(&self.colour[1])
    }

    pub(crate) const fn set_bit(&mut self, piece: ChessPiece, square: Square) {
        self.colour[piece.0 as usize].set_bit(square);
        self.piece[piece.1 as usize].set_bit(square);
    }

    pub(crate) const fn pop_bit(&mut self, piece: ChessPiece, square: Square) {
        self.colour[piece.0 as usize].pop_bit(square);
        self.piece[piece.1 as usize].pop_bit(square);
    }
}

// pieces: white pawn, white knight, white bishop, white rook, white queen, white king,
//         black pawn, black knight, black bishop, black rook, black queen, black king,
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PieceBoard([Bitboard; 12]);
impl Index<ChessPiece> for PieceBoard {
    type Output = Bitboard;

    fn index(&self, index: ChessPiece) -> &Self::Output {
        &self.0[((index.0 as usize) * 6) + (index.1 as usize)]
    }
}

impl PieceBoard {
    pub(crate) const EMPTY_BOARD: PieceBoard = PieceBoard([Bitboard::ZERO; 12]);

    pub(crate) const START_BOARD: PieceBoard = PieceBoard([
        Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_11111111_00000000), // ♟
        Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_01000010), // ♞
        Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00100100), // ♝
        Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_10000001), // ♜
        Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00010000), // ♛
        Bitboard::new(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00001000), // ♚
        Bitboard::new(0b00000000_11111111_00000000_00000000_00000000_00000000_00000000_00000000), // ♙
        Bitboard::new(0b01000010_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♘
        Bitboard::new(0b00100100_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♗
        Bitboard::new(0b10000001_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♖
        Bitboard::new(0b00010000_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♕
        Bitboard::new(0b00001000_00000000_00000000_00000000_00000000_00000000_00000000_00000000), // ♔
    ]);

    pub const fn piece_bitboard(&self, index: ChessPiece) -> Bitboard {
        self.0[((index.0 as usize) * 6) + (index.1 as usize)]
    }

    pub(crate) const fn colour_blockers(&self, side: Side) -> Bitboard {
        match side {
            Side::White => self.white_blockers(),
            Side::Black => self.black_blockers(),
        }
    }
    pub(crate) const fn white_blockers(&self) -> Bitboard {
        self.0[0].bit_or(&self.0[1]).bit_or(&self.0[2]).bit_or(&self.0[3]).bit_or(&self.0[4]).bit_or(&self.0[5])
    }

    pub(crate) const fn black_blockers(&self) -> Bitboard {
        self.0[6].bit_or(&self.0[7]).bit_or(&self.0[8]).bit_or(&self.0[9]).bit_or(&self.0[10]).bit_or(&self.0[11])
    }

    pub(crate) const fn blockers(&self) -> Bitboard {
        self.white_blockers().bit_or(&self.black_blockers())
    }

    pub(crate) const fn set_bit(&mut self, piece: ChessPiece, square: Square) {
        self.0[piece.to_index()].set_bit(square);
    }

    pub(crate) const fn pop_bit(&mut self, piece: ChessPiece, square: Square) {
        self.0[piece.to_index()].pop_bit(square);
    }
}
