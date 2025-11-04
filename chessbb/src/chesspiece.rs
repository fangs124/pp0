use std::slice;

use crate::{
    Bitboard,
    bitboard::attack::{get_bishop_attack, get_queen_attack, get_rook_attack},
    square::Square,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    White = 0,
    Black = 1,
}
#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PieceType {
    Pawn   = 0,
    Knight = 1,
    Bishop = 2,
    Rook   = 3,
    Queen  = 4,
    King   = 5,
}

#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SliderType { //this seems like an ugly hack
    Bishop = 2,
    Rook   = 3,
    Queen  = 4,
}

impl From<SliderType> for PieceType {
    fn from(value: SliderType) -> PieceType {
        PieceType::PIECES[value as usize]
    }
}

impl SliderType {
    pub(crate) fn get_attack(&self, square: Square, blockers: Bitboard) -> Bitboard {
        match self {
            SliderType::Bishop => get_bishop_attack(square, blockers),
            SliderType::Rook => get_rook_attack(square, blockers),
            SliderType::Queen => get_queen_attack(square, blockers),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChessPiece(pub Side, pub PieceType);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FenError {
    InvalidPieceChar,
}

impl Side {
    pub const fn update(&self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
}

impl TryFrom<char> for ChessPiece {
    type Error = FenError;

    fn try_from(c: char) -> Result<ChessPiece, Self::Error> {
        match c {
            'K' => Ok(ChessPiece(Side::White, PieceType::King)),
            'Q' => Ok(ChessPiece(Side::White, PieceType::Queen)),
            'N' => Ok(ChessPiece(Side::White, PieceType::Knight)),
            'B' => Ok(ChessPiece(Side::White, PieceType::Bishop)),
            'R' => Ok(ChessPiece(Side::White, PieceType::Rook)),
            'P' => Ok(ChessPiece(Side::White, PieceType::Pawn)),
            'k' => Ok(ChessPiece(Side::Black, PieceType::King)),
            'q' => Ok(ChessPiece(Side::Black, PieceType::Queen)),
            'n' => Ok(ChessPiece(Side::Black, PieceType::Knight)),
            'b' => Ok(ChessPiece(Side::Black, PieceType::Bishop)),
            'r' => Ok(ChessPiece(Side::Black, PieceType::Rook)),
            'p' => Ok(ChessPiece(Side::Black, PieceType::Pawn)),
            _ => Err(FenError::InvalidPieceChar),
        }
    }
}

impl PieceType {
    const PIECES: [PieceType; 6] = [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen, PieceType::King];

    pub fn iter() -> slice::Iter<'static, PieceType> {
        PieceType::PIECES.iter()
    }

    pub(crate) const fn to_uci_char(&self) -> char {
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

impl ChessPiece {
    #[inline(always)]
    pub const fn new(side: Side, piece_type: PieceType) -> ChessPiece {
        ChessPiece(side, piece_type)
    }

    #[inline(always)]
    pub const fn data(&self) -> (Side, PieceType) {
        (self.0, self.1)
    }

    #[inline(always)]
    pub const fn to_index(&self) -> usize {
        (self.0 as usize) * 6 + (self.1 as usize)
    }

    #[rustfmt::skip]
    #[inline(always)]
    pub const fn to_ascii(&self) -> char {
        match self {
            ChessPiece(Side::White, PieceType::King  ) => 'K',
            ChessPiece(Side::White, PieceType::Queen ) => 'Q',
            ChessPiece(Side::White, PieceType::Knight) => 'N',
            ChessPiece(Side::White, PieceType::Bishop) => 'B',
            ChessPiece(Side::White, PieceType::Rook  ) => 'R',
            ChessPiece(Side::White, PieceType::Pawn  ) => 'P',
            ChessPiece(Side::Black, PieceType::King  ) => 'k',
            ChessPiece(Side::Black, PieceType::Queen ) => 'q',
            ChessPiece(Side::Black, PieceType::Knight) => 'n',
            ChessPiece(Side::Black, PieceType::Bishop) => 'b',
            ChessPiece(Side::Black, PieceType::Rook  ) => 'r',
            ChessPiece(Side::Black, PieceType::Pawn  ) => 'p',
        }
    }

    const WHITE_PIECES: [ChessPiece; 6] = [
        ChessPiece(Side::White, PieceType::King),
        ChessPiece(Side::White, PieceType::Queen),
        ChessPiece(Side::White, PieceType::Knight),
        ChessPiece(Side::White, PieceType::Bishop),
        ChessPiece(Side::White, PieceType::Rook),
        ChessPiece(Side::White, PieceType::Pawn),
    ];

    const BLACK_PIECES: [ChessPiece; 6] = [
        ChessPiece(Side::Black, PieceType::King),
        ChessPiece(Side::Black, PieceType::Queen),
        ChessPiece(Side::Black, PieceType::Knight),
        ChessPiece(Side::Black, PieceType::Bishop),
        ChessPiece(Side::Black, PieceType::Rook),
        ChessPiece(Side::Black, PieceType::Pawn),
    ];

    pub(crate) const PIECES: [ChessPiece; 12] = [
        ChessPiece(Side::White, PieceType::Pawn),
        ChessPiece(Side::White, PieceType::Knight),
        ChessPiece(Side::White, PieceType::Bishop),
        ChessPiece(Side::White, PieceType::Rook),
        ChessPiece(Side::White, PieceType::Queen),
        ChessPiece(Side::White, PieceType::King),
        ChessPiece(Side::Black, PieceType::Pawn),
        ChessPiece(Side::Black, PieceType::Knight),
        ChessPiece(Side::Black, PieceType::Bishop),
        ChessPiece(Side::Black, PieceType::Rook),
        ChessPiece(Side::Black, PieceType::Queen),
        ChessPiece(Side::Black, PieceType::King),
    ];

    pub fn white_iter() -> slice::Iter<'static, ChessPiece> {
        ChessPiece::WHITE_PIECES.iter()
    }

    pub fn black_iter() -> slice::Iter<'static, ChessPiece> {
        ChessPiece::BLACK_PIECES.iter()
    }

    pub fn iter() -> slice::Iter<'static, ChessPiece> {
        ChessPiece::PIECES.iter()
    }

    pub const WK: ChessPiece = ChessPiece(Side::White, PieceType::King);
    pub const WQ: ChessPiece = ChessPiece(Side::White, PieceType::Queen);
    pub const WN: ChessPiece = ChessPiece(Side::White, PieceType::Knight);
    pub const WB: ChessPiece = ChessPiece(Side::White, PieceType::Bishop);
    pub const WR: ChessPiece = ChessPiece(Side::White, PieceType::Rook);
    pub const WP: ChessPiece = ChessPiece(Side::White, PieceType::Pawn);
    pub const BK: ChessPiece = ChessPiece(Side::Black, PieceType::King);
    pub const BQ: ChessPiece = ChessPiece(Side::Black, PieceType::Queen);
    pub const BN: ChessPiece = ChessPiece(Side::Black, PieceType::Knight);
    pub const BB: ChessPiece = ChessPiece(Side::Black, PieceType::Bishop);
    pub const BR: ChessPiece = ChessPiece(Side::Black, PieceType::Rook);
    pub const BP: ChessPiece = ChessPiece(Side::Black, PieceType::Pawn);
}
