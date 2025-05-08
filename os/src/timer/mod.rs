// os/src/timer.rs

use core::arch::global_asm;
use crate::sbi::{set_timer, get_time_sbi};
use crate::config::CLOCK_FREQ;
use riscv::register::{mie, mscratch, mstatus, mtvec};

const TICKS_PER_SEC: usize = 500; // interrupt frequency
const TIME_INTERVAL: usize = CLOCK_FREQ / TICKS_PER_SEC; // timer interval in seconds
const MICRO_PER_SEC: usize = 1_000_000;

global_asm!(include_str!("m_trap.s"));

#[unsafe(link_section = ".bss.stack")]
#[unsafe(no_mangle)]
pub static mut TIEMR_SCRATCH: [usize; 5] = [0; 5];

pub fn get_time() -> usize {
    get_time_sbi()
}

pub fn set_next_trigger() {
    println!("[kernel] Set next timer interrupt, time: {}, nxt_time: {}", get_time_sbi(), get_time_sbi() + TIME_INTERVAL);
    set_timer(get_time_sbi() + TIME_INTERVAL);
}

pub fn get_time_us() -> usize {
    get_time_sbi() / (CLOCK_FREQ / MICRO_PER_SEC)
}

pub unsafe fn init_timer() {
    let mscratch_ptr = unsafe { core::ptr::addr_of!(TIEMR_SCRATCH) as usize };
    mscratch::write(mscratch_ptr);
    //println!("TIMER_SCRATCH at {:x}", mscratch_ptr);
    
    // set the machine-mode trap handler
    unsafe extern "C" {
        fn m_trap_entry();
    }

    TIEMR_SCRATCH[3] = 0x2000000 + 0x4000;
    TIEMR_SCRATCH[4] = TIME_INTERVAL;

    mtvec::write(m_trap_entry as usize, mtvec::TrapMode::Direct);

    // enable machine-mode interrupts
    mstatus::set_mie();

    // enable machine-mode timer interrupts
    mie::set_mtimer();

    println!("[kernel] timer initialized");
}

// use "p ($mstatus >> 11) & 0x3" to check MPP