// fs/src/super_block.rs

use crate::config::{BLOCK_SIZE, EFS_MAGIC, INODE_PER_BLOCK};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SuperBlock {
    pub magic: u32,               // 魔数
    pub total_blocks: u32,        // 文件系统总块数
    pub inode_bitmap_blocks: u32, // inode位图占用块数
    pub inode_area_blocks: u32,   // inode区占用块数
    pub data_bitmap_blocks: u32,  // 数据位图占用块数
    pub data_area_blocks: u32,    // 数据区占用块数
}

impl SuperBlock {
    // ----- constructor -----
    pub fn new() -> Self {
        Self {
            magic: EFS_MAGIC,
            total_blocks: 0,
            inode_bitmap_blocks: 0,
            inode_area_blocks: 0,
            data_bitmap_blocks: 0,
            data_area_blocks: 0,
        }
    }
    // ----- methods -----
    pub fn initialize(&mut self, num_inode: u32, num_dnode: u32) {
        let inode_bitmap_blocks = (num_inode / 8 - 1) / BLOCK_SIZE + 1;
        let inode_area_blocks = (num_inode - 1) / INODE_PER_BLOCK + 1;
        let data_bitmap_blocks = (num_dnode / 8 - 1) / BLOCK_SIZE + 1;
        let data_area_blocks = num_dnode;
        
        let total_blocks = 1 + 
            inode_bitmap_blocks + inode_area_blocks + data_bitmap_blocks + data_area_blocks;

        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}