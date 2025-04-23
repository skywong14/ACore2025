use riscv::register::sstatus;
use riscv::register::sstatus::{Sstatus, SPP};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32], // general regs[0..31]
    pub sstatus: Sstatus, // CSR sstatus
    pub sepc: usize, // CSR sepc
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    
    // Self: the type of the struct itself
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); // previous privilege mode: U
        let mut ctx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry, // entry point of app
        };
        ctx.set_sp(sp);
        ctx
    }
}