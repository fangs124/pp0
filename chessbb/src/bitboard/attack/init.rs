use crate::Side;
use crate::bitboard::Bitboard;

pub(super) const fn init_pawn_attack(side: Side) -> [Bitboard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [Bitboard; 64] = [Bitboard::ZERO; 64];
    while i < 64usize {
        let mut data: u64 = 0u64;
        match side {
            Side::White => {
                if i < 56 {
                    if i % 8 > 0 {
                        data |= (1u64 << i) << 7
                    }
                    if i % 8 < 7 {
                        data |= (1u64 << i) << 9
                    }
                }
            }
            Side::Black => {
                if i > 7 {
                    if i % 8 > 0 {
                        data |= (1u64 << i) >> 9
                    }
                    if i % 8 < 7 {
                        data |= (1u64 << i) >> 7
                    }
                }
            }
        }
        attack_array[i] = Bitboard(data);
        i += 1;
    }
    return attack_array;
}

pub(super) const fn init_knight_attack() -> [Bitboard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [Bitboard; 64] = [Bitboard::ZERO; 64];
    while i < 64usize {
        let mut data: u64 = 0u64;
        if i < 48 {
            if i % 8 < 7 {
                //up left is "<< 17"
                data |= (1u64 << i) << 17
            }
            if i % 8 > 0 {
                //up right is "<< 15"
                data |= (1u64 << i) << 15
            }
        }
        if i < 56 {
            if i % 8 < 6 {
                //left up is "<< 10"
                data |= (1u64 << i) << 10
            }
            if i % 8 > 1 {
                //right up is "<<  6"
                data |= (1u64 << i) << 6
            }
        }
        if i > 7 {
            if i % 8 < 6 {
                //left down is ">> 6"
                data |= (1u64 << i) >> 6
            }
            if i % 8 > 1 {
                //right down is ">> 10"
                data |= (1u64 << i) >> 10
            }
        }
        if i > 15 {
            if i % 8 < 7 {
                //down left is ">> 15"
                data |= (1u64 << i) >> 15
            }
            if i % 8 > 0 {
                //down right is ">> 17"
                data |= (1u64 << i) >> 17
            }
        }
        attack_array[i] = Bitboard(data);
        i += 1;
    }
    return attack_array;
}

pub(super) const fn init_king_attack() -> [Bitboard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [Bitboard; 64] = [Bitboard::ZERO; 64];
    while i < 64usize {
        let mut data: u64 = 0u64;
        if i < 56 {
            //up
            data |= (1u64 << i) << 8;
        }
        if i > 7 {
            //down
            data |= (1u64 << i) >> 8;
        }
        if i % 8 > 0 {
            //right
            data |= (1u64 << i) >> 1;
        }
        if i % 8 < 7 {
            //left
            data |= (1u64 << i) << 1;
        }
        if i < 56 && i % 8 > 0 {
            //up right
            data |= ((1u64 << i) << 8) >> 1;
        }
        if i < 56 && i % 8 < 7 {
            //up left
            data |= ((1u64 << i) << 8) << 1;
        }
        if i > 7 && i % 8 > 0 {
            //down right
            data |= ((1u64 << i) >> 8) >> 1;
        }
        if i > 7 && i % 8 < 7 {
            //down left
            data |= ((1u64 << i) >> 8) << 1;
        }
        attack_array[i] = Bitboard(data);
        i += 1;
    }
    return attack_array;
}

pub(super) const fn init_bishop_attack() -> [Bitboard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [Bitboard; 64] = [Bitboard::ZERO; 64];
    while i < 64 {
        attack_array[i] = naive_bishop_attack(i, Bitboard::ZERO);
        i += 1;
    }
    return attack_array;
}

pub(super) const fn init_rook_attack() -> [Bitboard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [Bitboard; 64] = [Bitboard::ZERO; 64];
    while i < 64 {
        attack_array[i] = naive_rook_attack(i, Bitboard::ZERO);
        i += 1;
    }
    return attack_array;
}

pub(super) const fn naive_bishop_attack(i: usize, blockers: Bitboard) -> Bitboard {
    let i_rank: isize = (i as isize) / 8isize;
    let i_file: isize = (i as isize) % 8isize;
    let mut j: isize = 1;
    let mut data: u64 = 0u64;
    let mut ul_is_blocked: bool = false;
    let mut dl_is_blocked: bool = false;
    let mut ur_is_blocked: bool = false;
    let mut dr_is_blocked: bool = false;
    while j <= 7 {
        //    up left direction: (+,+)
        if i_rank + j <= 7 && i_file + j <= 7 {
            if !ul_is_blocked {
                data |= 1u64 << ((i_rank + j) * 8 + (i_file + j));
                if i_rank + j < 7 && i_file + j < 7 {
                    if 1u64 << ((i_rank + j) * 8 + (i_file + j)) & blockers.0 != 0 {
                        ul_is_blocked = true;
                    }
                }
            }
        }

        //  down left direction: (-,+)
        if 0 <= i_rank - j && i_file + j <= 7 {
            if !dl_is_blocked {
                data |= 1u64 << ((i_rank - j) * 8 + (i_file + j));
                if 0 < i_rank - j && i_file + j < 7 {
                    if 1u64 << ((i_rank - j) * 8 + (i_file + j)) & blockers.0 != 0 {
                        dl_is_blocked = true;
                    }
                }
            }
        }

        //    up right direction: (+,-)
        if i_rank + j <= 7 && 0 <= i_file - j {
            if !ur_is_blocked {
                data |= 1u64 << ((i_rank + j) * 8 + (i_file - j));
                if i_rank + j < 7 && 0 < i_file - j {
                    if 1u64 << ((i_rank + j) * 8 + (i_file - j)) & blockers.0 != 0 {
                        ur_is_blocked = true;
                    }
                }
            }
        }

        //  down right direction: (-,-)
        if 0 <= i_rank - j && 0 <= i_file - j {
            if !dr_is_blocked {
                data |= 1u64 << ((i_rank - j) * 8 + (i_file - j));
                if 0 < i_rank - j && 0 < i_file - j {
                    if 1u64 << ((i_rank - j) * 8 + (i_file - j)) & blockers.0 != 0 {
                        dr_is_blocked = true;
                    }
                }
            }
        }
        j += 1
    }

    Bitboard(data)
}

pub(super) const fn naive_rook_attack(i: usize, blockers: Bitboard) -> Bitboard {
    let i_rank: isize = (i as isize) / 8isize; // row
    let i_file: isize = (i as isize) % 8isize; // collumn
    let mut data: u64 = 0u64;

    let mut j: isize = 1;
    let mut r_is_blocked: bool = false;
    let mut l_is_blocked: bool = false;
    let mut u_is_blocked: bool = false;
    let mut d_is_blocked: bool = false;

    while j <= 7 {
        // right direction: (file - j, rank)
        if 0 <= i_file - j {
            if !r_is_blocked {
                data |= 1u64 << ((i_rank * 8) + (i_file - j));
                if 0 < i_file - j {
                    if 1u64 << ((i_rank * 8) + (i_file - j)) & blockers.0 != 0 {
                        r_is_blocked = true;
                    }
                }
            }
        }
        // left direction: (file + j, rank)
        if i_file + j <= 7 {
            if !l_is_blocked {
                data |= 1u64 << ((i_rank * 8) + (i_file + j));
                if i_file + j < 7 {
                    if 1u64 << ((i_rank * 8) + (i_file + j)) & blockers.0 != 0 {
                        l_is_blocked = true;
                    }
                }
            }
        }
        //   up direction: (file, rank + j)
        if i_rank + j <= 7 {
            if !u_is_blocked {
                data |= 1u64 << (((i_rank + j) * 8) + i_file);
                if i_rank + j < 7 {
                    if 1u64 << (((i_rank + j) * 8) + i_file) & blockers.0 != 0 {
                        u_is_blocked = true;
                    }
                }
            }
        }
        // down direction: (file, rank - j)
        if 0 <= i_rank - j {
            if !d_is_blocked {
                data |= 1u64 << (((i_rank - j) * 8) + i_file);
                if 0 < i_rank - j {
                    if 1u64 << (((i_rank - j) * 8) + i_file) & blockers.0 != 0 {
                        d_is_blocked = true;
                    }
                }
            }
        }
        j += 1
    }
    Bitboard(data)
}
