# 定义文件路径常量
TARGET_DIR := target/riscv64gc-unknown-none-elf/release
TARGET_ELF := $(TARGET_DIR)/ACore
TARGET_BIN := $(TARGET_DIR)/Acore.bin
BOOTLOADER := ../bootloader/rustsbi-qemu.bin
FS_IMG := ../fs-fuse/target/fs.img

LOAD_ADDR_SBI := 0x80200000
LOAD_ADDR_NOSBI := 0x80000000

.PHONY: qemu_sbi
qemu_sbi:
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios $(BOOTLOADER) \
    -device loader,file=$(TARGET_BIN),addr=$(LOAD_ADDR_SBI) \
    -s -S

.PHONY: qemu_nosbi
qemu_nosbi:
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -device loader,file=$(TARGET_BIN),addr=$(LOAD_ADDR_NOSBI) \
    -s -S

.PHONY: qemu_fs_gdb
qemu_fs_gdb:
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -device loader,file=$(TARGET_BIN),addr=$(LOAD_ADDR_NOSBI) \
    -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -s -S

.PHONY: qemu_fs_nogdb
qemu_fs_nogdb:
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -device loader,file=$(TARGET_BIN),addr=$(LOAD_ADDR_NOSBI) \
    -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

.PHONY: gdb
gdb:
	riscv64-unknown-elf-gdb \
    -ex 'file $(TARGET_ELF)' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'

.PHONY: build
build:
	cargo build --release
	rust-objcopy --strip-all $(TARGET_ELF) -O binary $(TARGET_BIN)

.PHONY: run
run: build qemu_fs_nogdb