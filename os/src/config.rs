// os/src/config.rs

// task
pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
pub const MAX_APP_NUM: usize = 4;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;

// trampoline
pub const TRAMPOLINE_START_ADDRESS: usize = usize::MAX - PAGE_SIZE + 1;

// timer
pub const CLOCK_FREQ: usize = 12500000; // 125MHz

// memory
pub const MEMORY_END: usize = 0x80800000;

// uart
pub const UART0_BASE_ADDR: usize = 0x10000000;
pub const UART0_SIZE: usize = 0x100;

// mm
pub const PAGE_SIZE_BITS: usize = 12; // 4KB
pub const PAGE_SIZE : usize = 1 << PAGE_SIZE_BITS;
pub const PA_WIDTH: usize = 56;
pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;

pub const VA_WIDTH: usize = 39;