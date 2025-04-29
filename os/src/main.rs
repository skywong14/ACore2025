// os/src/main.rs
#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod console;

pub mod task;
pub mod loader;
pub mod syscall;
pub mod trap;

mod uart;
mod lang_items;
mod sbi;
mod sync;
mod timer;
mod config;

use core::arch::global_asm;

// global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("boot.s"));
global_asm!(include_str!("link_app.s"));


#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    uart::init();
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    loader::load_apps();
    task::run_first_task();
    panic!("Unreachable in rust_main!");
}

fn clear_bss() {
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}