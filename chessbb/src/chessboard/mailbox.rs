use std::{fmt::Debug, ops::Index};

use crate::{ChessPiece, PieceType, Side, square::Square};

#[rustfmt::skip]
macro_rules! cpt {
    (K) => {Some(ChessPiece(Side::White, PieceType::King  ))};
    (Q) => {Some(ChessPiece(Side::White, PieceType::Queen ))};
    (N) => {Some(ChessPiece(Side::White, PieceType::Knight))};
    (B) => {Some(ChessPiece(Side::White, PieceType::Bishop))};
    (R) => {Some(ChessPiece(Side::White, PieceType::Rook  ))};
    (P) => {Some(ChessPiece(Side::White, PieceType::Pawn  ))};
    (k) => {Some(ChessPiece(Side::Black, PieceType::King  ))};
    (q) => {Some(ChessPiece(Side::Black, PieceType::Queen ))};
    (n) => {Some(ChessPiece(Side::Black, PieceType::Knight))};
    (b) => {Some(ChessPiece(Side::Black, PieceType::Bishop))};
    (r) => {Some(ChessPiece(Side::Black, PieceType::Rook  ))};
    (p) => {Some(ChessPiece(Side::Black, PieceType::Pawn  ))};
    (_) => {None};
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Mailbox([Option<ChessPiece>; 64]);

#[rustfmt::skip]
impl Debug for Mailbox {
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
impl Index<Square> for Mailbox {
    type Output = Option<ChessPiece>;

    fn index(&self, index: Square) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl Mailbox {
    pub const fn square_index(&self, index: Square) -> Option<ChessPiece> {
        self.0[index as usize]
    }

    pub(crate) const fn set(&mut self, piece: Option<ChessPiece>, square: Square) {
        self.0[square.to_usize()] = piece;
    }
    pub(crate) const EMPTY_MAILBOX: Mailbox = Mailbox([None; 64]);

    #[rustfmt::skip]
    pub(crate) const START_MAILBOX: Mailbox = Mailbox([
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
