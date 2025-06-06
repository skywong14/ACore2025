// os/src/task/task.rs

use alloc::vec::Vec;
use alloc::sync::{Arc, Weak};
use core::cell::RefMut;
use crate::mm::address::{PhyPageNum, VirAddr};
use crate::mm::KERNEL_SPACE;
use crate::mm::memory_set::MemorySet;
use crate::config::TRAP_CONTEXT_ADDRESS;
use crate::sync::UPSafeCell;
use crate::task::pid::{pid_alloc, KernelStack, PidHandle};
use crate::trap::{trap_handler, TrapContext};
use super::TaskContext;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

// ----- Task Control Block Inner -----
pub struct TaskControlBlockInner {
    pub task_status: TaskStatus,  // 任务状态
    pub task_ctx: TaskContext,    // TaskContext 实例

    pub memory_set: MemorySet,    // 地址空间
    pub trap_ctx_ppn: PhyPageNum, // 应用地址空间次高页的 Trap 上下文被实际存放在物理页帧的物理页号
    pub base_size: usize,         // 应用数据仅能出现在应用地址空间低于 base_size 字节的区域中
    pub heap_bottom: usize,       // 进程的 heap 区起始虚拟地址
    pub program_brk: usize,       // 进程的 heap 区末端虚拟地址

    pub exit_code: i32,           // 进程退出码
    pub parent: Option<Weak<TaskControlBlock>>, // 父进程的 Weak 引用
    pub children: Vec<Arc<TaskControlBlock>>,   // 子进程的强引用列表
}

impl TaskControlBlockInner {
    pub fn get_user_satp(&self) -> usize {
        self.memory_set.to_satp()
    }
    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.as_mut()
    }
    pub fn is_zombie(&self) -> bool {
        self.task_status == TaskStatus::Zombie
    }
}

// ----- Task Control Block -----
pub struct TaskControlBlock {
    // immutable
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    // mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.inner_exclusive_access().trap_ctx_ppn.as_mut()
    }

    pub fn to_satp(&self) -> usize {
        self.inner_exclusive_access().memory_set.to_satp()
    }

    // ----- new, exec, fork -----
    pub fn new_from_elf(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // user trap 存放上下文的物理页
        let trap_ctx_ppn = memory_set.translate(VirAddr::from(TRAP_CONTEXT_ADDRESS).into()) // PageTableEntry
            .unwrap().get_ppn();

        println!("[TCB] a new TCB from elf data, entry_point = {:#x}, user_sp = {:#x}, trap_ctx_ppn = {:#x}",
                 entry_point, user_sp, trap_ctx_ppn.0);

        // alloc a pid and a kernel stack in kernel space
        // 与之前不同的是，我们通过调用 KernelStack::new 来新建内核栈
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_kernel_top();

        let inner = TaskControlBlockInner {
            task_status: TaskStatus::Ready,
            // 任务上下文，设置切换回 trap_return，初始时 sp 为 kernel_stack_top [注意: 这里是内核栈顶]
            task_ctx: TaskContext::goto_trap_return(kernel_stack_top),

            memory_set,         // 进程的内存空间布局
            trap_ctx_ppn,       // 存放上下文的物理页
            base_size: user_sp, // 数据不可能超过 user_sp (用户栈顶)
            heap_bottom: user_sp,
            program_brk: user_sp,

            exit_code: 0,
            parent: None,
            children: Vec::new(),
        };

        let task_control_block = Self {
            pid: pid_handle, // 分配一个新的 PID
            kernel_stack, // 分配内核栈
            inner: unsafe { UPSafeCell::new(inner) }, // 内部数据结构
        };

        // 在 TrapContext 存入用户进程初始化上下文
        // trap_ctx 是 TrapContext 的可变引用
        let trap_ctx = task_control_block.get_trap_ctx();
        *trap_ctx = TrapContext::app_init_context(
            entry_point,                               // 用户程序入口地址
            user_sp,                                   // 用户栈指针
            KERNEL_SPACE.exclusive_access().to_satp(), // kernel satp
            kernel_stack_top,                          // 内核栈顶 (切回用户态时保存)
            trap_handler as usize,                     // trap_handler 地址
        );

        task_control_block
    }

    pub fn exec(&self, elf_data: &[u8]) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);

        // user trap 存放上下文的物理页
        let trap_ctx_ppn = memory_set.translate(VirAddr::from(TRAP_CONTEXT_ADDRESS).into()) // PageTableEntry
            .unwrap().get_ppn();

        let mut inner = self.inner_exclusive_access();

        // 替换当前任务的地址空间为新程序的地址空间
        // 丢弃原有 memory_set 的同时，内部的物理页会自动释放 
        inner.memory_set = memory_set;

        // 更新 trap_ctx_ppn
        inner.trap_ctx_ppn = trap_ctx_ppn;

        let trap_ctx = inner.get_trap_ctx();

        *trap_ctx = TrapContext::app_init_context(
            entry_point,                               // 用户程序入口地址
            user_sp,                                   // 用户栈指针
            KERNEL_SPACE.exclusive_access().to_satp(), // kernel satp
            self.kernel_stack.get_kernel_top(),        // 内核栈顶 (切回用户态时保存)
            trap_handler as usize,                     // trap_handler 地址
        );
        // 函数结束时自动释放 inner
    }

    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        // 获取父进程 TCB Inner 的独占访问权，
        let mut parent_inner = self.inner_exclusive_access();

        // copy memory_set
        let memory_set = MemorySet::new_from_another_user(&parent_inner.memory_set);

        // 子进程 trap_ctx 的物理页号
        let trap_ctx_ppn = memory_set
            .translate(VirAddr::from(TRAP_CONTEXT_ADDRESS).into()).unwrap().get_ppn();

        // 为子进程分配新的 PID 和 KernelStack
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_kernel_top();

        // 创建子进程的 TaskControlBlock
        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    task_status: TaskStatus::Ready,
                    task_ctx: TaskContext::goto_trap_return(kernel_stack_top),

                    memory_set,                            // 子进程的内存空间
                    trap_ctx_ppn,                          // 子进程的 trap 上下文物理页号
                    base_size: parent_inner.base_size,     // 继承
                    heap_bottom: parent_inner.heap_bottom, // 继承
                    program_brk: parent_inner.program_brk, // 继承

                    exit_code: 0,                           // 初始退出码为 0
                    parent: Some(Arc::downgrade(self)),     // 父进程为当前进程, downgrade from Arc to Weak
                    children: Vec::new(),                   // 初始化为空
                })
            },
        });

        // 添加到父进程的子进程列表
        parent_inner.children.push(task_control_block.clone());

        // 修改子进程 trap_ctx 中的内核栈指针
        // 确保子进程 trap 时使用自己的内核栈
        let trap_ctx = task_control_block.inner_exclusive_access().get_trap_ctx();
        trap_ctx.kernel_sp = kernel_stack_top;

        task_control_block
        // 函数结束时自动释放父子进程 TCB 的独占访问
    }


    // change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        None
        // todo
        // let old_brk = self.program_brk;
        // let new_brk = self.program_brk as isize + size as isize; // size may be negative!
        // 
        // // 下界安全性检查
        // if new_brk < self.heap_bottom as isize {
        //     return None;
        // }
        // 
        // // grow_to / shrink_to, 调整 heap 区
        // let result = if size < 0 {
        //     self.memory_set.shrink_to(VirAddr(self.heap_bottom), VirAddr(new_brk as usize))
        // } else {
        //     self.memory_set.grow_to(VirAddr(self.heap_bottom), VirAddr(new_brk as usize))
        // };
        // // success, or not
        // if result {
        //     self.program_brk = new_brk as usize;
        //     Some(old_brk)
        // } else {
        //     None
        // }
    }
}

