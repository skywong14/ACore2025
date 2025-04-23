// os/src/main.rs
#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod console;

mod uart;
mod lang_items;
mod sbi;
mod batch;
mod sync;
mod syscall;

use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));



#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();
    println!("Hello, world!");
    panic!("Shutdown machine!");
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