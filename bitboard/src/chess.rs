use crate::{init, BitBoard};

pub(crate) static W_PAWN_ATTACKS: [BitBoard; 64] = init::pawn_attack(Side::White);
pub(crate) static B_PAWN_ATTACKS: [BitBoard; 64] = init::pawn_attack(Side::Black);
pub(crate) static KNIGHT_ATTACKS: [BitBoard; 64] = init::knight_attack();
pub(crate) static KING_ATTACKS: [BitBoard; 64] = init::king_attack();

pub(crate) static BISHOP_MBB_MASK: [BitBoard; 64] = CONST_BISHOP_MBB_MASK;
pub(crate) static ROOK_MBB_MASK: [BitBoard; 64] = CONST_ROOK_MBB_MASK;
pub(crate) static BISHOP_ATTACKS_MBB: [[BitBoard; 1 << 9]; 64] = CONST_BISHOP_ATTACKS_MBB;
pub(crate) static ROOK_ATTACKS_MBB: [[BitBoard; 1 << 12]; 64] = CONST_ROOK_ATTACKS_MBB;

#[inline(always)]
pub fn get_pawn_attack(square: usize, side: Side) -> BitBoard {
    match side {
        Side::White => W_PAWN_ATTACKS[square],
        Side::Black => B_PAWN_ATTACKS[square],
    }
}

#[inline(always)]
pub fn get_knight_attack(square: usize) -> BitBoard {
    KNIGHT_ATTACKS[square]
}

#[inline(always)]
pub fn get_king_attack(square: usize) -> BitBoard {
    KING_ATTACKS[square]
}

pub fn get_bishop_attack(square: usize, blockers: BitBoard) -> BitBoard {
    let data = blockers.data & BISHOP_MBB_MASK[square].data;
    let m = magic_index(BISHOP_MAGICS[square], BitBoard { data }, BISHOP_OCC_BITCOUNT[square]);
    return BISHOP_ATTACKS_MBB[square][m];
}

pub fn get_rook_attack(square: usize, blockers: BitBoard) -> BitBoard {
    let data = blockers.data & ROOK_MBB_MASK[square].data;
    let m = magic_index(ROOK_MAGICS[square], BitBoard { data }, ROOK_OCC_BITCOUNT[square]);
    return ROOK_ATTACKS_MBB[square][m];
}

#[inline(always)]
pub fn get_queen_attack(square: usize, blockers: BitBoard) -> BitBoard {
    BitBoard { data: get_bishop_attack(square, blockers).data | get_rook_attack(square, blockers).data }
}

/* chessboard specific bitboard functions and definitions*/
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceType {
    pub const fn to_char(&self) -> char {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub const fn update(&self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }
}

/* magic bitboard related functions and definitions */

#[rustfmt::skip]
pub(crate) const BISHOP_MAGICS: [u64; 64] = [
    0x0140C80810488022, 0x0020021C01142000, 0x00308C2080200102, 0x0004040880000A09,
    0x0824042080000001, 0x00C1010840807080, 0x810C010403200000, 0x49CE404044202081,
    0x4405048410020200, 0x0000042104440080, 0x0801C12112008003, 0x0100080A43014001,
    0x0000020210010000, 0x0110020110080990, 0x0800004804042000, 0x0000002434020800,
    0x00C108E014890204, 0x0004040210440100, 0x4808001000801012, 0x0008004620801080,
    0x0481000290400A01, 0x0001000180A00921, 0x1204010900A80492, 0x0A88400024041C00,
    0x1002100088501014, 0x005045040818008C, 0x0002080081004408, 0x0208280005820002,
    0x0509010040104008, 0x8010004000241000, 0x8908108440540400, 0x0142060800404240,
    0x0231101010402410, 0x0002011140241020, 0x100A002A00101180, 0x2001010800110041,
    0x8118022401224100, 0x4420092A40020800, 0x22D000C880031400, 0x000102108002A420,
    0x4008044404102020, 0x8000842402002000, 0x000200242400080E, 0x0030004202208802,
    0x0000011214000601, 0x10C0008099011081, 0x10080104608A0C00, 0x0002285D00202700,
    0x009A182414050000, 0x020100A210223022, 0x0000002C02080102, 0x0000000020884010,
    0x0280029002022040, 0x8250102490342010, 0x0040020464048080, 0x4120040102042200,
    0x280A010401018800, 0x8010008084104200, 0x009009002484501A, 0x1A08830080420208,
    0x2000064022604100, 0x0012400420044101, 0x0040042818810C00, 0x1024211464008200,
];

#[rustfmt::skip]
pub(crate) const ROOK_MAGICS: [u64; 64] =  [
    0x818001C000802018, 0xA240100020004000, 0x0100081041002000, 0x1080048010000800,
    0x8600020020040810, 0x0580018002004400, 0x1080020000800100, 0x020000204A088401,
    0x4000800080204000, 0x0040804000200080, 0x0000801000200080, 0x0222000C10204200,
    0x0042000600081020, 0x00A2001004080200, 0x1000800100800200, 0x0082000092010044,
    0x0800848000400420, 0x0030044040002001, 0x8000110041002004, 0x00004200200A0010,
    0x0810808004000800, 0xC028808002000400, 0x0280040090080201, 0x0804020000508104,
    0x0080400480088024, 0x0400200440100241, 0x0401001100200040, 0x0000100080800800,
    0x0008010100041008, 0x8000020080800400, 0x1000012400024830, 0x0004008200210054,
    0x08084A0082002100, 0x4080201000404000, 0xC000102001004100, 0x0004082101001002,
    0x0009820800800400, 0x900C800400800200, 0x9040080204008150, 0x80B0140446000493,
    0x6040244000828000, 0x0210002000504000, 0x0015002002110040, 0x0041001000210008,
    0x0001004800050010, 0x0002000804010100, 0x5008081002040081, 0x00220040A1020004,
    0x0101400120800180, 0x2040002000C08180, 0x1120001000480040, 0x18001020400A0200,
    0x0004050010080100, 0x1023020080040080, 0x0001080102100400, 0x0001000282004300,
    0x0190401100800021, 0x0805854001021021, 0x600010400C200101, 0x0010210009100005,
    0x1001001002080005, 0x9801000C00080A29, 0x2006080A45029014, 0x0008804581022C02,
];

pub(crate) const BISHOP_OCC_BITCOUNT: [usize; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6, //
    5, 5, 5, 5, 5, 5, 5, 5, //
    5, 5, 7, 7, 7, 7, 5, 5, //
    5, 5, 7, 9, 9, 7, 5, 5, //
    5, 5, 7, 9, 9, 7, 5, 5, //
    5, 5, 7, 7, 7, 7, 5, 5, //
    5, 5, 5, 5, 5, 5, 5, 5, //
    6, 5, 5, 5, 5, 5, 5, 6, //
];

pub(crate) const ROOK_OCC_BITCOUNT: [usize; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, //
    11, 10, 10, 10, 10, 10, 10, 11, //
    11, 10, 10, 10, 10, 10, 10, 11, //
    11, 10, 10, 10, 10, 10, 10, 11, //
    11, 10, 10, 10, 10, 10, 10, 11, //
    11, 10, 10, 10, 10, 10, 10, 11, //
    11, 10, 10, 10, 10, 10, 10, 11, //
    12, 11, 11, 11, 11, 11, 11, 12, //
];

pub(crate) const CONST_BISHOP_MBB_MASK: [BitBoard; 64] = init::bishop_mbb_mask();
pub(crate) const CONST_ROOK_MBB_MASK: [BitBoard; 64] = init::rook_mbb_mask();
pub(crate) const CONST_BISHOP_ATTACKS_MBB: [[BitBoard; 1 << 9]; 64] = init::bishop_attack_mbb();
pub(crate) const CONST_ROOK_ATTACKS_MBB: [[BitBoard; 1 << 12]; 64] = init::rook_attack_mbb();

pub(crate) const fn magic_index(magic_num: u64, blockers: BitBoard, bitcount: usize) -> usize {
    ((blockers.data.wrapping_mul(magic_num)) >> (64 - bitcount)) as usize
}

pub(crate) const fn const_get_bishop_attack(square: usize, blockers: BitBoard) -> BitBoard {
    let mask = CONST_BISHOP_MBB_MASK[square];
    let data = blockers.data & mask.data;
    let blockers = BitBoard { data };
    let m = magic_index(BISHOP_MAGICS[square], blockers, BISHOP_OCC_BITCOUNT[square]);
    return CONST_BISHOP_ATTACKS_MBB[square][m];
}

pub(crate) const fn const_get_rook_attack(square: usize, blockers: BitBoard) -> BitBoard {
    let mask = CONST_ROOK_MBB_MASK[square];
    let data = blockers.data & mask.data;
    let blockers = BitBoard { data };
    let m = magic_index(ROOK_MAGICS[square], blockers, ROOK_OCC_BITCOUNT[square]);
    return CONST_ROOK_ATTACKS_MBB[square][m];
}

//pub(crate) const fn get_queen_attack(square: usize, blockers: BitBoard) -> BitBoard {
//    BitBoard { data: get_bishop_attack(square, blockers).data | get_rook_attack(square, blockers).data }
//}
//
//pub(crate) const CONST_W_PAWN_ATTACKS: [BitBoard; 64] = init::pawn_attack(Side::White);
//pub(crate) const CONST_B_PAWN_ATTACKS: [BitBoard; 64] = init::pawn_attack(Side::Black);
//pub(crate) const CONST_KNIGHT_ATTACKS: [BitBoard; 64] = init::knight_attack();
//pub(crate) const CONST_KING_ATTACKS: [BitBoard; 64] = init::king_attack();
