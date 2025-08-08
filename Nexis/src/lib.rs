mod memory;
use memory::{PhysicalMemoryManager, FRAME_SIZE, PhysFrame};

// Global manager (static)
use spin::Mutex as SpinMutex;
static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

// Simple safe wrapper to access PMM (unsafe still internally)
pub fn pmm() -> &'static PhysicalMemoryManager {
    unsafe { &PMM }
}
