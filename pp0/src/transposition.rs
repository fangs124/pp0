use std::{
    mem::{self, MaybeUninit},
    ops::{Index, IndexMut},
    sync::atomic::{AtomicU64, Ordering},
};

use atomic::Atomic;
use bytemuck::{NoUninit, cast};
use chessbb::{ChessMove, ZobristHash};
#[cfg(feature = "small_atomic_tt")]
pub type TranspositionTable = SmallAtomicTranspositionTable;

#[cfg(not(feature = "small_atomic_tt"))]
pub type TranspositionTable = AtomicTranspositionTable;

//16MB for stc, 128MB for ltc
//128MB is 1073741824 bits
//16MB is  0134217728 bits
//NodeData: 96 bit

#[derive(Debug, Copy, Clone, PartialEq, Eq, NoUninit)]
#[repr(C)]
pub struct PositionData {
    key: ZobristHash,
    depth: u16, //is u8 enough? u16 should be enough, its over 64 000+
    eval: i16,
    ty: NodeType,
    best: MaybeChessMove,
}

impl Default for PositionData {
    fn default() -> Self {
        return Self { key: ZobristHash::ZERO, depth: 0, eval: 0, ty: NodeType::None, best: MaybeChessMove::NONE };
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, NoUninit)]
#[repr(u16)]
pub enum NodeType {
    Exact,
    Alpha, //lower-bound
    Beta,  //upper-bound
    None,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, NoUninit)]
#[repr(transparent)]
struct MaybeChessMove(u16);

impl From<Option<ChessMove>> for MaybeChessMove {
    fn from(value: Option<ChessMove>) -> Self {
        match value {
            Some(chess_move) => MaybeChessMove(chess_move.data()), //assert! here?
            None => MaybeChessMove::NONE,
        }
    }
}

impl From<MaybeChessMove> for Option<ChessMove> {
    fn from(value: MaybeChessMove) -> Self {
        if value.0 == 0b10_11_0000_0000 {
            return None;
        } else {
            return Some(ChessMove::from_raw(value.0));
        }
    }
}

impl MaybeChessMove {
    const NONE: MaybeChessMove = MaybeChessMove(0b10_11_0000_0000);
}

impl PositionData {
    #[inline(always)]
    fn new(key: ZobristHash, depth: u16, eval: i16, ty: NodeType, best: Option<ChessMove>) -> Self {
        return Self { key, depth, eval: eval, ty, best: MaybeChessMove::from(best) };
    }

    #[inline(always)]
    const fn const_default() -> Self {
        return Self { key: ZobristHash::ZERO, depth: 0, eval: 0, ty: NodeType::None, best: MaybeChessMove::NONE };
    }

    #[inline(always)]
    pub fn is_valid(&self, hash: ZobristHash) -> bool {
        return (self.ty != NodeType::None) && (self.key == hash);
    }

    #[inline(always)]
    pub const fn key(&self) -> ZobristHash {
        return self.key;
    }

    #[inline(always)]
    pub const fn depth(&self) -> u16 {
        return self.depth;
    }

    #[inline(always)]
    pub const fn eval(&self) -> i16 {
        return self.eval;
    }

    #[inline(always)]
    pub const fn ty(&self) -> NodeType {
        return self.ty;
    }

    #[inline(always)]
    pub fn best(&self) -> Option<ChessMove> {
        return Option::<ChessMove>::from(self.best);
    }

    #[inline(always)]
    pub const fn value_type(value: i16, a: i16, b: i16) -> NodeType {
        if value <= a {
            return NodeType::Beta;
        } else if b <= value {
            return NodeType::Alpha;
        } else {
            return NodeType::Exact;
        }
    }
}

const DEFAULT_SIZE: usize = 1 << 22;

#[derive(Debug)]
pub struct AtomicTranspositionTable {
    data: Box<[AtomicPositionData; DEFAULT_SIZE]>,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct AtomicPositionData(Atomic<PositionData>);
const IS_POSITION_DATA_LOCK_FREE: bool = Atomic::<PositionData>::is_lock_free();

impl Default for AtomicPositionData {
    fn default() -> Self {
        Self(Atomic::default())
    }
}

impl Clone for AtomicPositionData {
    fn clone(&self) -> Self {
        Self(Atomic::new(self.0.load(Ordering::SeqCst)))
    }
}

impl AtomicPositionData {
    #[inline(always)]
    fn new(key: ZobristHash, depth: u16, eval: i16, ty: NodeType, best: Option<ChessMove>) -> Self {
        return AtomicPositionData(Atomic::new(PositionData::new(key, depth, eval, ty, best)));
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut PositionData {
        self.0.get_mut()
    }

    #[inline(always)]
    pub fn into_inner(self) -> PositionData {
        self.0.into_inner()
    }

    #[inline(always)]
    pub fn load(&self, order: Ordering) -> PositionData {
        self.0.load(order)
    }

    #[inline(always)]
    pub fn store(&self, val: PositionData, order: Ordering) {
        self.0.store(val, order);
    }

    #[inline(always)]
    pub fn swap(&self, val: PositionData, order: Ordering) -> PositionData {
        self.0.swap(val, order)
    }

    #[inline(always)]
    const fn const_default() -> Self {
        return Self(Atomic::new(PositionData::const_default()));
    }
}

impl AtomicTranspositionTable {
    #[inline(always)]
    pub fn new() -> Self {
        //this is ugly
        return AtomicTranspositionTable { data: vec![AtomicPositionData::default(); DEFAULT_SIZE].try_into().unwrap() };
        //return TranspositionTable { data: [NodeData::default(); DEFAULT_SIZE] };
    }

    #[rustfmt::skip]
    #[inline(always)]
    pub fn update_tt(&self, hash: ZobristHash, value: i16, chess_move: Option<ChessMove>, a: i16, b: i16, d: u16, order: Ordering) {
        let node_type = PositionData::value_type(value, a, b);
       //NOTE: when node_type == NodeType::Beta, used to store None
        self.store(hash, d, value, node_type, chess_move, order);

    }

    //TODO need to think of replacement policy. right now it uses the naive always replace
    #[rustfmt::skip]
    #[inline(always)]
    pub fn store(&self, hash: ZobristHash, depth: u16, eval: i16, ty: NodeType, best: Option<ChessMove>, order: Ordering) {
        self.data[hash.to_usize() % DEFAULT_SIZE].store(  PositionData::new(hash, depth, eval, ty, best), order);
    }

    #[inline(always)]
    pub(crate) fn load(&self, hash: ZobristHash, order: Ordering) -> Option<PositionData> {
        let data = self.data[hash.to_usize() % DEFAULT_SIZE].load(order);
        match data.is_valid(hash) {
            true => Some(data),
            false => None,
        }
    }

    #[inline(always)]
    pub fn permil_count(&self) -> usize {
        let mut total: usize = 0;
        for i in 0..1000 {
            if self.data[i].0.load(Ordering::Relaxed).ty != NodeType::None {
                total += 1;
            }
        }
        return total;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, NoUninit)]
#[repr(C, align(8))]
pub struct SmallPositionData {
    depth: u16, //is u8 enough? u16 should be enough, its over 64 000+
    eval: i16,
    ty: NodeType,
    best: MaybeChessMove,
}

const IS_SMALL_POSITION_DATA_LOCK_FREE: bool = Atomic::<SmallPositionData>::is_lock_free();
const IS_ZOBRIST_HASH_LOCK_FREE: bool = Atomic::<ZobristHash>::is_lock_free();

#[derive(Debug)]
pub struct SmallAtomicPositionData {
    hash: Atomic<ZobristHash>,
    data: Atomic<SmallPositionData>,
}

#[derive(Debug)]
pub struct SmallAtomicTranspositionTable {
    data: Box<[SmallAtomicPositionData; DEFAULT_SIZE]>,
}

impl Default for SmallAtomicPositionData {
    fn default() -> SmallAtomicPositionData {
        SmallAtomicPositionData { hash: Atomic::new(ZobristHash::ZERO), data: Atomic::new(SmallPositionData::ZERO) }
    }
}

impl SmallPositionData {
    const ZERO: SmallPositionData = SmallPositionData { depth: 0, eval: 0, ty: NodeType::None, best: MaybeChessMove::NONE };

    #[inline]
    fn new(depth: u16, eval: i16, ty: NodeType, best: Option<ChessMove>) -> SmallPositionData {
        SmallPositionData { depth, eval, ty, best: MaybeChessMove::from(best) }
    }

    #[inline(always)]
    pub const fn depth(&self) -> u16 {
        return self.depth;
    }

    #[inline(always)]
    pub const fn eval(&self) -> i16 {
        return self.eval;
    }

    #[inline(always)]
    pub const fn ty(&self) -> NodeType {
        return self.ty;
    }

    #[inline(always)]
    pub fn best(&self) -> Option<ChessMove> {
        return Option::<ChessMove>::from(self.best);
    }
}

impl SmallAtomicPositionData {
    const ZERO: SmallAtomicPositionData = SmallAtomicPositionData { hash: Atomic::new(ZobristHash::ZERO), data: Atomic::new(SmallPositionData::ZERO) };

    #[inline(always)]
    fn new(key: ZobristHash, depth: u16, eval: i16, ty: NodeType, best: Option<ChessMove>) -> SmallAtomicPositionData {
        let data: SmallPositionData = SmallPositionData::new(depth, eval, ty, best);
        return SmallAtomicPositionData { hash: Atomic::new(key ^ cast(data)), data: Atomic::new(data) };
    }

    //#[inline(always)]
    //pub fn get_mut(&mut self) -> &mut PositionData {
    //    self.0.get_mut()
    //}

    //#[inline(always)]
    //pub fn into_inner(self) -> PositionData {
    //    self.0.into_inner()
    //}

    #[inline(always)]
    pub fn load(&self, order: Ordering) -> (ZobristHash, SmallPositionData) {
        (self.hash.load(order), self.data.load(order))
    }

    //#[inline(always)]
    //pub fn store(&self, val: PositionData, order: Ordering) {
    //    self.0.store(val, order);
    //}

    //#[inline(always)]
    //pub fn swap(&self, val: PositionData, order: Ordering) -> PositionData {
    //    self.0.swap(val, order)
    //}

    //#[inline(always)]
    //const fn const_default() -> Self {
    //    return Self(Atomic::new(PositionData::const_default()));
    //}

    //#[inline(always)]
    //pub fn is_valid(&self, hash: ZobristHash, order: Ordering) -> bool {
    //    let data = self.load(order);
    //    return !matches!(data.ty, NodeType::None) && matches!(data, hash);
    //}
    //
    //#[inline(always)]
    //pub fn key(&self, order: Ordering) -> ZobristHash {
    //    return self.load(order).key;
    //}
    //
    //#[inline(always)]
    //pub fn depth(&self, order: Ordering) -> u16 {
    //    return self.load(order).depth;
    //}
    //
    //#[inline(always)]
    //pub fn eval(&self, order: Ordering) -> i16 {
    //    return self.load(order).eval;
    //}
    //
    //#[inline(always)]
    //pub fn ty(&self, order: Ordering) -> NodeType {
    //    return self.load(order).ty;
    //}
    //
    //#[inline(always)]
    //pub fn best(&self, order: Ordering) -> Option<ChessMove> {
    //    return Option::<ChessMove>::from(self.load(order).best);
    //}
    //
    //#[inline(always)]
    //pub fn pair(&self, order: Ordering) -> ScoredMove {
    //    let data = self.load(order);
    //    return ScoredMove::new(data.eval, Option::<ChessMove>::from(data.best));
    //}
    //
    //#[inline(always)]
    //pub const fn value_type(value: i16, a: i16, b: i16) -> NodeType {
    //    if value <= a {
    //        return NodeType::Beta;
    //    } else if b <= value {
    //        return NodeType::Alpha;
    //    } else {
    //        return NodeType::Exact;
    //    }
    //}
}

impl SmallAtomicTranspositionTable {
    #[inline(always)]
    pub fn new() -> Self {
        let data = {
            //this is ugly
            let mut data: Box<[MaybeUninit<SmallAtomicPositionData>; DEFAULT_SIZE]> = Box::new([const { MaybeUninit::uninit() }; DEFAULT_SIZE]);
            for i in 0..DEFAULT_SIZE {
                data[i] = MaybeUninit::new(SmallAtomicPositionData::ZERO);
            }
            unsafe { mem::transmute(data) }
        };

        return SmallAtomicTranspositionTable { data };
        //return TranspositionTable { data: [NodeData::default(); DEFAULT_SIZE] };
    }

    #[rustfmt::skip]
    #[inline(always)]
    pub fn update_tt(&self, hash: ZobristHash, eval: i16, chess_move: Option<ChessMove>, a: i16, b: i16, d: u16, order: Ordering) {
        let node_type = PositionData::value_type(eval, a, b);
       //NOTE: when node_type == NodeType::Beta, used to store None
        self.store(hash, d, eval, node_type, chess_move, order);

    }

    //TODO need to think of replacement policy. right now it uses the naive always replace
    #[rustfmt::skip]
    #[inline(always)]
    pub fn store(&self, hash: ZobristHash, depth: u16, eval: i16, ty: NodeType, best: Option<ChessMove>, order: Ordering) {
        let value: SmallPositionData = SmallPositionData::new( depth, eval, ty, best);
        self.data[hash.to_usize() % DEFAULT_SIZE].data.store(value, order);
        self.data[hash.to_usize() % DEFAULT_SIZE].hash.store(hash ^ cast(value), order);
    }

    #[inline(always)]
    pub(crate) fn load(&self, hash: ZobristHash, order: Ordering) -> Option<SmallPositionData> {
        let (tt_hash, value) = self.data[hash.to_usize() % DEFAULT_SIZE].load(order);

        return match tt_hash ^ cast(value) == hash {
            true => Some(value),
            false => None,
        };
    }

    #[inline(always)]
    pub fn permil_count(&self) -> usize {
        let mut total: usize = 0;
        for i in 0..1000 {
            if self.data[i].data.load(Ordering::Relaxed).ty != NodeType::None {
                total += 1;
            }
        }
        return total;
    }
}

/* ======== debug zone, const stuff, and to figure out allignment/size padding etc ======== */
//const fn check_sync<T: Sync>() {}
//const _: () = check_sync::<Rc<()>>(); //fails
//const _: () = check_sync::<SmallAtomicTranspositionTable>(); //passes

//const SIZE: usize = mem::size_of::<SmallPositionData>(); // 8
//const ALIGN: usize = mem::align_of::<SmallPositionData>(); // 2
//const HAS_ATOMIC_64: bool = cfg!(target_has_atomic = "64"); // true
//const HAS_ATOMIC_128: bool = cfg!(target_has_atomic = "128"); // false
//#[inline]
//pub const fn atomic_is_lock_free<T>() -> bool {
//    let size = mem::size_of::<T>();
//    let align = mem::align_of::<T>();
//
//    (cfg!(target_has_atomic = "8") & (size == 1) & (align >= 1))
//        | (cfg!(target_has_atomic = "16") & (size == 2) & (align >= 2))
//        | (cfg!(target_has_atomic = "32") & (size == 4) & (align >= 4))
//        | (cfg!(target_has_atomic = "64") & (size == 8) & (align >= 8))
//        | (cfg!(feature = "nightly")
//            & cfg!(target_has_atomic = "128")
//            & (size == 16)
//            & (align >= 16))
//}
