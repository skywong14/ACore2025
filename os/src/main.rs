// os/src/main.rs
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate bitflags;
extern crate alloc;

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
mod mm;

use core::arch::{asm, global_asm};
use riscv::register::{mepc, mideleg, mstatus, pmpaddr0, pmpcfg0, satp, sie};

global_asm!(include_str!("boot.s"));
global_asm!(include_str!("link_app.s"));

fn debug_info() {
    unsafe extern "C" {
        fn skernel();
        fn stext();
        fn etext();
        fn sbss();
        fn ebss();
        fn ekernel();
        fn app_0_start();
    }
    println!("====== Debug Info ====");
    println!("skernel: {:#x}", skernel as usize);
    println!("stext: {:#x}", stext as usize);
    println!("etext: {:#x}", etext as usize);
    println!("sbss: {:#x}", sbss as usize);
    println!("ebss: {:#x}", ebss as usize);
    println!("ekernel: {:#x}", ekernel as usize);
    println!("app_0_start: {:#x}", app_0_start as usize);
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
    timer::init_timer();

    // 全委托给 S-mode
    mideleg::set_stimer();
    mideleg::set_sext();
    mideleg::set_ssoft();

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
    println!("[kernel] Hello, world!");
    mm::init();
    // mm::remap_test();
    
    trap::init();
    timer::set_first_trigger();
    println!("===== init task manager =====");
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