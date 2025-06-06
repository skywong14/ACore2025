#![no_std]
#![feature(linkage)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

use core::ptr::addr_of_mut;
// we need to use String, thus support alloc
use buddy_system_allocator::LockedHeap;

const USER_HEAP_SIZE: usize = 16384;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    // init HEAP
    unsafe {
        HEAP.lock()
            .init(addr_of_mut!(HEAP_SPACE) as usize, USER_HEAP_SIZE);
    }
    exit(main());
    panic!("unreachable after sys_exit!");
}

#[linkage = "weak"] // if other module defines main, use that one
#[unsafe(no_mangle)]
fn main() -> i32 {
    panic!("Cannot find main!");
}

use syscall::*;

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_() -> isize { sys_yield() }

pub fn get_time() -> usize { sys_get_time() }

pub fn sbrk(size: i32) -> isize { sys_sbrk(size) }

pub fn getpid() -> isize { sys_getpid() }

pub fn fork() -> isize { sys_fork() }

pub fn exec(path: &str) -> isize { sys_exec(path) }

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => { yield_(); } // Exist child processes, but no child processes have exited yet
            exit_pid => return exit_pid, // -1 (no child processes) or a real pid
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => { yield_(); }  // Exist child processes, but no child processes have exited yet
            exit_pid => return exit_pid, // -1 (no child processes) or a real pid
        }
    }
}

pub fn sleep(period_ms: usize) {
    let start = sys_get_time();
    while sys_get_time() < start + period_ms {
        sys_yield();
    }
}
