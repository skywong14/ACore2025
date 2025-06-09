// fs/src/config.rs

pub const EFS_MAGIC: u32 = 0x3b800001;
pub const CACHE_SIZE: u32 = 512;
pub const BLOCK_SIZE: u32 = 512;
pub const INODE_SIZE: u32 = 32 * 4;
pub const DNODE_SIZE: u32 = 32 * 16;
pub const INODE_PER_BLOCK: u32 = BLOCK_SIZE / INODE_SIZE;

// inode & disk_inode
pub(crate) const INODE_DIRECT_COUNT: u32 = 28;
pub(crate) const INODE_INDIRECT1_COUNT: u32 = BLOCK_SIZE / 4;
pub(crate) const INODE_INDIRECT2_COUNT: u32 = INODE_INDIRECT1_COUNT * INODE_INDIRECT1_COUNT;
