// Nexis/src/alloc.rs
#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use spin::Mutex;

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

unsafe impl GlobalAlloc for Mutex<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        let alloc_start = (allocator.next + layout.align() - 1) & !(layout.align() - 1);
        let alloc_end = alloc_start.saturating_add(layout.size());

        if alloc_end > allocator.heap_end {
            null_mut()
        } else {
            allocator.next = alloc_end;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // bump allocator cannot free
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: Mutex<BumpAllocator> = Mutex::new(BumpAllocator {
    heap_start: 0,
    heap_end: 0,
    next: 0,
});

/// Initialize allocator with given heap range
pub unsafe fn init_heap(start: usize, size: usize) {
    let mut alloc = GLOBAL_ALLOCATOR.lock();
    alloc.heap_start = start;
    alloc.heap_end = start + size;
    alloc.next = start;
}