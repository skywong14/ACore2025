// fs/src/block_dev.rs

use core::any::Any;

/// Trait for block devices
pub trait BlockDevice : Send + Sync + Any {
    /// read a block from block to buffer
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    /// write a block from buffer to block
    fn write_block(&self, block_id: usize, buf: &[u8]);
}