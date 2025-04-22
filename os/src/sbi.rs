// os/src/sbi.rs

// console_putchar
pub fn console_putchar(c: u8) {
    crate::uart::putchar(c);
}

// shutdown
pub fn shutdown(failure: bool) -> ! {
    // QEMU 测试设备地址(从QEMU内存映射表确认)
    const TEST_DEVICE_ADDR: usize = 0x100000;

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