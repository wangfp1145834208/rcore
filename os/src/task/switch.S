.altmacro
.macro SAVE_SN n
    sd s\n, (\n+2)*8(a0)
.endm
.macro LOAD_SN n
    ld s\n, (\n+2)*8(a1)
.endm

    .section .text
    .globl __switch
__switch:
    # 保存当前任务上下文
    sd ra, 0(a0)
    sd sp, 8(a0)
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n+1
    .endr

    # 加载下一个任务的上下文
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n+1
    .endr
    ld ra, 0(a1)
    ld sp, 8(a1)
    ret
