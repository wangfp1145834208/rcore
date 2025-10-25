
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 6
    .quad app_0_start
    .quad app_0_name
    .quad app_1_start
    .quad app_1_name
    .quad app_2_start
    .quad app_2_name
    .quad app_3_start
    .quad app_3_name
    .quad app_4_start
    .quad app_4_name
    .quad app_5_start
    .quad app_5_name
    .quad app_5_end

    .global app_0_name
app_0_name:
    .byte 13
    .ascii "00hello_world"
    .global app_1_name
app_1_name:
    .byte 13
    .ascii "01store_fault"
    .global app_2_name
app_2_name:
    .byte 7
    .ascii "02power"
    .global app_3_name
app_3_name:
    .byte 11
    .ascii "03priv_inst"
    .global app_4_name
app_4_name:
    .byte 10
    .ascii "04priv_csr"
    .global app_5_name
app_5_name:
    .byte 11
    .ascii "05task_info"

    .section .data
    .global app_0_start
    .global app_0_end
app_0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/00hello_world.bin"
app_0_end:

    .section .data
    .global app_1_start
    .global app_1_end
app_1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/01store_fault.bin"
app_1_end:

    .section .data
    .global app_2_start
    .global app_2_end
app_2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/02power.bin"
app_2_end:

    .section .data
    .global app_3_start
    .global app_3_end
app_3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/03priv_inst.bin"
app_3_end:

    .section .data
    .global app_4_start
    .global app_4_end
app_4_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/04priv_csr.bin"
app_4_end:

    .section .data
    .global app_5_start
    .global app_5_end
app_5_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/05task_info.bin"
app_5_end:
