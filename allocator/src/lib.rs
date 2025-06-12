#![no_std]

pub mod safe_buddy_allocator;
pub mod buddy_allocator;
pub mod linked_list;

extern crate alloc;

pub use safe_buddy_allocator::SafeBuddyHeap;