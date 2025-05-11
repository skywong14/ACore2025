// os/src/mm/memory_set.rs
// implementation of MapArea and MemorySet

use alloc::vec::Vec;
use crate::config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE_START_ADDRESS, UART0_BASE_ADDR, UART0_SIZE};
use crate::mm::address::{PhyAddr, VirAddr};
use crate::mm::area::{MapArea, MapPermission};
use crate::mm::area::MapType::Identical;
use crate::mm::page_table::{PTEFlags, PageTable};


// ----- MemorySet -----
pub struct MemorySet {
    pub(crate) page_table: PageTable,
    pub(crate) areas: Vec<MapArea>,
}

impl MemorySet {
    // ----- constructor -----
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    // ----- methods -----
    // map a new MapArea to the MemorySet
    // 'data' as the initial data (when map_type is Framed)
    fn map_area(&mut self, mut area: MapArea, data: Option<&[u8]>) {
        println!(
            "[mem] Map area of [{:#x}, {:#x})",
            area.vpn_range.start.0,
            area.vpn_range.end.0,
        );
        area.map_page_table(&mut self.page_table);
        if let Some(data) = data {
            area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(area);
    }
    
    // map_trampoline
    fn map_trampoline(&mut self) {
        unsafe extern "C" {
            fn strampoline();
        }
        println!("[strampoline] [{:#x}, {:#x})", TRAMPOLINE_START_ADDRESS, TRAMPOLINE_START_ADDRESS + PAGE_SIZE);
        self.page_table.map(
            VirAddr::from(TRAMPOLINE_START_ADDRESS).floor(), // VirPageNum
            PhyAddr::from(strampoline as usize).floor(), // PhyPageNum
            PTEFlags::R | PTEFlags::X,
        );
    }

    // create kernel space
    pub fn new_kernel() -> Self {
        unsafe extern "C" {
            fn stext();
            fn etext();
            fn srodata();
            fn erodata();
            fn sdata();
            fn edata();
            fn sbss_with_stack();
            fn ebss();
            fn ekernel();
        }

        let mut result = Self::new_bare();

        // .text (R-X)
        println!("[kernel] Mapping .text section [{:#x}, {:#x})", stext as usize, etext as usize);
        result.map_area(
            MapArea::new_with_address(
                (stext as usize).into(), (etext as usize).into(),
                Identical, MapPermission::R | MapPermission::X
            ), None
        );

        // .rodata (R--)
        println!("[kernel] Mapping .rodata section [{:#x}, {:#x})", srodata as usize, erodata as usize);
        result.map_area(
            MapArea::new_with_address(
                (srodata as usize).into(), (erodata as usize).into(),
                Identical, MapPermission::R
            ), None
        );

        // .data (R-W)
        println!("[kernel] Mapping .data section [{:#x}, {:#x})", sdata as usize, edata as usize);
        result.map_area(
            MapArea::new_with_address(
                (sdata as usize).into(), (edata as usize).into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );

        // .bss (R-W)
        println!("[kernel] Mapping .bss section [{:#x}, {:#x})", sbss_with_stack as usize, ebss as usize);
        result.map_area(
            MapArea::new_with_address(
                (sbss_with_stack as usize).into(), (ebss as usize).into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );

        // allocated
        println!("[kernel] Mapping allocated section [{:#x}, {:#x})", ekernel as usize, MEMORY_END);
        result.map_area(
            MapArea::new_with_address(
                (ekernel as usize).into(), MEMORY_END.into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );

        // memory-mapped registers
        println!("[kernel] Mapping memory-mapped registers [{:#x}, {:#x})", UART0_BASE_ADDR, UART0_BASE_ADDR + UART0_SIZE);
        result.map_area(
            MapArea::new_with_address(
                UART0_BASE_ADDR.into(), (UART0_BASE_ADDR + UART0_SIZE).into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );

        // trampoline
        println!("[kernel] Mapping trampoline");
        result.map_trampoline();

        result
    }

}