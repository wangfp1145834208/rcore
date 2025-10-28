.altmacro

.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

    .section .text
    .globl __all_traps
    .globl __restore
    .align 2

__all_traps:
    sd tp, -8(sp)
    csrr tp, sstatus  # 这里其实会破坏tp寄存器的值
    addi tp, tp, 0x100
    bnez tp, __real_trap_entry  # spp不为0时，则无需换栈
    csrrw sp, sscratch, sp

__real_trap_entry:
    ld tp, -8(sp)
    addi sp, sp, -34*8
    SAVE_GP 1  # ra
    SAVE_GP 3
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    csrr t0, sscratch
    sd t0, 2*8(sp)  # sp
    mv a0, sp
    call trap_handler

__restore:
    # mv sp, a0  # sp 通过TaskContext获取，而不再以参数方式传递
    ld t0, 32*8(sp)  # 获取sstatus
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    LOAD_GP 1
    LOAD_GP 3
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    addi sp, sp, 34*8

    sd tp, -8(sp)
    csrr tp, sstatus
    addi tp, tp, 0x100
    bnez tp, __real_trap_end  # spp不为0时，则无需换栈
    csrrw sp, sscratch, sp
__real_trap_end:
    ld tp, -8(sp)
    sret
