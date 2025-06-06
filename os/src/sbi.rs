// os/src/sbi.rs

// use crate::uart::{get_time_uart, set_timer_uart};

use crate::config::TEST_DEVICE_ADDR;

// console_putchar
pub fn console_putchar(c: usize) {
    crate::uart::putchar(c);
}

// console_getchar
pub fn console_getchar() -> usize {
    crate::uart::getchar() as usize
}

// shutdown
pub fn shutdown(failure: bool) -> ! {
    // QEMU关机魔数
    const SHUTDOWN_CODE: u32 = 0x5555;  // 正常关机
    const FAILURE_CODE: u32 = 0x3333;   // 错误关机

    unsafe {
        if !failure {
            // 正常关机
            core::ptr::write_volatile(TEST_DEVICE_ADDR as *mut u32, SHUTDOWN_CODE);
        } else {
            // 错误关机
            core::ptr::write_volatile(TEST_DEVICE_ADDR as *mut u32, FAILURE_CODE);
        }
    }

    loop {}
}

// set timer
// pub fn set_timer(timer: usize) {
//     set_timer_uart(timer);
// }

// get timer
// pub fn get_time_sbi() -> usize {
//     get_time_uart()
// }