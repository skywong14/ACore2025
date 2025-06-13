// os/src/mm/page_table.rs

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::null;
use bitflags::*;
use super::address::{PhyAddr, PhyPageNum, VirAddr, VirPageNum};

use crate::config::{PAGE_SIZE_BITS, PA_WIDTH};
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
    pub bits: usize, // 标志位(低10位) + 物理页号
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
    
    pub fn writable(&self) -> bool { self.get_flags().contains(PTEFlags::W) }
    
    pub fn executable(&self) -> bool { self.get_flags().contains(PTEFlags::X) }
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
        println_gray!("[page_table] Allocated page table frame: {:#x}", frame.ppn.0);
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
                println_gray!("[page_table] Allocated page table frame in 'create_entry': {:#x}", frame.ppn.0);
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
    // from_satp_token() will create a new page table, then we can use translate() to look up a PTE by VPN
    pub fn from_satp_token(satp: usize) -> Self {
        Self {
            root_ppn: PhyPageNum(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    // VPN -> PTE
    pub fn translate_vpn(&self, vpn: VirPageNum) -> Option<PageTableEntry> {
        let entry_option = self.find_entry(vpn);
        if let Some(pte) = entry_option {
            Some(pte.clone())
        } else {
            None
        }
    }
    // VA -> PA
    pub fn translate_va(&self, va: VirAddr) -> Option<PhyAddr> {
        // 获取虚拟地址所在的虚拟页号
        let vpn = va.clone().floor();
        // 对应的页表项
        let entry_option = self.find_entry(vpn);

        if let Some(pte) = entry_option {
            // 获取物理页号并转换为页对齐的物理地址
            let aligned_pa: PhyAddr = pte.get_ppn().into();
            let offset = va.page_offset(); // 页内的偏移量
            // 最终物理地址（页对齐地址 + 偏移量）
            let physical_addr = (usize::from(aligned_pa) + offset).into();
            Some(physical_addr)
        } else {
            None
        }
    }
}

// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(satp_token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let mut result = Vec::new();
    let page_table = PageTable::from_satp_token(satp_token);
    let mut start = ptr as usize;
    let end = start + len;
    
    while start < end {
        let start_va = VirAddr::from(start);
        let mut start_vpn = start_va.floor();
        let ppn = page_table.translate_vpn(start_vpn).unwrap().get_ppn();
        start_vpn.0 += 1; // next page
        let mut end_va: VirAddr = start_vpn.into();
        end_va = end_va.min(VirAddr::from(end));
        if end_va.page_offset() == 0 {
            // aligned, copy a whole page
            result.push(&mut ppn.as_raw_bytes()[start_va.page_offset()..]);
        } else {
            // unaligned
            result.push(&mut ppn.as_raw_bytes()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    result
}

// 根据 ptr 找到要执行的应用名，返回 String
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_satp_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let pa = page_table.translate_va(VirAddr::from(va)).unwrap();
        let ch: u8 = unsafe { *(pa.0 as *const u8) };
        if ch == 0 {
            break; // 结束符
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

// translate a ptr and return a mutable reference
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_satp_token(token);
    let va = ptr as usize;
    let phys_addr = page_table.translate_va(VirAddr::from(va)).unwrap();
    unsafe {
        (phys_addr.0 as *mut T).as_mut().unwrap()
    }
}

// ----- User Buffer -----
pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for slice in self.buffers.iter() {
            total += slice.len();
        }
        total
    }
}