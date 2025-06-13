// os/src/main.rs
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
extern crate alloc;
// extern crate buddy_system_allocator;

#[macro_use]
mod console;

pub mod task;
pub mod syscall;
pub mod trap;

mod uart;
mod lang_items;
mod sbi;
mod sync;
mod timer;
mod config;
mod mm;
mod fs;
mod drivers;

use core::arch::{asm, global_asm};
use riscv::register::{mepc, mideleg, mstatus, pmpaddr0, pmpcfg0, satp, sie, sstatus};
use crate::fs::ROOT_INODE;

global_asm!(include_str!("boot.s"));
// global_asm!(include_str!("link_app.s"));

fn debug_info() {
    unsafe extern "C" {
        fn skernel();
        fn stext();
        fn etext();
        fn sbss();
        fn ebss();
        fn ekernel();
    }
    println!("====== Debug Info ====");
    println!("skernel: {:#x}", skernel as usize);
    println!("stext: {:#x}", stext as usize);
    println!("etext: {:#x}", etext as usize);
    println!("sbss: {:#x}", sbss as usize);
    println!("ebss: {:#x}", ebss as usize);
    println!("ekernel: {:#x}", ekernel as usize);
    println!("=======================");
}

#[unsafe(no_mangle)]
unsafe fn rust_boot() {
    // M mode now
    
    // mstatus.MPP = S-mode (1)
    mstatus::set_mpp(mstatus::MPP::Supervisor);
    
    // mepc = rust_main (S mode)
    mepc::write(rust_main as usize);

    // 关闭分页
    satp::write(0);

    // 设置 PMP 允许全物理访问
    pmpaddr0::write(0x3fffffffffffffusize);
    pmpcfg0::write(0xf);

    // init timer in M mode
    // 需要注意的是，RISCV 中的时钟中断是被 CLINT 硬连线为一个 M-Mode 中断的，并且这个中断不能被委派到 S-Mode
    timer::init_timer();

    // 全委托给 S-mode
    mideleg::set_stimer();
    mideleg::set_sext();
    mideleg::set_ssoft(); // 将软件中断的处理权从 M 模式委托给 S 模式

    // 全委托给 S-mode
    asm!(
    "csrw mideleg, {mideleg}",
    "csrw medeleg, {medeleg}",
    "mret",
    medeleg = in(reg) !0,
    mideleg = in(reg) !0,
    options(noreturn),
    );
}

pub fn list_apps() {
    println!("===== List of Apps =====");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("========================");
}

#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    // S mode now
    unsafe {
        sie::set_sext();
        sie::set_stimer();
        sie::set_ssoft(); // 使能S模式下的软件中断 this is necessary!
    }
    // init bss & uart
    clear_bss();
    uart::init();
    debug_info();
    
    // init heap, frame_allocator, kernel space
    println_green!("[kernel] Hello, Rust kernel!");
    mm::init();
    trap::init();
    list_apps();
    // mm::remap_test();

    task::run_initproc();
    timer::set_first_trigger();
    println!("[kernel] All apps loaded, start running tasks...");
    task::run_tasks();
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