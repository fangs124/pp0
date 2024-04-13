use crate::chess::*;
use crate::*;

pub(crate) const fn rays() -> [[BitBoard; 64]; 64] {
    let mut rays: [[BitBoard; 64]; 64] = [[BitBoard::ZERO; 64]; 64];
    let mut i: usize = 0;
    while i < 64 {
        let mut j: usize = 0;
        while j < 64 {
            let data = (1u64 << i) | (1u64 << j);
            let squares = BitBoard { data };
            if (ROWS[i] == ROWS[j]) || (COLS[i] == COLS[j]) {
                let data: u64 = const_get_rook_attack(i, squares).data & const_get_rook_attack(j, squares).data;
                rays[i][j].data = data;
            } else if (DDIAG[i] == DDIAG[j]) || (ADIAG[i] == ADIAG[j]) {
                let data = const_get_bishop_attack(i, squares).data & const_get_bishop_attack(j, squares).data;
                rays[i][j].data = data;
            }
            j += 1;
        }
        i += 1;
    }
    rays
}

pub(crate) const fn pawn_attack(side: Side) -> [BitBoard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [BitBoard; 64] = [BitBoard::ZERO; 64];
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
        attack_array[i] = BitBoard { data };
        i += 1;
    }
    return attack_array;
}

pub(crate) const fn knight_attack() -> [BitBoard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [BitBoard; 64] = [BitBoard::ZERO; 64];
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
        attack_array[i] = BitBoard { data };
        i += 1;
    }
    return attack_array;
}

pub(crate) const fn king_attack() -> [BitBoard; 64] {
    let mut i: usize = 0;
    let mut attack_array: [BitBoard; 64] = [BitBoard::ZERO; 64];
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
        attack_array[i] = BitBoard { data };
        i += 1;
    }
    return attack_array;
}

pub(crate) const fn naive_bishop_attack(i: usize, blockers: BitBoard) -> BitBoard {
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
                data |= 1u64 << (i_rank + j) * 8 + (i_file + j);
                if i_rank + j < 7 && i_file + j < 7 {
                    if 1u64 << (i_rank + j) * 8 + (i_file + j) & blockers.data != BitBoard::ZERO.data {
                        ul_is_blocked = true;
                    }
                }
            }
        }

        //  down left direction: (-,+)
        if 0 <= i_rank - j && i_file + j <= 7 {
            if !dl_is_blocked {
                data |= 1u64 << (i_rank - j) * 8 + (i_file + j);
                if 0 < i_rank - j && i_file + j < 7 {
                    if 1u64 << (i_rank - j) * 8 + (i_file + j) & blockers.data != BitBoard::ZERO.data {
                        dl_is_blocked = true;
                    }
                }
            }
        }

        //    up right direction: (+,-)
        if i_rank + j <= 7 && 0 <= i_file - j {
            if !ur_is_blocked {
                data |= 1u64 << (i_rank + j) * 8 + (i_file - j);
                if i_rank + j < 7 && 0 < i_file - j {
                    if 1u64 << (i_rank + j) * 8 + (i_file - j) & blockers.data != BitBoard::ZERO.data {
                        ur_is_blocked = true;
                    }
                }
            }
        }

        //  down right direction: (-,-)
        if 0 <= i_rank - j && 0 <= i_file - j {
            if !dr_is_blocked {
                data |= 1u64 << (i_rank - j) * 8 + (i_file - j);
                if 0 < i_rank - j && 0 < i_file - j {
                    if 1u64 << (i_rank - j) * 8 + (i_file - j) & blockers.data != BitBoard::ZERO.data {
                        dr_is_blocked = true;
                    }
                }
            }
        }
        j += 1
    }

    BitBoard { data }
}

pub(crate) const fn naive_rook_attack(i: usize, blockers: BitBoard) -> BitBoard {
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
                data |= 1u64 << (i_rank * 8) + (i_file - j);
                if 0 < i_file - j {
                    if 1u64 << (i_rank * 8) + (i_file - j) & blockers.data != BitBoard::ZERO.data {
                        r_is_blocked = true;
                    }
                }
            }
        }
        // left direction: (file + j, rank)
        if i_file + j <= 7 {
            if !l_is_blocked {
                data |= 1u64 << (i_rank * 8) + (i_file + j);
                if i_file + j < 7 {
                    if 1u64 << (i_rank * 8) + (i_file + j) & blockers.data != BitBoard::ZERO.data {
                        l_is_blocked = true;
                    }
                }
            }
        }
        //   up direction: (file, rank + j)
        if i_rank + j <= 7 {
            if !u_is_blocked {
                data |= 1u64 << ((i_rank + j) * 8) + i_file;
                if i_rank + j < 7 {
                    if 1u64 << ((i_rank + j) * 8) + i_file & blockers.data != BitBoard::ZERO.data {
                        u_is_blocked = true;
                    }
                }
            }
        }
        // down direction: (file, rank - j)
        if 0 <= i_rank - j {
            if !d_is_blocked {
                data |= 1u64 << ((i_rank - j) * 8) + i_file;
                if 0 < i_rank - j {
                    if 1u64 << ((i_rank - j) * 8) + i_file & blockers.data != BitBoard::ZERO.data {
                        d_is_blocked = true;
                    }
                }
            }
        }
        j += 1
    }
    BitBoard { data }
}

// each bitboard flags relevant squares to a bishop in any given location on the chessboard
pub(crate) const fn bishop_mbb_mask() -> [BitBoard; 64] {
    let mut attack_array: [BitBoard; 64] = [BitBoard::ZERO; 64];

    let mut i: usize = 0;
    while i < 64usize {
        let i_rank: isize = (i as isize) / 8isize;
        let i_file: isize = (i as isize) % 8isize;
        let mut j: isize = 1;
        let mut data: u64 = 0u64;
        while j < 7 {
            //    up left direction: (+,+)
            if i_rank + j < 7 && i_file + j < 7 {
                data |= 1u64 << (i_rank + j) * 8 + (i_file + j);
            }
            //  down left direction: (-,+)
            if 0 < i_rank - j && i_file + j < 7 {
                data |= 1u64 << (i_rank - j) * 8 + (i_file + j);
            }
            //    up right direction: (+,-)
            if i_rank + j < 7 && 0 < i_file - j {
                data |= 1u64 << (i_rank + j) * 8 + (i_file - j);
            }
            //    up right direction: (-,-)
            if 0 < i_rank - j && 0 < i_file - j {
                data |= 1u64 << (i_rank - j) * 8 + (i_file - j);
            }
            j += 1
        }
        attack_array[i] = BitBoard { data };
        i += 1;
    }
    return attack_array;
}

// each bitboard flags relevant squares to a rook in any given location on the chessboard
pub(crate) const fn rook_mbb_mask() -> [BitBoard; 64] {
    let mut attack_array: [BitBoard; 64] = [BitBoard::ZERO; 64];

    let mut i: usize = 0;
    while i < 64usize {
        let i_rank: isize = (i as isize) / 8isize; // row
        let i_file: isize = (i as isize) % 8isize; // collumn
        let mut j: isize = 1;
        let mut data: u64 = 0u64;
        while j < 7 {
            // right direction: (file - j, rank)
            if 0 < i_file - j {
                data |= 1u64 << (i_rank * 8) + (i_file - j);
            }
            // left direction: (file + j, rank)
            if i_file + j < 7 {
                data |= 1u64 << (i_rank * 8) + (i_file + j);
            }
            //   up direction: (file, rank + j)
            if i_rank + j < 7 {
                data |= 1u64 << ((i_rank + j) * 8) + i_file;
            }
            // down direction: (file, rank - j)
            if 0 < i_rank - j {
                data |= 1u64 << ((i_rank - j) * 8) + i_file;
            }
            j += 1
        }
        attack_array[i] = BitBoard { data };
        i += 1;
    }
    return attack_array;
}

pub(crate) const fn compute_occ_bb(index: usize, mask_bitcount: usize, attack_mask: BitBoard) -> BitBoard {
    /* use pdep? */
    let mut attack_mask: BitBoard = attack_mask;
    let mut occupancy_bb: BitBoard = BitBoard::ZERO;
    let mut i: usize = 0;
    // while attack_mask is non-zero
    while i < mask_bitcount && attack_mask.data != 0 {
        // square_index is index of least_significant bit
        if let Some(square_index) = attack_mask.lsb_index() {
            attack_mask = attack_mask.pop_bit(square_index);
            // check that square is within range of index
            if index & (1 << i) != 0usize {
                occupancy_bb.data |= 1u64 << square_index
            }
        }
        i += 1;
    }
    return occupancy_bb;
}

const SIZE_BISHOP: usize = 1 << 9; //size of the index for bishop magic bitboard index in bits
const SIZE_ROOK: usize = 1 << 12; //size of the index for rook magic bitboard index in bits

pub const fn bishop_attack_mbb() -> [[BitBoard; SIZE_BISHOP]; 64] {
    let mut i: usize = 0;
    let mut attacks: [[BitBoard; 1 << 9]; 64] = [[BitBoard::ZERO; 1 << 9]; 64];
    let bishop_mbb_mask = CONST_BISHOP_MBB_MASK;
    let bishop_occ_bitcount = BISHOP_OCC_BITCOUNT;
    while i < 64 {
        let mask = bishop_mbb_mask[i];
        let bitcount = bishop_occ_bitcount[i];
        let max_index: usize = 1 << bitcount;

        let mut j: usize = 0;
        while j < max_index {
            let blockers = compute_occ_bb(j, bitcount, mask);
            let m = magic_index(BISHOP_MAGICS[i], blockers, bitcount);

            if attacks[i][m].data == BitBoard::ZERO.data {
                attacks[i][m] = naive_bishop_attack(i, blockers);
            } else if attacks[i][m].data != naive_bishop_attack(i, blockers).data {
                panic!("bishop_attack_mbb error: invalid colision!");
            }
            j += 1;
        }
        i += 1;
    }
    return attacks;
}

pub const fn rook_attack_mbb() -> [[BitBoard; SIZE_ROOK]; 64] {
    let mut i: usize = 0;
    let mut attacks: [[BitBoard; 1 << 12]; 64] = [[BitBoard::ZERO; 1 << 12]; 64];
    let rook_mbb_mask = CONST_ROOK_MBB_MASK;
    let rook_occ_bitcount = ROOK_OCC_BITCOUNT;
    while i < 64 {
        let mask = rook_mbb_mask[i];
        let bitcount = rook_occ_bitcount[i];
        let max_index: usize = 1 << bitcount;

        let mut j: usize = 0;
        while j < max_index {
            let blockers = compute_occ_bb(j, bitcount, mask);
            let m = magic_index(ROOK_MAGICS[i], blockers, bitcount);

            if attacks[i][m].data == BitBoard::ZERO.data {
                attacks[i][m] = naive_rook_attack(i, blockers);
            } else if attacks[i][m].data != naive_rook_attack(i, blockers).data {
                panic!("rook_attack_mbb error: invalid colision!");
            }
            j += 1;
        }
        i += 1;
    }
    return attacks;
}

#[cfg(feature = "rand")]
use rand::Rng;

#[cfg(feature = "rand")]
pub fn init_magics(piece_type: PieceType) -> [u64; 64] {
    let mut i: usize = 0;
    let mut magic_nums: [u64; 64] = [0u64; 64];
    match piece_type {
        PieceType::Bishop => {
            println!("Finding magic numbers for: Bishop")
        }
        PieceType::Rook => {
            println!("Finding magic numbers for: Rook")
        }
        _ => panic!("error: invalid piece_type parameter"),
    }

    while i < 64 {
        println!("calculating nth magic number: {}", i);
        let mut magic_found = false;
        let mut magic: u64 = 0;
        while !magic_found {
            magic = match piece_type {
                PieceType::Bishop => find_magic_number(i, crate::chess::BISHOP_OCC_BITCOUNT[i], piece_type),
                PieceType::Rook => find_magic_number(i, crate::chess::ROOK_OCC_BITCOUNT[i], piece_type),
                _ => panic!("init_magic_numbers error: invalid PieceType variable"),
            };
            magic_found = true;
            for x in magic_nums {
                if x == magic {
                    magic_found = false;
                    break;
                };
            }
        }
        magic_nums[i] = magic;
        i += 1;
    }
    return magic_nums;
}

#[cfg(feature = "rand")]
pub fn find_magic_number(square: usize, mask_bitcount: usize, piece_type: PieceType) -> u64 {
    let max_index: usize = 1 << mask_bitcount;
    let mut rng = rand::thread_rng();
    let mut blockers: Vec<BitBoard> = vec![BitBoard::ZERO; max_index];
    let mut attacks: Vec<BitBoard> = vec![BitBoard::ZERO; max_index];
    //let mut attack_history: Vec<BB> = vec![BitBoard::ZERO; max_index];
    let mask = match piece_type {
        PieceType::Bishop => crate::chess::BISHOP_MBB_MASK[square],
        PieceType::Rook => crate::chess::ROOK_MBB_MASK[square],
        _ => panic!("find_magic_number error: invalid piece type!"),
    };

    let mut i: usize = 0;
    // precalculate table
    while i < max_index {
        blockers[i] = compute_occ_bb(i, mask_bitcount, mask);
        attacks[i] = match piece_type {
            PieceType::Bishop => naive_bishop_attack(square, blockers[i]),
            PieceType::Rook => naive_rook_attack(square, blockers[i]),
            _ => panic!("find_magic_number error: invalid piece type!"),
        };
        i += 1;
    }

    let mut _attempts: usize = 0;
    // bruteforce magic number
    while _attempts < usize::MAX {
        let magic_num: u64 = rng.gen::<u64>() & rng.gen::<u64>() & rng.gen::<u64>();

        // skip bad magic_num
        if (mask.data.wrapping_mul(magic_num) & 0xFF00000000000000u64).count_ones() < 6 {
            continue;
        }

        let mut attack_history = vec![BitBoard::ZERO; max_index];
        let mut i: usize = 0;
        let mut is_failed = false;

        while !is_failed && i < max_index {
            let m = magic_index(magic_num, blockers[i], mask_bitcount);

            if attack_history[m] == BitBoard::ZERO {
                attack_history[m] = attacks[i];
            } else {
                is_failed = attack_history[m] != attacks[i];
            }
            i += 1
        }
        if !is_failed {
            return magic_num;
        }
        _attempts += 1;
    }
    panic!("find_magic_number error: failed to find magic!");
}
