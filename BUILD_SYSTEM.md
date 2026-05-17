# Build System & Platform Configuration

## Overview

Emboar OS uses a multi-language build system:
- **Bootloader Stage 1**: x86_64 NASM Assembly (512 bytes MBR)
- **Bootloader Stage 2**: C with GCC
- **Kernel**: Rust (64-bit)
- **Init System**: Rust
- **Services**: Rust (multiple binaries)
- **Libraries**: Rust shared library

## Prerequisites

### Linux (Ubuntu/Debian)
```bash
# Install build tools
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    nasm \
    gcc \
    cargo \
    rust \
    qemu-system-x86 \
    git

# Verify installation
gcc --version
nasm -version
cargo --version
```

### macOS
```bash
# Using Homebrew
brew install nasm gcc rust

# Verify
gcc --version
nasm -version
cargo --version
```

## Build System Components

### 1. Makefile

Main build orchestrator at project root.

**Targets:**
- `make boot` - Build bootloader (Stage 1 + Stage 2)
- `make kernel` - Build kernel using Cargo
- `make all` - Build entire system
- `make clean` - Clean build artifacts
- `make run` - Run in QEMU emulator
- `make image` - Create bootable disk image
- `make verify` - Verify Stage 1 size constraints

**Example:**
```bash
cd /home/alexz/Desktop/emboar_os
make all        # Build everything
make run        # Run in QEMU
```

### 2. Cargo Workspace

Multi-crate Rust workspace at project root.

**Workspace structure:**
```
Cargo.toml (workspace root)
├── src/kernel/Cargo.toml
├── src/init/Cargo.toml
├── src/services/Cargo.toml
└── src/libs/Cargo.toml
```

**Build all Rust components:**
```bash
cargo build --release
```

**Build individual crates:**
```bash
cargo build --release -p emboar-kernel
cargo build --release -p emboar-init
cargo build --release -p emboar-services
cargo build --release -p emboar-libs
```

### 3. Assembly Build

Stage 1 bootloader compiled with NASM.

**Location:** `src/bootloader/stage1.asm`

**Build:**
```bash
nasm -f bin -o build/stage1.bin src/bootloader/stage1.asm
```

**Constraints:**
- Binary size: exactly 512 bytes (1 sector)
- Format: Binary (executable code)
- Entry point: 0x7C00 (BIOS MBR load address)

### 4. C Compilation

Stage 2 bootloader compiled with GCC.

**Location:** `src/bootloader/stage2.c`

**Build:**
```bash
gcc -nostdlib -no-pie -z max-page-size=0x1000 \
    -c src/bootloader/stage2.c -o build/stage2.o

ld -nostdlib -Ttext 0x8000 build/stage2.o -o build/stage2.bin
```

**Flags:**
- `-nostdlib`: Don't link libc (running at ring 0)
- `-no-pie`: Disable position-independent code
- `-Ttext 0x8000`: Load address (after Stage 1)

## File Layout

### Bootable Disk Image

```
Sector  0-0:    Stage 1 bootloader (512 bytes, MBR)
Sector  1-255:  Reserved (254 sectors)
Sector  256:    Kernel header
Sector  257+:   Kernel binary
```

### Memory Layout (at runtime)

```
0x00000000 - 0x00000FFF:  Real mode vectors (1 page)
0x00001000 - 0x0009FFFF:  Real mode memory
0x000A0000 - 0x000FFFFF:  Video + reserved
0x00100000 - 0x003FFFFF:  Bootloader (Stage 1+2)
0x00400000 - 0x03FFFFFF:  Kernel (64 MB)
0x04000000 - 0xFFFEFFFF:  Application memory
```

## Compilation Process

### Step 1: Build Bootloader

```bash
make boot
# Produces: build/bootloader.bin (combined Stage 1+2)
```

**Output:**
- `build/stage1.bin` (512 bytes)
- `build/stage2.bin` (GCC compiled C)
- `build/bootloader.bin` (concatenated)

### Step 2: Build Kernel

```bash
make kernel
# Uses cargo to compile src/kernel/
```

**Output:**
- `src/kernel/target/x86_64-unknown-linux-gnu/release/kernel`

**Key files:**
- `src/kernel/Cargo.toml` - Dependencies (ring, argon2, etc.)
- `src/kernel/src/main.rs` - Entry point
- `src/kernel/.cargo/config.toml` - Compiler settings

### Step 3: Create Disk Image

```bash
make image
# Produces: build/emboar.img (512 MB disk image)
```

**Image format:**
- Raw disk image
- Bootable (bootloader in first 512 bytes)
- Kernel at sector 256

### Step 4: Test in QEMU

```bash
make run
# Launches: qemu-system-x86_64 -drive file=build/emboar.img ...
```

**QEMU flags:**
- `-drive file=build/emboar.img,format=raw` - Boot drive
- `-m 512M` - 512 MB RAM
- `-serial stdio` - Serial output to terminal

## Debugging

### Build with Debug Symbols

```bash
make build-debug
# Uses -g flag, includes debug info
```

### Run QEMU with GDB

```bash
make run-gdb
# Starts QEMU waiting for GDB at port 1234
```

**In another terminal:**
```bash
gdb
(gdb) target remote :1234
(gdb) load
(gdb) continue
```

### Serial Console Output

Bootloader and kernel log to COM1 (serial port).

**View in QEMU:**
```
QEMU terminal shows all serial output
```

**Redirect to file:**
```bash
qemu-system-x86_64 -serial file:serial.log ...
```

## Dependency Management

### Rust Dependencies

Managed via `Cargo.toml` in each crate.

**Key dependencies:**
```toml
ring = "0.17"           # Cryptography (RSA, SHA-512)
argon2 = "0.5"          # Password hashing
x86_64 = "0.14"         # CPU control
nix = "0.27"            # Unix system calls
libc = "0.2"            # C library bindings
```

**Update dependencies:**
```bash
cargo update
```

### System Dependencies

Install via package manager before building.

**Required:**
- GCC (for Stage 2 bootloader)
- NASM (for Stage 1 bootloader)
- Rust toolchain
- QEMU (for testing)

## Performance Optimization

### Cargo Release Profile

```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit (slower, faster binary)
strip = false           # Keep debug symbols
```

### Assembly Optimizations

Stage 1: Minimal (size-critical, must be ≤ 512 bytes)
Stage 2: Standard (-O2 with GCC)

## Continuous Integration

### GitHub Actions Example

```yaml
name: Build
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install nasm gcc
      - name: Build
        run: make all
      - name: Run tests
        run: make verify
```

## Troubleshooting

### Stage 1 exceeds 512 bytes

```
$ make verify
ERROR: Stage 1 bootloader exceeds 512 bytes
```

**Solution:** Reduce assembly code, use shorter instructions

### GCC cannot find libraries

```
gcc: error: crt1.o: No such file or directory
```

**Solution:** Use `-nostdlib` flag (included in Makefile)

### QEMU crashes on boot

```
Segmentation fault (core dumped)
```

**Solution:** Check kernel entry point and memory layout

### Kernel doesn't print anything

```
No serial output in QEMU
```

**Solution:** Verify serial port I/O functions and bootloader handoff

## Next Steps

1. **Build:** `make all`
2. **Test:** `make run`
3. **Debug:** Use GDB with `make run-gdb`
4. **Modify:** Edit source files and rebuild
5. **Deploy:** Copy `build/emboar.img` to USB or disk

## References

- **x86_64 Architecture**: Intel 64 and IA-32 Architectures Software Developer Manual
- **UEFI Specification**: Unified Extensible Firmware Interface Forum
- **Rust NO_STD**: https://docs.rust-embedded.org/book/
- **GRUB Legacy Bootloader**: GNU GRUB Manual

---

## Summary

| Component | Language | Location | Output |
|-----------|----------|----------|--------|
| Stage 1 | NASM | `src/bootloader/stage1.asm` | `build/stage1.bin` (512 B) |
| Stage 2 | C | `src/bootloader/stage2.c` | `build/stage2.bin` |
| Kernel | Rust | `src/kernel/src/main.rs` | `src/kernel/target/.../kernel` |
| Init | Rust | `src/init/src/main.rs` | `target/.../init` |
| Services | Rust | `src/services/src/*.rs` | `target/.../` |
| Libraries | Rust | `src/libs/src/*.rs` | `target/.../libemboar_libs.rlib` |

Build everything with `make all` and test with `make run`!
