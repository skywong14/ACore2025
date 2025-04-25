# boot.s
    .section .text.init
    .globl _start
    .align 2
_start:
    # init sp in M-mode
    la      sp,   _stack_top

    # 关闭分页
    li      t0,   0
    csrw    satp, t0

    # 设置 PMP 允许全物理访问
    li      t0,  -1
    csrw    pmpaddr0, t0
    li      t0,   0xf
    csrw    pmpcfg0,  t0

    # 全委托给 S-mode (包括 S-mode 的 ecall)
    li      t0,   0xffff
    csrw    medeleg,  t0
    li      t0,   0xffff
    csrw    mideleg,  t0

    # M-mode trap handler (deadloop)
    la      t0,   m_trap_deadloop
    csrw    mtvec, t0

    # switch to S-mode
    # mstatus.MPP = S-mode (1)
    csrr    t0,  mstatus
    li      t1,  ~(3<<11)
    and     t0,  t0, t1
    li      t1,  (1<<11)
    or      t0,  t0, t1
    csrw    mstatus, t0

    # set mepc = entry of S-mode (_rust_main)
    la      t0,   _rust_main
    csrw    mepc, t0

    # jump to S-mode (_rust_main)
    mret

#---------------------
# may not be used
.section .text.trap
.globl m_trap_deadloop
.align 2
m_trap_deadloop:
    wfi
    j m_trap_deadloop

#---------------------
# symbol
.section .bss
.globl _stack_top
_stack_top:
    .space 4096

#---------------------
.section .text.smode
.globl _rust_main
_rust_main:
    j   rust_main