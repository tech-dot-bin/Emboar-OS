 Emboar OS - Library & Dependency Recommendations

## 1. Cryptography & Security

### 1.1 Rust Crates (Preferred)

| Crate | Purpose | Version | Why? |
|-------|---------|---------|------|
| **ring** | Cryptographic primitives | ^0.17 | Fast, audited, no system deps |
| **argon2** | Password hashing (Argon2id) | ^0.5 | OWASP recommended, memory-hard |
| **getrandom** | CSPRNG | ^0.2 | Secure random numbers |
| **rustls** | TLS/SSL | ^0.21 | Rust native, no OpenSSL |
| **rcgen** | X.509 certificate generation | ^0.11 | Generate PKI certificates |
| **ed25519-dalek** | EdDSA signing | ^2.0 | Fast elliptic curve signatures |

### 1.2 C Libraries (Fallback)

| Library | Purpose | Why |
|---------|---------|-----|
| **libcrypto (OpenSSL)** | Universal crypto library | De facto standard, widely tested |
| **libgcrypt** | GNUPG crypto library | Audited, GPL-compatible |
| **libsodium** | Modern crypto API | Easy-to-use, good defaults |
| **libargon2** | Argon2id hashing | Reference implementation |

### 1.3 Assembly Optimization

```asm
; Encrypted data copying (constant-time to prevent timing attacks)
; src/security/constant_time_copy.asm

; void constant_time_memcpy(void *dest, const void *src, size_t n)
; rdi = dest
; rsi = src
; rdx = size

constant_time_memcpy:
    push rbp
    mov rbp, rsp
    push r12

    xor r8, r8              ; counter
    xor r9, r9              ; accumulator for timing

.loop:
    cmp r8, rdx
    jge .done

    movzx r10d, byte [rsi + r8]
    mov byte [rdi + r8], r10b

    ; Dummy operations to prevent branch prediction
    add r9, r10
    and r9, 0xFF
    add r8, 1

    jmp .loop

.done:
    pop r12
    pop rbp
    ret
```

---

## 2. Kernel & System Programming

### 2.1 Rust Crates

| Crate | Purpose | Why |
|-------|---------|-----|
| **libc** | C library bindings | syscall interfaces, errno |
| **nix** | Unix/Linux syscalls | Type-safe wrappers around libc |
| **procfs** | Parse /proc filesystem | Process stats, memory info |
| **x86_64** | x86_64 CPU abstractions | Paging, interrupts, registers |
| **asm** | Inline assembly macros | CPU-specific optimizations |

### 2.2 HAL (Hardware Abstraction Layer)

```rust
// src/kernel/hal/mod.rs

pub mod memory {
    extern "C" {
        pub fn setup_paging(kernel_end: u64) -> Result<(), &'static str>;
        pub fn enable_pax() -> Result<(), &'static str>;
        pub fn enable_smep() -> Result<(), &'static str>;
    }
}

pub mod cpu {
    use x86_64::instructions::interrupts;
    use x86_64::registers::model_specific::Msr;

    pub fn enable_nxe() {
        // Enable No-Execute bit via EFER MSR
        let efer = unsafe { Msr::new(0xC0000080).read() };
        unsafe { Msr::new(0xC0000080).write(efer | 0x800); }
    }

    pub fn enable_smep() {
        // Enable Supervisor Mode Execution Prevention
        let mut cr4: u64;
        asm!("mov rax, cr4", out("rax") cr4);
        cr4 |= 1 << 20;  // SMEP bit
        unsafe {
            asm!("mov cr4, rax", in("rax") cr4);
        }
    }
}
```

---

## 3. Compilation & Build System

### 3.1 Build Tools

| Tool | Purpose | Config File |
|------|---------|------------|
| **cargo** | Rust package manager | `Cargo.toml` |
| **make** | C/Assembly build automation | `Makefile` |
| **gcc** / **clang** | C/C++ compiler | `.gcc-flags`, `.clang-flags` |
| **nasm** | x86_64 assembler | `.asm` files |
| **ld** / **lld** | Linker | `linker.ld` (linker script) |

### 3.2 Build Configuration Example

**`Cargo.toml`** (for kernel):

```toml
[package]
name = "emboar-kernel"
version = "1.0.0"
edition = "2021"

[dependencies]
x86_64 = "0.14"
libc = "0.2"
nix = { version = "0.27", features = ["process", "signal"] }
procfs = "0.15"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
ring = "0.17"
argon2 = "0.5"
sha2 = "0.10"
getrandom = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.dev]
debug = true
opt-level = 0
```

**`Makefile`** (for bootloader):

```makefile
.PHONY: build clean

NASM = nasm
NASM_FLAGS = -f elf64 -g -F dwarf

# Bootloader compilation
build: stage1.o stage2.o
	ld -m elf_x86_64 -o bootloader.elf stage1.o stage2.o
	objcopy -O binary bootloader.elf bootloader.bin

stage1.o: src/bootloader/stage1.asm
	$(NASM) $(NASM_FLAGS) -o $@ $<

stage2.o: src/bootloader/stage2.asm
	$(NASM) $(NASM_FLAGS) -o $@ $<

clean:
	rm -f *.o *.elf *.bin
```

---

## 4. Command-Line Interface (CLI)

### 4.1 Rust Crates

| Crate | Purpose | Why |
|-------|---------|-----|
| **clap** | Argument parsing | Easy, derive macros, automatic help |
| **tracing** | Logging framework | Async-friendly, structured logging |
| **anyhow** | Error handling | Context-rich error propagation |
| **colored** | ANSI colors | Simple color output |
| **indicatif** | Progress bars | Interactive progress display |
| **dialoguer** | Interactive prompts | User interaction (yes/no, input) |

### 4.2 CLI Example

```rust
// src/bin/ebm/main.rs: Package manager CLI

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use colored::*;
use indicatif::ProgressBar;

#[derive(Parser)]
#[command(name = "ebm")]
#[command(about = "Emboar Package Manager", version = "1.0")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package
    Install {
        /// Package name
        package: String,
        
        /// Repository name
        #[arg(long)]
        from: Option<String>,
    },

    /// Remove a package
    Remove {
        package: String,

        #[arg(long)]
        with_deps: bool,
    },

    /// Update packages
    Update {
        #[arg(value_name = "PACKAGE")]
        package: Option<String>,

        #[arg(long)]
        dry_run: bool,
    },

    /// List packages
    List {
        #[arg(long)]
        installed: bool,

        #[arg(long)]
        available: bool,
    },

    /// Search for packages
    Search {
        query: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Install { package, from } => {
            println!("{}", format!("Installing {}...", package).green().bold());

            let pb = ProgressBar::new(100);
            for _ in 0..100 {
                pb.inc(1);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            pb.finish_with_message("✓ Done!");

            println!("{}", "Successfully installed!".green());
            Ok(())
        }
        _ => Ok(()),
    }
}
```

---

## 5. Data Serialization

### 5.1 Rust Crates

| Crate | Format | Why |
|-------|--------|-----|
| **serde** | Generic serialization | Framework, many format backends |
| **serde_json** | JSON | Human-readable configs |
| **serde_yaml** | YAML | Config files, easy to edit |
| **toml** | TOML | Package metadata |
| **bincode** | Binary | Fast, compact storage |

### 5.2 Example: Metadata Serialization

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub dependencies: Vec<Dependency>,
    pub description: String,
    pub checksum_sha512: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub required: bool,
}

// Load from JSON
fn load_metadata(path: &str) -> anyhow::Result<PackageMetadata> {
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

// Save to JSON
fn save_metadata(metadata: &PackageMetadata, path: &str) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(metadata)?;
    std::fs::write(path, json)?;
    Ok(())
}
```

---

## 6. Testing & QA

### 6.1 Rust Testing Crates

| Crate | Purpose | Why |
|-------|---------|-----|
| **criterion** | Benchmarking | Statistical analysis, regression detection |
| **proptest** | Property testing | Find edge cases automatically |
| **tempfile** | Temporary files | Safe test file creation |
| **mockito** | HTTP mocking | Mock external services |

### 6.2 Testing Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_argon2_verification() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();
        
        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    proptest! {
        #[test]
        fn test_rsa_roundtrip(data in ".*") {
            let encrypted = rsa_encrypt(&data).unwrap();
            let decrypted = rsa_decrypt(&encrypted).unwrap();
            prop_assert_eq!(data, decrypted);
        }
    }

    // Benchmark
    use criterion::Criterion;

    fn bench_encryption(c: &mut Criterion) {
        c.bench_function("aes-256-gcm encrypt 1MB", |b| {
            b.iter(|| {
                let data = vec![0u8; 1024 * 1024];
                encrypt_aes256gcm(&data)
            });
        });
    }
}
```

---

## 7. System Monitoring & Logging

### 7.1 Rust Crates

| Crate | Purpose | Why |
|-------|---------|-----|
| **tracing** | Distributed tracing | Async logging, spans |
| **tracing-subscriber** | Log formatting | JSON, structured output |
| **syslog** | Syslog protocol | Send logs to syslog system |
| **chrono** | DateTime handling | Timestamps, timezone |

### 7.2 Logging Configuration

```rust
use tracing_subscriber::fmt::format::FmtSpan;

fn init_logging() {
    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(tracing::Level::DEBUG)
        .with_span_events(FmtSpan::FULL)
        .with_writer(std::io::stderr)
        .init();
}

#[tracing::instrument]
async fn process_request(user: &str, command: &str) -> Result<()> {
    tracing::info!("Processing request from {}", user);
    
    match execute_command(command) {
        Ok(result) => {
            tracing::info!("Command succeeded: {:?}", result);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Command failed: {}", e);
            Err(e)
        }
    }
}
```

---

## 8. FFI (Foreign Function Interface)

### 8.1 Rust-to-C Bindings

```rust
// src/ffi/lib.rs

use std::os::raw::{c_char, c_int};

extern "C" {
    // Link to libcrypto
    pub fn SHA512_Init(c: *mut SHA512_CTX) -> c_int;
    pub fn SHA512_Update(c: *mut SHA512_CTX, data: *const u8, len: usize) -> c_int;
    pub fn SHA512_Final(md: *mut u8, c: *mut SHA512_CTX) -> c_int;
}

#[repr(C)]
pub struct SHA512_CTX {
    h: [u64; 8],
    Nl: u64,
    Nh: u64,
    p: [u8; 128],
    num: usize,
    md_len: usize,
}

pub fn sha512_hash(data: &[u8]) -> Vec<u8> {
    unsafe {
        let mut ctx: SHA512_CTX = std::mem::zeroed();
        let mut md = [0u8; 64];

        SHA512_Init(&mut ctx);
        SHA512_Update(&mut ctx, data.as_ptr(), data.len());
        SHA512_Final(md.as_mut_ptr(), &mut ctx);

        md.to_vec()
    }
}
```

---

## 9. Dependency Tree (High Level)

```
Emboar OS Kernel (Rust)
├── x86_64 (CPU abstractions)
├── nix (syscalls)
├── libc (C bindings)
└── ring (cryptography)

EmShell (Rust)
├── nom (parser combinator)
├── clap (CLI parsing)
└── colored (output)

Package Manager (Rust)
├── serde_json (metadata)
├── zip (package format)
├── ring (signatures)
└── sha2 (checksums)

Security Services (Rust)
├── argon2 (password hashing)
├── ring (encryption)
├── chrono (timestamps)
└── tracing (audit logging)

System Utilities (C/Rust mixed)
├── libc (POSIX APIs)
├── openssl (legacy crypto)
└── libsodium (modern crypto)
```

---

## 10. Build Optimization

### 10.1 Cargo.toml Optimization

```toml
[profile.release]
opt-level = 3                    # Aggressive optimization
lto = true                       # Link-time optimization
codegen-units = 1               # Single codegen unit
strip = true                    # Strip debug symbols
panic = "abort"                 # Smaller binaries

[profile.dev]
debug = true                    # Full debug info
opt-level = 0                   # No optimization
```

### 10.2 Assembly Optimization

- Use SIMD instructions (SSE4.2, AVX2) for hashing
- Constant-time comparisons for crypto
- Prefetching for cache efficiency
- Branch prediction hints on hot paths

---

*Emboar OS Library Recommendations v1.0*
