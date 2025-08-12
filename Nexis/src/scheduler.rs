#![no_std]

use crate::task::{Task, TaskState, TASK_TABLE};
use crate::memory::PhysicalMemoryManager;
use spin::Mutex;
use lazy_static::lazy_static;

extern "C" {
    fn context_switch(old_rsp_ptr: *mut usize, new_rsp: usize);
}

lazy_static! {
    pub static ref CURRENT: Mutex<Option<usize>> = Mutex::new(None);
}

pub fn schedule_loop() -> ! {
    loop {
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
            None => {
                core::hint::spin_loop();
                continue;
            }
        };

        let mut table = TASK_TABLE.lock();
        let mut current_guard = CURRENT.lock();

        {
            let next_task = &mut table.tasks[next];
            next_task.state = TaskState::Running;
        }

        match *current_guard {
            Some(cur_idx) => {
                if cur_idx == next {
                    continue;
                }
                let cur_task = &mut table.tasks[cur_idx];
                let next_rsp = table.tasks[next].rsp;
                let mut old_rsp: usize = 0;
                unsafe {
                    context_switch(&mut old_rsp as *mut usize, next_rsp);
                }
                cur_task.rsp = old_rsp;
                cur_task.state = TaskState::Ready;
            }
            None => {
                let next_rsp = table.tasks[next].rsp;
                let mut old_rsp: usize = 0;
                unsafe {
                    context_switch(&mut old_rsp as *mut usize, next_rsp);
                }
            }
        }

        *current_guard = Some(next);
    }
}

pub fn task_yield() {
    let cur_idx_opt = *CURRENT.lock();
    if cur_idx_opt.is_none() {
        return;
    }
    let cur_idx = cur_idx_opt.unwrap();

    let mut table = TASK_TABLE.lock();

    table.tasks[cur_idx].state = TaskState::Ready;

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
        table.tasks[next].state = TaskState::Running;
        let mut old_rsp: usize = 0;
        let next_rsp = table.tasks[next].rsp;
        unsafe {
            context_switch(&mut old_rsp as *mut usize, next_rsp);
        }
        table.tasks[cur_idx].rsp = old_rsp;
        *CURRENT.lock() = Some(next);
    } else {
        table.tasks[cur_idx].state = TaskState::Running;
    }
}

pub fn spawn(entry: extern "C" fn(), pmm: &PhysicalMemoryManager, pages: usize) -> Option<usize> {
    let mut tbl = TASK_TABLE.lock();
    let slot = tbl.find_free_slot()?;
    let tid = tbl.alloc_tid();

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

pub fn task_exit(slot: usize, pmm: &PhysicalMemoryManager) {
    let mut tbl = TASK_TABLE.lock();
    let t = &mut tbl.tasks[slot];
    t.state = TaskState::Finished;
    if t.stack_size != 0 {
        crate::task::free_stack(pmm, t.stack_base, t.stack_size);
    }
}

pub fn schedule_tick() {
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
        None => return,
    };

    let mut table = TASK_TABLE.lock();
    let mut current_guard = CURRENT.lock();

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

pub fn check_and_schedule() {
    schedule_tick();
}

pub fn tick_maintenance() {
    crate::process::wake_sleepers();
}

pub fn current_index() -> Option<usize> {
    *CURRENT.lock()
}