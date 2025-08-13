#![no_std]

use linked_list_allocator::LockedHeap;
use crate::memory::FRAME_SIZE;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 2 * 1024 * 1024; // 2 MiB heap (adjust if you want larger)

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub unsafe fn init_heap(pmm: &crate::memory::PhysicalMemoryManager) {
    let heap_start = HEAP_START;
    let heap_size = HEAP_SIZE;

    let frames = (heap_size + FRAME_SIZE - 1) / FRAME_SIZE;
    let mut pa = heap_start;

    for _ in 0..frames {
        pmm.mark_used(pa);
        pa = pa.wrapping_add(FRAME_SIZE);
    }

    ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    crate::vga::vprintln!("ALLOC ERROR: {:?}", layout);
    loop { core::hint::spin_loop(); }
}