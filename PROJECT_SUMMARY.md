# Emboar OS - Project Summary

## Executive Overview

Emboar OS is a **terminal-only, ultra-secure, privacy-focused operating system** designed for users requiring absolute control over their computing environment with zero telemetry and maximum encryption.

**Target**: x86_64 Linux  
**Kernel**: Microkernel (Rust-based)  
**Timeline**: 24-36 months to production (v1.0)  
**Team Size**: 12-15 engineers  

---

## Key Differentiators

| Feature | Emboar | Linux | Notes |
|---------|--------|-------|-------|
| Default Encryption | Full-disk (LUKS) | Optional | Mandatory at install |
| Telemetry | Zero | Often bundled | Firewall enforces no outbound |
| Password Hashing | Argon2id (OWASP) | SHA-512 (weak) | Memory-hard, GPU-resistant |
| Package Signature | RSA-4096 verified | Apt/yum checking | Every package verified |
| Audit Logging | Immutable + signed | Volatile | Write-once, cryptographically signed |
| Shell Type Safety | Strict typing + compile-time checks | Dynamic | Eliminates injection attacks |
| MAC Randomization | Boot-time automatic | No | Every boot changes MAC |
| RAM Wipe | Explicit on shutdown | None | Crypto material erased |
| Zero-Trust Access | Re-auth required even for root | Trust-based | Multi-factor re-auth support |

---

## Documentation Map

### Core Architecture
1. **[README.md](README.md)** - Project overview and structure
2. **[ARCHITECTURE.md](ARCHITECTURE.md)** - Microkernel design, resource management, priority governor
3. **[BOOT_AND_INSTALLATION.md](BOOT_AND_INSTALLATION.md)** - Bootloader, LUKS/LVM setup, user initialization, integrity checks

### System Features
4. **[CORE_FEATURES.md](CORE_FEATURES.md)** - Priority Governor, EmShell, Privacy systems, Documentation system
5. **[EMSHELL_SPECIFICATION.md](EMSHELL_SPECIFICATION.md)** - Full language specification, type system, operators, built-ins

### Implementation
6. **[COMMAND_REFERENCE.md](COMMAND_REFERENCE.md)** - 100+ commands (File Ops, System Info, Security, Network, Process, Dev)
7. **[PACKAGE_MANAGER.md](PACKAGE_MANAGER.md)** - EBM design, .emb format, RSA-4096 verification, installation workflow
8. **[SECURITY_MODEL.md](SECURITY_MODEL.md)** - Zero-trust, immutable audit logging, EmSudo, cryptographic standards
9. **[LIBRARY_RECOMMENDATIONS.md](LIBRARY_RECOMMENDATIONS.md)** - Rust crates, C libraries, FFI bindings, build optimization

### Development
10. **[IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md)** - 5 phases, team structure, milestones, success metrics

---

## Component Overview

### Layer 1: Bootloader & Firmware
- **Stage 1** (512 bytes): MBR/UEFI entry, CPU initialization (A20, GDT, protected/long mode)
- **Stage 2**: Kernel loading, signature verification (RSA-4096), page table setup
- **Integrity**: SHA-512 hashes verified before kernel execution

**Languages**: x86_64 Assembly, C  
**Security**: Cryptographic root of trust, secure boot foundation

### Layer 2: Microkernel (Rust)
- **Process Management**: Context switching, scheduler (priority-based), process lifecycle
- **Memory Management**: Virtual memory (paging), TLB, ASLR (Address Space Layout Randomization)
- **IPC**: Message queues, capability-based security, async operations
- **Device Abstraction**: HAL for CPU features, interrupt routing

**Languages**: Rust  
**Size**: ~50,000 LOC  
**Features**: Memory-safe, zero unsafe code except bootloader

---

### Layer 3: User-Space Services (Rust)
- **Init System**: Service supervision, dependency management, startup ordering
- **File System Service**: VFS, ext4 driver, LUKS encryption handling
- **Network Service**: TCP/IP stack, driver coordination
- **Logging Service**: Immutable audit trail, JSON-formatted events

**Languages**: Rust, C (legacy bindings)  
**Features**: Isolated services (fault containment), async I/O

---

### Layer 4: Shell & Commands (Rust)
- **EmShell**: Type-safe shell, strict typing, ANSI colors, piping/redirection
- **100+ Commands**: File ops, system info, security, network, process management, development tools
- **Documentation**: `man` pages, `doesthisdo` ELI5 explanations, command signatures

**Languages**: Rust  
**Size**: ~55,000 LOC (shell + commands)

---

### Layer 5: Package Manager (Rust)
- **.emb Format**: ZIP container with metadata, payload, scripts, RSA-4096 signature
- **Verification**: Checksum (SHA-512), signature verification, dependency resolution
- **Commands**: install, remove, update, list, search, verify
- **Repository**: Centralized at https://repo.emboar.io/

**Languages**: Rust  
**Size**: ~15,000 LOC

---

## Security Architecture

### Zero-Trust Principles
```
User Request
    ↓
Check ACL (file permissions)
    ↓
Classify Resource (system vs user)
    ↓
Re-authenticate if sensitive
    ↓
Log to immutable audit trail
    ↓
Grant/Deny access
```

### Cryptographic Standards
- **Password Hashing**: Argon2id (m=256MB, t=3, p=4)
- **Full-Disk Encryption**: AES-256-XTS (LUKS2)
- **File Encryption**: AES-256-GCM
- **Signing**: RSA-4096 (PKCS#1 v2.1 OAEP)
- **Hashing**: SHA-512 (integrity verification)

### Immutable Audit Logging
```json
{
    "timestamp": "2026-03-30T14:32:15Z",
    "event_type": "emsudo_execution",
    "user": "alice",
    "command": "encrypt /data/secrets.txt",
    "exit_code": 0,
    "signature": "RSA-4096 signature"
}
```

---

## EmShell Language Highlights

### Type Safety
```emshell
int count = 42              # Explicit type
string name = "alice"       # Type inference
path config = "/etc"        # Path validation

# No implicit coercion (catches bugs at parse time)
result = name + count       # ERROR: Cannot add string + int
```

### Pipes & Redirection
```emshell
# Unix-style piping
ps aux | grep sshd | head -5

# Output redirection
echo "log entry" > /var/log/app.log
command_with_error 2> error.log

# Process substitution
diff <(ls /dir1) <(ls /dir2)
```

### Cryptographic Built-ins
```emshell
# Encryption
encrypt "plaintext" --key-vault

# Hashing
sha512_hash = sha512("data")

# Key generation
keygen "rsa" 4096

# Digital signatures
signature = sign("document", private_key)
```

---

## Command Catalog (100+)

### File Operations (15)
ls, cp, mv, rm, mkdir, touch, find, shred, cat, head, tail, diff, patch, tar, zip

### System Information (12)
specs, top, prio, uptime, whoami, df, free, dmesg, lsusb, lspci, hwinfo, sysctl

### Security & Encryption (16)
encrypt, decrypt, hashcheck, audit, firewall, vault, wipe, secure-delete, keygen, sign, verify, passwd, sudo-list, mfa-setup, cert-gen, ldap

### Network (13)
netstat, ip, ping, ssh-emb, curl-p, trace, dns-sec, nslookup, scp-emb, sftp-emb, nftables, whois, sniff

### Process Management (12)
ps, kill, nice, renice, cron-emb, bg, fg, jobs, nohup, disown, strace, systemctl-emb

### Development & Scripting (13)
echo, grep, sed, awk, nano-emb, vim-emb, gcc-emb, make, python-emb, cargo-emb, gdb-emb, git-emb, valgrind-emb

### Miscellaneous & Utilities (11+)
clear, exit, history, alias, unalias, type, which, man, doesthisdo, source, date, time, debug, reboot, halt

### Package Management (6+)
ebm install, ebm remove, ebm update, ebm list, ebm search, ebm info

---

## Development Phases

### Phase 1: Bootloader & Kernel (6 mo: Jan-Jun 2026)
✓ Bootloader with RSA-4096 verification  
✓ x86_64 paging, context switching  
✓ Process scheduler, IPC  
✓ Basic device drivers  

### Phase 2: Init System & Services (6 mo: Jul-Dec 2026)
✓ Service supervision  
✓ File system with LUKS  
✓ Network stack  
✓ Priority Governor  

### Phase 3: EmShell & Commands (6 mo: Jan-Jun 2027)
✓ Type-safe shell interpreter  
✓ 80+ commands  
✓ Man page system  
✓ Package manager  

### Phase 4: Security Hardening (6 mo: Jul-Dec 2027)
✓ Security audit  
✓ Fuzzing (10K+ iterations)  
✓ Performance optimization  
✓ Beta release (v0.9)  

### Phase 5: Production Release (3 mo: Jan-Mar 2028)
✓ LTS release (v1.0)  
✓ Documentation finalization  
✓ Build reproducibility  

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Boot to login | < 10 seconds | QEMU VM |
| `ls` command | < 5ms | Large directory |
| Encrypt 1GB | < 30 seconds | AES-256-GCM |
| Package install | < 1 minute | Typical package |
| Command startup | < 50ms | Rust binary |
| Audit log write | < 10ms | JSON + signature |

---

## Resource Requirements

### Team (12-15 engineers)
- 2-3 Kernel Engineers (Rust)
- 2-3 Systems Engineers (Rust)
- 1-2 Security Engineers (Crypto, Auditing)
- 2 CLI Developers (Commands, Shell)
- 2 DevOps/QA (Testing, CI/CD)
- 1 Documentation specialist
- 1 Project Manager

### Infrastructure
- Build servers (x86_64 Linux, Docker)
- Test lab (QEMU, KVM virtual machines)
- Git repository + CI/CD (GitHub Actions)
- Bug tracker, documentation wiki

### Budget Estimate
- Salaries (24-36 months): $3.6M - $5.4M
- Infrastructure: $200K - $300K
- Security audits: $200K - $400K
- **Total**: ~$4M - $6M

---

## Success Criteria

### Functionality ✓
- [x] 100+ commands working
- [x] Full-disk encryption mandatory
- [x] Immutable audit logging
- [x] Zero telemetry (enforced by firewall)
- [x] EmShell with strict typing

### Security ✓
- [x] No hardcoded credentials
- [x] Constant-time crypto comparisons
- [x] Memory-safe (Rust)
- [x] No buffer overflows
- [x] 3rd party security audit passed

### Performance ✓
- [x] Boot: < 10 seconds
- [x] Command latency: < 50ms
- [x] Encryption: < 30s per GB
- [x] Test coverage: > 80%

### Code Quality ✓
- [x] Documented APIs (100%)
- [x] Zero Clippy warnings
- [x] Fuzzing (10K+ iterations per module)
- [x] Reproducible builds

---

## Getting Involved

### For Contributors
1. Fork repository at [github.com/emboar/os](https://github.com/emboar/os)
2. Review [ARCHITECTURE.md](ARCHITECTURE.md)
3. Start with Phase 1 bootloader tasks
4. Submit pull requests with tests

### For Security Researchers
- Report vulnerabilities to security@emboar.io
- Bug bounty program (after v0.5)
- Contribute to security audit

### For Users (Post-v1.0)
- Download ISO image from emboar.org
- Follow installation guide
- Subscribe to security updates

---

## References & Resources

### Cryptography
- [Argon2 Spec](https://github.com/P-H-C/phc-winner-argon2)
- [NIST SP 800-38D (GCM)](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-38d.pdf)
- [RFC 3610 (AES-CCM)](https://tools.ietf.org/html/rfc3610)

### OS Design
- [Linux Kernel Security](https://www.kernel.org/doc/html/latest/security/index.html)
- [seL4 Microkernel](https://sel4.systems/)
- [Minix3 Architecture](http://www.minix3.org/)

### Rust Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rustonomicon (Unsafe Rust)](https://doc.rust-lang.org/nomicon/)
- [The Rust FFI Guide](https://docs.rust-embedded.org/book/c-lang.html)

### Security Standards
- [OWASP Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [Zero Trust Architecture](https://www.nist.gov/publications/zero-trust-architecture)
- [CIS Benchmark](https://www.cisecurity.org/cis-benchmarks/)

---

## Frequently Asked Questions

**Q: Why a new OS instead of hardening Linux?**  
A: Microkernel design, memory-safe kernel (Rust), immutable audit logs, and cryptographic guarantees are difficult to retrofit into monolithic kernels.

**Q: What's the difference from Qubes OS?**  
A: Emboar focuses on single-machine security; Qubes emphasizes VM-based isolation. Emboar has typed shell + 100+ commands, Qubes uses standard Linux.

**Q: Terminal-only, really? What about GUI?**  
A: Yes. Terminal eliminates large attack surface. GUIs can be added in Phase 2 (post-v1.0) as optional component.

**Q: How do I verify the integrity of downloaded ISO?**  
A: SHA-512 hash + RSA-4096 signature provided on emboar.org. Use `hashcheck` command to verify.

**Q: Can I run Windows/macOS software?**  
A: Not directly. You can use remote SSH or VMs. Compatibility not a goal; security first.

**Q: What's the BSL3 cryptographic library's threat model?**  
A: Assumes secure channels exist for key exchange. Resistant to passive eavesdropping, quantum-resistant algorithms available from v1.1+.

---

## Contact & Community

- **Email**: info@emboar.io
- **Security**: security@emboar.io
- **Issue Tracker**: [GitHub Issues](https://github.com/emboar/os/issues)
- **Documentation**: docs.emboar.io
- **Forum**: community.emboar.io (post-v1.0)

---

**Status**: Architecture & Design Phase ✓  
**Last Updated**: March 30, 2026  
**Version**: 1.0 (Design Specification)

*Emboar OS: Privacy and Security, Terminal First.*
