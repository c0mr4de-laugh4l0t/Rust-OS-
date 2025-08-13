#![no_std]
#![no_main]

pub mod interrupts;
pub mod vga;
pub mod kb;
pub mod memory;
pub mod scheduler;
pub mod task;
pub mod fs;

// re-export PMM if needed
pub use memory::PhysicalMemoryManager;