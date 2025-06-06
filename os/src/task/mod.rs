// os/src/task/mod.rs

mod switch;
mod context;
mod task;
mod pid;
pub(crate) mod task_manager;
pub(crate) mod processor;
pub(crate) use processor::run_tasks;

use alloc::sync::Arc;
use lazy_static::lazy_static;
pub use context::TaskContext;
use crate::loader::get_app_data_by_name;
use crate::task::processor::{schedule, take_current_task};
use crate::task::task::{TaskControlBlock, TaskStatus};
use crate::task::task_manager::{add_task, fetch_task};

// ----- INIT_PORC -----
// 创建一个子进程来运行 user_shell
// 作为所有进程的祖先，负责回收成为孤儿的僵尸进程
// 永不退出，持续监控系统中的进程状态
lazy_static! {
    // the init process
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new_from_elf(
        get_app_data_by_name("initproc").unwrap()
    ));
}

// ----- Task Control Flow -----
pub fn suspend_current_and_run_next() {
    let task = take_current_task().unwrap();
    
    let mut task_inner = task.inner_exclusive_access();
    let task_ctx_ptr = &mut task_inner.task_ctx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    
    add_task(task); // push task back to ready queue.
    
    schedule(task_ctx_ptr); // jump to scheduling cycle, schedule is a __switch
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();

    let mut inner = task.inner_exclusive_access();

    // change status to Zombie
    // 等待父进程获取其退出状态
    inner.task_status = TaskStatus::Zombie;

    // 记录退出码
    inner.exit_code = exit_code;

    // 将子进程的父进程设置为初始进程 initproc

    // 获取对初始进程的独占访问
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();

        // 将当前任务的所有子任务移交给初始进程
        for child in inner.children.iter() {
            // 更改每个子任务的父任务引用为初始进程
            // Arc::downgrade创建弱引用以避免循环引用导致的内存泄漏
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));

            // 将子任务添加到初始进程的子任务列表中
            initproc_inner.children.push(child.clone());
        }
    }
    // 自动释放对初始进程的独占访问

    // 清空当前任务的子任务列表，因为它们已被移交给初始进程
    inner.children.clear();

    // 释放用户空间内存
    // 这一步很重要，它回收了进程使用的所有物理页面，
    // 但保留内核栈和任务控制结构，以便父进程可以获取退出状态
    inner.memory_set.recycle_data_pages();

    // 释放对当前任务内部数据的独占访问
    drop(inner);

    // 手动释放task的引用计数，确保资源正确管理
    // 由于任务可能被多处引用，这确保引用计数正确维护
    drop(task);

    // 创建一个未使用的任务上下文
    // 我们不需要保存当前任务的上下文，因为它不会再次运行
    let mut _unused = TaskContext::zero_init();

    // 调用调度器切换到下一个任务
    // 传入未使用的上下文的指针，因为当前任务已不再需要保存上下文
    schedule(&mut _unused as *mut _);
}

pub fn run_initproc() {
    println!("===== initing initproc =====");
    let initproc = INITPROC.clone();
    println!("===== adding initproc =====");
    add_task(initproc);
}

// todo
// pub fn change_program_brk(size: i32) -> Option<usize> {
//     TASK_MANAGER.change_current_program_brk(size)
// }