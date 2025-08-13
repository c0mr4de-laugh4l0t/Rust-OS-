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

// re-export PMM if needed
pub use memory::PhysicalMemoryManager;