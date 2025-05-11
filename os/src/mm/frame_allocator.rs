// os/src/mm/frame_allocator.rs

use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::config::{MEMORY_END, PAGE_SIZE};
use crate::sync::UPSafeCell;
use crate::mm::address::{PhyAddr, PhyPageNum};

// ----- FrameTracker -----
// the smallest granularity provided by frame allocator.
pub struct FrameTracker {
    pub ppn: PhyPageNum,
}

// The creation(`new`) of frame is equivalent to the allocation of frame allocator,
// The dropping of frame is equivalent to the deallocation of frame allocator.
impl FrameTracker {
    // ----- constructor -----
    pub fn new() -> Self {
        let frame = frame_alloc();
        match frame {
            Some(frame) => {
                let ret =  Self { ppn: frame.ppn };
                ret.init();
                ret
            },
            None => panic!("[panic] frame_alloc failed"),
        }
    }
    
    pub fn from_existed_and_init(ppn: PhyPageNum) -> Self {
        // page cleaning
        let ret = Self {ppn};
        ret.init();
        ret
    }

    pub fn from_existed(ppn: PhyPageNum) -> Self {
        Self { ppn }
    }

    // ----- methods -----
    pub fn init(&self) {
        // page cleaning
        let ptr = usize::from(self.ppn) as *mut u8;
        unsafe {
            core::slice::from_raw_parts_mut(ptr, PAGE_SIZE).fill(0);
        }
    }

    pub fn get_ppn(&self) -> PhyPageNum { self.ppn }
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

impl StackFrameAllocator {
    // ----- constructor -----
    pub fn new() -> StackFrameAllocator {
        StackFrameAllocator {
            current: PhyPageNum(0),
            end: PhyPageNum(0),
            recycled: Vec::new(),
        }
    }

    pub fn new_with_range(start: usize, end: usize) -> StackFrameAllocator {
        StackFrameAllocator {
            current: PhyAddr::from(start).ceil(),
            end: PhyAddr::from(end).floor(),
            recycled: Vec::new(),
        }
    }

    // ----- methods -----
    pub fn init(&mut self, start: usize, end: usize) {
        if self.current != PhyPageNum(0) || self.end != PhyPageNum(0) {
            panic!("[frame_allocator] Frame allocator cannot be initialize twice.");
        }
        self.current = PhyAddr::from(start).ceil();
        self.end = PhyAddr::from(end).floor();
    }

    pub fn alloc(&mut self) -> Option<PhyPageNum> {
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

    pub fn dealloc(&mut self, ppn: PhyPageNum) {
        if ppn >= self.current || self.recycled
            .iter()
            .find(|&v| {*v == ppn})
            .is_some() {
            panic!("[frame_allocator] Frame ppn={:#x} deallocation failed.", ppn.0);
        }
        self.recycled.push(ppn);
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
        Some(ppn) => Some(FrameTracker::from_existed_and_init(ppn)),
        None => None,
    }
}

fn frame_dealloc(ppn: PhyPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}