# Machine Timer ISR
.section .text.trap
.globl m_trap_entry
.align 2
m_trap_entry:
    # 换栈指针 mscratch 预设置为指向 time-trap 专用的安全空间
    # 而原始的 `sp` (S-mode 或 U-mode 的栈指针) 被保存在 `mscratch` 中，
    # 避免 M-mode 中断处理污染被打断程序的栈。
    csrrw sp, mscratch, sp
    sd t0, 0*8(sp)
    sd t1, 1*8(sp)
    sd t2, 2*8(sp)

    # we can modify mtimecmp here
    ld t0, 3*8(sp) # address of mtimercmp
    ld t1, 4*8(sp) # timer interval
    ld t2, 0(t0)   # current time
    add t2, t2, t1 # new time
    sd t2, 0(t0)   # set new time

    # raise supervisor software interrupt (set sip.SSIP)
    li t0, 2
    csrw sip, t0

    # restore t-registers
    ld t0, 0*8(sp)
    ld t1, 1*8(sp)
    ld t2, 2*8(sp)

    # 将当前 sp (指向 M-mode 栈) 写入 mscratch CSR (为下一次 M-mode 中断保存 M-mode 栈指针)
    csrrw sp, mscratch, sp

    mret