// os/src/mm/frame_allocator.rs

use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::config::MEMORY_END;
use crate::sync::UPSafeCell;
use crate::mm::address::{PhyAddr, PhyPageNum};

// ----- FrameTracker -----
// the smallest granularity provided by frame allocator.
pub struct FrameTracker {
    pub ppn: PhyPageNum,
}

// The dropping of frame is equivalent to the deallocation of frame allocator.
// but we need `frame_alloc` to allocate a new frame
impl FrameTracker {
    // ----- constructor -----
    pub fn new(ppn: PhyPageNum) -> Self {
        let bytes = ppn.as_raw_bytes();
        for i in bytes {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}


// ----- FrameAllocator -----
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhyPageNum>;
    fn dealloc(&mut self, ppn: PhyPageNum);
}

pub struct StackFrameAllocator {
    current: PhyPageNum,  // 空闲内存的起始 PPN
    end: PhyPageNum,      // 空闲内存的结束 PPN
    recycled: Vec<PhyPageNum>,
}

impl FrameAllocator for StackFrameAllocator {
    // ----- constructor -----
    fn new() -> StackFrameAllocator {
        StackFrameAllocator {
            current: PhyPageNum(0),
            end: PhyPageNum(0),
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhyPageNum> {        
        let candidate = self.recycled.pop();
        if let Some(ppn) = candidate {
            Some(ppn)
        } else if self.current < self.end {
            let ppn = self.current;
            self.current.0 += 1;
            Some(ppn)
        } else {
            None // no available frame
        }
    }

    fn dealloc(&mut self, ppn: PhyPageNum) {
        if ppn >= self.current || self.recycled
            .iter()
            .find(|&v| {*v == ppn})
            .is_some() {
            panic!("[frame_allocator] Frame ppn={:#x} deallocation failed.", ppn.0);
        }
        self.recycled.push(ppn);
    }
}

impl StackFrameAllocator {
    fn new_with_range(start: usize, end: usize) -> StackFrameAllocator {
        StackFrameAllocator {
            current: PhyAddr::from(start).ceil(),
            end: PhyAddr::from(end).floor(),
            recycled: Vec::new(),
        }
    }

    // ----- methods -----
    fn init(&mut self, start: usize, end: usize) {
        if self.current != PhyPageNum(0) || self.end != PhyPageNum(0) {
            panic!("[frame_allocator] Frame allocator cannot be initialize twice.");
        }
        self.current = PhyAddr::from(start).ceil();
        self.end = PhyAddr::from(end).floor();
    }
}

type FrameAllocatorImpl = StackFrameAllocator;
lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> = unsafe {
        UPSafeCell::new(FrameAllocatorImpl::new())
    };
}

pub fn init_frame_allocator() {
    unsafe extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(ekernel as usize, MEMORY_END);
}

// ----- frame allocator interface -----

pub fn frame_alloc() -> Option<FrameTracker> {
    let opt_ppn = FRAME_ALLOCATOR.exclusive_access().alloc();
    match opt_ppn {
        Some(ppn) => Some(FrameTracker::new(ppn)),
        None => None,
    }
}

pub(crate) fn frame_dealloc(ppn: PhyPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}