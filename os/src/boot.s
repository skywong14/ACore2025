# boot.s
    .section .text.entry
    .globl _start
    .align 2
_start:
    # init sp in M-mode
    la      sp,   _boot_stack_top
    j       rust_boot 

#---------------------
.section .bss.stack
.globl _boot_stack_lower_bound
_boot_stack_lower_bound:
    .space 4096 * 16
.globl _boot_stack_top
_boot_stack_top:

#---------------------
.section .text.smode
.globl _rust_main
_rust_main:
    j   rust_main