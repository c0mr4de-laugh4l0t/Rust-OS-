use crate::task::Task;
use crate::context::context_switch;
use alloc::vec::Vec;
use spin::Mutex;

lazy_static::lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

pub struct Scheduler {
    tasks: Vec<Task>,
    current: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current: 0,
        }
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn schedule(&mut self) {
        if self.tasks.is_empty() {
            return;
        }

        let prev = self.current;
        self.current = (self.current + 1) % self.tasks.len();

        let prev_task = &mut self.tasks[prev];
        let next_task = &self.tasks[self.current];

        unsafe {
            context_switch(&mut prev_task.stack_pointer, next_task.stack_pointer);
        }
    }

    pub fn current_task(&self) -> Option<&Task> {
        self.tasks.get(self.current)
    }
}