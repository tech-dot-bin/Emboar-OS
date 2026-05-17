# Development Guide

## Overview

This guide explains how to develop Emboar OS components, extend functionality, and integrate new features.

## Architecture Summary

Emboar OS follows a microkernel architecture:

```
┌─────────────────────────────────────────┐
│     User Applications                   │
│  (Shell, Commands, Package Manager)     │
└────────────────┬────────────────────────┘
                 │ System calls
┌────────────────▼────────────────────────┐
│     Init System (Services)              │
│  (syslogd, auditd, udevd, mount, etc.)  │
└────────────────┬────────────────────────┘
                 │ IPC / Capability-based
┌────────────────▼────────────────────────┐
│     Kernel (64-bit Rust/x86_64)         │
│  - Process management                   │
│  - Memory management (paging)           │
│  - Interrupt/exception handling         │
│  - IPC primitives                       │
│  - Device drivers                       │
└────────────────┬────────────────────────┘
                 │ Bootloader (C + ASM)
┌────────────────▼────────────────────────┐
│     Hardware (x86_64)                   │
│  - CPU (Protected mode → Long mode)     │
│  - Memory (Paging, 4-level tables)      │
│  - I/O (Disk, Serial, Network)          │
└─────────────────────────────────────────┘
```

## Component Development

### 1. Bootloader (src/bootloader/)

**Stage 1: Boot sector (stage1.asm)**

File: `src/bootloader/stage1.asm`

Responsibilities:
- Enable A20 line (memory > 1MB access)
- Load GDT (Global Descriptor Table)
- Switch: Real mode → Protected mode → Long mode (64-bit)
- Jump to Stage 2

To modify:
```asm
; Add new instruction in long_mode_entry
mov rsi, new_message
call print_string_64
```

Constraints:
- ≤ 512 bytes (must fit in MBR sector)
- Use minimal stack (stack at 0x100000 in long mode)
- All debugging output via serial port

**Stage 2: Kernel loading (stage2.c)**

File: `src/bootloader/stage2.c`

Responsibilities:
- Load kernel from disk
- Verify RSA-4096 signature
- Verify SHA-512 hash
- Set up memory map (E820)
- Jump to kernel

To extend:
```c
// Add new verification step
int verify_kernel_authenticity() {
    // Use ring crate's RSA verification
    // Check against embedded public key
}
```

Build: `make boot`

### 2. Kernel (src/kernel/)

**Main entry point: src/kernel/src/main.rs**

Core initialization sequence:
```rust
#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // 1. Load GDT
    load_gdt();
    
    // 2. Load IDT
    load_idt();
    
    // 3. Enable interrupts
    unsafe {
        core::arch::x86_64::__asm!("sti");
    }
    
    // 4. Boot init system
    // (will be implemented in Phase 2)
}
```

**To add subsystems:**

1. Create new module in `src/kernel/`:
```rust
// src/kernel/src/memory.rs
pub mod paging {
    pub fn setup_paging() {
        // Initialize 4-level page tables
    }
}
```

2. Import and initialize in `main.rs`:
```rust
mod memory;

kernel_main() {
    // ...
    memory::paging::setup_paging();
}
```

**Key modules to implement:**

- `cpu/` - GDT, IDT, exceptions
- `memory/` - Paging, heap allocator
- `process/` - Scheduler, process table
- `ipc/` - Message passing, capabilities
- `drivers/` - PCI, disk, network

Build: `make kernel` or `cargo build --release -p emboar-kernel`

### 3. Init System (src/init/)

**Main: src/init/src/main.rs**

Responsibilities (PID 1):
1. Mount essential filesystems
2. Parse /etc/fstab
3. Load services
4. Manage process respawning

To add new services:
```rust
// In load_services():
services.push(ServiceConfig {
    name: "my-service".to_string(),
    command: "/usr/bin/my-service".to_string(),
    args: vec!["--daemon".to_string()],
    restart_policy: RestartPolicy::Always,
    user: Some("root".to_string()),
    group: Some("root".to_string()),
});
```

Build: `cargo build --release -p emboar-init`

### 4. Services (src/services/)

**Available services:**

1. **syslogd** (src/services/src/syslogd.rs)
   - Listens on /dev/log
   - Logs to /var/log/audit/syslog.log
   - Immutable audit log with SHA-512

2. **auditd** (src/services/src/auditd.rs)
   - Monitors system calls
   - Enforces security policies
   - Logs access violations

3. **udevd** (src/services/src/udevd.rs)
   - Manages device nodes
   - Handles hotplug events
   - Applies device permissions

**To create new service:**

```rust
// Create src/services/src/myservice.rs
pub struct MyService {
    // State
}

impl MyService {
    fn new() -> Self {
        MyService {}
    }
    
    fn start(&self) -> Result<()> {
        // Initialize and run
        Ok(())
    }
}

fn main() -> Result<()> {
    let service = MyService::new();
    service.start()
}
```

Update `Cargo.toml`:
```toml
[[bin]]
name = "myservice"
path = "src/myservice.rs"
```

Build: `cargo build --release -p emboar-services`

### 5. Shared Libraries (src/libs/)

**Cryptographic utilities: src/libs/src/crypto.rs**

Available functions:
```rust
// Hashing
Crypto::sha512(data) -> [u8; 64]

// HMAC
HMAC::sha512(key, data) -> [u8; 64]

// Random
Crypto::random_bytes(n) -> Result<Vec<u8>>

// Key derivation
Crypto::derive_key_argon2id(password, salt, key_len) -> Result<Vec<u8>>
```

**Password hashing: src/libs/src/password.rs**

```rust
// Hash password
let hash = PasswordHash::new(password)?;

// Verify password
if hash.verify(password)? {
    println!("Password correct");
}

// Store/load
let phc_string = hash.to_phc_string();
let restored = PasswordHash::from_phc_string(&phc_string)?;
```

Use in your code:
```rust
use emboar_libs::prelude::*;

fn authenticate_user(password: &str) -> Result<bool> {
    let stored_hash = load_hash_from_file()?;
    stored_hash.verify(password.as_bytes())
}
```

Build: `cargo build --release -p emboar-libs`

## Workflow: Adding a New Feature

### Example: Add clock service

**Step 1: Create the service**

```bash
# Create src/services/src/clockd.rs (based on existing services)
cp src/services/src/syslogd.rs src/services/src/clockd.rs
# Edit to implement clock service
```

**Step 2: Update Cargo.toml**

```toml
[[bin]]
name = "clockd"
path = "src/services/src/clockd.rs"
```

**Step 3: Register with init**

Edit `src/init/src/main.rs`:
```rust
services.push(ServiceConfig {
    name: "clockd".to_string(),
    command: "/usr/sbin/clockd".to_string(),
    args: vec![],
    restart_policy: RestartPolicy::Always,
    user: Some("root".to_string()),
    group: Some("root".to_string()),
});
```

**Step 4: Build and test**

```bash
cargo build --release -p emboar-services
cargo build --release -p emboar-init
make run
```

## Testing

### Unit Tests

Run tests in individual crates:
```bash
cargo test --release -p emboar-libs
```

### Integration Testing

Create test in QEMU:
```bash
make run
# QEMU boots and runs init system
# Services should start successfully
```

### Manual Testing

Connect via serial console:
```bash
# From another terminal while running make run
telnet localhost 5555  # (if using telnet serial plugin)
# or
nc localhost 5555      # (netcat)
```

## Debugging

### Enable Debug Output

In kernel or services:
```rust
eprintln!("[component] Debug message");
// Output appears on serial port
```

### Use GDB

```bash
make run-gdb
# In another terminal:
gdb
(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
(gdb) step
(gdb) print rax
```

### Check Serial Output

```bash
make run 2>&1 | tee output.log
# Output captured to file
```

## Performance Optimization

### Profiling

For Rust code:
```bash
cargo build --release
# Use perf on Linux
perf record -g ./target/release/component
perf report
```

### Code Size Optimization

For bootloader:
```asm
; Use short forms of instructions
mov al, 1       ; Shorter than mov rax, 1
xor ax, ax      ; Commonly used, short encoding
```

For Rust:
```toml
[profile.release]
opt-level = "z"     # Optimize for size instead of speed
lto = true
```

## Security Considerations

### When modifying security modules:

1. **Cryptography:** Always use "ring" or "libcrypto" crate, never implement crypto yourself
2. **Passwords:** Use Argon2id with recommended parameters
3. **Random numbers:** Use SystemRandom, never seed-based
4. **Audit logging:** Write immutable records with SHA-512 verification
5. **Access control:** Check capabilities before all privileged operations

### Security checklist before commit:

- [ ] No hardcoded secrets or passwords
- [ ] Cryptographic operations use established libraries
- [ ] Random number generation uses SystemRandom
- [ ] All audit-relevant operations logged
- [ ] No unsafe Rust except bootloader
- [ ] Input validation on all external data

## Common Tasks

### Build everything
```bash
make all
```

### Build just kernel
```bash
cargo build --release -p emboar-kernel
```

### Run in QEMU
```bash
make run
```

### Clean build artifacts
```bash
make clean
```

### Verify project structure
```bash
./verify-build.sh
```

## Next Implementation Phases

**Phase 2 (Kernel features):**
- Complete memory management (paging, heap)
- Process scheduler
- Interrupt handlers
- IPC primitives

**Phase 3 (User programs):**
- EmShell interpreter
- Command implementations
- Package manager

**Phase 4 (Security):**
- Full disk encryption (LUKS)
- Access control lists (ACLs)
- Immutable audit logs
- Security policies

**Phase 5 (Optimization):**
- Performance tuning
- Code optimization
- Hardening

## Resources

- **x86_64 ISA**: Intel 64 and IA-32 Architectures Developer Manual
- **Rust embedded book**: https://docs.rust-embedded.org/book/
- **OSDev wiki**: https://wiki.osdev.org/
- **Ring crate docs**: https://briansmith.org/rustdocs/ring/
- **Argon2id spec**: https://github.com/P-H-C/phc-winner-argon2

---

## Summary

1. **Bootloader**: Modify assembly/C in `src/bootloader/`
2. **Kernel**: Add modules to `src/kernel/src/`
3. **Services**: Create new services in `src/services/src/`
4. **Libraries**: Shared utilities in `src/libs/src/`
5. **Build**: `make all` to compile everything
6. **Test**: `make run` to boot in QEMU
7. **Debug**: Use `make run-gdb` with GDB

Happy developing!
