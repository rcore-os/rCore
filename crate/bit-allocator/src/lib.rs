#![no_std]
#![feature(asm)]

extern crate bit_field;

use bit_field::BitField;
use core::ops::Range;

pub trait BitAlloc: Default {
    const CAP: usize;
    fn alloc(&mut self) -> Option<usize>;
    fn dealloc(&mut self, key: usize);
    fn remove(&mut self, range: Range<usize>);
    fn any(&self) -> bool;
    fn test(&self, key: usize) -> bool;
}

pub type BitAlloc256 = BitAllocCascade16<BitAlloc16>;
pub type BitAlloc4K = BitAllocCascade16<BitAlloc256>;
pub type BitAlloc64K = BitAllocCascade16<BitAlloc4K>;
pub type BitAlloc1M = BitAllocCascade16<BitAlloc64K>;
pub type BitAlloc16M = BitAllocCascade16<BitAlloc1M>;
pub type BitAlloc256M = BitAllocCascade16<BitAlloc16M>;

pub struct BitAllocCascade16<T: BitAlloc> {
    bitset: u16,
    sub: [T; 16],
}

impl<T: BitAlloc> Default for BitAllocCascade16<T> {
    fn default() -> Self {
        Self {
            bitset: 0xffff,
            sub: <[T; 16]>::default(),
        }
    }
}

impl<T: BitAlloc> BitAlloc for BitAllocCascade16<T> {
    const CAP: usize = T::CAP * 16;

    fn alloc(&mut self) -> Option<usize> {
        if self.any() {
            let i = log2(self.bitset);
            let res = self.sub[i].alloc().unwrap() + i * T::CAP;
            self.bitset.set_bit(i, self.sub[i].any());
            Some(res)
        } else {
            None
        }
    }
    fn dealloc(&mut self, key: usize) {
        let i = key / T::CAP;
        self.sub[i].dealloc(key % T::CAP);
        self.bitset.set_bit(i, true);
    }
    fn remove(&mut self, range: Range<usize>) {
        let Range { start, end } = range;
        assert!(end <= Self::CAP);
        for i in start / T::CAP..=(end - 1) / T::CAP {
            let begin = if start / T::CAP == i { start % T::CAP } else { 0 };
            let end = if end / T::CAP == i { end % T::CAP } else { T::CAP };
            self.sub[i].remove(begin..end);
            self.bitset.set_bit(i, self.sub[i].any());
        }
    }
    fn any(&self) -> bool {
        self.bitset != 0
    }
    fn test(&self, key: usize) -> bool {
        self.sub[key / T::CAP].test(key % T::CAP)
    }
}

pub struct BitAlloc16(u16);

impl Default for BitAlloc16 {
    fn default() -> Self {
        BitAlloc16(0xffff)
    }
}

impl BitAlloc for BitAlloc16 {
    const CAP: usize = 16;

    fn alloc(&mut self) -> Option<usize> {
        if self.any() {
            let i = log2(self.0);
            self.0.set_bit(i, false);
            Some(i)
        } else {
            None
        }
    }
    fn dealloc(&mut self, key: usize) {
        assert!(!self.test(key));
        self.0.set_bit(key, true);
    }
    fn remove(&mut self, range: Range<usize>) {
        self.0.set_bits(range, 0);
    }
    fn any(&self) -> bool {
        self.0 != 0
    }
    fn test(&self, key: usize) -> bool {
        self.0.get_bit(key)
    }
}

#[inline(always)]
fn log2(x: u16) -> usize {
    assert_ne!(x, 0);
    let pos: u16;
    unsafe { asm!("bsrw $1, $0" :"=r"(pos) :"r"(x) : :"volatile") };
    pos as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[inline(always)]
    fn log2_naive(mut x: u16) -> usize {
        assert_ne!(x, 0);
        let mut pos = -1;
        while x != 0 {
            pos += 1;
            x >>= 1;
        }
        pos as usize
    }

    #[test]
    fn log2_() {
        for x in 1..=0xffff {
            assert_eq!(log2(x), log2_naive(x), "log2 failed: {}", x);
        }
    }

    #[test]
    fn bitalloc16() {
        let mut ba = BitAlloc16::default();
        assert_eq!(BitAlloc16::CAP, 16);
        for i in 0..16 {
            assert_eq!(ba.test(i), true);
        }
        ba.remove(8..14);
        assert_eq!(ba.alloc(), Some(15));
        assert_eq!(ba.alloc(), Some(14));
        assert_eq!(ba.alloc(), Some(7));
        ba.dealloc(14);
        ba.dealloc(15);
        ba.dealloc(7);

        for _ in 0..10 {
            assert!(ba.alloc().is_some());
        }
        assert!(!ba.any());
        assert!(ba.alloc().is_none());
    }

    #[test]
    fn bitalloc4k() {
        let mut ba = BitAlloc4K::default();
        assert_eq!(BitAlloc4K::CAP, 4096);
        for i in 0..4096 {
            assert_eq!(ba.test(i), true);
        }
        ba.remove(8..4094);
        for i in 0..4096 {
            assert_eq!(ba.test(i), i < 8 || i >= 4094);
        }
        assert_eq!(ba.alloc(), Some(4095));
        assert_eq!(ba.alloc(), Some(4094));
        assert_eq!(ba.alloc(), Some(7));
        ba.dealloc(4095);
        ba.dealloc(4094);
        ba.dealloc(7);

        for _ in 0..10 {
            assert!(ba.alloc().is_some());
        }
        assert!(ba.alloc().is_none());
    }
}
