#![no_std]

use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

pub type Pid = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    Runnable,
    Running,
    Sleeping(u64),
    Zombie,
    Finished,
}

#[derive(Clone, Copy)]
pub struct Process {
    pub pid: Pid,
    pub slot: usize,
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
            if self.procs[i].state == ProcState::Zombie || self.procs[i].state == ProcState::Finished {
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

pub fn spawn(entry: extern "C" fn(), pages: usize, parent: Option<Pid>) -> Option<Pid> {
    if let Some(slot_idx) = crate::scheduler::spawn(entry, unsafe { &crate::PMM }, pages) {
        let mut table = PROC_TABLE.lock();
        if let Some(pt_slot) = table.alloc_slot() {
            let pid = table.alloc_pid();
            table.procs[pt_slot] = Process {
                pid,
                slot: slot_idx,
                state: ProcState::Runnable,
                stack_base: 0,
                stack_size: pages * crate::memory::FRAME_SIZE,
                parent,
                name: [0u8; 16],
            };
            return Some(pid);
        } else {
            crate::scheduler::task_exit(slot_idx, unsafe { &crate::PMM });
            return None;
        }
    }
    None
}

pub fn current_pid() -> Option<Pid> {
    if let Some(cur_slot) = crate::scheduler::current_index() {
        let table = PROC_TABLE.lock();
        for p in table.procs.iter() {
            if p.slot == cur_slot && p.state != ProcState::Zombie && p.state != ProcState::Finished {
                return Some(p.pid);
            }
        }
    }
    None
}

pub fn exit_self(pid: Pid) -> bool {
    let mut table = PROC_TABLE.lock();
    for i in 0..ProcessTable::MAX_PROCS {
        if table.procs[i].pid == pid {
            let slot = table.procs[i].slot;
            table.procs[i].state = ProcState::Finished;
            crate::scheduler::task_exit(slot, unsafe { &crate::PMM });
            return true;
        }
    }
    false
}

pub fn sleep_ms(pid: Pid, ms: u64) {
    let wake = crate::pit::ticks().saturating_add(ms);
    let mut table = PROC_TABLE.lock();
    for i in 0..ProcessTable::MAX_PROCS {
        if table.procs[i].pid == pid {
            table.procs[i].state = ProcState::Sleeping(wake);
            break;
        }
    }
}

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