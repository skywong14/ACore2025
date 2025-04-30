// os/src/timer.rs

use crate::sbi::{set_timer, get_time_sbi};
use crate::config::CLOCK_FREQ;
const TICKS_PER_SEC: usize = 100; // interrupt frequency
const MICRO_PER_SEC: usize = 1_000_000;

pub fn get_time() -> usize {
    get_time_sbi()
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub fn get_time_us() -> usize {
    get_time() / (CLOCK_FREQ / MICRO_PER_SEC)
}