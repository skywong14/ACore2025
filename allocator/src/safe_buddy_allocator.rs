// allocator/src/safe_buddy_allocator.rs

use core::alloc::GlobalAlloc;
use spin::Mutex;
use crate::buddy_allocator::BuddyAllocator;

/// A thread-safe wrapper around the BuddyAllocator
pub struct SafeBuddyHeap {
    pub allocator: Mutex<BuddyAllocator>
}

impl SafeBuddyHeap {
    /// Create a new SafeBuddyHeap
    /// `gran` - 粒度值，定义了伙伴分配器中最小块的大小。
    pub fn empty(gran: usize) -> Self {
        // Ensure gran is a power of 2
        assert!(gran > 0 && (gran & (gran - 1)) == 0, "Granularity must be a power of 2");
        Self {
            allocator: Mutex::new(BuddyAllocator::empty(gran)),
        }
    }
    /// Add a memory segment to the allocator
    pub unsafe fn add_segment(&self, start: usize, end: usize) {
        self.allocator.lock().add_segment(start, end);
    }
}

///  Implementation of `GlobalAlloc`,
///  which makes the SafeBuddyHeap usable as a global allocator
unsafe impl GlobalAlloc for SafeBuddyHeap {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.allocator.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.allocator.lock().dealloc(ptr, layout);
    }
}