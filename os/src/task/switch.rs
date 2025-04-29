// os/src/task/switch.rs

use super::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.s"));

unsafe extern "C" {
    pub fn __switch(
        current_task_cx_ptr: *mut TaskContext,
        next_task_cx_ptr: *const TaskContext,
    );
}
