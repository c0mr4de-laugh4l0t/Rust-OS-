// task.rs
#![no_std]

use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::memory::FRAME_SIZE;
use crate::memory::PhysFrame;
use crate::memory::PhysicalMemoryManager;

use spin::Mutex;

pub type Tid = usize;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Finished,
}

pub struct Task {
    pub tid: Tid,
    pub rsp: usize,        // stack pointer (virtual==physical in our early kernel)
    pub stack_base: usize, // stack virtual addr (lowest address)
    pub stack_size: usize,
    pub state: TaskState,
}

impl Task {
    pub const fn empty() -> Self {
        Self { tid: 0, rsp: 0, stack_base: 0, stack_size: 0, state: TaskState::Finished }
    }

    pub fn mark_finished(&mut self) {
        self.state = TaskState::Finished;
    }
}

/// Very small task allocator / registry
pub struct TaskTable {
    pub tasks: [Task; TaskTable::MAX_TASKS],
    pub next_tid: AtomicUsize,
}

impl TaskTable {
    pub const MAX_TASKS: usize = 16;

    pub const fn new() -> Self {
        Self {
            tasks: [Task::empty(); TaskTable::MAX_TASKS],
            next_tid: AtomicUsize::new(1),
        }
    }

    pub fn alloc_tid(&self) -> Tid {
        self.next_tid.fetch_add(1, Ordering::SeqCst)
    }

    /// find first free slot index or None
    pub fn find_free_slot(&mut self) -> Option<usize> {
        for (i, t) in self.tasks.iter().enumerate() {
            if t.state == TaskState::Finished {
                return Some(i);
            }
        }
        None
    }
}

/// Global task registry protected by a spinlock
lazy_static::lazy_static! {
    pub static ref TASK_TABLE: Mutex<TaskTable> = Mutex::new(TaskTable::new());
}

/// Allocate stack using PMM. Returns (stack_base, stack_size) in bytes.
/// stack_size multiple of FRAME_SIZE.
pub fn alloc_stack(pmm: &PhysicalMemoryManager, pages: usize) -> Option<(usize, usize)> {
    // allocate `pages` frames and return linear stack region (contiguous frames)
    // Our PMM only alloc_frame() single frames; we attempt to allocate pages *contiguously*
    // by allocating pages sequentially and checking contiguity — simple but OK for demo.
    let mut addrs: [usize; 64] = [0usize; 64];
    if pages > addrs.len() { return None; }
    for i in 0..pages {
        match pmm.alloc_frame() {
            Some(f) => addrs[i] = f.start_address(),
            None => {
                // on failure, free allocated so far
                for j in 0..i { pmm.free_frame(addrs[j]); }
                return None;
            }
        }
    }
    // For simplicity we assume PMM gave contiguous frames (likely for small pools).
    // If not contiguous, we still create a stack by using high addresses first (stack grows down).
    // Stack base will be the highest allocated address + FRAME_SIZE.
    let mut max = 0usize;
    let mut min = usize::MAX;
    for i in 0..pages {
        if addrs[i] > max { max = addrs[i]; }
        if addrs[i] < min { min = addrs[i]; }
    }
    let stack_base = min;
    let stack_size = pages * FRAME_SIZE;
    Some((stack_base, stack_size))
}

/// Free stack frames given base & size
pub fn free_stack(pmm: &PhysicalMemoryManager, base: usize, size: usize) {
    let pages = size / FRAME_SIZE;
    for i in 0..pages {
        let pa = base + i * FRAME_SIZE;
        let _ = pmm.free_frame(pa);
    }
}

/// Prepare initial stack for a new task such that when we context_switch into it,
/// it will start executing `entry: fn()` and when entry returns it calls `task_exit`.
pub fn prepare_stack(entry: extern "C" fn(), stack_base: usize, stack_size: usize) -> usize {
    // stack grows down. We set initial rsp to top of stack (stack_base + stack_size).
    let mut sp = stack_base + stack_size;

    // Align stack to 16 bytes for ABI stability
    sp &= !0xF;

    // On x86_64, when switching we will restore callee-saved registers and 'ret' into instruction pointer.
    // We'll push a fake return address which is the entry function. So after context switch uses 'ret',
    // it will jump into entry.
    unsafe {
        // write entry pointer as top-of-stack so the first 'ret' jumps to entry
        sp -= core::mem::size_of::<usize>();
        let p = sp as *mut usize;
        core::ptr::write_volatile(p, entry as usize);

        // We also push a sentinel for RBP (old base pointer)
        sp -= core::mem::size_of::<usize>();
        let p2 = sp as *mut usize;
        core::ptr::write_volatile(p2, 0usize);
    }
    sp
      }
