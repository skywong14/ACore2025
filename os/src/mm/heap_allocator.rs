// os/src/mm/heap_allocator.rs

use crate::config::KERNEL_HEAP_SIZE;
// use buddy_system_allocator::LockedHeap;
use buddy_allocator::SafeBuddyHeap;
use core::ptr::addr_of_mut;

#[global_allocator]
// static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
static HEAP_ALLOCATOR: SafeBuddyHeap = SafeBuddyHeap::empty(8);

#[alloc_error_handler]
// panic when heap allocation error occurs
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

// heap space: [u8; KERNEL_HEAP_SIZE]
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

// init
pub fn init_heap() {
    unsafe {
        // HEAP_ALLOCATOR.lock().init(addr_of_mut!(HEAP_SPACE) as usize, KERNEL_HEAP_SIZE);
        HEAP_ALLOCATOR.add_segment(
            addr_of_mut!(HEAP_SPACE) as usize,
            addr_of_mut!(HEAP_SPACE) as usize + KERNEL_HEAP_SIZE,
        );
    }
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for (i, val) in v.iter().take(500).enumerate() {
        assert_eq!(*val, i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
