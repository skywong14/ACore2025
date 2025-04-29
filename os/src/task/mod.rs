// os/src/task/mod.rs

mod switch;
mod context;
mod task;

use lazy_static::lazy_static;
pub use context::TaskContext;
use task::TaskControlBlock;
use crate::config::MAX_APP_NUM;
use crate::sync::UPSafeCell;

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

use crate::loader::LOADER_MANAGER;
use crate::task::task::TaskStatus;
use crate::loader::init_app_ctx;

fn init_task_manager() -> TaskManager {
    let num_app = LOADER_MANAGER.exclusive_access().num_app;
    let mut tasks = [
        TaskControlBlock {
            task_ctx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit
        };
        MAX_APP_NUM
    ];
    for i in 0..num_app {
        tasks[i].task_ctx = TaskContext::goto_restore(init_app_ctx(i)); // sp at init_app_ctx, ra at __restore, s-regs zero
        tasks[i].task_status = TaskStatus::Ready;
    }
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
            println!("All applications completed!");
            shutdown(false);
        }
    }

    fn run_first_task(&self) -> ! {
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