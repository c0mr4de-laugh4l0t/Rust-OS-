#![no_std]

pub mod alloc;
pub mod context;
pub mod interrupts;
pub mod pit;
pub mod kb;
pub mod vga;
pub mod memory;
pub mod task;
pub mod scheduler;
pub mod process;
pub mod syscall;
pub mod syscall_dispatch;
pub mod fs;
pub mod userland;

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}

// re-export PMM if needed
pub use memory::PhysicalMemoryManager;