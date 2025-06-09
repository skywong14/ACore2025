// os/src/timer.rs

use core::arch::global_asm;
use core::ptr::{read_volatile, write_volatile};
use crate::config::{CLINT_MTIME, CLINT_MTIMECMP, CLOCK_FREQ};
use riscv::register::{mie, mscratch, mstatus, mtvec};

const TICKS_PER_SEC: usize = 500; // interrupt frequency
const TIME_INTERVAL: usize = CLOCK_FREQ / TICKS_PER_SEC; // timer interval in seconds
const MICRO_PER_SEC: usize = 1_000_000;

global_asm!(include_str!("m_trap.s"));

#[unsafe(link_section = ".bss.stack")]
#[unsafe(no_mangle)]
pub static mut TIMER_SCRATCH: [usize; 5] = [0; 5];

pub fn get_time() -> usize {
    unsafe {
        let mtime = CLINT_MTIME as *const u32;
        let high = read_volatile(mtime.add(1)) as usize;
        let low = read_volatile(mtime) as usize;
        (high << 32) | low
    }
}

pub fn set_first_trigger() {
    let cur_time = get_time();
    let next_time = cur_time + TIME_INTERVAL;
    println_gray!("[kernel] Set first timer interrupt, time: {}, nxt_time: {}", cur_time, next_time);
    set_timer(get_time() + TIME_INTERVAL);
}

pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / MICRO_PER_SEC)
}


pub fn set_timer(time: usize) {
    unsafe {
        let mtimecmp = CLINT_MTIMECMP as *mut u32;
        write_volatile(mtimecmp.add(1), 0xFFFF_FFFF);
        write_volatile(mtimecmp, 0xFFFF_FFFF);
        write_volatile(mtimecmp.add(1), (time >> 32) as u32);
        write_volatile(mtimecmp, time as u32);
    }
}

pub unsafe fn init_timer() {
    let mscratch_ptr = unsafe { core::ptr::addr_of!(TIMER_SCRATCH) as usize };
    mscratch::write(mscratch_ptr);
    
    // set the machine-mode trap handler
    unsafe extern "C" {
        fn m_trap_entry();
    }

    TIMER_SCRATCH[3] = CLINT_MTIMECMP;
    TIMER_SCRATCH[4] = TIME_INTERVAL;

    mtvec::write(m_trap_entry as usize, mtvec::TrapMode::Direct);

    // enable machine-mode interrupts
    mstatus::set_mie();

    // enable machine-mode timer interrupts
    mie::set_mtimer();

    // setup timer
    // set_timer(get_time() + TIME_INTERVAL); // not here
}