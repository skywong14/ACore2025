// os/src/config.rs

// task
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;

// trampoline
pub const TRAMPOLINE_START_ADDRESS: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT_ADDRESS: usize = TRAMPOLINE_START_ADDRESS - PAGE_SIZE;

// timer
pub const CLOCK_FREQ: usize = 12500000; // 125MHz

// memory
pub const MEMORY_END: usize = 0x80800000;
pub const KERNEL_HEAP_SIZE: usize = 0x300000;

// uart and sbi
pub const TEST_DEVICE_ADDR: usize = 0x100000; // shutdown devic, QEMU 测试设备地址

pub const UART0_BASE_ADDR: usize = 0x10000000;
pub const UART0_SIZE: usize = 0x100;

pub const VIRTIO0_BASE_ADDR: usize = 0x10001000;
pub const VIRTIO0_SIZE: usize = 0x1000; // 4KB

pub const CLINT_BASE:     usize = 0x2000000;
pub const CLINT_SIZE: usize = 0x10000;  // 64KB
pub const CLINT_MTIMECMP: usize = CLINT_BASE + 0x4000; // hart 0, if single core
pub const CLINT_MTIME:    usize = CLINT_BASE + 0xBFF8;

// mm
pub const PAGE_SIZE : usize = 1 << PAGE_SIZE_BITS;
pub const PA_WIDTH: usize = 56;
pub const PAGE_SIZE_BITS: usize = 12; // 4KB
pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;

pub const VA_WIDTH: usize = 39;

// Return (bottom, top) of a kernel stack in kernel space
// 次高空间为内核栈
// 分配 KERNEL_STACK_SIZE + 1 PAGE 作为每个用户的内核栈
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE_START_ADDRESS - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}