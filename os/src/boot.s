# boot.s
    .section .text.entry
    .globl _start
    .align 2
_start:
    # init sp in M-mode
    la      sp,   _stack_top
    j rust_boot 
    
#---------------------
# symbol
.section .bss
.globl _stack_top
_stack_top:
    .space 4096

#---------------------
.section .bss.stack
.globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
.globl boot_stack_top
boot_stack_top:

#---------------------
.section .text.smode
.globl _rust_main
_rust_main:
    j   rust_main