// os/src/mm/memory_set.rs
// implementation of MapArea and MemorySet

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::arch::asm;
use lazy_static::lazy_static;
use riscv::register::satp;
use riscv::register::scause::Exception::VirtualInstruction;
use crate::config::{CLINT_BASE, CLINT_SIZE, MEMORY_END, PAGE_SIZE, TEST_DEVICE_ADDR, TRAMPOLINE_START_ADDRESS, TRAP_CONTEXT_ADDRESS, UART0_BASE_ADDR, UART0_SIZE};
use crate::console::print;
use crate::mm::address::{PhyAddr, VirAddr, VirPageNum};
use crate::mm::area::{MapArea, MapPermission};
use crate::mm::area::MapType::{Framed, Identical};
use crate::mm::page_table::{PTEFlags, PageTable, PageTableEntry};
use crate::sync::UPSafeCell;

// ----- MemorySet -----
pub struct MemorySet {
    pub(crate) page_table: PageTable,
    pub(crate) areas: Vec<MapArea>,
}


impl MemorySet {
    // ----- change brk (Heap) -----
    pub fn shrink_to(&mut self, start: VirAddr, new_end: VirAddr) -> bool {
        let start_vpn = start.floor();
        let mut found = false;
        for area in self.areas.iter_mut() {
            // find the Heap Area
            if area.vpn_range.start == start_vpn {
                area.shrink_to(&mut self.page_table, new_end.ceil());
                found = true;
                break;
            }
        }
        found
    }
    
    pub fn grow_to(&mut self, start: VirAddr, new_end: VirAddr) -> bool {
        let mut found = false;
        for area in self.areas.iter_mut() {
            // find the Heap Area
            if area.vpn_range.start == start.floor() {
                area.grow_to(&mut self.page_table, new_end.ceil());
                found = true;
                break;
            }
        }
        found
    }
    
    
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
    pub fn map_area(&mut self, mut area: MapArea, data: Option<&[u8]>) {
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
    // trampoline is at the same place in EVERY memory_set
    fn map_trampoline(&mut self) {
        unsafe extern "C" {
            fn strampoline();
        }
        println!("[strampoline] [{:#x}, {:#x}]", TRAMPOLINE_START_ADDRESS, TRAMPOLINE_START_ADDRESS - 1 + PAGE_SIZE);
        println!("[strampoline] V -> P: {:#x} -> {:#x}", VirAddr::from(TRAMPOLINE_START_ADDRESS).floor().0, PhyAddr::from(strampoline as usize).floor().0);
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
        
        // TEST_DEVICE
        println!("[kernel] Mapping test device [{:#x}, {:#x})", TEST_DEVICE_ADDR, TEST_DEVICE_ADDR + PAGE_SIZE);
        result.map_area(
            MapArea::new_with_address(
                TEST_DEVICE_ADDR.into(), (TEST_DEVICE_ADDR + PAGE_SIZE).into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );

        // UART (Universal Asynchronous Receiver/Transmitter)
        println!("[kernel] Mapping memory-mapped registers (UART) [{:#x}, {:#x})", UART0_BASE_ADDR, UART0_BASE_ADDR + UART0_SIZE);
        result.map_area(
            MapArea::new_with_address(
                UART0_BASE_ADDR.into(), (UART0_BASE_ADDR + UART0_SIZE).into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );
        
        // CLINT (Core Local Interruptor)
        println!("[kernel]  memory-mapped registers (CLINT) [{:#x}, {:#x})", CLINT_BASE, CLINT_BASE + CLINT_SIZE);
        result.map_area(
            MapArea::new_with_address(
                CLINT_BASE.into(), (CLINT_BASE + CLINT_SIZE).into(),
                Identical, MapPermission::R | MapPermission::W
            ), None
        );

        // trampoline
        println!("[kernel] Mapping trampoline");
        result.map_trampoline();

        result
    }

    /*
    ...   // 代码、数据等Load段的空间
    +-----------------------------+
    | ELF段映射结束                 |
    +-----------------------------+  <-  max_end_va
    | Guard Page                  |  // 空页/保护页，不分配物理内存，不可访问，防止栈溢出
    +-----------------------------+  <-  user_stack_bottom
    | User Stack Page             |
    |                             |  // 分配一页，初始sp指向顶部
    |                             |
    +-----------------------------+  <-  user_stack_top  （初始sp = 这里）
    | [未使用空间...]               |
    | ...                         |
     */
    // also returns `user_sp` and `entry point`.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut result = Self::new_bare();

        // trampoline
        result.map_trampoline();

        // headers of elf (U)
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap(); //todo "Did not find ELF magic number"
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count(); // program header count
        let mut max_end_vpn = VirPageNum(0); // 最大结束虚拟页号，用于后续确定用户栈的位置
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            // 只处理 Load 类型的段
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                // 该段的起始和结束
                let start_va: VirAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();

                // 内存访问权限
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read()    { map_perm |= MapPermission::R; }
                if ph_flags.is_write()   { map_perm |= MapPermission::W; }
                if ph_flags.is_execute() { map_perm |= MapPermission::X; }

                // create Area
                let map_area = MapArea::new_with_address(
                    start_va, end_va,
                    Framed, map_perm
                );

                // update max_end_vpn
                max_end_vpn = map_area.vpn_range.end;

                // map area
                result.map_area(
                    map_area,
                    // 只映射该段数据区(文件偏移到偏移+文件大小)
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize])
                );
            }
        }

        // map user stack with U flags
        let max_end_va: VirAddr = max_end_vpn.into(); // ELF 段映射结束
        let mut user_stack_bottom: usize = max_end_va.into();

        // guard page and stack page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top: usize = user_stack_bottom + PAGE_SIZE;
        println!("[user] Mapping user stack [{:#x}, {:#x})", user_stack_bottom, user_stack_top);
        result.map_area(
            MapArea::new_with_address(
                user_stack_bottom.into(), user_stack_top.into(),
                Framed, MapPermission::R | MapPermission::W | MapPermission::U
            ), None
        );

        // map TrapContext
        result.map_area(
            MapArea::new_with_address(
                TRAP_CONTEXT_ADDRESS.into(), TRAMPOLINE_START_ADDRESS.into(),
                Framed, MapPermission::R | MapPermission::W
            ), None
        );

        (result, user_stack_top, elf.header.pt2.entry_point() as usize)
    }

    pub fn activate(&self) {
        let satp_bits = self.page_table.to_satp();
        println!("[kernel] activate satp: {:#x}", satp_bits);
        unsafe {
            satp::write(satp_bits);
            asm!("sfence.vma");
        }
    }
    
    pub fn to_satp(&self) -> usize {
        self.page_table.to_satp()
    }
    
    pub fn translate(&self, vpn: VirPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(unsafe {
        UPSafeCell::new(MemorySet::new_kernel())
    });
}


// test
pub fn remap_test() {
    unsafe extern "C" {
        fn stext();
        fn srodata();
        fn etext();
        fn erodata();
        fn sdata();
        fn edata();
    }

    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space.page_table.translate(mid_text.floor()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_rodata.floor()).unwrap().writable(),
        false,
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_data.floor()).unwrap().executable(),
        false,
    );
    println!("remap_test passed!");
}