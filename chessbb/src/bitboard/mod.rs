use crate::square::Square;

pub(crate) mod attack;

pub mod bit_ops;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Bitboard(u64);

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

impl std::fmt::Display for Bitboard {
    //fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //    let mut s = String::new();
    //    for i in 0..8u64 {
    //        s.push_str(&format!("{:08b}", (self.0 & (0xFFu64 << (8 * (7 - i)))) >> (8 * (7 - i))));
    //        s.push('\n');
    //    }
    //    write!(f, "{s}")
    //}

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        for i in 0..8u64 {
            s.push_str(&format!("{:08b}", (self.0 & (0xFFu64 << (8 * (7 - i)))) >> (8 * (7 - i))).chars().rev().collect::<String>());
            s.push('\n');
        }
        write!(f, "{s}")
    }
}

impl Bitboard {
    pub(crate) const ZERO: Bitboard = Bitboard(0u64);
    pub(crate) const ONES: Bitboard = Bitboard(u64::MAX);

    pub(crate) const NOT_A_FILE: Bitboard = Bitboard(0b01111111_01111111_01111111_01111111_01111111_01111111_01111111_01111111);
    pub(crate) const NOT_H_FILE: Bitboard = Bitboard(0b11111110_11111110_11111110_11111110_11111110_11111110_11111110_11111110);

    pub(crate) const ROWS: [Bitboard; 8] = [
        Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_11111111),
        Bitboard(0b00000000_00000000_00000000_00000000_00000000_00000000_11111111_00000000),
        Bitboard(0b00000000_00000000_00000000_00000000_00000000_11111111_00000000_00000000),
        Bitboard(0b00000000_00000000_00000000_00000000_11111111_00000000_00000000_00000000),
        Bitboard(0b00000000_00000000_00000000_11111111_00000000_00000000_00000000_00000000),
        Bitboard(0b00000000_00000000_11111111_00000000_00000000_00000000_00000000_00000000),
        Bitboard(0b00000000_11111111_000000000_0000000_00000000_00000000_00000000_00000000),
        Bitboard(0b11111111_00000000_00000000_00000000_00000000_00000000_00000000_00000000),
    ];

    #[inline(always)]
    pub(crate) const fn rows(nth: usize) -> Bitboard {
        Bitboard::ROWS[nth]
    }
    #[inline(always)]
    pub(crate) const fn nth(sq: Square) -> Self {
        Bitboard(1u64 << sq.to_usize())
    }

    #[inline(always)]
    pub(crate) const fn new(data: u64) -> Self {
        Bitboard(data)
    }

    #[inline(always)]
    pub(crate) const fn nth_is_zero(&self, sq: Square) -> bool {
        self.0 & (1u64 << sq.to_usize()) == 0
    }

    #[inline(always)]
    pub(crate) const fn nth_is_not_zero(&self, sq: Square) -> bool {
        self.0 & (1u64 << sq.to_usize()) != 0
    }

    #[inline(always)]
    pub(crate) const fn is_zero(&self) -> bool {
        self.0 == 0u64
    }

    #[inline(always)]
    pub(crate) const fn is_not_zero(&self) -> bool {
        self.0 != 0u64
    }

    #[inline(always)]
    pub(crate) const fn set_bit(&mut self, square: Square) {
        self.0 |= 1u64 << square.to_usize();
    }

    #[inline(always)]
    pub(crate) const fn get_bit(&self, i: usize) -> Bitboard {
        Bitboard(self.0 & (1u64 << i))
    }

    #[inline(always)]
    pub(crate) const fn pop_bit(&mut self, square: Square) {
        self.0 &= !(1u64 << square.to_usize());
    }

    #[inline(always)]
    pub(crate) const fn pop_lsb(&mut self) {
        self.0 &= self.0.wrapping_sub(1);
    }

    // index of least-significant-bit (lsb)
    #[inline(always)]
    pub(crate) const fn lsb_index(&self) -> Option<usize> {
        match self.0 {
            0u64 => return None,
            x => return Some(x.trailing_zeros() as usize),
        }
    }

    // square of least-significant-bit (lsb)
    #[inline(always)]
    pub(crate) const fn lsb_square(&self) -> Option<Square> {
        match self.0 {
            0u64 => return None,
            x => return Some(Square::nth(x.trailing_zeros() as usize)),
        }
    }

    // bitboard of least-significant-bit (lsb)
    #[inline(always)]
    pub(crate) const fn lsb_bitboard(&self) -> Bitboard {
        return Bitboard(self.0 & self.0.wrapping_neg());
    }

    pub(crate) const fn count_ones(&self) -> u32 {
        self.0.count_ones()
    }

    /* const bitwise operations */

    #[inline(always)]
    pub(crate) const fn bit_and(&self, other: &Bitboard) -> Bitboard {
        Bitboard(self.0 & other.0)
    }

    #[inline(always)]
    pub(crate) const fn bit_or(&self, other: &Bitboard) -> Bitboard {
        Bitboard(self.0 | other.0)
    }

    #[inline(always)]
    pub(crate) const fn bit_xor(&self, other: &Bitboard) -> Bitboard {
        Bitboard(self.0 ^ other.0)
    }

    #[inline(always)]
    pub(crate) const fn bit_not(&self) -> Bitboard {
        Bitboard(!self.0)
    }

    #[inline(always)]
    pub(crate) const fn flip(&self) -> Self {
        Bitboard(self.0.swap_bytes())
    }

    #[inline(always)]
    pub(crate) const fn shl(&self, rhs: u32) -> Bitboard {
        Bitboard(self.0.unbounded_shl(rhs))
    }

    #[inline(always)]
    pub(crate) const fn shr(&self, rhs: u32) -> Bitboard {
        Bitboard(self.0.unbounded_shr(rhs))
    }
}
