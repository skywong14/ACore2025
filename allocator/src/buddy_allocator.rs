// allocator/src/buddy_allocator.rs

use alloc::alloc::Layout;
use core::cmp::max;
use super::linked_list::LinkedList;

// maximum memory block size: 2^31 Bytes (2GB)
const BUDDY_ALLOCATOR_LEVEL: usize = 32;

/// Buddy Allocator
/// Implementation Techniques: Intrusive Linked List, Multiple Free Lists(especially for initialization)
pub struct BuddyAllocator {
    gran: usize,   // granularity
    free_lists: [LinkedList; BUDDY_ALLOCATOR_LEVEL], // free lists for each level

    total: usize,     // 管理的内存总量
    user: usize,      // 用户请求的内存总量
    allocated: usize  // 实际分配的内存总量（含内部碎片）
}

/// constructors: `new`, `empty`
/// methods: `add_segment`, `alloc`, `dealloc`
/// utils:  `split`, `merge`
impl BuddyAllocator {
    // ----- constructors -----
    /// Create an empty BuddyAllocator with given `gran`
    /// empty cannot be const, because usize is not const
    pub fn empty(gran: usize) -> Self {
        Self {
            free_lists: [LinkedList::new(); BUDDY_ALLOCATOR_LEVEL],
            user: 0,
            allocated: 0,
            total: 0,
            gran: max(gran, size_of::<usize>()) // 确保最小粒度不小于指针大小，以便能够存储空闲列表指针
        }
    }

    /// Create a BuddyAllocator with given `gran`, `start`, and `end`
    /// [unsafe] user must ensure that the provided memory segments are non-overlapping and available
    pub unsafe fn new(gran: usize, start: usize, end: usize) -> Self {
        let mut allocator = Self::empty(gran);
        allocator.add_segment(start, end);
        allocator
    }

    // ----- methods -----
    /// add memory segment [`start`,`end`) to the allocator
    /// [unsafe] user must ensure that the provided memory segments are non-overlapping and available
    pub unsafe fn add_segment(&mut self, mut start: usize, mut end: usize) {

    }

    /// Allocate a block of memory with the given `layout`
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {

    }

    /// Deallocate a block of memory at `ptr` with the given `layout`
    /// [unsafe] user must ensure that the pointer is valid and was allocated by this allocator
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {

    }


    // ----- utils -----
    /// Split a `start` level to `end` levels
    fn split(&mut self, start: usize, end: usize) {

    }

    /// Try to merge a block at `ptr` with its buddy, starting from level `start`
    fn merge(&mut self, start: usize, ptr: *mut u8) {

    }
}