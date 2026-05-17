# ============================================================================
# Emboar OS Build System
# 
# Targets:
#   make boot         Build bootloader (Stage 1 + Stage 2)
#   make kernel       Build kernel using Cargo
#   make all          Build entire system
#   make clean        Remove build artifacts
#   make run          Run in QEMU emulator
#   make image        Create bootable disk image
#
# File: Makefile
# ============================================================================

.PHONY: all boot kernel clean run image help

# Build configuration
CROSS_COMPILE     ?=
CC                := $(CROSS_COMPILE)gcc
AS                := $(CROSS_COMPILE)as
LD                := $(CROSS_COMPILE)ld
NASM              := nasm
CARGO             := cargo
QEMU              := qemu-system-x86_64

# Directories
BUILD_DIR         := build
BOOTLOADER_DIR    := src/bootloader
KERNEL_DIR        := src/kernel
TOOLS_DIR         := tools

# Build outputs
STAGE1_BIN        := $(BUILD_DIR)/stage1.bin
STAGE2_BIN        := $(BUILD_DIR)/stage2.bin
BOOTLOADER_BIN    := $(BUILD_DIR)/bootloader.bin
KERNEL_BIN        := $(BUILD_DIR)/kernel.bin
DISK_IMAGE        := $(BUILD_DIR)/emboar.img

# Compiler flags
NASM_FLAGS        := -f bin -i$(BOOTLOADER_DIR)/
CFLAGS            := -nostdlib -no-pie -m64 -fno-pie -z max-page-size=0x1000
CFLAGS            += -O2 -Wall -Wextra
LDFLAGS           := -nostdlib -n
CARGO_FLAGS       := --release --target x86_64-unknown-linux-gnu

# ============================================================================
# BUILD TARGETS
# ============================================================================

all: bootloader kernel
	@echo "✓ Build complete: $(BOOTLOADER_BIN) $(KERNEL_BIN)"

help:
	@echo "Emboar OS Build System"
	@echo ""
	@echo "Targets:"
	@echo "  make boot     - Build bootloader"
	@echo "  make kernel   - Build kernel"
	@echo "  make all      - Build everything"
	@echo "  make clean    - Remove build artifacts"
	@echo "  make run      - Run in QEMU"
	@echo "  make image    - Create bootable disk image"
	@echo ""

$(BUILD_DIR):
	@mkdir -p $(BUILD_DIR)

# ============================================================================
# STAGE 1 BOOTLOADER (x86_64 Assembly)
# ============================================================================

$(STAGE1_BIN): $(BUILD_DIR) $(BOOTLOADER_DIR)/stage1.asm
	@echo "Assembling Stage 1 bootloader..."
	@$(NASM) $(NASM_FLAGS) $(BOOTLOADER_DIR)/stage1.asm -o $@
	@echo "✓ Stage 1 bootloader: $@ (size: $$(stat -f%z '$@' 2>/dev/null || stat -c%s '$@') bytes)"

# ============================================================================
# STAGE 2 BOOTLOADER (C)
# ============================================================================

$(STAGE2_BIN): $(BUILD_DIR) $(BOOTLOADER_DIR)/stage2.c
	@echo "Compiling Stage 2 bootloader..."
	@$(CC) $(CFLAGS) -c $(BOOTLOADER_DIR)/stage2.c -o $(BUILD_DIR)/stage2.o
	@$(LD) $(LDFLAGS) -Ttext 0x8000 $(BUILD_DIR)/stage2.o -o $@
	@echo "✓ Stage 2 bootloader: $@"

# ============================================================================
# COMBINED BOOTLOADER
# ============================================================================

bootloader: $(STAGE1_BIN) $(STAGE2_BIN)
	@echo "Combining bootloader stages..."
	@cat $(STAGE1_BIN) $(STAGE2_BIN) > $(BOOTLOADER_BIN)
	@echo "✓ Combined bootloader: $(BOOTLOADER_BIN)"

# ============================================================================
# KERNEL (Rust)
# ============================================================================

kernel:
	@echo "Building kernel (Rust)..."
	@cd $(KERNEL_DIR) && $(CARGO) build $(CARGO_FLAGS)
	@echo "✓ Kernel built"

# ============================================================================
# DISK IMAGE
# ============================================================================

image: bootloader kernel
	@echo "Creating bootable disk image..."
	@dd if=/dev/zero of=$(DISK_IMAGE) bs=1M count=512
	@dd if=$(BOOTLOADER_BIN) of=$(DISK_IMAGE) bs=512 conv=notrunc
	@echo "✓ Disk image: $(DISK_IMAGE)"

# ============================================================================
# EMULATION
# ============================================================================

run: image
	@echo "Starting QEMU emulator..."
	@$(QEMU) -drive file=$(DISK_IMAGE),format=raw -m 512M -serial stdio

run-gdb: image
	@echo "Starting QEMU with GDB support..."
	@$(QEMU) -drive file=$(DISK_IMAGE),format=raw -m 512M -serial stdio \
		-gdb tcp::1234 -S

# ============================================================================
# CLEANUP
# ============================================================================

clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(BUILD_DIR)
	@cd $(KERNEL_DIR) && $(CARGO) clean
	@echo "✓ Clean complete"

# ============================================================================
# INSTALLATION TARGETS
# ============================================================================

install: image
	@echo "Installing to $(INSTALL_PATH)..."
	@cp $(DISK_IMAGE) $(INSTALL_PATH)/
	@echo "✓ Installation complete"

# ============================================================================
# DEBUG TARGETS
# ============================================================================

.PHONY: size-stage1 size-stage2 verify

size-stage1: $(STAGE1_BIN)
	@echo "Stage 1 bootloader size:"
	@stat -c "  %s bytes" $(STAGE1_BIN) || stat -f "  %z bytes" $(STAGE1_BIN)
	@echo "  (must be ≤ 512 bytes for MBR)"

size-stage2: $(STAGE2_BIN)
	@echo "Stage 2 bootloader size:"
	@stat -c "  %s bytes" $(STAGE2_BIN) || stat -f "  %z bytes" $(STAGE2_BIN)

verify: size-stage1
	@if [ $(shell stat -c%s $(STAGE1_BIN) 2>/dev/null || stat -f%z $(STAGE1_BIN)) -gt 512 ]; then \
		echo "ERROR: Stage 1 bootloader exceeds 512 bytes"; \
		exit 1; \
	fi
	@echo "✓ All verifications passed"

# ============================================================================
# PHONY TARGETS
# ============================================================================

.PHONY: all bootloader kernel image run clean help size-stage1 size-stage2 verify install run-gdb
