#![no_std]
#![no_main]

extern crate alloc;
pub mod interrupts;
pub mod vga;
pub mod kb;
pub mod memory;
pub mod scheduler;
pub mod task;
pub mod fs;
mod alloc;

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}
// re-export PMM if needed
pub use memory::PhysicalMemoryManager;