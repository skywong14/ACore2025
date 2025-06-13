use super::BlockDevice;
use crate::mm::frame_allocator::{FrameTracker, frame_alloc, frame_dealloc};
use crate::mm::page_table::{PageTable};
use crate::mm::address::{PhyAddr, PhyPageNum, VirAddr};
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::{Hal, VirtIOBlk, VirtIOHeader};
use crate::config::VIRTIO0_BASE_ADDR;
use crate::mm::KERNEL_SPACE;

// frame_alloc 得到的物理页帧都会被保存在全局的 QUEUE_FRAMES 中
// 延长了它们的生命周期，避免提前被回收
lazy_static! {
    static ref QUEUE_FRAMES: UPSafeCell<Vec<FrameTracker>> = unsafe { 
        UPSafeCell::new(Vec::new()) 
    };
}

// ----- VirtioHal -----
/// A zero-sized struct that implements the Hal trait, 
/// providing hardware abstraction for the VirtIO driver.
pub struct VirtioHal;
impl Hal for VirtioHal {
    /// Allocate a **contiguous** block of physical memory for DMA operations.
    fn dma_alloc(pages: usize) -> usize {
        let mut ppn_base = PhyPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            QUEUE_FRAMES.exclusive_access().push(frame);
        }
        let pa: PhyAddr = ppn_base.into();
        pa.0
    }
    /// Deallocate a block of physical memory previously allocated for DMA operations.
    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let pa = PhyAddr::from(pa);
        let mut ppn_base: PhyPageNum = pa.into();
        for i in 0..pages {
            frame_dealloc((ppn_base.0 + i).into());
        }
        0
    }
    fn phys_to_virt(addr: usize) -> usize {
        addr
    }
    fn virt_to_phys(vaddr: usize) -> usize {
        PageTable::from_satp_token(KERNEL_SPACE.exclusive_access().to_satp())
            .translate_va(VirAddr::from(vaddr)).unwrap().0
    }
}


// ----- VirtIOBlk -----
/// Wrapper for VirtIO block device driver, implements the `BlockDevice` trait,
/// provides thread-safe block-level storage read/write interface for interacting with virtual block devices.
pub struct VirtIOBlock(UPSafeCell<VirtIOBlk<'static, VirtioHal>>);

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0.exclusive_access()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0.exclusive_access()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

impl VirtIOBlock {
    pub fn new() -> Self {
        unsafe {
            Self(UPSafeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0_BASE_ADDR as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}