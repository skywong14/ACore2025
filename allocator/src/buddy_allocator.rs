// allocator/src/buddy_allocator.rs

use alloc::alloc::Layout;
use core::cmp::{max, min};
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
    /// ATTENTION: gran must be at least sizeof(usize)!
    pub const fn empty(gran: usize) -> Self {
        Self {
            free_lists: [LinkedList::new(); BUDDY_ALLOCATOR_LEVEL],
            user: 0,
            allocated: 0,
            total: 0,
            gran
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
        // 对齐 start 和 end 到粒度的边界
        start = (start + self.gran - 1) & (!self.gran + 1);
        end = end & (!self.gran + 1);
        self.total += end - start;

        // 以尽可能大的块的形式添加内存到 free_lists
        while start < end {
            // 计算当前可以放入的最大块的大小级别
            let level = (end - start).trailing_zeros() as usize;
            // 将块添加到对应级别的空闲列表中
            self.free_lists[level].push_front(start as *mut usize);
            // 移动start指针到下一个未处理区域
            start += 1 << level;
        }
    }

    /// Allocate a block of memory with the given `layout`
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // 计算需要分配的实际大小
        let size = self.calculate_size(&layout);
        let level = size.trailing_zeros() as usize;

        // 从适当级别开始，查找可用的内存块
        for i in level..self.free_lists.len() {
            if !self.free_lists[i].is_empty() {
                // 如果找到较大的块，需要分割成合适的大小
                self.split(level, i);
                let result = self.free_lists[level]
                    .pop_front()
                    .expect("[buddy_allocator] Expect non-empty free list.");

                // 更新统计信息
                self.user += layout.size();
                self.allocated += size;
                return result as *mut u8;
            }
        }
        panic!(
            "[buddy_allocator] Unable to allocate more space for size {}.",
            size
        );
    }

    /// Deallocate a block of memory at `ptr` with the given `layout`
    /// [unsafe] user must ensure that the pointer is valid and was allocated by this allocator
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        // 计算实际分配的大小和级别
        let size = self.calculate_size(&layout);
        let level = size.trailing_zeros() as usize;

        // 尝试合并内存块
        self.merge(level, ptr);
    }


    // ----- utils -----
    /// Split a `start` level to `end` levels
    fn split(&mut self, start: usize, end: usize) {
        // 从高级别向低级别分割
        for i in (start..end).rev() {
            // 从当前级别取出一个块
            let ptr = self.free_lists[i + 1]
                .pop_front()
                .expect("[buddy_allocator] Expect non-empty free list.");

            // 将块分割成两个更小的块并加入到低一级的空闲列表
            unsafe {
                // 右半部分（buddy）
                self.free_lists[i].push_front((ptr as usize + (1 << i)) as *mut usize);
                // 左半部分
                self.free_lists[i].push_front(ptr);
            }
        }
    }

    /// Try to merge a block at `ptr` with its buddy, starting from level `start`
    fn merge(&mut self, start: usize, ptr: *mut u8) {
        let mut curr = ptr as usize;
        // 尝试逐级合并
        for i in start..self.free_lists.len() {
            // 计算当前级别的伙伴块地址
            let buddy = curr ^ (1 << i);
            // 在当前级别的空闲列表中查找伙伴块
            let target = self.free_lists[i]
                .iter_mut()
                .find(|node| node.as_ptr() as usize == buddy);

            if let Some(node) = target {
                // 找到伙伴块，将其从空闲列表中移除
                node.pop();
                // 合并后的块地址是两个块中较小的那个
                curr = min(curr, buddy);
            } else {
                // 没有找到伙伴块，将当前块加入到空闲列表
                unsafe {
                    self.free_lists[i].push_front(curr as *mut usize);
                }
                break;
            }
        }
    }

    /// Calculate the required size for the given layout
    /// Ensures the size is:
    /// 1. A power of 2
    /// 2. At least as large as the layout's size
    /// 3. Aligned to the layout's alignment and the allocator's granularity
    fn calculate_size(&self, layout: &Layout) -> usize {
        return max(
            layout.size().next_power_of_two(),
            max(layout.align(), self.gran),
        );
    }
}