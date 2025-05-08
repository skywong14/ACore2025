// os/src/syscall/syscall.rs

// use crate::batch::run_next_app;

use crate::task::exit_current_and_run_next;
use crate::task::suspend_current_and_run_next;

// SYSCALL_EXIT 93;
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    println!("[kernel] Current time: {}", crate::timer::get_time());
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    println!("[kernel] Application yielded");
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    crate::timer::get_time() as isize
}