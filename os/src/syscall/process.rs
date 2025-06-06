// os/src/syscall/syscall.rs

use alloc::sync::Arc;
use crate::loader::get_app_data_by_name;
use crate::mm::address::VirAddr;
use crate::mm::page_table::{translated_refmut, translated_str};
// use crate::task::{change_program_brk, current_user_satp};
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::task::processor::{current_task, current_user_satp};
use crate::task::task_manager::add_task;

// SYSCALL_EXIT 93;
pub fn sys_exit(exit_code: i32) -> ! {
    if exit_code != 0 {
        println_red!("[kernel] Application exited with code {}", exit_code);
        println_red!("[kernel] Current time: {}", crate::timer::get_time());
    } else {
        println_green!("[kernel] Application exited with code 0");
        println_green!("[kernel] Current time: {}", crate::timer::get_time());
    }
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    println_yellow!("[kernel] Application yielded");
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
    crate::timer::get_time() as isize
}

pub fn sys_sbrk(size: i32) -> isize {
    -1
    // todo
    // if let Some(old_brk) = change_program_brk(size) {
    //     old_brk as isize
    // } else {
    //     -1
    // }
}

pub fn sys_getpid() -> isize {
    let current_task = current_task().unwrap();
    current_task.pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork(); // TCB
    
    let new_pid = new_task.pid.0; // return to parent process
    
    let trap_ctx = new_task.inner_exclusive_access().get_trap_ctx();
    trap_ctx.x[10] = 0; //x[10]: a0, for child process, fork returns 0
    
    add_task(new_task); // add new task to scheduler
    new_pid as isize
    // then trap_return
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_satp();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        // elf data in `data`
        current_task().unwrap().exec(data);
        0
    } else {
        -1
    }
}

// 回收子进程的资源, 并将 exit_code 写入 exit_code_ptr
// 如果没有符合 pid 的子进程，返回 -1
// 如果子进程仍在运行，返回 -2
// 这是一个立即返回的系统调用，用户库中的 wait_pid 会在返回 -2 时调用 yield_ 来实现阻塞，并在最外层使用 loop 直至子进程退出
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();

    // ---- access current TCB exclusively
    let mut inner = task.inner_exclusive_access();

    // 检查是否有满足条件的子进程
    let mut has_child = false;
    let len = inner.children.len();
    for i in 0..len {
        if pid == -1 || pid as usize == inner.children[i].get_pid() {
            has_child = true;
            break;
        }
    }

    if !has_child {
        return -1;
    }

    // 寻找已经结束的子进程
    let mut found_idx = None;
    for i in 0..inner.children.len() {
        let is_match = {
            // ++++ temporarily access child PCB exclusively
            let child_inner = inner.children[i].inner_exclusive_access();
            let is_zombie = child_inner.is_zombie();
            let pid_match = pid == -1 || pid as usize == inner.children[i].get_pid();
            is_zombie && pid_match
            // ++++ stop exclusively accessing child PCB
        };

        if is_match {
            found_idx = Some(i);
            break;
        }
    }

    if let Some(idx) = found_idx {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.get_pid();

        // ++++ temporarily access child TCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ stop exclusively accessing child PCB

        *translated_refmut(inner.memory_set.to_satp(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- stop exclusively accessing current PCB automatically
}