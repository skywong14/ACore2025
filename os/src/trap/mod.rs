// os/src/trap/mod.rs

/*
Exception codes in RISC-V
pub enum Exception {
    InstructionMisaligned,
    InstructionFault,
    IllegalInstruction,
    Breakpoint,
    LoadFault,
    StoreMisaligned,
    StoreFault,
    UserEnvCall,
    VirtualSupervisorEnvCall,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
    InstructionGuestPageFault,
    LoadGuestPageFault,
    VirtualInstruction,
    StoreGuestPageFault,
    Unknown,
}
 */

mod context;

use crate::syscall::syscall;
use core::arch::{asm, global_asm};
use riscv::register::{mtvec::TrapMode, scause::{self, Exception, Trap, Interrupt}, sip, stval, stvec};
use crate::task::suspend_current_and_run_next;
pub(crate) use crate::trap::context::TrapContext;

global_asm!(include_str!("trap.s"));

pub fn init() {
    unsafe extern "C" { fn __alltraps(); }
    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

#[unsafe(no_mangle)]
pub fn trap_handler(ctx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    //println!("[trap_handler] scause = {:?}, stval = {:#x}, sepc = {:#x}", scause.bits(), stval, ctx.sepc);
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            println!("[timer] ssoft(Timer Interrupt), time:{}", crate::timer::get_time());
            let sip = sip::read().bits();
            unsafe {
                asm! {"csrw sip, {sip}", sip = in(reg) sip ^ 2};
            }
            // set_next_trigger(); // next time interrupt already set in "m_trap_entry"
            suspend_current_and_run_next();
        }
        Trap::Exception(Exception::UserEnvCall) => {
            //println!("[kernel] UserEnvCall");
            ctx.sepc += 4; // skip ecall instruction
            // a7 | a0, a1, a2
            ctx.x[10] = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) => {
            println!(
                "[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                stval, ctx.sepc
            );
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction");
        }
        _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", scause.cause(), stval);
        }
    }
    ctx
}
