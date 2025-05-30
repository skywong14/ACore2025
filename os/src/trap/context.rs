use riscv::register::sstatus;
use riscv::register::sstatus::{Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],      // general regs[0..31]
    pub sstatus: Sstatus,    // CSR sstatus
    pub sepc: usize,         // CSR sepc
    pub kernel_satp: usize,  // kernel satp token (include PA of kernel's page table)
    pub kernel_sp: usize,    // (VA) kernel stack pointer
    pub trap_handler: usize, // (VA) kernel's trap handler pointer, we only jump to it in S mode
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    pub fn app_init_context(entry: usize, sp: usize,
                            kernel_satp: usize, kernel_sp: usize, trap_handler: usize,) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); // previous privilege mode: U
        let mut ctx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,  // entry point of app
            kernel_satp,  // addr of page table
            kernel_sp,    // kernel stack
            trap_handler, // addr of trap_handler function
        };
        ctx.set_sp(sp);
        ctx
    }
}