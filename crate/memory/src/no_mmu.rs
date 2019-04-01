use alloc::alloc::{GlobalAlloc, Layout};
use alloc::vec::Vec;
use core::marker::PhantomData;

pub trait NoMMUSupport {
    type Alloc: GlobalAlloc + 'static;
    fn allocator() -> &'static Self::Alloc;
}

#[derive(Clone, Debug)]
pub struct MemorySet<S: NoMMUSupport> {
    areas: Vec<MemoryArea<S>>,
    support: PhantomData<S>,
}

impl<S: NoMMUSupport> MemorySet<S> {
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            support: PhantomData,
        }
    }
    /// Allocate `size` bytes space. Return the slice.
    pub fn push(&mut self, size: usize) -> &'static mut [u8] {
        let area = MemoryArea::new(size);
        let slice = unsafe { area.as_buf() };
        self.areas.push(area);
        slice
    }
    // empty impls
    pub fn with<T>(&self, f: impl FnOnce() -> T) -> T {
        f()
    }
    pub fn token(&self) -> usize {
        0
    }
    pub unsafe fn activate(&self) {}
}

#[derive(Debug)]
struct MemoryArea<S: NoMMUSupport> {
    ptr: usize,
    layout: Layout,
    support: PhantomData<S>,
}

impl<S: NoMMUSupport> MemoryArea<S> {
    fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, 1).unwrap();
        let ptr = unsafe { S::allocator().alloc(layout) } as usize;
        MemoryArea {
            ptr,
            layout,
            support: PhantomData,
        }
    }
    unsafe fn as_buf(&self) -> &'static mut [u8] {
        core::slice::from_raw_parts_mut(self.ptr as *mut u8, self.layout.size())
    }
}

impl<S: NoMMUSupport> Clone for MemoryArea<S> {
    fn clone(&self) -> Self {
        let new_area = MemoryArea::new(self.layout.size());
        unsafe { new_area.as_buf().copy_from_slice(self.as_buf()) }
        new_area
    }
}

impl<S: NoMMUSupport> Drop for MemoryArea<S> {
    fn drop(&mut self) {
        unsafe { S::allocator().dealloc(self.ptr as *mut u8, self.layout) }
    }
}
