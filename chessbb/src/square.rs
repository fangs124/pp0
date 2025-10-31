//Rank refers to the eight horizontal rows on the board, labelled 1 to 8.
//File refers to the eight vertical columns on the board, labelled a to h.

/* indexing the 64-squares:
   -----------------------
8 |56 57 58 59 60 61 62 63|
7 |48 49 50 51 52 53 57 55|
6 |40 41 42 43 44 45 46 47|
5 |32 33 34 35 36 37 38 39|
4 |24 25 26 27 28 29 30 31|
3 |16 17 18 19 20 21 22 23|
2 | 8  9 10 11 12 13 14 15|
1 | 0  1  2  3  4  5  6  7|
   -----------------------
    A  B  C  D  E  F  G  H */

use std::fmt::Display;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Column {
    A, B, C, D, E, F, G, H
}

impl Column {
    const COLUMNS: [Column; 8] = [Column::A, Column::B, Column:: C, Column::D, Column::E, Column::F, Column::G, Column::H];

    pub const fn nth(index: usize) -> Column{
        Column::COLUMNS[index]
    }
}

#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}
/* indexing the 64-squares:
   -----------------------
8 |56 57 58 59 60 61 62 63|
7 |48 49 50 51 52 53 57 55|
6 |40 41 42 43 44 45 46 47|
5 |32 33 34 35 36 37 38 39|
4 |24 25 26 27 28 29 30 31|
3 |16 17 18 19 20 21 22 23|
2 | 8  9 10 11 12 13 14 15|
1 | 0  1  2  3  4  5  6  7|
   -----------------------
    A  B  C  D  E  F  G  H */
#[rustfmt::skip]
const FLIPPED_INDEX: [usize; 64] = [
    56, 57, 58, 59, 60, 61, 62, 63, 
    48, 49, 50, 51, 52, 53, 54, 55,
    40, 41, 42, 43, 44, 45, 46, 47,
    32, 33, 34, 35, 36, 37, 38, 39,
    24, 25, 26, 27, 28, 29, 30, 31,
    16, 17, 18, 19, 20, 21, 22, 23,
    08, 09, 10, 11, 12, 13, 14, 15,
    00, 01, 02, 03, 04, 05, 06, 07,
];

impl Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Square::SQUARE_SYM[*self as usize])
    }
}

impl Square {
    #[inline(always)]
    pub const fn as_usize(&self) -> usize {
        *self as usize
    }

    #[inline(always)]
    pub const fn as_usize_flipped(&self) -> usize {
        FLIPPED_INDEX[*self as usize]
    }

    #[inline(always)]
    pub const fn as_u8(&self) -> u8 {
        *self as u8
    }

    #[inline(always)]
    pub const fn as_col_usize(&self) -> usize {
        (*self as usize) % 8
    }

    #[inline(always)]
    pub const fn as_col(&self) -> Column {
        Column::COLUMNS[(*self as usize) % 8]
    }

    #[inline(always)]
    pub const fn as_row_usize(&self) -> usize {
        (*self as usize) / 8
    }

    #[inline(always)]
    pub(crate)const fn is_same_diag(s1: Square, s2: Square, s3: Square) -> bool {
        Square::is_same_adiag(s1, s2) && Square::is_same_adiag(s2, s3) || Square::is_same_ddiag(s1, s2) && Square::is_same_ddiag(s2, s3)
    }

    //#[inline(always)]
    //pub(crate) const fn is_same_diag(s1: Square, s2: Square) -> bool {
    //    Square::is_same_ddiag(s1, s2) || Square::is_same_adiag(s1, s2)
    //}

     #[cfg(not(feature = "diagmath"))]
    #[inline(always)]
    pub(crate)const fn is_same_ddiag(s1: Square, s2: Square) -> bool {
        Square::DDIAG[s1.as_usize()] == Square::DDIAG[s2.as_usize()]
    }

    #[cfg(not(feature = "diagmath"))]
    #[inline(always)]
    pub(crate)const fn is_same_adiag(s1: Square, s2: Square) -> bool {
        Square::ADIAG[s1.as_usize()] == Square::ADIAG[s2.as_usize()]
    }
    
    #[cfg(feature = "rowcolmath")]
    #[inline(always)]
    pub(crate)const fn is_same_row(s1: Square, s2: Square) -> bool {
        s1.as_row_usize() == s2.as_row_usize()
    }
    
    #[cfg(feature = "rowcolmath")]
    #[inline(always)]
    pub(crate) const fn is_same_col(s1: Square, s2: Square) -> bool {
        s1.as_col_usize() == s2.as_col_usize()
    }

    #[cfg(feature = "diagmath")]
    #[inline(always)]
    pub(crate) const fn is_same_ddiag(s1: Square, s2: Square) -> bool {
        (s1.to_row_usize().abs_diff(s2.to_row_usize())) == (s1.to_col_usize().abs_diff(s2.to_col_usize()))
    }
    
    #[cfg(feature = "diagmath")]
    #[inline(always)]
    pub(crate) const fn is_same_adiag(s1: Square, s2: Square) -> bool {
        (s1.to_row_usize().abs_diff(s2.to_row_usize())) + (s1.to_col_usize().abs_diff(s2.to_col_usize())) == 0
    }
    
    #[cfg(not(feature = "rowcolmath"))]
    #[inline(always)]
    pub(crate) const fn is_same_row(s1: Square, s2: Square) -> bool {
        Square::ROWS[s1.to_usize()] == Square::ROWS[s2.to_usize()]
    }
    
    #[cfg(not(feature = "rowcolmath"))]
    #[inline(always)]
    pub(crate) const fn is_same_col(s1: Square, s2: Square) -> bool {
        Square::COLS[s1.to_usize()] == Square::COLS[s2.to_usize()]
    }
    
    #[rustfmt::skip]
    const DDIAG: [u8; 64] = [
        00, 01, 02, 03, 04, 05, 06, 07,
        01, 02, 03, 04, 05, 06, 07, 08,
        02, 03, 04, 05, 06, 07, 08, 09,
        03, 04, 05, 06, 07, 08, 09, 10,
        04, 05, 06, 07, 08, 09, 10, 11,
        05, 06, 07, 08, 09, 10, 11, 12,
        06, 07, 08, 09, 10, 11, 12, 13,
        07, 08, 09, 10, 11, 12, 13, 14,
    ];

    #[rustfmt::skip]
    const ADIAG: [u8; 64] = [
        07, 06, 05, 04, 03, 02, 01, 00,
        08, 07, 06, 05, 04, 03, 02, 01,
        09, 08, 07, 06, 05, 04, 03, 02,
        10, 09, 08, 07, 06, 05, 04, 03,
        11, 10, 09, 08, 07, 06, 05, 04,
        12, 11, 10, 09, 08, 07, 06, 05,
        13, 12, 11, 10, 09, 08, 07, 06,
        14, 13, 12, 11, 10, 09, 08, 07,
    ];

    #[rustfmt::skip]
    const ROWS: [u8; 64] = [
        0, 0, 0, 0, 0, 0, 0, 0,
        1, 1, 1, 1, 1, 1, 1, 1,
        2, 2, 2, 2, 2, 2, 2, 2,
        3, 3, 3, 3, 3, 3, 3, 3,
        4, 4, 4, 4, 4, 4, 4, 4,
        5, 5, 5, 5, 5, 5, 5, 5,
        6, 6, 6, 6, 6, 6, 6, 6,
        7, 7, 7, 7, 7, 7, 7, 7,
    ];
    
    #[rustfmt::skip]
    const COLS: [u8; 64] = [
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
        0, 1, 2, 3, 4, 5, 6, 7,
    ];

    #[inline(always)]
    pub const fn left(&self) -> Square {
        Square::SQUARES[(*self as usize) - 1]
    }

    #[inline(always)]
    pub const fn right(&self) -> Square {
        Square::SQUARES[(*self as usize) + 1]
    }

    #[inline(always)]
    pub const fn up(&self) -> Square {
        Square::SQUARES[(*self as usize) + 8]
    }

    #[inline(always)]
    pub const fn down(&self) -> Square {
        Square::SQUARES[(*self as usize) - 8]
    }

    #[inline(always)]
    pub const fn upup(&self) -> Square {
        Square::SQUARES[(*self as usize) + 16]
    }

    #[inline(always)]
    pub const fn downdown(&self) -> Square {
        Square::SQUARES[(*self as usize) - 16]
    }

    #[inline(always)]
    pub const fn nth(n: usize) -> Square {
        Square::SQUARES[n]
    }

    pub(crate) const W_KING_SQUARE: Square = Square::E1;
    pub(crate) const W_KINGSIDE_CASTLE_SQUARE: Square = Square::G1;
    pub(crate) const W_QUEENSIDE_CASTLE_SQUARE: Square = Square::C1;
    pub(crate) const B_KING_SQUARE: Square = Square::E8;
    pub(crate) const B_KINGSIDE_CASTLE_SQUARE: Square = Square::G8;
    pub(crate) const B_QUEENSIDE_CASTLE_SQUARE: Square = Square::C8;
    pub(crate) const W_KINGSIDE_ROOK_SQ_SOURCE: Square = Square::H1;
    pub(crate) const W_KINGSIDE_ROOK_SQ_TARGET: Square = Square::F1;
    pub(crate) const W_QUEENSIDE_ROOK_SQ_SOURCE: Square = Square::A1;
    pub(crate) const W_QUEENSIDE_ROOK_SQ_TARGET: Square = Square::D1;
    pub(crate) const B_KINGSIDE_ROOK_SQ_SOURCE: Square = Square::H8;
    pub(crate) const B_KINGSIDE_ROOK_SQ_TARGET: Square = Square::F8;
    pub(crate) const B_QUEENSIDE_ROOK_SQ_SOURCE: Square = Square::A8;
    pub(crate) const B_QUEENSIDE_ROOK_SQ_TARGET: Square = Square::D8;
    
    #[rustfmt::skip]
    const SQUARES: [Square; 64] = [
        Square::A1, Square::B1, Square::C1, Square::D1, Square::E1, Square::F1, Square::G1, Square::H1,
        Square::A2, Square::B2, Square::C2, Square::D2, Square::E2, Square::F2, Square::G2, Square::H2,
        Square::A3, Square::B3, Square::C3, Square::D3, Square::E3, Square::F3, Square::G3, Square::H3,
        Square::A4, Square::B4, Square::C4, Square::D4, Square::E4, Square::F4, Square::G4, Square::H4,
        Square::A5, Square::B5, Square::C5, Square::D5, Square::E5, Square::F5, Square::G5, Square::H5,
        Square::A6, Square::B6, Square::C6, Square::D6, Square::E6, Square::F6, Square::G6, Square::H6,
        Square::A7, Square::B7, Square::C7, Square::D7, Square::E7, Square::F7, Square::G7, Square::H7,
        Square::A8, Square::B8, Square::C8, Square::D8, Square::E8, Square::F8, Square::G8, Square::H8,
    ];

    

    pub fn iter() -> std::slice::Iter<'static, Square> {
        Square::SQUARES.iter()
    }

    #[rustfmt::skip]
    pub fn parse_str(token: &str) -> Square {
        assert!(token.len() == 2, "parse_str error: incorrect token length");
        return match token.to_ascii_lowercase().as_str() {
            "a1" => Square::A1, "a2" => Square::A2, "a3" => Square::A3, "a4" => Square::A4,
            "a5" => Square::A5, "a6" => Square::A6, "a7" => Square::A7, "a8" => Square::A8,
            "b1" => Square::B1, "b2" => Square::B2, "b3" => Square::B3, "b4" => Square::B4,
            "b5" => Square::B5, "b6" => Square::B6, "b7" => Square::B7, "b8" => Square::B8,
            "c1" => Square::C1, "c2" => Square::C2, "c3" => Square::C3, "c4" => Square::C4,
            "c5" => Square::C5, "c6" => Square::C6, "c7" => Square::C7, "c8" => Square::C8,
            "d1" => Square::D1, "d2" => Square::D2, "d3" => Square::D3, "d4" => Square::D4,
            "d5" => Square::D5, "d6" => Square::D6, "d7" => Square::D7, "d8" => Square::D8,
            "e1" => Square::E1, "e2" => Square::E2, "e3" => Square::E3, "e4" => Square::E4,
            "e5" => Square::E5, "e6" => Square::E6, "e7" => Square::E7, "e8" => Square::E8,
            "f1" => Square::F1, "f2" => Square::F2, "f3" => Square::F3, "f4" => Square::F4, 
            "f5" => Square::F5, "f6" => Square::F6, "f7" => Square::F7, "f8" => Square::F8,
            "g1" => Square::G1, "g2" => Square::G2, "g3" => Square::G3, "g4" => Square::G4,
            "g5" => Square::G5, "g6" => Square::G6, "g7" => Square::G7, "g8" => Square::G8,
            "h1" => Square::H1, "h2" => Square::H2, "h3" => Square::H3, "h4" => Square::H4,
            "h5" => Square::H5, "h6" => Square::H6, "h7" => Square::H7, "h8" => Square::H8,
            _ => panic!("parse_str error: invalid square token"),
        };
    }

    const SQUARE_SYM: [&str; 64] = [
        "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1", //
        "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2", //
        "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3", //
        "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4", //
        "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5", //
        "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6", //
        "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7", //
        "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8", //
    ];
    
}
