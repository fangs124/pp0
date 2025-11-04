use std::fmt::Debug;
use std::hint::unreachable_unchecked;
use std::num::NonZero;

use crate::PieceType;
use crate::Side;
use crate::bitboard::*;
use crate::chessboard::MoveList;
use crate::square::Square;

/*
   indexing the 64-squares:
  |-----------------------| BLACK KING SIDE
8 |63 62 61 60 59 58 57 56|
7 |55 54 53 52 51 50 49 48|
6 |47 46 45 44 43 42 41 40|
5 |39 38 37 36 35 34 33 32|
4 |31 30 29 28 27 26 25 24| //30
3 |23 22 21 20 19 18 17 16| //20
2 |15 14 13 12 11 10  9  8|
1 | 7  6  5  4  3  2  1  0|
  |-----------------------| WHITE KING SIDE
    A  B  C  D  E  F  G  H                  */

/*  binary masks           description         hexidecimal masks
0000 0000 00XX XXXX    source square       0x3f
0000 XXXX XX00 0000    target square       0xfc0
00XX 0000 0000 0000    promoted piece data 0x3000
                       castling type
XX00 0000 0000 0000    move type           0xc000

note: move types are encoded as follows
00 - normal move
01 - castle move
10 - en passant
11 - promotion

note: promoted piece data are encoded as follows
00 - knight
01 - bishop
10 - rook
11 - queen

note: castling move are encoded as follows
00 - White Kingside
01 - White Queenside
02 - Black Kingside
03 - Black Queenside
*/
//note: a fully unpacked ChessMove would look like (from: Square, to: Square, move_type: MoveType)

//API traits: Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display, Default
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ChessMove {
    data: NonZero<u16>,
}

pub trait LexiOrd {
    fn lexi_cmp(&self, other: &Self) -> std::cmp::Ordering;
}
//needed to sort chess moves
impl LexiOrd for ChessMove {
    fn lexi_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.print_move().cmp(&other.print_move())
    }
}

//impl Display for ChessMove {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        let s = self.print_move();
//        write!(f, "{}", s)
//    }
//}

//impl Debug for ChessMove {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        let mut s = self.print_move();
//        s.push_str(format!(" {:?}", self.move_type()).as_str());
//        write!(f, "{}", s)
//    }
//}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MoveType {
    Normal,
    Castle(Castling),
    EnPassant,
    Promotion(PieceType),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Castling {
    Kingside(Side),
    Queenside(Side),
}

impl ChessMove {
    #[inline(always)]
    pub const fn source(&self) -> Square {
        Square::nth((self.data.get() & 0b000000_111111u16) as usize)
    }

    #[inline(always)]
    pub const fn target(&self) -> Square {
        Square::nth(((self.data.get() & 0b111111_000000u16) >> 6) as usize)
    }

    const PROMOTION_PIECE: [PieceType; 4] = [
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
    ];

    const CASTLING: [Castling; 4] = [
        Castling::Kingside(Side::White),
        Castling::Queenside(Side::White),
        Castling::Kingside(Side::Black),
        Castling::Queenside(Side::Black),
    ];

    pub const fn move_type(&self) -> MoveType {
        let (piece, castling): (PieceType, Castling) =
            match ((self.data.get() & 0b11_000000_000000u16) as usize) >> 12 {
                0b00 => (PieceType::Knight, Castling::Kingside(Side::White)),
                0b01 => (PieceType::Bishop, Castling::Queenside(Side::White)),
                0b10 => (PieceType::Rook, Castling::Kingside(Side::Black)),
                0b11 => (PieceType::Queen, Castling::Queenside(Side::Black)),
                _ => unreachable!(),
                //_ => unsafe { unreachable_unchecked() },
            };

        match ((self.data.get() & 0b11_00_000000_000000) as usize) >> 14 {
            0 => MoveType::Normal,
            1 => MoveType::Castle(castling),
            2 => MoveType::EnPassant,
            3 => MoveType::Promotion(piece),
            _ => unreachable!(),
            //_ => unsafe { unreachable_unchecked() },
        }
    }
    //pub(crate) const fn move_type(&self) -> MoveType {
    //    let index = ((self.data.get() & 0b11_000000_000000u16) as usize) >> 12;
    //
    //    match ((self.data.get() & 0b11_00_000000_000000) as usize) >> 14 {
    //        0 => MoveType::Normal,
    //        1 => MoveType::Castle(ChessMove::CASTLING[index]),
    //        2 => MoveType::EnPassant,
    //        3 => MoveType::Promotion(ChessMove::PROMOTION_PIECE[index]),
    //        _ => unreachable!(),
    //    }
    //}

    #[inline(always)]
    pub(crate) const fn set_source(&mut self, index: usize) {
        self.data = NonZero::new(self.data.get() & (index & 0b111111) as u16)
            .expect("a legal move can not have zero bit-pattern.");
    }

    #[inline(always)]
    pub(crate) const fn set_target(&mut self, index: usize) {
        self.data = NonZero::new(self.data.get() & ((index << 6) & 0b111111_000000) as u16)
            .expect("a legal move can not have zero bit-pattern.");
    }

    #[inline(always)]
    pub const fn from_raw(data: u16) -> ChessMove {
        ChessMove {
            data: NonZero::new(data).expect("a legal move can not have zero bit-pattern."),
        }
    }

    #[inline(always)]
    pub const fn data(&self) -> u16 {
        self.data.get()
    }

    //TODO: make this const when able
    pub fn add_normal_moves(s: Square, ts: Bitboard, moves: &mut MoveList) {
        let mut ts = ts;
        let base_data: u16 = (s.as_usize() & 0b111111) as u16;
        while ts.is_not_zero() {
            let t = ts.lsb_square().unwrap();
            let data: u16 = base_data | ((t.as_usize() << 6) & 0b111111_000000) as u16;
            moves.push(ChessMove {
                data: NonZero::new(data).expect("a legal move can not have zero bit-pattern."),
            });
            ts.pop_lsb();
        }
    }

    //TODO: make this const when able
    pub fn add_pawn_moves(s: Square, ts: Bitboard, moves: &mut MoveList) {
        let mut ts_normal = ts & Bitboard::NOT_PROMOTION_SQUARES;
        let mut ts_promotion = ts & Bitboard::PROMOTION_SQUARES;
        let base_data: u16 = (s.as_usize() & 0b111111) as u16;

        while ts_normal.is_not_zero() {
            let t = ts_normal.lsb_square().unwrap();
            let data: u16 = base_data | ((t.as_usize() << 6) & 0b111111_000000) as u16;
            moves.push(ChessMove {
                data: NonZero::new(data).expect("a legal move can not have zero bit-pattern."),
            });
            ts_normal.pop_lsb();
        }

        while ts_promotion.is_not_zero() {
            let t = ts_promotion.lsb_square().unwrap();
            #[cfg(feature = "arrayvec")]
            //safe because: https://lichess.org/@/Tobs40/blog/why-a-position-cant-have-more-than-218-moves/a5xdxeqs
            unsafe {
                moves
                    .try_extend_from_slice(&ChessMove::promotions(s, t))
                    .unwrap_unchecked()
            };
            #[cfg(not(any(feature = "arrayvec")))]
            moves.extend_from_slice(&ChessMove::promotions(source, target));
            ts_promotion.pop_lsb();
        }
    }

    pub const fn new(s: Square, t: Square, m: MoveType) -> Self {
        // can't promote to king/pawn
        // ps: !matches!(...) is ugly
        debug_assert!(matches!(m, MoveType::Promotion(PieceType::King)) == false);
        debug_assert!(matches!(m, MoveType::Promotion(PieceType::Pawn)) == false);
        let mut data: u16 =
            ((s.as_usize() & 0b111111) | ((t.as_usize() << 6) & 0b111111_000000)) as u16;

        let move_type_data: usize = match m {
            MoveType::Normal => 0b00_00,
            MoveType::Castle(Castling::Kingside(Side::White)) => 0b01_00,
            MoveType::Castle(Castling::Queenside(Side::White)) => 0b01_01,
            MoveType::Castle(Castling::Kingside(Side::Black)) => 0b01_10,
            MoveType::Castle(Castling::Queenside(Side::Black)) => 0b01_11,
            MoveType::EnPassant => 0b10_00,
            MoveType::Promotion(PieceType::Knight) => 0b11_00,
            MoveType::Promotion(PieceType::Bishop) => 0b11_01,
            MoveType::Promotion(PieceType::Rook) => 0b11_10,
            MoveType::Promotion(PieceType::Queen) => 0b11_11,
            MoveType::Promotion(_) => unreachable!(),
        };

        data |= ((move_type_data << 12) & 0b11_11_000000_000000) as u16;
        ChessMove {
            data: NonZero::new(data).expect("a legal move can not have zero bit-pattern."),
        }
    }

    pub(crate) const fn promotions(source: Square, target: Square) -> [ChessMove; 4] {
        [
            ChessMove::new(source, target, MoveType::Promotion(PieceType::Queen)),
            ChessMove::new(source, target, MoveType::Promotion(PieceType::Knight)),
            ChessMove::new(source, target, MoveType::Promotion(PieceType::Bishop)),
            ChessMove::new(source, target, MoveType::Promotion(PieceType::Rook)),
        ]
    }

    #[inline(always)]
    pub(crate) const fn kingside_castle(side: Side) -> ChessMove {
        ChessMove::KINGSIDE_CASTLE_MOVES[side as usize]
    }

    #[inline(always)]
    pub(crate) const fn queenside_castle(side: Side) -> ChessMove {
        ChessMove::QUEENSIDE_CASTLE_MOVES[side as usize]
    }
    const KINGSIDE_CASTLE_MOVES: [ChessMove; 2] =
        [ChessMove::W_KINGSIDE_CASTLE, ChessMove::B_KINGSIDE_CASTLE];
    const QUEENSIDE_CASTLE_MOVES: [ChessMove; 2] =
        [ChessMove::W_QUEENSIDE_CASTLE, ChessMove::B_QUEENSIDE_CASTLE];
    pub(crate) const W_KINGSIDE_CASTLE: ChessMove = ChessMove::new(
        Square::W_KING_SQUARE,
        Square::W_KINGSIDE_CASTLE_SQUARE,
        MoveType::Castle(Castling::Kingside(Side::White)),
    );

    pub(crate) const W_QUEENSIDE_CASTLE: ChessMove = ChessMove::new(
        Square::W_KING_SQUARE,
        Square::W_QUEENSIDE_CASTLE_SQUARE,
        MoveType::Castle(Castling::Queenside(Side::White)),
    );

    pub(crate) const B_KINGSIDE_CASTLE: ChessMove = ChessMove::new(
        Square::B_KING_SQUARE,
        Square::B_KINGSIDE_CASTLE_SQUARE,
        MoveType::Castle(Castling::Kingside(Side::Black)),
    );

    pub(crate) const B_QUEENSIDE_CASTLE: ChessMove = ChessMove::new(
        Square::B_KING_SQUARE,
        Square::B_QUEENSIDE_CASTLE_SQUARE,
        MoveType::Castle(Castling::Queenside(Side::Black)),
    );

    pub fn print_move(&self) -> String {
        if let MoveType::Promotion(piece) = self.move_type() {
            format!("{}{}{}", self.source(), self.target(), piece.to_uci_char())
        } else {
            format!("{}{}", self.source(), self.target())
        }
    }
}
