// os/src/syscall/syscall.rs

use crate::batch::run_next_app;

// SYSCALL_EXIT 93;
pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    // syscall(SYSCALL_EXIT, [xstate as usize, 0, 0])
    run_next_app();
}