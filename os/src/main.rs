// os/src/main.rs
#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod console;

pub mod batch;
pub mod syscall;
pub mod trap;

mod uart;
mod lang_items;
mod sbi;
mod sync;

use core::arch::global_asm;

// global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("boot.s"));
global_asm!(include_str!("link_app.s"));


#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    uart::init();
    unsafe extern "C" {
        fn stext(); // begin addr of text segment
        fn etext(); // end addr of text segment
        fn srodata(); // start addr of Read-Only data segment
        fn erodata(); // end addr of Read-Only data ssegment
        fn sdata(); // start addr of data segment
        fn edata(); // end addr of data segment
        fn sbss(); // start addr of BSS segment
        fn ebss(); // end addr of BSS segment
        fn boot_stack_lower_bound(); // stack lower bound
        fn boot_stack_top(); // stack top
    }
    clear_bss();
    println!("[kernel] Hello, world!");
    trap::init();
    batch::init();
    batch::run_next_app();
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