// os/src/trap/mod.rs

mod context;

use crate::syscall::syscall;
use core::arch::{asm, global_asm};
use riscv::register::{mtvec::TrapMode, scause::{self, Exception, Trap, Interrupt}, sip, stval, stvec};
use crate::config::{TRAMPOLINE_START_ADDRESS, TRAP_CONTEXT_ADDRESS};
use crate::task::{current_trap_ctx, current_user_satp, exit_current_and_run_next, suspend_current_and_run_next};
pub(crate) use crate::trap::context::TrapContext;

global_asm!(include_str!("trap.s"));

pub fn init() {
    set_kernel_trap_entry();
}

fn set_user_trap_entry() {

    unsafe extern "C" {
        fn strampoline();
    }
    unsafe {
        stvec::write(TRAMPOLINE_START_ADDRESS, TrapMode::Direct);
    }
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

// Unimplement: traps/interrupts/exceptions from kernel mode
// todo: Chapter 9: I/O device
#[unsafe(no_mangle)]
pub fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

#[unsafe(no_mangle)]
pub fn trap_handler() -> ! {
    let scause = scause::read();
    let stval = stval::read();
    let ctx = current_trap_ctx();
    // println!("[trap_handler] scause = {:?}, stval = {:#x}, sepc = {:#x}", scause.bits(), stval, ctx.sepc);
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            println!("[timer] ssoft(Timer Interrupt), time:{}", crate::timer::get_time());
            let sip = sip::read().bits();
            unsafe {
                asm! {"csrw sip, {sip}", sip = in(reg) sip ^ 2};
            }
            // next time interrupt already set in "m_trap_entry"
            suspend_current_and_run_next();
        }

        Trap::Exception(Exception::UserEnvCall) => {
            // println!("[kernel] UserEnvCall");
            ctx.sepc += 4; // skip ecall instruction
            // a7 | a0, a1, a2
            ctx.x[10] = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]) as usize;
        }

        Trap::Exception(Exception::LoadFault) |
        Trap::Exception(Exception::LoadPageFault) |
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) => {
            println!(
                "[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                stval, ctx.sepc
            );
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction");
        }
        _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    // println!("[kernel] return from trap_handler");
    trap_return();
}

#[unsafe(no_mangle)]
pub fn trap_return() -> ! {
    // 设置用户态trap的入口地址
    set_user_trap_entry();

    // TrapContext 的虚拟地址
    let trap_ctx_ptr = TRAP_CONTEXT_ADDRESS;

    // 取出当前用户地址空间的 satp
    let user_satp = current_user_satp();

    unsafe extern "C" {
        fn __alltraps();
        fn __restore();
    }

    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE_START_ADDRESS;

    // 设置寄存器 a0 为 trap_ctx_ptr, a1 为用户页表的 satp
    // 跳转到 __restore 汇编函数的新地址
    unsafe {
        asm!(
        "fence.i",
        "jr {restore_va}",                    // jump to __restore
        restore_va = in(reg) restore_va,
        in("a0") trap_ctx_ptr,                // a0: TrapContext pointer
        in("a1") user_satp,                   // a1: user's satp
        options(noreturn)                     // 不再返回
        );
    }
    unreachable!("unreachable in trap_return!");
}