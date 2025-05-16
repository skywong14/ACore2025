// os/src/mm/area.rs

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::config::PAGE_SIZE;
use crate::mm::address::{PhyPageNum, VirAddr, VirPageNum};
use crate::mm::frame_allocator::{frame_alloc, FrameTracker};
use crate::mm::page_table::{PTEFlags, PageTable};
use crate::mm::range::Range;

// ----- MapType & MapPermission -----
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

// ----- MapArea -----
// frames 保存 MapArea 使用的所有物理页面，在 MapArea 释放时自动释放物理页。
pub struct MapArea {
    pub(crate) vpn_range: Range<VirPageNum>,
    pub(crate) frames: BTreeMap<VirPageNum, FrameTracker>,  // map_one & unmap_one need Map
    pub(crate) map_type: MapType,
    pub(crate) map_perm: MapPermission,
}

impl MapArea {
    // ----- constructor -----
    pub fn new_with_address(start: VirAddr, end: VirAddr, map_type: MapType, map_perm: MapPermission) -> Self {
        let start_vpn: VirPageNum = start.floor();
        let end_vpn: VirPageNum = end.ceil();
        Self::new_with_pagenum(start_vpn, end_vpn, map_type, map_perm)
    }


    pub fn new_with_pagenum(start: VirPageNum, end: VirPageNum, map_type: MapType, map_perm: MapPermission) -> Self {
        let frames = BTreeMap::new();
        println!("  debug: [map_area] new map area: {:#x} - {:#x}", start.0, end.0); //debug
        let vpn_range = Range::new(start, end);
        /*
        match map_type {
            MapType::Framed => {
                for _ in vpn_range.iter() {
                    frames.push(FrameTracker::new());
                }
            }
            MapType::Identical => {
                for vpn in vpn_range.iter() {
                    frames.push(FrameTracker::from_existed(vpn.into()));
                }
            }
        };
        */
        Self {
            vpn_range,
            frames,
            map_type,
            map_perm,
        }
    }

    // ----- utils -----
    pub fn get_frame(&self, i: VirPageNum) -> &FrameTracker {
        &self.frames[&i]
    }

    // ----- map methods -----
    // single VPN -> PPN
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirPageNum) {
        let ppn: PhyPageNum;
        match self.map_type {
            MapType::Identical => {
                // 无需分配实际物理页
                ppn = PhyPageNum(vpn.0);
            }
            MapType::Framed => {
                // 分配实际物理页，并作记录
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        // println!("[map_area] V -> P {:#x} to {:#x}", vpn.0, ppn.0); //debug
        page_table.map(vpn, ppn, pte_flags);
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirPageNum) {
        if self.map_type == MapType::Framed {
            self.frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }

    pub fn map_page_table(&mut self, page_table: &mut PageTable) {
        for (i,vpn) in self.vpn_range.iter().enumerate() {
            self.map_one(page_table, vpn);
        }
    }

    pub fn unmap_page_table(&mut self, page_table: &mut PageTable) {
        for (i,vpn) in self.vpn_range.iter().enumerate() {
            self.unmap_one(page_table, vpn);
        }
    }
    
    // ----- other methods -----
    // data: start-aligned but maybe with shorter length
    // assume that all frames were cleared before
    // 将给定的 data 按页面拷贝到内存区间对应的物理地址
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let data_len = data.len();
        for cur_vpn in self.vpn_range.iter() {
            let src = &data[start..data_len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(cur_vpn)
                .unwrap() // PageTableEntry
                .get_ppn() // PhyPageNum
                .as_raw_bytes()[..src.len()];
            dst.copy_from_slice(src); // 把这部分数据拷贝到目标物理页
            start += PAGE_SIZE;
            if start >= data_len {
                break;
            }
        }
    }

    // heap area: change brk
    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirPageNum) {
        assert_eq!(self.map_type, MapType::Framed);
        assert!(new_end >= self.vpn_range.start);
        assert!(new_end <= self.vpn_range.end);
        let old_end = self.vpn_range.end;
        self.vpn_range.end = new_end;
        for i in new_end.0..old_end.0 {
            self.unmap_one(page_table, VirPageNum(i)); // unmap all pages
        }
    }
    pub fn grow_to(&mut self, page_table: &mut PageTable, new_end: VirPageNum) {
        assert_eq!(self.map_type, MapType::Framed);
        assert!(new_end >= self.vpn_range.start);
        assert!(new_end <= self.vpn_range.end);
        let old_end = self.vpn_range.end;
        self.vpn_range.end = new_end;
        for i in old_end.0..new_end.0 {
            self.map_one(page_table, VirPageNum(i)); // map all pages
        }
    }
}

