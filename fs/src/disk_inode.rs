// fs/src/disk_inode.rs

use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::config::BLOCK_SIZE;

use crate::config::{INODE_DIRECT_COUNT, INODE_INDIRECT1_COUNT, INODE_INDIRECT2_COUNT};

pub type DataBlock = [u8; BLOCK_SIZE as usize];
pub type IndirectBlock = [u32; (BLOCK_SIZE / 4) as usize];

// ----- Disk Inode -----

#[repr(u32)] // should be u32
#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

/// a DiskInode represents an inode on disk
/// size: 128 bytes(32 * 4), 4 inodes per block
/// this is a disk structure
#[repr(C)]
pub struct DiskInode {
    pub size: u32,    // 单位: Bytes
    pub direct: [u32; INODE_DIRECT_COUNT as usize],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}

impl DiskInode {
    /// 因为 DiskInode 是一个磁盘结构体，所以初始化无需分配内存，直接在磁盘上修改数据内容即可
    /// 故用 `initialize` 方法来初始化一个新的 DiskInode
    pub fn initialize(&mut self, type_: DiskInodeType) {
        *self = Self{
            size: 0,
            direct: [0; INODE_DIRECT_COUNT as usize],
            indirect1: 0,
            indirect2: 0,
            type_,
        }
    }

    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }
    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }
    pub fn data_block_num(&self) -> u32 {
        Self::data_block_num_(self.size)
    }
    fn data_block_num_(size: u32) -> u32 {
        (size + BLOCK_SIZE - 1) / (BLOCK_SIZE)
    }
    /// 根据内部块编号 `inner_id`，返回对应的物理块编号。
    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id as usize]
        } else if inner_id < INODE_DIRECT_COUNT + INODE_INDIRECT1_COUNT {
            get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[(inner_id - INODE_DIRECT_COUNT) as usize]
                })
        } else {
            let last = inner_id - INODE_DIRECT_COUNT - INODE_INDIRECT1_COUNT;
            let indirect1 = get_block_cache(self.indirect2 as usize, Arc::clone(block_device)).lock()
                .read(0, |indirect2: &IndirectBlock| {
                    indirect2[(last / INODE_INDIRECT1_COUNT) as usize]
                });
            get_block_cache(indirect1 as usize, Arc::clone(block_device)).lock()
                .read(0, |indirect1: &IndirectBlock| {
                    indirect1[(last % INODE_INDIRECT1_COUNT) as usize]
                })
        }
    }
    /// number of blocks needed, include indirect1/2
    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::data_block_num_(size);

        if data_blocks <= INODE_DIRECT_COUNT {
            data_blocks
        } else if data_blocks <= INODE_DIRECT_COUNT + INODE_INDIRECT1_COUNT {
            data_blocks + 1
        } else {
            // 需要使用二级间接索引

            // 除了直接索引和第一个一级间接索引外的剩余数据块数
            let remaining_blocks = data_blocks - (INODE_DIRECT_COUNT + INODE_INDIRECT1_COUNT);

            // 需要多少个额外的一级间接索引块来存储这些剩余块, 向上取整
            let sub_indirect1_blocks = (remaining_blocks + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;

            // 总块数 = 数据块数 + 1个一级间接索引块 + 1个二级间接索引块 + 额外的一级间接索引块
            data_blocks + 1 + 1 + sub_indirect1_blocks
        }
    }
    /// number of extra blocks needed to increase the size of the inode
    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    /// 从 disk inode 的指定偏移处读取数据到缓冲区
    /// 返回实际读取的字节数
    pub fn read_at(&self, offset: usize, buf: &mut [u8], block_device: &Arc<dyn BlockDevice>) -> usize {
        let len = buf.len() as u32;
        let mut start = offset as u32;
        let end = (start + len).min(self.size);

        if start >= end {
            return 0;
        }

        // 计算起始数据块的索引(逻辑块号)
        let mut start_block = start / BLOCK_SIZE;
        let mut read_size = 0u32; // 已经读取的字节数

        loop {
            // 计算当前块的结束位置
            // 首先假设读到当前块的末尾
            let mut end_current_block = (start / BLOCK_SIZE + 1) * BLOCK_SIZE;

            // 文件结束位置在当前块
            end_current_block = end_current_block.min(end);

            // 当前块要读取的字节数
            let block_read_size = end_current_block - start;

            // 确定缓冲区中存放当前块数据的范围
            let dst = &mut buf[read_size as usize..(read_size + block_read_size) as usize];

            // 获取当前数据块的缓存
            get_block_cache(self.get_block_id(start_block, block_device) as usize,Arc::clone(block_device))
                .lock().read(0, |data_block: &DataBlock| {
                let src = &data_block[(start % BLOCK_SIZE) as usize..(start % BLOCK_SIZE + block_read_size) as usize];
                dst.copy_from_slice(src);
            });

            // 已读取的总字节数
            read_size += block_read_size;

            if end_current_block == end {
                break;
            }

            // 准备读取下一个块
            start_block += 1;
            start = end_current_block;
        }

        read_size as usize
    }

    /// 将数据写入到 disk inode 的指定偏移处
    /// 返回实际写入的字节数
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>
    ) -> usize {
        let mut start = offset as u32;
        let end = ((offset + buf.len()) as u32).min(self.size);
        assert!(start <= end);
        let mut start_block = start / BLOCK_SIZE;
        let mut write_size = 0u32;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SIZE + 1) * BLOCK_SIZE;
            end_current_block = end_current_block.min(end);
            // write and update write size
            let block_write_size = end_current_block - start;
            get_block_cache(
                self.get_block_id(start_block, block_device) as usize,
                Arc::clone(block_device),
            )
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    let src = &buf[write_size as usize..(write_size + block_write_size) as usize];
                    let dst = &mut data_block[(start % BLOCK_SIZE) as usize..(start % BLOCK_SIZE + block_write_size) as usize];
                    dst.copy_from_slice(src);
                });
            write_size += block_write_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        write_size as usize
    }

    /// Inncrease the size of current disk inode
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>
    ) {
        let mut current_blocks = self.data_block_num();
        self.size = new_size;
        let mut total_blocks = self.data_block_num();
        let mut new_blocks = new_blocks.into_iter();
        // fill direct
        while current_blocks < total_blocks.min(INODE_DIRECT_COUNT) {
            self.direct[current_blocks as usize] = new_blocks.next().unwrap();
            current_blocks += 1;
        }
        // alloc indirect1
        if total_blocks > INODE_DIRECT_COUNT {
            if current_blocks == INODE_DIRECT_COUNT {
                self.indirect1 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_DIRECT_COUNT;
            total_blocks -= INODE_DIRECT_COUNT;
        } else {
            return;
        }
        // fill indirect1
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                while current_blocks < total_blocks.min(INODE_INDIRECT1_COUNT) {
                    indirect1[current_blocks as usize] = new_blocks.next().unwrap();
                    current_blocks += 1;
                }
            });
        // alloc indirect2
        if total_blocks > INODE_INDIRECT1_COUNT {
            if current_blocks == INODE_INDIRECT1_COUNT {
                self.indirect2 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_INDIRECT1_COUNT;
            total_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return;
        }
        // fill indirect2 from (a0, b0) -> (a1, b1)
        let mut a0 = current_blocks / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks % INODE_INDIRECT1_COUNT;
        // alloc low-level indirect1
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                while (a0 < a1) || (a0 == a1 && b0 < b1) {
                    if b0 == 0 {
                        indirect2[a0 as usize] = new_blocks.next().unwrap();
                    }
                    // fill current
                    get_block_cache(indirect2[a0 as usize] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            indirect1[b0 as usize] = new_blocks.next().unwrap();
                        });
                    // move to next
                    b0 += 1;
                    if b0 == INODE_INDIRECT1_COUNT {
                        b0 = 0;
                        a0 += 1;
                    }
                }
            });
    }

    /// Clear size to zero and return blocks that should be deallocated.
    /// We will clear the block contents to zero later (in memory_inode.clear())
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();  // 需要释放的块编号
        let mut data_block_num = self.data_block_num();
        self.size = 0; // resize
        
        // collect blocks to deallocate
        let mut current_blocks = 0u32;
        // direct
        while current_blocks < data_block_num.min(INODE_DIRECT_COUNT) {
            v.push(self.direct[current_blocks as usize]);
            self.direct[current_blocks as usize] = 0;
            current_blocks += 1;
        }
        // indirect1 block
        if data_block_num > INODE_DIRECT_COUNT {
            v.push(self.indirect1);
            data_block_num -= INODE_DIRECT_COUNT;
            current_blocks = 0;
        } else {
            return v;
        }
        // indirect1
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device)).lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                while current_blocks < data_block_num.min(INODE_INDIRECT1_COUNT) {
                    v.push(indirect1[current_blocks as usize]);
                    //indirect1[current_blocks] = 0;
                    current_blocks += 1;
                }
            });
        self.indirect1 = 0;
        // indirect2 block
        if data_block_num > INODE_INDIRECT1_COUNT {
            v.push(self.indirect2);
            data_block_num -= INODE_INDIRECT1_COUNT;
        } else {
            return v;
        }
        // indirect2
        assert!(data_block_num <= INODE_INDIRECT2_COUNT);
        let a1 = data_block_num / INODE_INDIRECT1_COUNT;
        let b1 = data_block_num % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                // full indirect1 blocks
                for entry in indirect2.iter_mut().take(a1 as usize) {
                    v.push(*entry);
                    get_block_cache(*entry as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for entry in indirect1.iter() {
                                v.push(*entry);
                            }
                        });
                }
                // last indirect1 block
                if b1 > 0 {
                    v.push(indirect2[a1 as usize]);
                    get_block_cache(indirect2[a1 as usize] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for entry in indirect1.iter().take(b1 as usize) {
                                v.push(*entry);
                            }
                        });
                }
            });
        self.indirect2 = 0;
        v
    }
}

// ----- DirEntry -----
pub const NAME_LENGTH_LIMIT: u32 = 27; // 最大允许保存长度为 27 的文件/目录名
pub const DIRENT_SIZE: u32 = 32;

/// DirEntry represents a directory entry on disk
/// size: 32 bytes, 16 entries per block
#[repr(C)]
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT as usize + 1], // +1 for `\0` terminator
    inode_number: u32,
}

impl DirEntry {
    // ----- constructor -----
    pub fn new_empty() -> Self {
        Self {
            name: [0; NAME_LENGTH_LIMIT as usize + 1],
            inode_number: 0,
        }
    }

    pub fn new(name: &str, inode_number: u32) -> Self {
        assert!(name.len() <= NAME_LENGTH_LIMIT as usize, "Name too long");
        let mut name_bytes = [0; NAME_LENGTH_LIMIT as usize + 1];
        name_bytes[..name.len()].copy_from_slice(name.as_bytes());
        name_bytes[name.len()] = 0; // null terminator

        Self {
            name: name_bytes,
            inode_number,
        }
    }

    // ----- methods -----
    pub fn get_name(&self) -> &str {
        let null_pos = self.name.iter().position(|&b| b == 0).unwrap_or(NAME_LENGTH_LIMIT as usize);
        core::str::from_utf8(&self.name[..null_pos]).expect("invalid UTF-8 bytes in inode name")
    }
    pub fn get_inode_number(&self) -> u32 {
        self.inode_number
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self as *const _ as usize as *const u8, DIRENT_SIZE as usize)
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(self as *mut _ as usize as *mut u8, DIRENT_SIZE as usize)
        }
    }
}
