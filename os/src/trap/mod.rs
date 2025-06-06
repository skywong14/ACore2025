// os/src/trap/mod.rs

mod context;

use crate::syscall::syscall;
use core::arch::{asm, global_asm};
use crate::config::{TRAMPOLINE_START_ADDRESS, TRAP_CONTEXT_ADDRESS};
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use riscv::register::{mtvec::TrapMode, scause::{self, Exception, Trap, Interrupt}, sip, stval, stvec};
use crate::task::processor::{current_trap_ctx, current_user_satp};
pub(crate) use crate::trap::context::TrapContext;
use crate::uart::read_spp;

global_asm!(include_str!("trap.s"));

pub fn init() {
    set_kernel_trap_entry();
}

fn set_user_trap_entry() {
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
    // Question: SSoftInterrupt 不会影响吗
    println_red!("a trap from kernel!, time: {}", crate::timer::get_time());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}

#[unsafe(no_mangle)]
pub fn trap_handler() -> ! {
    // trap_handler 只会处理来自用户态的 trap
    let scause = scause::read();
    let stval = stval::read();
    let mut ctx = current_trap_ctx();
    // println!("[trap_handler] scause = {:?}, stval = {:#x}, sepc = {:#x}", scause.bits(), stval, ctx.sepc);
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            println_gray!("[timer] ssoft(Timer Interrupt), time:{}", crate::timer::get_time());
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
            let a0 = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]) as usize;
            // syscall might be 'sys_exec', we need to update the trap context
            ctx = current_trap_ctx();
            ctx.x[10] = a0 as usize;
        }

        Trap::Exception(Exception::LoadFault) |
        Trap::Exception(Exception::LoadPageFault) |
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) => {
            println_red!(
                "[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                stval, ctx.sepc
            );
            exit_current_and_run_next(-2); // page fault exit code: -2
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println_red!("[kernel] IllegalInstruction");
            exit_current_and_run_next(-3); // illegal instruction exit code: -3
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