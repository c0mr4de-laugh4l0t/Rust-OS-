// scheduler.rs
#![no_std]

use crate::task::{Task, TaskState, TASK_TABLE};
use crate::memory::PhysicalMemoryManager;
use spin::Mutex;
use lazy_static::lazy_static;

extern "C" {
    /// Assembly routine that switches contexts.
    /// old_rsp_ptr: pointer to a usize where current RSP will be stored
    /// new_rsp: the RSP value to load for the new context.
    fn context_switch(old_rsp_ptr: *mut usize, new_rsp: usize);
}

lazy_static! {
    // CURRENT holds the index into TASK_TABLE of the currently running task (if any)
    pub static ref CURRENT: Mutex<Option<usize>> = Mutex::new(None);
}

/// Start scheduler: should be called once after tasks are created. Will never return.
/// Simple cooperative loop: finds next Ready task and switches to it.
pub fn schedule_loop() -> ! {
    loop {
        // find next ready task (naive round-robin starting from current+1)
        let next_opt = {
            let table = TASK_TABLE.lock();
            let n = crate::task::TaskTable::MAX_TASKS;
            let start = match *CURRENT.lock() {
                Some(idx) => idx,
                None => n - 1, // start from -1 so first found is 0
            };
            let mut found: Option<usize> = None;
            for off in 1..=n {
                let idx = (start + off) % n;
                if table.tasks[idx].state == TaskState::Ready {
                    found = Some(idx);
                    break;
                }
            }
            found
        };

        let next = match next_opt {
            Some(i) => i,
            None => {
                // nothing to run: halt (or spin)
                core::hint::spin_loop();
                continue;
            }
        };

        // Acquire table for switching
        let mut table = TASK_TABLE.lock();
        let mut current_guard = CURRENT.lock();

        // Prepare next task
        {
            let next_task = &mut table.tasks[next];
            next_task.state = TaskState::Running;
        }

        match *current_guard {
            Some(cur_idx) => {
                if cur_idx == next {
                    // same task chosen; nothing to do
                    continue;
                }

                // Preempt current task: save its rsp and switch to next
                let cur_task = &mut table.tasks[cur_idx];
                let next_rsp = table.tasks[next].rsp;

                // local storage for old rsp
                let mut old_rsp: usize = 0;
                unsafe {
                    // Save current RSP into old_rsp and switch to next_rsp
                    context_switch(&mut old_rsp as *mut usize, next_rsp);
                }
                // When we return here, we've been switched back from the other task.
                // Store saved rsp back into the preempted task's PCB and mark it Ready.
                cur_task.rsp = old_rsp;
                cur_task.state = TaskState::Ready;
            }
            None => {
                // First scheduling: no current task. Jump to next.
                let next_rsp = table.tasks[next].rsp;
                let mut old_rsp: usize = 0;
                unsafe {
                    context_switch(&mut old_rsp as *mut usize, next_rsp);
                }
                // When we return here, scheduler regained control.
                // Nothing to store for a non-existing previous task.
            }
        }

        // Update current
        *current_guard = Some(next);
    }
}

/// Yield the CPU (cooperative). Save current state and return to scheduler.
/// This is a library routine called by tasks to voluntarily yield.
pub fn task_yield() {
    let cur_idx_opt = *CURRENT.lock();
    if cur_idx_opt.is_none() {
        return;
    }
    let cur_idx = cur_idx_opt.unwrap();

    let mut table = TASK_TABLE.lock();

    // mark current task as ready
    table.tasks[cur_idx].state = TaskState::Ready;

    // find next ready task
    let n = crate::task::TaskTable::MAX_TASKS;
    let mut next_idx: Option<usize> = None;
    for i in 0..n {
        if i == cur_idx { continue; }
        if table.tasks[i].state == TaskState::Ready {
            next_idx = Some(i);
            break;
        }
    }

    if let Some(next) = next_idx {
        // prepare next
        table.tasks[next].state = TaskState::Running;

        // perform context switch: save current rsp into old_rsp and load next.rsp
        let mut old_rsp: usize = 0;
        let next_rsp = table.tasks[next].rsp;
        unsafe {
            context_switch(&mut old_rsp as *mut usize, next_rsp);
        }
        // when we return, scheduler regained control and old_rsp contains the saved RSP
        table.tasks[cur_idx].rsp = old_rsp;
        *CURRENT.lock() = Some(next);
    } else {
        // no other ready task; continue running current
        table.tasks[cur_idx].state = TaskState::Running;
    }
}

/// Create a new task from given entry function. Uses PMM to allocate stack pages.
/// Returns slot index (task id) or None on failure.
pub fn spawn(entry: extern "C" fn(), pmm: &PhysicalMemoryManager, pages: usize) -> Option<usize> {
    let mut tbl = TASK_TABLE.lock();
    let slot = tbl.find_free_slot()?;
    let tid = tbl.alloc_tid();

    // allocate stack (uses your task::alloc_stack/prepare_stack logic)
    if let Some((stack_base, stack_size)) = crate::task::alloc_stack(pmm, pages) {
        let rsp = crate::task::prepare_stack(entry, stack_base, stack_size);
        let t = Task {
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

/// Mark task finished and free its stack (helper)
pub fn task_exit(slot: usize, pmm: &PhysicalMemoryManager) {
    let mut tbl = TASK_TABLE.lock();
    let t = &mut tbl.tasks[slot];
    t.state = TaskState::Finished;
    // free stack if non-zero
    if t.stack_size != 0 {
        crate::task::free_stack(pmm, t.stack_base, t.stack_size);
    }
}

/// schedule_tick: Called in cautious mode from check_and_schedule() or (later) from IRQ.
/// Performs one scheduling decision (round-robin) and context switch.
/// Safe to call from a normal context. If you call this from IRQ directly you must ensure
/// your context_switch and stack discipline allow it.
pub fn schedule_tick() {
    // reuse much of the logic from schedule_loop for a single tick
    let next_opt = {
        let table = TASK_TABLE.lock();
        let n = crate::task::TaskTable::MAX_TASKS;
        let start = match *CURRENT.lock() {
            Some(idx) => idx,
            None => n - 1,
        };
        let mut found: Option<usize> = None;
        for off in 1..=n {
            let idx = (start + off) % n;
            if table.tasks[idx].state == TaskState::Ready {
                found = Some(idx);
                break;
            }
        }
        found
    };

    let next = match next_opt {
        Some(i) => i,
        None => return, // nothing to run
    };

    let mut table = TASK_TABLE.lock();
    let mut current_guard = CURRENT.lock();

    // prepare next
    table.tasks[next].state = TaskState::Running;

    match *current_guard {
        Some(cur_idx) => {
            if cur_idx == next {
                return;
            }
            let cur_task = &mut table.tasks[cur_idx];
            let next_rsp = table.tasks[next].rsp;
            let mut old_rsp: usize = 0;
            unsafe {
                context_switch(&mut old_rsp as *mut usize, next_rsp);
            }
            cur_task.rsp = old_rsp;
            cur_task.state = TaskState::Ready;
            *current_guard = Some(next);
        }
        None => {
            let next_rsp = table.tasks[next].rsp;
            let mut old_rsp: usize = 0;
            unsafe {
                context_switch(&mut old_rsp as *mut usize, next_rsp);
            }
            *current_guard = Some(next);
        }
    }
}

/// check_and_schedule wrapper used by cautious PIT mode:
pub fn check_and_schedule() {
    if crate::pit::NEED_RESCHED.swap(false, core::sync::atomic::Ordering::SeqCst) {
        // Perform any maintenance first (waking sleepers etc.)
        crate::scheduler::tick_maintenance();
        // then schedule one tick
        schedule_tick();
    }
}

/// Maintenance hook called from scheduler to wake sleepers etc.
pub fn tick_maintenance() {
    crate::process::wake_sleepers();
}

/// Query current running task slot index (if any)
pub fn current_index() -> Option<usize> {
    *CURRENT.lock()
}