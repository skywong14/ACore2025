// os/src/uart.rs
use core::ptr::{read_volatile, write_volatile};

// https://github.com/qemu/qemu/blob/7598971167080a8328a1b8e22425839cb4ccf7b7/hw/riscv/virt.c#L97
// UART0 的物理地址
const UART0_BASE_ADDR: usize = 0x10000000;
const UART0_SIZE: usize = 0x100;

// 寄存器偏移量
const RBR: usize = 0;  // 接收缓冲寄存器
const THR: usize = 0;  // 发送保持寄存器
const DLL: usize = 0;  // 除数锁存器 (低)
const DLM: usize = 1;  // 除数锁存器 (高)
const IER: usize = 1;  // 中断使能寄存器
const FCR: usize = 2;  // FIFO 控制寄存器
const LCR: usize = 3;  // 线路控制寄存器
const MCR: usize = 4;  // Modem 控制寄存器
const LSR: usize = 5;  // 线路状态寄存器

// LSR 寄存器状态位
const LSR_RX_READY: u8 = 1 << 0;  // 数据可读
const LSR_TX_IDLE: u8 = 1 << 5;   // 发送器空闲

// init UART
pub fn init() {
    unsafe {
        // 禁用所有中断 避免初始化时意外中断
        write_uart_reg(IER, 0x00);

        // 设置 DLAB 位来访问波特率寄存器
        write_uart_reg(LCR, 0x80);

        // 设置波特率 (38.4K)
        // 波特率 = 时钟频率[22.729 MHz in QEMU] / (分频值 × 16)
        // 分频值 = 22729000 / (16 * 38400) ≈ 37
        write_uart_reg(DLL, 0x25);  // low byte of 37
        write_uart_reg(DLM, 0x00);  // high byte of 37

        // 8位数据，无奇偶校验，1位停止位，关闭 DLAB
        write_uart_reg(LCR, 0x03);

        // 启用 FIFO，清空接收/发送队列，设置中断阈值
        write_uart_reg(FCR, 0xC7);

        // 设置 RTS 和 DTR 信号
        write_uart_reg(MCR, 0x03);
    }
}

// getchar
pub fn getchar() -> u8 {
    // wait till data is available
    unsafe {
        while (read_uart_reg(LSR) & LSR_RX_READY) == 0 {
            // wait
        }

        read_uart_reg(RBR)
    }
}

// putchar
pub fn putchar(c: usize) {
    unsafe {
        // wait till the transmitter is idle
        while (read_volatile((UART0_BASE_ADDR + LSR) as *const u8) & LSR_TX_IDLE) == 0 {
            // wait
        }

        write_volatile((UART0_BASE_ADDR + THR) as *mut u8, c as u8);
    }
}

unsafe fn read_uart_reg(offset: usize) -> u8 {
    assert!(offset < UART0_SIZE, "UART register offset out of range");
    read_volatile((UART0_BASE_ADDR + offset) as *const u8)
}

unsafe fn write_uart_reg(offset: usize, value: u8) {
    assert!(offset < UART0_SIZE, "UART register offset out of range");
    write_volatile((UART0_BASE_ADDR + offset) as *mut u8, value);
}

const CLINT_BASE:     usize = 0x2000000;
const CLINT_MTIMECMP: usize = CLINT_BASE + 0x4000; // hart 0, if single core
const CLINT_MTIME:    usize = CLINT_BASE + 0xBFF8;

pub fn set_timer_uart(timer: usize) {
    unsafe {
        let mtimecmp = CLINT_MTIMECMP as *mut u32;
        write_volatile(mtimecmp.add(1), (timer >> 32) as u32);
        write_volatile(mtimecmp,(timer & 0xFFFF_FFFF) as u32);
    }
}

pub fn get_time_uart() -> usize {
    unsafe {
        let mtime = CLINT_MTIME as *const u32;
        let high = read_volatile(mtime.add(1)) as usize;
        let low = read_volatile(mtime) as usize;
        (high << 32) | low
    }
}