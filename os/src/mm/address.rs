// os/src/mm/address.rs

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhyAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhyPageNum(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirPageNum(pub usize);

use core::ops::{self, AddAssign};
use crate::config::{
    PAGE_SIZE,
    PAGE_SIZE_BITS,
    PA_WIDTH,
    PPN_WIDTH,
};
use crate::mm::page_table::PageTableEntry;
use crate::mm::range::Step;

// ----- usize -----
impl From<usize> for PhyAddr {
    fn from(v: usize) -> Self { Self(v & ( (1 << PA_WIDTH) - 1 )) }
}
impl From<usize> for PhyPageNum {
    fn from(v: usize) -> Self { Self(v & ( (1 << PPN_WIDTH) - 1 )) }
}
impl From<PhyAddr> for usize {
    fn from(v: PhyAddr) -> Self { v.0 }
}
impl From<PhyPageNum> for usize {
    fn from(v: PhyPageNum) -> Self { v.0 }
}

impl From<usize> for VirAddr {
    fn from(v: usize) -> Self { Self(v & ( (1 << PA_WIDTH) - 1 )) }
}

impl From<usize> for VirPageNum {
    fn from(v: usize) -> Self { Self(v & ( (1 << PPN_WIDTH) - 1 )) }
}

impl From<VirAddr> for usize {
    fn from(v: VirAddr) -> Self { v.0 }
}

impl From<VirPageNum> for usize {
    fn from(v: VirPageNum) -> Self { v.0 }
}

// ----- Identical -----
impl From<PhyPageNum> for VirPageNum {
    fn from(v: PhyPageNum) -> Self { VirPageNum(v.0) }
}

impl From<VirPageNum> for PhyPageNum {
    fn from(v: VirPageNum) -> Self { PhyPageNum(v.0) }
}

// ----- Addr/PageNum -----

impl From<PhyAddr> for PhyPageNum {
    fn from(v: PhyAddr) -> Self {
        assert_eq!(v.page_offset(), 0); // 物理地址必须位于页的起始
        v.floor()
    }
}

impl From<PhyPageNum> for PhyAddr {
    fn from(v: PhyPageNum) -> Self { Self(v.0 << PAGE_SIZE_BITS) }
}

// ----- methods for Addr -----

impl VirAddr {
    pub fn ceil(&self) -> VirPageNum {
        if self.page_offset() == 0 {
            VirPageNum(self.0 / PAGE_SIZE)
        } else {
            VirPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
        }
    }
    pub fn floor(&self) -> VirPageNum { VirPageNum(self.0 / PAGE_SIZE) }
    pub fn page_offset(&self) -> usize { self.0 & (PAGE_SIZE - 1) }
    pub fn aligned(&self) -> bool { self.page_offset() == 0 }
}

impl PhyAddr {
    pub fn ceil(&self) -> PhyPageNum {
        if self.page_offset() == 0 {
            PhyPageNum(self.0 / PAGE_SIZE)
        } else {
            PhyPageNum((self.0 + PAGE_SIZE - 1) / PAGE_SIZE)
        }
    }
    pub fn floor(&self) -> PhyPageNum { PhyPageNum(self.0 / PAGE_SIZE) }
    pub fn page_offset(&self) -> usize { self.0 & (PAGE_SIZE - 1) }
    pub fn aligned(&self) -> bool { self.page_offset() == 0 }
}

// ----- methods for PageNum -----
impl VirPageNum {
    // convert VPN to three parts
    // 26-18: VPN0, 17-9: VPN1, 8-0: VPN2
    pub fn get_indexes(&self) -> [usize; 3] {
        let mask = (1 << 9) - 1;
        let vpn2 = self.0 & mask;
        let vpn1 = (self.0 >> 9) & mask;
        let vpn0 = (self.0 >> 18) & mask;
        [vpn0, vpn1, vpn2]
    }
}

impl PhyPageNum {
    // 把 物理页 当作 页表项数组，返回 PageTableEntry 数组
    pub fn as_raw_ptes(&self) -> &'static mut [PageTableEntry] {
        let start_ptr = usize::from(*self) as *mut PageTableEntry;
        unsafe {
            core::slice::from_raw_parts_mut(start_ptr, PAGE_SIZE / size_of::<PageTableEntry>())
        }
    }

    // 把 物理页 当作 原始字节数组，返回 u8 数组
    pub fn as_raw_bytes(&self) -> &'static mut [u8] {
        let start_ptr = usize::from(*self) as *mut u8;
        unsafe { core::slice::from_raw_parts_mut(start_ptr, PAGE_SIZE) }
    }
}

// ----- override operators -----
impl AddAssign<usize> for PhyAddr { fn add_assign(&mut self, rhs: usize) { self.0 += rhs; } }
impl AddAssign<usize> for VirAddr { fn add_assign(&mut self, rhs: usize) { self.0 += rhs; } }
impl AddAssign<usize> for PhyPageNum { fn add_assign(&mut self, rhs: usize) { self.0 += rhs; } }
impl AddAssign<usize> for VirPageNum { fn add_assign(&mut self, rhs: usize) { self.0 += rhs; } }

impl Step for PhyAddr { fn step(&mut self) { self.add_assign(1); } }
impl Step for VirAddr { fn step(&mut self) { self.add_assign(1); } }
impl Step for PhyPageNum { fn step(&mut self) { self.add_assign(1); } }
impl Step for VirPageNum { fn step(&mut self) { self.add_assign(1); } }