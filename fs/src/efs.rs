// fs/src/efs.rs

use alloc::sync::Arc;
use spin::Mutex;
use crate::bitmap::Bitmap;
use crate::block_cache::{block_cache_sync_all, get_block_cache};
use crate::block_dev::BlockDevice;
use crate::config::{BLOCK_SIZE, INODE_PER_BLOCK, INODE_SIZE};
use crate::inode::Inode;
use crate::disk_inode::{DiskInode, DiskInodeType, DataBlock};
use crate::super_block::SuperBlock;

/// Easy FIle System (EFS) implementation
/// this is a structure on Memory
pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: Bitmap,
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32, // inode 区域的起始块号
    data_area_start_block: u32,  // 数据区的起始块号
}

impl EasyFileSystem {
    // ----- constructor -----
    /// 根据总块数和 inode 位图块数来创建文件系统
    pub fn create(block_device: Arc<dyn BlockDevice>, total_blocks: u32, inode_bitmap_blocks: u32) -> Arc<Mutex<Self>> {
        // 1. 计算各区域大小并创建位图
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.maximum() as u32;
        let inode_area_blocks = (inode_num + INODE_PER_BLOCK - 1) / INODE_PER_BLOCK;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        let data_bitmap_blocks = (data_total_blocks * size_of::<u8>() as u32 * 8 + 1) / (BLOCK_SIZE * 8 + 1);
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_num = data_area_blocks;
        let data_bitmap = Bitmap::new(
            (1 + inode_total_blocks) as usize,
            data_bitmap_blocks as usize,
        );

        // 2. 初始化文件系统元数据
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };

        // 3. 清空所有块
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut [u8; BLOCK_SIZE as usize]| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }

        // 4. 初始化 SuperBlock
        get_block_cache(0, Arc::clone(&block_device)).lock()
            .modify(0, |super_block: &mut SuperBlock| {
                super_block.initialize(inode_num, data_num);
            });

        // 5. 创建根目录 "/" 的 inode
        assert_eq!(efs.alloc_inode(), 0); // 分配 inode 0
        // 获取 inode 0 在磁盘上的位置
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        // 初始化为 Directory 类型
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });

        // 6. 将所有缓存写回磁盘
        block_cache_sync_all();

        Arc::new(Mutex::new(efs))
    }

    // ----- methods -----
    /// Allocate a new inode
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }
    /// Allocate a new data block (contains offset!)
    pub fn alloc_data_block(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }
    /// Deallocate a data block (contains offset!)
    pub fn dealloc_data_block(&mut self, block_id: u32) {
        // 获取该块的缓存，并将其内容全部清零
        get_block_cache(block_id as usize, Arc::clone(&self.block_device)).lock()
            .modify(
                0, |data_block: &mut DataBlock| {
                    data_block.iter_mut().for_each(|p| { *p = 0; })
                });
        
        // 释放 Bitmap 中的位
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize
        )
    }

    /// 根据 inode ID 计算其在磁盘上存储的位置 (块号，偏移量)
    /// 一个块有四个 disk_inode
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let block_id = self.inode_area_start_block + inode_id / INODE_PER_BLOCK;
        (
            block_id,
            ((inode_id % INODE_PER_BLOCK) * INODE_SIZE) as usize,
        )
    }

    /// 从一个已写入 efs 镜像的块设备上打开我们的 easy-fs
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        // 读取 0 号块 (SuperBlock)
        get_block_cache(0, Arc::clone(&block_device)).lock()
            .read(0, |super_block: &SuperBlock| {
                // 检查魔数
                assert!(super_block.is_valid(), "Error loading EFS!");
                // 计算 inode 位图和 inode 数据区的总块数
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                // 在内存中构建 EasyFileSystem 实例
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(
                        1, // inode bitmap 从 1 号块开始
                        super_block.inode_bitmap_blocks as usize
                    ),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks, // inode 区的起始块号
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks, // 数据区的起始块号
                };
                Arc::new(Mutex::new(efs))
            })
    }
    /// 获取根目录的 inode
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        Inode::new(
            block_id,
            block_offset,
            Arc::clone(efs),
            block_device,
        )
    }
}
