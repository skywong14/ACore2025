.PHONY: qemu_run
qemu_run:
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=target/riscv64gc-unknown-none-elf/release/Acore.bin,addr=0x80200000 \
    -s -S

.PHONY: qemu_nosbi
qemu_nosbi:
	qemu-system-riscv64 \
      -machine virt \
      -nographic \
      -bios none \
      -device loader,file=target/riscv64gc-unknown-none-elf/release/Acore.bin,addr=0x80000000 \
      -s -S

.PHONY: gdb
gdb:
	riscv64-unknown-elf-gdb \
      -ex 'file target/riscv64gc-unknown-none-elf/release/ACore' \
      -ex 'set arch riscv:rv64' \
      -ex 'target remote localhost:1234'

.PHONY: build
build:
	cargo build --release

.PHONY: format
format:
	rust-objcopy --strip-all target/riscv64gc-unknown-none-elf/release/ACore -O binary target/riscv64gc-unknown-none-elf/release/Acore.bin
