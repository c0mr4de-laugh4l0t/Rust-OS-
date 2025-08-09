// scheduler.rs
#![no_std]

use crate::task::{Task, TaskState, TASK_TABLE, alloc_stack, free_stack};
use crate::memory::PhysicalMemoryManager;
use spin::Mutex;
use lazy_static::lazy_static;

extern "C" {
    /// Assembly routine that switches contexts.
    /// old_rsp: pointer to save current rsp (will write new saved rsp into *old_rsp)
    /// new_rsp: value of next task's rsp to load.
    fn context_switch(old_rsp_ptr: *mut usize, new_rsp: usize);
}

lazy_static! {
    pub static ref RUN_QUEUE: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    pub static ref CURRENT: Mutex<Option<usize>> = Mutex::new(None); // index in TASK_TABLE
}

/// Start scheduler: should be called once after tasks are created. Will never return.
pub fn schedule_loop() -> ! {
    loop {
        // Simple round robin: find next ready task in TASK_TABLE
        let next_idx = {
            let mut table = TASK_TABLE.lock();
            let mut found: Option<usize> = None;
            for i in 0..TaskTable::MAX_TASKS {
                let t = &table.tasks[i];
                if t.state == TaskState::Ready {
                    found = Some(i);
                    break;
                }
            }
            found
        };
        let next = match next_idx {
            Some(i) => i,
            None => {
                // nothing to run: halt
                core::hint::spin_loop();
                continue;
            }
        };

        // switch to next
        // get mutable references
        let mut tbl = TASK_TABLE.lock();
        let cur_idx_opt = *CURRENT.lock();
        let next_task = &mut tbl.tasks[next];
        next_task.state = TaskState::Running;

        match cur_idx_opt {
            Some(cur_idx) => {
                if cur_idx == next { continue; }
                let cur_task = &mut tbl.tasks[cur_idx];
                // Save current rsp pointer location on stack of cur_task?
                // We'll store the current rsp into cur_task.rsp via context_switch.
                unsafe {
                    context_switch(&mut (cur_task.rsp as *mut usize).read(), next_task.rsp);
                }
                // After switch back, mark tasks appropriately (cooperative)
            }
            None => {
                // first scheduling: just jump to next_task
                unsafe {
                    // create local old_rsp storage to be filled by context_switch
                    let mut old_rsp: usize = 0;
                    unsafe {context_switch(&mut old_rsp as *mut usize, next_task.rsp); }
                    cur_task.rsp = old_rsp; // Store saved rsp back to the current task's PCB
                }
            }
        }
        *CURRENT.lock() = Some(next);
    }
}

/// Yield the CPU (cooperative). Save current state and return to scheduler.
/// This is a library routine called by tasks to voluntarily yield.
pub fn task_yield() {
    // get current task index
    let cur_idx_opt = *CURRENT.lock();
    if cur_idx_opt.is_none() { return; }

    let cur_idx = cur_idx_opt.unwrap();
    let mut tbl = TASK_TABLE.lock();
    let cur_task = &mut tbl.tasks[cur_idx];

    // mark ready
    cur_task.state = TaskState::Ready;

    // find next ready (naive)
    let mut next_idx = None;
    for i in 0..TaskTable::MAX_TASKS {
        if i == cur_idx { continue; }
        if tbl.tasks[i].state == TaskState::Ready {
            next_idx = Some(i);
            break;
        }
    }

    if let Some(next) = next_idx {
        let next_task = &mut tbl.tasks[next];
        next_task.state = TaskState::Running;
        // switch contexts
        unsafe {
            context_switch(&mut (cur_task.rsp as *mut usize).read(), next_task.rsp);
        }
        *CURRENT.lock() = Some(next);
    }
}

/// Create a new task from given entry function. Uses PMM to allocate stack pages.
/// Returns TID or None on failure.
pub fn spawn(entry: extern "C" fn(), pmm: &PhysicalMemoryManager, pages: usize) -> Option<usize> {
    let mut tbl = TASK_TABLE.lock();
    let slot = tbl.find_free_slot()?;
    let tid = tbl.alloc_tid();

    // allocate stack
    if let Some((stack_base, stack_size)) = alloc_stack(pmm, pages) {
        let rsp = crate::task::prepare_stack(entry, stack_base, stack_size);
        let mut t = Task {
            tid,
            rsp,
            stack_base,
            stack_size,
            state: TaskState::Ready,
        };
        tbl.tasks[slot] = t;
        Some(slot)
    } else {
        None
    }
}

/// When a task finishes, call this to mark finished and free stack
pub fn check_and_schedule() {
    if crate::pit::NEED_RESCHED.swap(false, core::sync::atomic::Ordering::SeqCst) {
        // Only run scheduler if flag was set
        schedule_tick();
    }
}
