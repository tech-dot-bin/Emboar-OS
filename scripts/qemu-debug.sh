#!/bin/bash
# Debug kernel with QEMU + GDB

set -e

KERNEL_BIN="target/release/kernel.bin"
KERNEL_ELF="target/release/kernel.elf"

if [ ! -f "$KERNEL_BIN" ]; then
    echo "Building kernel..."
    cargo build --release --package emboar-kernel
fi

echo "Starting QEMU with GDB support..."
qemu-system-x86_64 \
    -kernel "$KERNEL_BIN" \
    -m 512M \
    -serial stdio \
    -gdb tcp::1234 \
    -S \
    &

QEMU_PID=$!
echo "QEMU PID: $QEMU_PID"

# Give QEMU time to start
sleep 2

echo "Starting GDB..."
gdb -ex "file $KERNEL_ELF" \
    -ex "target remote :1234" \
    -ex "break kernel_main" \
    -ex "continue"

# Cleanup
kill $QEMU_PID 2>/dev/null || true