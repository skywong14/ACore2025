// fs/src/bitmap.rs

use alloc::sync::Arc;
use crate::block_cache::get_block_cache;
use crate::block_dev::BlockDevice;
use crate::config::BLOCK_SIZE;

const BLOCK_BITS: usize = (BLOCK_SIZE * 8) as usize; // 512 * 8 = 4096 bits


// ----- Bitmap -----
// 每个位图都由若干个块组成，每个块大小为 512 bytes(4096 bits)

// 磁盘数据结构
type BitmapBlock = [u64; 64]; // 将位图区域中的一个磁盘块解释为长度为 64 的一个 u64 数组 (4096 bits)

/*
整个位图(block_num 个块):
[块0][块1][块2]...[块num-1]

每个块:
[u64_0][u64_1][u64_2]...[u64_63]

每个 u64(每一位对应一个块的使用状态):
[bit_0][bit_1][bit_2]...[bit_63]
 */
/// 内存中的位图结构体
pub struct Bitmap {
    start_block_id: usize,
    block_num: usize,
}

impl Bitmap {
    // ----- constructor -----
    pub fn new(start_block_id: usize, block_num: usize) -> Self {
        Self {
            start_block_id,
            block_num,
        }
    }
    // ----- methods -----
    /// 从位图中分配一位（一个空闲块）
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for index in 0..self.block_num {
            let block_cache = get_block_cache(
                index + self.start_block_id,
                block_device.clone()
            );
            // 取出并修改，视作 BitmapBlock
            let pos = block_cache.lock().modify(
                0, 
                |bitmap_block: &mut BitmapBlock| {
                let mut result = None;
                // 遍历位图块中的每个 u64
                for bits64_pos in 0..bitmap_block.len() {
                    let bits64 = bitmap_block[bits64_pos];

                    // 如果找到一个非满的 u64
                    if bits64 != u64::MAX {
                        // 计算第一个为 0 的位的位置
                        let inner_pos = bits64.trailing_ones() as usize;

                        // 修改位图，将该位置为 1
                        bitmap_block[bits64_pos] |= 1u64 << inner_pos;

                        // 计算全局位图中的位置
                        let pos = index * BLOCK_BITS + bits64_pos * 64 + inner_pos;
                        result = Some(pos);

                        break;
                    }
                }
                result
            });
            // 如果找到了可用位置，则返回
            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    /// 释放位图中的指定位
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, pos: usize) {
        let block_pos = pos / BLOCK_BITS;     // 块位置
        let bit_in_block = pos % BLOCK_BITS;  // 块内偏移
        let bits64_pos = bit_in_block / 64;   // 对应块的第几个 u64
        let inner_pos = bit_in_block % 64;    // u64 内第几个位

        // 计算物理块编号
        let physical_block_id = block_pos + self.start_block_id;

        // 获取块缓存并修改位图
        get_block_cache(
            physical_block_id,
            Arc::clone(block_device)
        )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // 检查位是否已设置
                let bit_mask = 1u64 << inner_pos;
                let is_set = bitmap_block[bits64_pos] & bit_mask > 0;

                // 确保要释放的位确实已被分配
                assert!(is_set, "Trying to deallocate an unallocated bit");

                bitmap_block[bits64_pos] &= !bit_mask;
            });
    }
    
    /// 获取位图表示数据的最大数量
    pub fn maximum(&self) -> usize {
        self.block_num * BLOCK_BITS
    }
}