// os/src/task/pid.rs

use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::config::{kernel_stack_position, KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE_START_ADDRESS};
use crate::mm::address::VirAddr;
use crate::mm::area::{MapArea, MapPermission};
use crate::mm::area::MapType::Framed;
use crate::mm::KERNEL_SPACE;
use crate::sync::UPSafeCell;

// ----- pid allocator -----
pub struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    // ----- constructor -----
    pub(super) fn new() -> Self {
        PidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }
    // ----- methods -----
    pub(super) fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            // 有回收的 pid，直接使用
            PidHandle(pid)
        } else {
            // 分配一个新的 pid
            let pid = self.current;
            self.current += 1;
            PidHandle(pid)
        }
    }
    pub(super) fn dealloc(&mut self, pid: usize) {
        if pid < self.current {
            if self.recycled.contains(&pid) {
                panic!("pid {} has already been recycled", pid);
            }
            self.recycled.push(pid);
        } else {
            panic!("pid {} is out of range", pid);
        }
    }
}

// ----- global pid allocator -----
lazy_static! {
    pub static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> = unsafe { 
        UPSafeCell::new(PidAllocator::new()) 
    };
}

// ----- PidHandle -----
// 分配出的 pid 由 PidHandle 管理，PidHandle 被丢弃时自动回收 pid
pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        println_gray!("[pid] dealloc pid {}", self.0);
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

pub fn pid_alloc() -> PidHandle {
    let ret = PID_ALLOCATOR.exclusive_access().alloc();
    println_gray!("[pid] alloc pid {}", ret.0);
    ret
}
// no need to dealloc pid manually


// ----- kernel stack (for app) -----

pub struct KernelStack {
    pid: usize,
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        println_gray!("[pid] dealloc kernel stack for pid {}", self.pid);
        // 栈底地址，栈内存区域的起始位置
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirAddr = kernel_stack_bottom.into();
        
        KERNEL_SPACE.exclusive_access()
            .unmap_area_with_start_vpn(kernel_stack_bottom_va.into());
        // unmap 中会自动释放物理页 (Frame)
    }
}

impl KernelStack {
    // ----- constructor -----
    // Create a new kernel stack for the given PidHandle.
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        KERNEL_SPACE
            .exclusive_access()
            .map_area(
                MapArea::new_with_address(
                    kernel_stack_bottom.into(),
                    kernel_stack_top.into(),
                    Framed,
                    MapPermission::R | MapPermission::W,
                ),
                None
            );
        KernelStack {
            pid: pid_handle.0,
        }
    }
    
    // ----- methods -----
    // 将一个类型为 T 的变量压入栈顶，返回其裸指针
    // 注意：不检查栈溢出，且可能覆盖原有数据
    // requires T to be Sized
    pub fn push_on_top<T: Sized>(&self, value: T) -> *mut T {
        let kernel_stack_top = self.get_kernel_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe { *ptr_mut = value; }
        ptr_mut
    }
    // 获取当前内核栈顶在内核地址空间中的地址
    pub fn get_kernel_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}

