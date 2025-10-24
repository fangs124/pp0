use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use super::ZobristHash;

impl BitAnd for ZobristHash {
    type Output = ZobristHash;

    #[inline(always)]
    fn bitand(self, rhs: ZobristHash) -> Self::Output {
        ZobristHash(self.0 & rhs.0)
    }
}

impl BitAndAssign for ZobristHash {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for ZobristHash {
    type Output = ZobristHash;

    #[inline(always)]
    fn bitor(self, rhs: ZobristHash) -> Self::Output {
        ZobristHash(self.0 | rhs.0)
    }
}

impl BitOrAssign for ZobristHash {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for ZobristHash {
    type Output = ZobristHash;

    #[inline(always)]
    fn bitxor(self, rhs: ZobristHash) -> Self::Output {
        ZobristHash(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for ZobristHash {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for ZobristHash {
    type Output = ZobristHash;

    #[inline(always)]
    fn not(self) -> Self::Output {
        ZobristHash(!self.0)
    }
}
