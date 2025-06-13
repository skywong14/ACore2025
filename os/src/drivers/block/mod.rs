// os/src/drivers/block/mod.rs

mod virtio_blk;

pub use virtio_blk::VirtIOBlock;

use alloc::sync::Arc;
use easy_fs::BlockDevice;
use lazy_static::*;

pub type BlockDeviceImpl = VirtIOBlock;

const BLOCK_SIZE: usize = 512;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(VirtIOBlock::new());
}

pub fn block_device_test() {
    let block_device = BLOCK_DEVICE.clone();
    let mut write_buffer = [0u8; BLOCK_SIZE];
    let mut read_buffer = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        for byte in write_buffer.iter_mut() {
            *byte = i as u8;
        }
        block_device.write_block(i, &write_buffer);
        block_device.read_block(i, &mut read_buffer);
        assert_eq!(write_buffer, read_buffer);
    }
    println!("block device test passed!");
}
