use std::ops::Index;

#[derive(Debug, Copy, Clone)]
pub struct ArrayVec<T: Copy + ConstDefault, const C: usize> {
    data: [T; C], //note: C should be strictly bigger than 0
    count: usize,
}

// Note: the Default trait associated default function is not const,
// and functions in traits can't be declared const.
// This trait is used as a const-compatible Default replacement.
pub trait ConstDefault: Sized {
    const DEFAULT: Self;
}

impl<T: Copy + ConstDefault, const C: usize> Index<usize> for ArrayVec<T, C> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.count);
        &self.data[index]
    }
}

impl<T: Copy + ConstDefault, const C: usize> ArrayVec<T, C> {
    const CAPACITY: usize = C; // 0..CAPACITY
    const DEFAULT: T = T::DEFAULT;

    pub const fn new() -> Self {
        ArrayVec { data: [Self::DEFAULT; C], count: 0 }
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

    pub fn push(&mut self, t: T) {
        assert!(self.len() < (u32::MAX as usize));
        self.data[self.len()] = t;
        self.count += 1;
    }

    //const push alternative
    pub const fn append(self, t: T) -> Self {
        assert!(self.count < (u32::MAX as usize));
        let mut new = ArrayVec::<T, C> { data: self.data, count: self.count };
        new.data[self.count] = t;
        new.count += 1;
        return new;
    }

    //indexing alternative
    pub const fn nth(&self, index: usize) -> T {
        assert!(index < Self::CAPACITY);
        self.data[index]
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub const fn is_full(&self) -> bool {
        self.count == Self::CAPACITY
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
        let mut new = ArrayVec::<T, C> { data: self.data, count: self.count - 1 };
        let x = self.data[self.count - 1];
        new.data[self.count - 1] = Self::DEFAULT;
        return (new, Some(x));
    }
}
