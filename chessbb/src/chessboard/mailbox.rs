use std::{fmt::Debug, ops::Index};

use crate::{ChessPiece, PieceType, Side, square::Square};

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
macro_rules! maybe_cpt {
    (P) => {MaybeChessPiece(0b1111_0001)};
    (N) => {MaybeChessPiece(0b1111_0010)};
    (B) => {MaybeChessPiece(0b1111_0011)};
    (R) => {MaybeChessPiece(0b1111_0100)};
    (Q) => {MaybeChessPiece(0b1111_0101)};
    (K) => {MaybeChessPiece(0b1111_0110)};
    (p) => {MaybeChessPiece(0b1111_0111)};
    (n) => {MaybeChessPiece(0b1111_1000)};
    (b) => {MaybeChessPiece(0b1111_1001)};
    (r) => {MaybeChessPiece(0b1111_1010)};
    (q) => {MaybeChessPiece(0b1111_1011)};
    (k) => {MaybeChessPiece(0b1111_1100)};
    (_) => {MaybeChessPiece(0b0000_0000)};
}

#[cfg(feature = "maybemailbox")]
pub(crate) type Mailbox = MaybeMailbox;

#[cfg(not(feature = "maybemailbox"))]
pub(crate) type Mailbox = OptionMailbox;

/*  binary masks           description         hexidecimal masks
0000 XXXX                  chess piece         0x15
XXXX 0000                  option              0xf0

note: ChessPiece are encoded as follows
0000  - White Pawn
0001  - White Knight
0010  - White Bishop
0011  - White Rook
0100  - White Queen
0101  - White King
1000  - Black Pawn
1001  - Black Knight
1010  - Black Bishop
1011  - Black Rook
1100  - Black Queen
1101  - Black King

note: Option are encoded as follows
0000 - None
1111 - Some
//                                                           */

#[derive(Copy, Clone, Eq, PartialEq)]
struct MaybeChessPiece(u8);

//impl PartialEq for MaybeChessPiece {
//    fn eq(&self, other: &Self) -> bool {
//        self.0 == other.0
//    }
//}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct MaybeMailbox([MaybeChessPiece; 64]);

impl From<Option<ChessPiece>> for MaybeChessPiece {
    fn from(value: Option<ChessPiece>) -> MaybeChessPiece {
        let data: u8 = match value {
            Some(ChessPiece(side, piece_type)) => 0b1111_0000 | ((side as u8) * 6) + (piece_type as u8),
            None => 0b0000_0000,
        };
        MaybeChessPiece(data)
    }
}

impl From<MaybeChessPiece> for Option<ChessPiece> {
    fn from(value: MaybeChessPiece) -> Option<ChessPiece> {
        if 0b1111_0000 & value.0 == 0b0000_0000 {
            return None;
        }
        let piece_index: usize = (value.0 & 0b1111) as usize;
        return Some(ChessPiece::PIECES[piece_index]);
    }
}

impl MaybeChessPiece {
    const NONE: MaybeChessPiece = MaybeChessPiece(0b0000_0000);

    const fn convert(value: MaybeChessPiece) -> Option<ChessPiece> {
        if 0b1111_0000 & value.0 == 0b0000_0000 {
            return None;
        }
        let piece_index: usize = (value.0 & 0b1111) as usize;
        return Some(ChessPiece::PIECES[piece_index]);
    }
}

#[rustfmt::skip]
impl Debug for MaybeMailbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut rows: Vec<String> = Vec::new();
        let mut r = String::new();
        let mut i: usize = 0;
        while i < 64 {
            let symbol = match self.0[i].0 {
                0b1111_0000 => "[P]",
                0b1111_0001 => "[N]",
                0b1111_0010 => "[B]", 
                0b1111_0011 => "[R]", 
                0b1111_0100 => "[Q]", 
                0b1111_0101 => "[K]", 
                0b1111_0110 => "[p]", 
                0b1111_0111 => "[n]", 
                0b1111_1000 => "[b]", 
                0b1111_1001 => "[r]", 
                0b1111_1010 => "[q]", 
                0b1111_1011 => "[k]",
                _ => "[.]",
                //0b0000_0000 => "[.]",
                //_ => panic!("Invalid MaybeChessPiece state! {:#08b}", self.0[i].0),
            };
            r.push_str(symbol);
            //write!(f, "{}", symbol)?;
            if i % 8 == 7 {
                //write!(f, "\n")?;
                r.push('\n');
                rows.push(r.clone());
                r = String::new();
            }
            i += 1;
        }
        rows.reverse();
        f.write_str(&rows.join(""))
    }
}

impl MaybeMailbox {
    pub(crate) const fn square_index(&self, s: Square) -> Option<ChessPiece> {
        MaybeChessPiece::convert(self.0[s as usize])
    }

    pub(crate) const fn index(&self, i: usize) -> Option<ChessPiece> {
        MaybeChessPiece::convert(self.0[i])
    }

    pub(crate) fn set(&mut self, piece: Option<ChessPiece>, square: Square) {
        self.0[square.to_usize()] = MaybeChessPiece::from(piece);
    }
    pub(crate) const EMPTY_MAILBOX: MaybeMailbox = MaybeMailbox([MaybeChessPiece::NONE; 64]);

    #[rustfmt::skip]
    pub(crate) const START_MAILBOX: MaybeMailbox = MaybeMailbox([
        maybe_cpt!(R), maybe_cpt!(N), maybe_cpt!(B), maybe_cpt!(Q), maybe_cpt!(K), maybe_cpt!(B), maybe_cpt!(N), maybe_cpt!(R),
        maybe_cpt!(P), maybe_cpt!(P), maybe_cpt!(P), maybe_cpt!(P), maybe_cpt!(P), maybe_cpt!(P), maybe_cpt!(P), maybe_cpt!(P),
        maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_),
        maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_),
        maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_),
        maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_), maybe_cpt!(_),
        maybe_cpt!(p), maybe_cpt!(p), maybe_cpt!(p), maybe_cpt!(p), maybe_cpt!(p), maybe_cpt!(p), maybe_cpt!(p), maybe_cpt!(p),
        maybe_cpt!(r), maybe_cpt!(n), maybe_cpt!(b), maybe_cpt!(q), maybe_cpt!(k), maybe_cpt!(b), maybe_cpt!(n), maybe_cpt!(r),
    ]);
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OptionMailbox([Option<ChessPiece>; 64]);

#[rustfmt::skip]
impl Debug for OptionMailbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut rows: Vec<String> = Vec::new();
        let mut r = String::new();
        let mut i: usize = 0;
        while i < 64 {
            let symbol = match self.0[i] {
                Some(ChessPiece(Side::White, PieceType::King  )) => "[K]",
                Some(ChessPiece(Side::White, PieceType::Queen )) => "[Q]", 
                Some(ChessPiece(Side::White, PieceType::Knight)) => "[N]", 
                Some(ChessPiece(Side::White, PieceType::Bishop)) => "[B]", 
                Some(ChessPiece(Side::White, PieceType::Rook  )) => "[R]", 
                Some(ChessPiece(Side::White, PieceType::Pawn  )) => "[P]", 
                Some(ChessPiece(Side::Black, PieceType::King  )) => "[k]", 
                Some(ChessPiece(Side::Black, PieceType::Queen )) => "[q]", 
                Some(ChessPiece(Side::Black, PieceType::Knight)) => "[n]", 
                Some(ChessPiece(Side::Black, PieceType::Bishop)) => "[b]", 
                Some(ChessPiece(Side::Black, PieceType::Rook  )) => "[r]", 
                Some(ChessPiece(Side::Black, PieceType::Pawn  )) => "[p]",
                None => "[.]",
            };
            r.push_str(symbol);
            //write!(f, "{}", symbol)?;
            if i % 8 == 7 {
                //write!(f, "\n")?;
                r.push('\n');
                rows.push(r.clone());
                r = String::new();
            }
            i += 1;
        }
        rows.reverse();
        f.write_str(&rows.join(""))
    }
}
impl Index<Square> for OptionMailbox {
    type Output = Option<ChessPiece>;

    fn index(&self, index: Square) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl OptionMailbox {
    pub const fn square_index(&self, s: Square) -> Option<ChessPiece> {
        self.0[s as usize]
    }

    pub(crate) const fn index(&self, i: usize) -> Option<ChessPiece> {
        self.0[i]
    }

    pub(crate) const fn set(&mut self, piece: Option<ChessPiece>, square: Square) {
        self.0[square.to_usize()] = piece;
    }
    pub(crate) const EMPTY_MAILBOX: OptionMailbox = OptionMailbox([None; 64]);

    #[rustfmt::skip]
    pub(crate) const START_MAILBOX: OptionMailbox = OptionMailbox([
        cpt!(R), cpt!(N), cpt!(B), cpt!(Q), cpt!(K), cpt!(B), cpt!(N), cpt!(R),
        cpt!(P), cpt!(P), cpt!(P), cpt!(P), cpt!(P), cpt!(P), cpt!(P), cpt!(P),
        cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_),
        cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_),
        cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_),
        cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_), cpt!(_),
        cpt!(p), cpt!(p), cpt!(p), cpt!(p), cpt!(p), cpt!(p), cpt!(p), cpt!(p),
        cpt!(r), cpt!(n), cpt!(b), cpt!(q), cpt!(k), cpt!(b), cpt!(n), cpt!(r),
    ]);
}
