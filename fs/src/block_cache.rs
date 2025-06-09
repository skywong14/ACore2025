// fs/src/block_cache.rs

use alloc::sync::Arc;
use crate::block_dev::BlockDevice;
use crate::config::BLOCK_SIZE;

const BLOCK_CACHE_SIZE: usize = 16;

// ----- BlockCache -----
// 位于内存缓存中，因此可使用 usize
pub struct BlockCache {
    cache: [u8; BLOCK_SIZE as usize],   // 缓存的块数据
    block_id: usize,                    // 块的ID
    block_device: Arc<dyn BlockDevice>, // 块设备的引用
    modified: bool,                     // 是否被修改过
}

impl BlockCache {
    // ----- constructor -----
    // load a new BlockCache from disk.
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SIZE as usize];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }
    // ----- methods -----
    /// get the memory address of an offset inside the cached block data
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    /// get a reference of an offset
    pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE as usize);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }
    /// get a mutable reference of an offset
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE as usize);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    /// 将块缓存同步到硬盘
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            // 将缓存数据写入块设备，参数为 block_id 和 缓存数据的切片引用（胖指针）
            self.block_device
                .write_block(self.block_id, self.cache.as_ref());
        }
    }

    /// interface for reading data
    /// T: the type of data to read, V: the return type of the closure
    /// `f` is a closure that takes a reference to `T` and returns `V`.
    /// return the result of the closure
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }
    /// interface for modifying data
    /// return the result of the closure
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync() // 在丢弃前需同步到硬盘
    }
}

// ----- BlockCacheManager -----

use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

pub struct BlockCacheManager {
    queue: Vec<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    // ----- constructor -----
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }
    // ----- methods -----
    /// 获取指定块ID的缓存，如果不存在则创建新的缓存
    pub fn get_block_cache(&mut self, block_id: usize, block_device: Arc<dyn BlockDevice>)
        -> Arc<Mutex<BlockCache>> {
        // 在现有缓存中查找指定块ID
        let mut existing_cache = None;
        for pair in &self.queue {
            if pair.0 == block_id {
                existing_cache = Some(Arc::clone(&pair.1));
                break;
            }
        }
        if let Some(cache) = existing_cache {
            return cache; // 找到缓存块，直接返回
        }

        // 如果缓存已满，进行替换
        if self.queue.len() == BLOCK_CACHE_SIZE {
            // 查找只有一个强引用(只被管理器引用)的缓存
            let mut victim_idx = None;
            for i in 0..self.queue.len() {
                if Arc::strong_count(&self.queue[i].1) == 1 {
                    victim_idx = Some(i);
                    break;
                }
            }
            if let Some(idx) = victim_idx {
                self.queue.remove(idx);
            } else {
                // 所有缓存都有多个引用，无法释放
                panic!("Run out of BlockCache!");
            }
        }

        // 创建和添加新的缓存
        let block_cache = Arc::new(Mutex::new(BlockCache::new(
            block_id,
            Arc::clone(&block_device),
        )));
        self.queue.push((block_id, Arc::clone(&block_cache)));

        block_cache
    }
}

lazy_static! {
    /// 全局块缓存管理器实例，使用 Mutex 保护
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

/// 获取指定块ID的块缓存
pub fn get_block_cache(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER.lock().get_block_cache(block_id, block_device)
}

/// 将所有块缓存同步到块设备
pub fn block_cache_sync_all() {
    // 获取管理器锁
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}