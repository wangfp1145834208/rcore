TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm

KERNEL_ENTRY_PA := 0x80200000

ifeq ($(MODE), release)
	MODE_ARGS := --release
endif

OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

DISASM ?= -x

kernel:
	@cargo build $(MODE_ARGS)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

build: link_app $(KERNEL_BIN)

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

QEMU_ARGS := -machine virt \
			 -nographic \
			 -bios ../bootloader/rustsbi-qemu.bin \
			 -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)

run: run-inner

run-inner: build
	@qemu-system-riscv64 ${QEMU_ARGS}

gdbserver: build
	@qemu-system-riscv64 $(QEMU_ARGS) -s -S

gdbclient:
	@riscv64-unknown-elf-gdb \
		-ex 'file $(KERNEL_ELF)' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'

link_app:
	@rustc build.rs && ./build

.PHONY: kernel build disasm run run-inner gdbserver gdbclient link_app
