// os/src/mm/area.rs

use alloc::vec::Vec;
use crate::config::PAGE_SIZE;
use crate::mm::address::{VirAddr, VirPageNum};
use crate::mm::frame_allocator::FrameTracker;
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
    pub(crate) frames: Vec<FrameTracker>,
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
        let mut frames = Vec::new();
        let vpn_range = Range::new(start, end);
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
        Self {
            vpn_range,
            frames,
            map_type,
            map_perm,
        }
    }

    // ----- utils -----
    pub fn frames(&self, i: usize) -> &FrameTracker {
        &self.frames[i]
    }

    // ----- map methods -----
    pub fn map_page_table(&mut self, page_table: &mut PageTable) {
        for (i,vpn) in self.vpn_range.iter().enumerate() {
            let ppn = self.frames(i).get_ppn();
            let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
            page_table.map(vpn, ppn, pte_flags);
        }
    }

    pub fn unmap_page_table(&mut self, page_table: &mut PageTable) {
        for (i,vpn) in self.vpn_range.iter().enumerate() {
            let ppn = self.frames(i).get_ppn();
            page_table.unmap(vpn);
        }
    }


    // ----- other methods -----
    // data: start-aligned but maybe with shorter length
    // assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let data_len = data.len();
        for (i, cur_vpn) in self.vpn_range.iter().enumerate() {
            let ppn = self.frames(i).get_ppn();
            let src = &data[start..data_len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(cur_vpn)
                .unwrap()
                .get_ppn()
                .as_raw_bytes()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= data_len {
                break;
            }
        }
    }
}

