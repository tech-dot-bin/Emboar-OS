# Emboar OS - Ultra-Secure Terminal-Only Operating System

**Version**: 1.0 Alpha  
**Status**: Architecture & Design Phase  
**Target Platform**: x86_64 Linux (UEFI/BIOS)  

## Overview

Emboar OS is a terminal-only, privacy-focused, security-hardened operating system designed for users who require absolute control over their computing environment with zero telemetry and maximum encryption.

### Key Principles

- **Privacy First**: Zero telemetry, automatic MAC randomization, RAM wiping on shutdown
- **Security by Default**: Full-disk encryption (LUKS), Argon2id hashing, immutable audit logs
- **Memory Safety**: Rust for critical modules, minimal unsafe code, buffer overflow protections
- **Lean & Fast**: No bloatware, assembly optimization, minimal dependencies
- **Microkernel Architecture**: Isolated services, fault containment, modular design

## Project Structure

```
emboar_os/
├── README.md                           # This file
├── ARCHITECTURE.md                     # System architecture & design decisions
├── BOOT_AND_INSTALLATION.md            # Setup, partitioning, encrypted install
├── CORE_FEATURES.md                    # Priority Governor, EmShell, Privacy systems
├── EMSHELL_SPECIFICATION.md            # EmShell language & syntax
├── COMMAND_REFERENCE.md                # 80+ commands with detailed specs
├── SECURITY_MODEL.md                   # Zero-trust, permissions, audit logging
├── PACKAGE_MANAGER.md                  # EBM package manager design
├── LIBRARY_RECOMMENDATIONS.md          # Crates, libraries, dependencies
├── IMPLEMENTATION_ROADMAP.md           # Phase-by-phase development plan
└── src/                                # Implementation starts here
    ├── bootloader/                     # x86_64 bootloader (Assembly)
    ├── kernel/                         # Microkernel (Rust)
    ├── drivers/                        # Device drivers (C/Rust)
    ├── init/                           # Init system (Rust)
    ├── emshell/                        # Shell implementation (Rust)
    ├── commands/                       # Command implementations
    ├── package_manager/                # EBM implementation (Rust)
    ├── security/                       # Security modules (Rust/C)
    └── libs/                           # Shared libraries
```

## Development Roadmap

See [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md) for phases and timelines.

## Getting Started

1. Review [ARCHITECTURE.md](ARCHITECTURE.md) for system design
2. Study [BOOT_AND_INSTALLATION.md](BOOT_AND_INSTALLATION.md)
3. Implement kernel in [src/kernel/](src/kernel/)
4. Build commands from [COMMAND_REFERENCE.md](COMMAND_REFERENCE.md)

---

*Emboar OS: Privacy and Security, Terminal First.*
