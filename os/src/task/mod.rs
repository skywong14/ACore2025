// os/src/task/mod.rs

mod switch;
mod context;
mod task;

use alloc::vec::Vec;
use lazy_static::lazy_static;
pub use context::TaskContext;
use task::TaskControlBlock;
use crate::sync::UPSafeCell;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

use crate::loader::get_app_data;
use crate::task::task::TaskStatus;

use crate::loader::get_num_app;

fn init_task_manager() -> TaskManager {
    let num_app = get_num_app();
    println!("init TASK_MANAGER, num_app = {}", num_app);

    let mut tasks: Vec<TaskControlBlock> = Vec::new();
    for i in 0..num_app {
        tasks.push(TaskControlBlock::from_elf(get_app_data(i),i));
    }
    println!("finish init TASK_MANAGER");
    TaskManager {
        num_app,
        inner: unsafe { UPSafeCell::new(TaskManagerInner {
            tasks,
            current_task: 0,
        })},
    }
}

lazy_static! {
    // TASK_MANAGER is only used in task module
    pub static ref TASK_MANAGER: TaskManager = init_task_manager();
}

// --------------------

use crate::sbi;
use sbi::shutdown;
use crate::task::switch::__switch;
use crate::trap::TrapContext;

impl TaskManager {
    pub fn suspend_current(&self) {
        let mut task_manager = self.inner.exclusive_access();
        let cur : usize = task_manager.current_task;
        task_manager.tasks[cur].task_status = TaskStatus::Ready;
    }

    fn exit_current(&self) {
        let mut task_manager = self.inner.exclusive_access();
        let cur : usize = task_manager.current_task;
        task_manager.tasks[cur].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self, cur: usize) -> Option<usize> {
        let task_manager = self.inner.exclusive_access();
        for i in 1..=self.num_app {
            let id = (i + cur) % self.num_app;
            if task_manager.tasks[id].task_status == TaskStatus::Ready {
                return Some(id);
            }
        }
        None
    }

    fn run_next(&self) {
        println!("[kernel] run next! current time: {}", crate::timer::get_time());
        let cur : usize = self.inner.exclusive_access().current_task;
        if let Some(nxt) = self.find_next_task(cur) {
            let mut task_manager = self.inner.exclusive_access();
            task_manager.current_task = nxt;
            task_manager.tasks[nxt].task_status = TaskStatus::Running;
            // get two ptrs, switch to next task
            let cur_task_ctx_ptr = &mut task_manager.tasks[cur].task_ctx as *mut TaskContext;
            let nxt_task_ctx_ptr = &mut task_manager.tasks[nxt].task_ctx as *mut TaskContext;
            drop(task_manager);
            unsafe {
                __switch(cur_task_ctx_ptr, nxt_task_ctx_ptr);
            }
            // already switch to next task, running (U mode)
        } else {
            println!("===== Finish time: {} =====", crate::timer::get_time());
            println!("All applications completed! Shutting down...");
            shutdown(false);
        }
    }

    fn run_first_task(&self) -> ! {
        println!("===== start first task! =====");
        let mut task_manager = self.inner.exclusive_access();
        task_manager.tasks[0].task_status = TaskStatus::Running;
        let cur_task_ctx_ptr = &mut TaskContext::zero_init() as *mut TaskContext;
        let nxt_task_ctx_ptr = &mut task_manager.tasks[0].task_ctx as *mut TaskContext;
        drop(task_manager);
        unsafe {
            __switch(cur_task_ctx_ptr, nxt_task_ctx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    fn get_current_satp(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].to_satp()
    }

    fn get_current_trap_ctx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_ctx()
    }

    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }
}


pub fn suspend_current_and_run_next() {
    TASK_MANAGER.suspend_current();
    TASK_MANAGER.run_next();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.exit_current();
    TASK_MANAGER.run_next();
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn current_user_satp() -> usize {
    TASK_MANAGER.get_current_satp()
}

pub fn current_trap_ctx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_ctx()
}

pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}