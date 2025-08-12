// process.rs
#![no_std]

use core::sync::atomic::{AtomicU32, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::memory::FRAME_SIZE;
use crate::scheduler; // we use scheduler::spawn/task_yield/task_exit
use crate::memory::PhysicalMemoryManager;

pub type Pid = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    Runnable,
    Running,
    Sleeping(u64), // wakeup tick
    Zombie,
}

/// Small PCB entry that maps a scheduler task-slot to a PID and holds meta
#[derive(Clone, Copy)]
pub struct Process {
    pub pid: Pid,
    pub slot: usize, // scheduler/task table slot index
    pub state: ProcState,
    pub stack_base: usize,
    pub stack_size: usize,
    pub parent: Option<Pid>,
    pub name: [u8; 16],
}

impl Process {
    pub const fn empty() -> Self {
        Self {
            pid: 0,
            slot: 0,
            state: ProcState::Zombie,
            stack_base: 0,
            stack_size: 0,
            parent: None,
            name: [0u8; 16],
        }
    }
}

pub struct ProcessTable {
    pub procs: [Process; ProcessTable::MAX_PROCS],
    pub next_pid: AtomicU32,
}

impl ProcessTable {
    pub const MAX_PROCS: usize = 64;

    pub const fn new() -> Self {
        Self {
            procs: [Process::empty(); ProcessTable::MAX_PROCS],
            next_pid: AtomicU32::new(1),
        }
    }

    fn alloc_slot(&mut self) -> Option<usize> {
        for i in 0..Self::MAX_PROCS {
            if self.procs[i].state == ProcState::Zombie {
                return Some(i);
            }
        }
        None
    }

    fn alloc_pid(&self) -> Pid {
        self.next_pid.fetch_add(1, Ordering::SeqCst)
    }
}

lazy_static! {
    pub static ref PROC_TABLE: Mutex<ProcessTable> = Mutex::new(ProcessTable::new());
}

/// Spawn a new process (kernel-thread) using scheduler::spawn.
/// Returns Some(pid) on success.
///
/// entry: extern "C" fn() entry point (must be 'static)
/// pages: number of stack pages to allocate (passed to scheduler spawn)
pub fn spawn(entry: extern "C" fn(), pages: usize, parent: Option<Pid>) -> Option<Pid> {
    // call scheduler to create a task and allocate stack
    // we assume scheduler::spawn returns Some(slot_index)
    if let Some(slot) = scheduler::spawn(entry, unsafe { &crate::main::PMM }, pages) {
        let mut table = PROC_TABLE.lock();
        if let Some(pt_slot) = table.alloc_slot() {
            let pid = table.alloc_pid();
            // retrieve stack info from scheduler's task table if available.
            // For simplicity we set stack_base/size to 0 (not strictly required).
            table.procs[pt_slot] = Process {
                pid,
                slot,
                state: ProcState::Runnable,
                stack_base: 0,
                stack_size: pages * FRAME_SIZE,
                parent,
                name: [0u8; 16],
            };
            return Some(pid);
        } else {
            // no process slot available — cleanup: ask scheduler to remove task
            // we don't have a scheduler.remove; for now mark task finished via scheduler::task_exit
            scheduler::task_exit(slot, unsafe { &crate::main::PMM });
            return None;
        }
    }
    None
}

/// Get process PID from current scheduler slot, if any
pub fn current_pid() -> Option<Pid> {
    if let Some(cur_slot) = scheduler::current_index() {
        let table = PROC_TABLE.lock();
        for p in table.procs.iter() {
            if p.slot == cur_slot && p.state != ProcState::Zombie {
                return Some(p.pid);
            }
        }
    }
    None
}

/// Mark the process (by pid) as exited and free resources.
/// Returns true if found and cleaned.
pub fn exit_self(pid: Pid) -> bool {
    let mut table = PROC_TABLE.lock();
    for i in 0..ProcessTable::MAX_PROCS {
        if table.procs[i].pid == pid {
            let slot = table.procs[i].slot;
            table.procs[i].state = ProcState::Zombie;
            // free scheduler task/stack
            scheduler::task_exit(slot, unsafe { &crate::main::PMM });
            return true;
        }
    }
    false
}

/// Find pid -> slot mapping (helper)
pub fn pid_to_slot(pid: Pid) -> Option<usize> {
    let table = PROC_TABLE.lock();
    for p in table.procs.iter() {
        if p.pid == pid && p.state != ProcState::Zombie {
            return Some(p.slot);
        }
    }
    None
}

/// Sleep current process for ms milliseconds (cooperative).
/// Uses PIT tick counter as time base (ms tick increment not implemented here).
pub fn sleep_ms(pid: Pid, ms: u64) {
    // for simplicity, we use tick count from pit::ticks() where 1 tick ~ 1 ms is not guaranteed.
    let wake = crate::pit::ticks().saturating_add(ms);
    let mut table = PROC_TABLE.lock();
    for i in 0..ProcessTable::MAX_PROCS {
        if table.procs[i].pid == pid {
            table.procs[i].state = ProcState::Sleeping(wake);
            break;
        }
    }
}

/// Called from scheduler tick to wake sleeping processes if their wake time arrived.
pub fn wake_sleepers() {
    let ticks = crate::pit::ticks();
    let mut table = PROC_TABLE.lock();
    for i in 0..ProcessTable::MAX_PROCS {
        if let ProcState::Sleeping(until) = table.procs[i].state {
            if ticks >= until {
                table.procs[i].state = ProcState::Runnable;
            }
        }
    }
}