// os/src/task/task.rs

use crate::mm::address::{PhyPageNum, VirAddr};
use crate::mm::KERNEL_SPACE;
use crate::mm::area::{MapArea, MapPermission};
use crate::mm::memory_set::MemorySet;
use crate::config::{kernel_stack_position, TRAP_CONTEXT_ADDRESS};
use crate::mm::area::MapType::Framed;
use crate::trap::{trap_handler, TrapContext};
use super::TaskContext;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_ctx: TaskContext,

    pub memory_set: MemorySet,    // 地址空间
    pub trap_ctx_ppn: PhyPageNum, // 应用地址空间次高页的 Trap 上下文被实际存放在物理页帧的物理页号
    pub base_size: usize,         // 用户栈顶的虚拟地址
    pub heap_bottom: usize,       // 进程的 heap 区起始虚拟地址
    pub program_brk: usize,       // 进程的 heap 区末端虚拟地址
}

impl TaskControlBlock {

    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.as_mut()
    }

    pub fn to_satp(&self) -> usize {
        self.memory_set.to_satp()
    }

    // create a new TCB from ELF
    pub fn from_elf(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // user trap 存放上下文的物理页
        let trap_ctx_ppn = memory_set.translate(VirAddr::from(TRAP_CONTEXT_ADDRESS).into()) // PageTableEntry
            .unwrap().get_ppn();

        println!("[debug TCB] app_id = {}, entry_point = {:#x}, user_sp = {:#x}, trap_ctx_ppn = {:#x}", app_id, entry_point, user_sp, trap_ctx_ppn.0);

        let task_status = TaskStatus::Ready;

        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);

        // 为该任务分配一块内核栈，并映射为读写权限
        KERNEL_SPACE.exclusive_access().map_area(
                MapArea::new_with_address(
                    kernel_stack_bottom.into(),
                    kernel_stack_top.into(),
                    Framed,
                    MapPermission::R | MapPermission::W,
                ), None
            );

        // construct TCB
        let task_control_block = Self {
            task_status,        // Ready
            // 任务上下文，设置切换回 trap_return，初始时 sp=kernel_stack_top
            task_ctx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,         // 进程的内存空间布局
            trap_ctx_ppn,       // 存放上下文的物理页
            base_size: user_sp, // 用户栈指针，可视为内存基线
            heap_bottom: user_sp,
            program_brk: user_sp,
        };

        // 在 TrapContext 存入用户进程初始化上下文
        // trap_ctx 是 TrapContext 的可变引用
        let trap_ctx = task_control_block.get_trap_ctx();
        *trap_ctx = TrapContext::app_init_context(
            entry_point,                               // 用户程序入口地址
            user_sp,                                   // 用户栈指针
            KERNEL_SPACE.exclusive_access().to_satp(), // 内核地址空间 satp
            kernel_stack_top,                          // 内核栈顶 (切回用户态时保存)
            trap_handler as usize,                     // trap_handler 地址
        );

        task_control_block
    }

    // change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_brk = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize; // size may be negative!

        // 下界安全性检查
        if new_brk < self.heap_bottom as isize {
            return None;
        }

        // grow_to / shrink_to, 调整 heap 区
        let result = if size < 0 {
            self.memory_set.shrink_to(VirAddr(self.heap_bottom), VirAddr(new_brk as usize))
        } else {
            self.memory_set.grow_to(VirAddr(self.heap_bottom), VirAddr(new_brk as usize))
        };
        // success, or not
        if result {
            self.program_brk = new_brk as usize;
            Some(old_brk)
        } else {
            None
        }
    }
}