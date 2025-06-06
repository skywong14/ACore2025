// os/src/task/processor.rs

use alloc::sync::Arc;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use crate::task::task::{TaskControlBlock, TaskStatus};
use crate::task::{fetch_task, TaskContext};
use crate::task::switch::__switch;
use crate::trap::TrapContext;

pub struct Processor {
    current: Option<Arc<TaskControlBlock>>, // the currently running task
    idle_task_ctx: TaskContext,             // for thread switching
}

impl Processor {
    // ----- constructor -----
    pub fn new_empty() -> Self {
        Self {
            current: None,
            idle_task_ctx: TaskContext::zero_init(),
        }
    }
    // ----- methods -----
    // 取出当前正在执行的任务
    // take: 如果有正在执行的任务，则将其从当前任务中取出并返回，同时将当前任务置为 None
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    // 当前正在执行的任务，返回一个克隆的 Arc 引用
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        if let Some(ref task) = self.current {
            Some(Arc::clone(task))
        } else {
            None
        }
    }
    // get mutable ref
    fn get_idle_task_ctx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_ctx as *mut TaskContext
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe {
        UPSafeCell::new(Processor::new_empty())
    };
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}
pub fn current_user_satp() -> usize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    let token = task.inner_exclusive_access().get_user_satp();
    token
}
pub fn current_trap_ctx() -> &'static mut TrapContext {
    PROCESSOR.exclusive_access().current()
        .unwrap().inner_exclusive_access().get_trap_ctx()
}

pub fn schedule(switched_task_ctx_ptr: *mut TaskContext) {
    // 让出当前任务的上下文
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_ctx_ptr = processor.get_idle_task_ctx_ptr();
    drop(processor);
    unsafe {
        __switch(
            switched_task_ctx_ptr,
            idle_task_ctx_ptr,
        );
    }
}

// idle 控制流
// 循环调用 fetch_task 直到顺利取出一个任务，随后通过 __switch 来执行
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            // 成功取出一个任务
            let idle_task_ctx_ptr = processor.get_idle_task_ctx_ptr();

            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_ctx_ptr = &task_inner.task_ctx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;

            // 让出 TCB Inner 独占访问权
            drop(task_inner);
            processor.current = Some(task);

            println_yellow!("[kernel] switching to task, pid = {:?}", processor.current.as_ref().unwrap().pid.0);

            drop(processor); // 让出 PROCESSOR 独占访问权
            
            // 切换到新任务
            unsafe {
                __switch(
                    idle_task_ctx_ptr,
                    next_task_ctx_ptr,
                );
            }
        }
    }
}