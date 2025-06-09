// fs/src/inode.rs

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::disk_inode::{DirEntry, DiskInode, DiskInodeType, DIRENT_SIZE};
use crate::efs::EasyFileSystem;


// ----- Memory Inode -----

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    // ----- constructor -----
    pub fn new(block_id: u32, block_offset: usize, fs: Arc<Mutex<EasyFileSystem>>, block_device: Arc<dyn BlockDevice>)
        -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    // ----- methods -----

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(
            self.block_id,
            Arc::clone(&self.block_device)
        ).lock().read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(
            self.block_id,
            Arc::clone(&self.block_device)
        ).lock().modify(self.block_offset, f)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        assert!(disk_inode.is_dir());
        // 目录中条目数量
        let file_count = disk_inode.size / DIRENT_SIZE;
        let mut dirent = DirEntry::new_empty(); // 用于读取
        // 遍历目录中的每个条目
        for i in 0..file_count {
            // 从 disk inode 中读取目录条目数据
            assert_eq!(
                disk_inode.read_at(
                    (DIRENT_SIZE * i) as usize,
                    dirent.as_bytes_mut(),
                    &self.block_device,
                ) as u32,
                DIRENT_SIZE
            );
            if dirent.get_name() == name {
                // 若找到
                return Some(dirent.get_inode_number());
            }
        }
        None
    }

    /// find an inode by name in the current directory inode(root inode)
    pub fn find_inode(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode: &DiskInode| {
            // 尝试在目录中查找指定名称的文件的 inode ID
            let inode_id_option = self.find_inode_id(name, disk_inode);
            if inode_id_option.is_none() {
                return None;
            }
            // 找到 inode ID
            let inode_id = inode_id_option.unwrap();
            // 获取 inode 在磁盘上的位置(块ID, 块内偏移)
            let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
            // 创建一个新的 Inode 实例并返回
            Some(Arc::new(Self::new(
                block_id,
                block_offset,
                self.fs.clone(),
                self.block_device.clone(),
            )))
        })
    }


    /// ls, only root inode can use it
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = disk_inode.size / DIRENT_SIZE;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::new_empty();
                assert_eq!(
                    disk_inode.read_at(
                        (i * DIRENT_SIZE) as usize, // offset
                        dirent.as_bytes_mut(),      // slice
                        &self.block_device,
                    ) as u32,
                    DIRENT_SIZE,
                );
                v.push(String::from(dirent.get_name()));
            }
            v
        })
    }

    /// create, only root inode can use it
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();

        // 检查同名文件
        if self.modify_disk_inode(|root_inode| {
            assert!(root_inode.is_dir());
            self.find_inode_id(name, root_inode)
        }).is_some() {
            return None;
        }

        // 分配新的 inode
        let new_inode_id = fs.alloc_inode();

        // 获取新 inode 在磁盘上的位置(块ID, 块内偏移)
        let (new_inode_block_id, new_inode_block_offset)
            = fs.get_disk_inode_pos(new_inode_id);

        // 初始化新的 inode 为 File Type
        get_block_cache(
            new_inode_block_id as usize,
            Arc::clone(&self.block_device)
        ).lock().modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
            // 将新inode初始化为文件类型
            new_inode.initialize(DiskInodeType::File);
        });

        // 修改当前目录inode，添加新文件的目录项
        self.modify_disk_inode(|root_inode| {
            let file_count = root_inode.size / DIRENT_SIZE;
            // 添加新条目后的目录大小
            let new_size = (file_count + 1) * DIRENT_SIZE;
            // 扩容
            self.increase_size(new_size, root_inode, &mut fs);
            // 创建并写入目录
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                (file_count * DIRENT_SIZE) as usize,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        // 获取新创建的inode在磁盘上的位置
        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);

        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    /// 清空文件内容并释放文件占用的数据块
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert_eq!(data_blocks_dealloc.len(), DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data_block(data_block);
            }
        });
    }

    /// 从文件的指定偏移位置读取数据到缓冲区，实质上是 disk inode 的读取操作
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.read_at(offset, buf, &self.block_device)
        })
    }

    /// 写入数据到 inode 的指定偏移处，实质上是 disk inode 的写入操作
    // 注意: write_at 之前先调用 increase_size 扩容
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }

    /// 增加文件大小，必要时分配新的数据块
    fn increase_size(&self, new_size: u32, disk_inode: &mut DiskInode, fs: &mut MutexGuard<EasyFileSystem>) {
        if new_size < disk_inode.size {
            return; // 无需扩容
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        // 分配所需的数据块
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data_block());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
}