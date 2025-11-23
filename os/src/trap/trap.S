.altmacro

.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm

# 判断是否为内核陷入，如果是的话，则跳转到jump_label
.macro KERNEL_TRAP_JUMP rd, jump_label
    csrr \rd, sstatus
    andi \rd, \rd, 0x100
    bnez \rd, \jump_label
.endm

    .section .text.trampoline
    .globl __all_traps
    .globl __restore
    .align 2

__all_traps:
    # === 用户trap ===
    # 用户trap后，sstatus.SPP==SUPERVISOR，用户态的sp不可访问
    csrrw sp, sscratch, sp
    j __real_trap_entry
__kernel_trap:
    # === 内核trap ===
    addi sp, sp, -37*8  # 内核陷入需要重新分配栈空间

__real_trap_entry:
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
    sd t0, 2*8(sp)  # sp  # 这里存储的可能是用户态的sp，也可能是内核态的sp

    KERNEL_TRAP_JUMP tp, __kernel_hander
    # === 用户陷入 ===
    # load kernel_satp
    ld t0, 34*8(sp)
    # load trap_handler
    ld t2, 36*8(sp)
    # load sp
    ld sp, 35*8(sp)
    # switch to kernel space
    csrw satp, t0
    sfence.vma
    mv a0, x0  # None?
    jalr ra, 0(t2)  # 跳转到trap_handler，并记录返回地址
    j __restore
__kernel_hander:
    # === 内核陷入 ===
    mv a0, sp  # 将TrapContext作为参数(Some(TrapContext)?)
    call trap_handler
    # j __restore  # 可要，可不要

__restore:
    KERNEL_TRAP_JUMP tp, __common_restore
    # === 用户恢复 ===
    # switch to user space
    csrw satp, a1
    sfence.vma
__common_restore:
    # set sp by param
    csrw sscratch, a0
    mv sp, a0
    # start restoring TrapContext
    ld t0, 32*8(sp)  # 获取sstatus
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    LOAD_GP 1
    LOAD_GP 3
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr

    sd tp, -8(sp)
    KERNEL_TRAP_JUMP tp, __kernel_end
    # === 用户恢复 ===
    ld tp, -8(sp)
    ld sp, 2*8(sp)
    sret
__kernel_end:
    # === 内核恢复 ===
    ld tp, -8(sp)
    addi sp, sp, 37*8  # 释放申请的栈
    sret
