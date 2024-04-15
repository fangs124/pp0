use std::ops::Index;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ConstVec<T: Copy + ConstDefault, const C: usize> {
    data: [T; C], //note: C should be strictly bigger than 0
    count: usize,
}

// Note: the Default trait associated default function is not const,
// and functions in traits can't be declared const.
// This trait is used as a const-compatible Default replacement.
pub trait ConstDefault: Sized {
    const DEFAULT: Self;
}

impl<T: Copy + ConstDefault, const C: usize> Index<usize> for ConstVec<T, C> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.count);
        &self.data[index]
    }
}

impl<T: Copy + ConstDefault, const C: usize> ConstVec<T, C> {
    pub const MAX_CAPACITY: usize = C; // 0..CAPACITY
    const DEFAULT: T = T::DEFAULT;

    pub const fn new() -> Self {
        ConstVec { data: [Self::DEFAULT; C], count: 0 }
    }

    pub const fn new_from(val: T) -> Self {
        let mut data: [T; C] = [Self::DEFAULT; C];
        data[0] = val;
        ConstVec { data, count: 1 }
    }

    pub const fn const_clone(&self) -> Self {
        let data = self.data;
        let count = self.count;
        return Self { data, count };
    }

    pub const fn data(&self) -> [T; C] {
        self.data
    }

    pub const fn len(&self) -> usize {
        return self.count;
    }

    pub const fn capacity(&self) -> usize {
        return Self::MAX_CAPACITY - self.len();
    }

    pub fn push(&mut self, t: T) {
        assert!(self.len() < (u32::MAX as usize));
        self.data[self.len()] = t;
        self.count += 1;
    }

    //const push alternative
    pub const fn append_one(self, t: T) -> Self {
        assert!(self.count < (u32::MAX as usize));
        let mut new = ConstVec::<T, C> { data: self.data, count: self.count };
        new.data[self.count] = t;
        new.count += 1;
        return new;
    }

    //const push alternative
    pub const fn append(self, ts: &[T], len: usize) -> Self {
        assert!(self.count + len < (u32::MAX as usize));
        let mut new = ConstVec::<T, C> { data: self.data, count: self.count };
        let mut i = 0;
        while i < len {
            new.data[self.count + i] = ts[i];
            i += 1;
        }
        new.count += len;
        return new;
    }

    //indexing alternative
    pub const fn nth(&self, index: usize) -> T {
        assert!(index < Self::MAX_CAPACITY);
        self.data[index]
    }

    //indexing alternative
    pub fn set(&mut self, index: usize, data: T) {
        self.data[index] = data;
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub const fn is_full(&self) -> bool {
        self.count == Self::MAX_CAPACITY
    }

    pub const fn head(&self) -> T {
        assert!(self.count > 0);
        return self.data[0];
    }

    pub const fn tail(&self) -> T {
        assert!(self.count > 0);
        return self.data[self.count - 1];
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }
        Some(self.data[self.count - 1])
    }

    //const pop alternative
    pub const fn unappend(self) -> (Self, Option<T>) {
        if self.count == 0 {
            return (self, None);
        }
        let mut new = ConstVec::<T, C> { data: self.data, count: self.count - 1 };
        let x = self.data[self.count - 1];
        new.data[self.count - 1] = Self::DEFAULT;
        return (new, Some(x));
    }
}
