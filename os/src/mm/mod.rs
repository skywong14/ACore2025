pub(crate)  mod address;
pub(crate)  mod page_table;
pub(crate) mod frame_allocator;
pub(crate) mod memory_set;
pub(crate) mod range;
pub(crate) mod area;
pub(crate) mod heap_allocator;

pub use memory_set::KERNEL_SPACE;

pub use page_table::UserBuffer;

pub use memory_set::remap_test;

pub fn init() {
    heap_allocator::init_heap();
    println_green!("[kernel] init_heap finished");
    frame_allocator::init_frame_allocator();
    println_green!("[kernel] init_frame_allocator finished");
    KERNEL_SPACE.exclusive_access().activate();
    println_green!("[kernel] activate kernel space finished");
}