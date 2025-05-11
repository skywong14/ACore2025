// os/src/mm/page_table.rs

use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;
use super::address::{PhyPageNum, VirPageNum};

use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS, PA_WIDTH, VA_WIDTH};
use crate::mm::frame_allocator::FrameTracker;

// bit0-7: PTEFlags
// bit8/bit9: RSW (Reserved for software)
const PTE_FLAG_WIDTH: usize = 10;
const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;

bitflags! {
    // page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0; // validity, must be 1
        const R = 1 << 1; // readability
        const W = 1 << 2; // writability
        const X = 1 << 3; // executable
        const U = 1 << 4; // user's accessibility
        const G = 1 << 5; // global (if 1, this page won't be flushed from TLB)
        const A = 1 << 6; // accessed (R/W/X)
        const D = 1 << 7; // dirty (whether a page has been modified)
    }
}

// ----- PageTableEntry -----

#[derive(Clone, Copy)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}


impl PageTableEntry {
    // ----- constructor -----
    pub fn new(ppn: PhyPageNum, flags: PTEFlags) -> Self {
        PageTableEntry { bits: ppn.0 << 10 | flags.bits as usize, }
    }
    pub fn empty() -> Self { PageTableEntry { bits: 0, } }
    
    // ----- methods -----
    pub fn get_ppn(&self) -> PhyPageNum {
        let mut bits = self.bits;
        bits = bits >> PTE_FLAG_WIDTH;
        bits = bits & ((1 << PPN_WIDTH) - 1);
        PhyPageNum(bits)
    }

    pub fn set_ppn(&mut self, ppn: PhyPageNum) {
        let flags = self.bits & ((1 << PTE_FLAG_WIDTH) - 1);
        self.bits = ppn.0 << PTE_FLAG_WIDTH | flags;
    }

    pub fn set_flags(&mut self, flags: PTEFlags) {
        let ppn = self.get_ppn();
        self.bits = ppn.0 << PTE_FLAG_WIDTH | flags.bits as usize;
    }
    
    pub fn get_flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.get_flags().contains(PTEFlags::V)
    }
}

// ----- PageTable -----
use super::frame_allocator::frame_alloc;

pub struct PageTable {
    root_ppn: PhyPageNum,
    frames: Vec<FrameTracker>,
}

impl PageTable {
    // ----- constructor -----
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        println!("[page_table] Allocated page table frame: {:#x}", frame.ppn.0);
        Self {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    // ----- methods -----

    // Find the PTE with given VPN, create new PTE when necessary
    // we might modify PageTable itself
    fn create_entry(&mut self, vpn: VirPageNum) -> &mut PageTableEntry {
        let indexes = vpn.get_indexes(); // vpn2, vpn1, vpn0
        let mut ptes = self.root_ppn.as_raw_ptes();
        for i in 0..=2 {
            let current_pte = &mut ptes[indexes[i]];
            if i == 2 {
                return current_pte;
            }
            if !current_pte.is_valid() {
                let frame = frame_alloc().unwrap();
                println!("[page_table] Allocated page table frame in 'create_entry': {:#x}", frame.ppn.0);
                current_pte.set_ppn(frame.ppn);
                current_pte.set_flags(PTEFlags::V);
                self.frames.push(frame);
            }
            ptes = current_pte.get_ppn().as_raw_ptes(); // when i == 0/1, go to the next page table
        }
        unreachable!();
    }

    // Find the PTE with given VPN, return None if not found
    fn find_entry(&self, vpn: VirPageNum) ->  Option<&mut PageTableEntry> {
        let indexes = vpn.get_indexes(); // vpn2, vpn1, vpn0
        let mut ptes = self.root_ppn.as_raw_ptes();
        for i in 0..=2 {
            let current_pte = &mut ptes[indexes[i]];
            if !current_pte.is_valid() {
                return None;
            }
            if i == 2 {
                return Some(current_pte);
            }
            ptes = current_pte.get_ppn().as_raw_ptes(); // when i == 0/1, go to the next page table
        }
        unreachable!();
    }

    // map a virtual page to a physical page
    pub fn map(&mut self, vpn: VirPageNum, ppn: PhyPageNum, flags: PTEFlags) {
        let pte = self.create_entry(vpn); // the final PTE
        pte.set_ppn(ppn);
        pte.set_flags(flags | PTEFlags::V);
    }

    // unmap
    pub fn unmap(&mut self, vpn: VirPageNum) {
        if let Some(pte) = self.find_entry(vpn) {
            pte.set_flags(PTEFlags::empty()); // 重置页表项 
        } else {
            panic!("[page_table] unmap failed: {:#x}", vpn.0);
        }
    }

    // satp-register, the value of MODE & PPN
    pub fn to_satp(&self) -> usize {
        let ppn = self.root_ppn.0;
        let mode = 8; // Sv39
        (mode << 60) | ppn 
    }

    // temporarily used to get arguments from user space
    // from_token() will create a new page table, then we can use translate() to look up a PTE by VPN
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhyPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    pub fn translate(&self, vpn: VirPageNum) -> Option<PageTableEntry> {
        self.find_entry(vpn).map(|pte| {pte.clone()})
    }
}