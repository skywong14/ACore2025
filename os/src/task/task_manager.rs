use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use crate::task::task::TaskControlBlock;

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

// A simple FIFO scheduler.
impl TaskManager {
    // ----- constructor -----
    pub fn new() -> Self {
        Self { ready_queue: VecDeque::new(), }
    }
    // ----- methods -----
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    // 现在 TASK_MANAGER 的功能仅限于管理就绪队列，执行和切换全部交给 Processor 来完成
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = unsafe {
        UPSafeCell::new(TaskManager::new())
    };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

// --------------------
//     pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
//         let mut inner = self.inner.exclusive_access();
//         let cur = inner.current_task;
//         inner.tasks[cur].change_program_brk(size)
//     }
// }