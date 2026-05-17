# Emboar OS - System Architecture

## 1. Microkernel Architecture

### 1.1 Core Design Principles

Emboar OS uses a **modular microkernel architecture** where critical system services run in isolated user-space processes. This prevents cascading failures and improves security through privilege separation.

```
┌─────────────────────────────────────────────────────┐
│           User Applications / EmShell               │
├─────────────────────────────────────────────────────┤
│        Service Layer (Isolated Processes)           │
│  ┌──────────────┬──────────────┬──────────────┐    │
│  │ FileSystem   │  Networking  │  Device I/O  │    │
│  │ Service      │  Service     │  Service     │    │
│  └──────────────┴──────────────┴──────────────┘    │
├─────────────────────────────────────────────────────┤
│           Microkernel (Rust)                        │
│  - Process Management                              │
│  - Memory Management (Virtual Memory, Paging)      │
│  - IPC (Inter-Process Communication)               │
│  - Interrupt/Exception Handling                    │
│  - Hardware Abstraction Layer (HAL)                │
├─────────────────────────────────────────────────────┤
│  Bootloader (x86_64 Assembly + C)                  │
│  - UEFI/BIOS Support                               │
│  - Secure Boot Verification                        │
│  - Cryptographic Root of Trust                     │
│  - Handoff to Kernel                               │
├─────────────────────────────────────────────────────┤
│               Hardware (x86_64)                     │
└─────────────────────────────────────────────────────┘
```

### 1.2 Microkernel Responsibilities

**In-kernel only**:
- CPU context switching & process scheduling
- Virtual memory management (paging, MMU setup)
- IPC primitives (message passing)
- Hardware interrupt routing
- Basic HAL (CPU flags, memory barriers)

**User-space services**:
- File system (VFS, FS drivers)
- Network stack (TCP/IP, packet routing)
- Device drivers (USB, disk, serial)
- Credential management
- Logging service

### 1.3 IPC & Service Communication

- **Message-based IPC**: All inter-process communication uses kernel-mediated message queues
- **Async/streaming I/O**: Efficient handling of network and disk I/O through shared buffers
- **Capability-based security**: Services hold capabilities granting access to specific resources

---

## 2. Boot & Installation Phase

### 2.1 Bootloader Stage (x86_64 Assembly)

**Purpose**: Initialize hardware, verify kernel signature, establish cryptographic root of trust.

**Components**:
1. **Stage 1 (512 bytes)**: MBR/UEFI entry point
   - Enable A20 line
   - Load GDT (Global Descriptor Table)
   - Switch to protected mode → long mode (64-bit)
   - Jump to Stage 2

2. **Stage 2 (C/Assembly)**: Load kernel
   - Read kernel image from encrypted disk
   - Verify SHA-512 hash and RSA-4096 signature
   - Set up initial page tables
   - Call `kernel_main()`

**Security Checks**:
- Signature verification before kernel load
- Memory protection bits (NX, DEP)
- SMEP (Supervisor Mode Execution Protection)

### 2.2 Secure Installation Process

**Pre-Installation**:
```
1. User boots Emboar Live Image
2. emboar-setup script invoked
3. System detects available disks
```

**Partitioning (LVM/LUKS)**:
```bash
# Example flow (pseudo-code)
disk = /dev/sda
partition_disk(disk, [
  { size: 512MB,  mount: /boot/efi,  fs: FAT32,  encrypt: false },
  { size: 256MB,  mount: /boot,      fs: ext4,   encrypt: false },
  { size: remaining, mount: /,       fs: ext4,   encrypt: true  }
])

# Create LUKS encrypted container
luks_create(disk_partition_root, password=hash(user_password))
lvm_pvcreate(luks_device)
lvm_vgcreate(emboar_vg, luks_device)
lvm_lvcreate(emboar_vg, size=root_size, name=root)
lvm_lvcreate(emboar_vg, size=var_size, name=var)
```

**User Identity Setup**:
```
1. Create root account (hash password with Argon2id)
   - Parameters: m=256MB, t=3 iterations, parallelism=4
   - Salt: 32 bytes from /dev/urandom
   - Stored in: /etc/emboar/shadow (0600 root:root)

2. Create primary user account
   - Home directory: /home/<username>
   - Shell: /bin/emshell
   - Groups: users, wheel (for sudo)

3. Hash verification setup
   - Generate SHA-512 hashes of all binaries in /bin, /sbin
   - Store in: /etc/emboar/integrity.db (signed)
```

**Security Integration**:
```
1. Create sudo-equivalent (emsudo)
   - Configuration: /etc/emsudo/conf
   - Commands requiring emsudo logged to /var/log/emsudo.log
   - Session timeout: 15 minutes (configurable)
   - Failed attempts logged, quota enforced

2. Boot integrity initialization
   - Generate /etc/emboar/boot_manifest with hashes
   - Sign with private key (generated during install)
```

---

## 3. Core OS Features

### 3.1 Priority Governor (CPU/RAM Resource Management)

**Goal**: Automatically manage resource allocation between background tasks and active terminal sessions.

**Architecture**:

```
┌──────────────────────────────┐
│   Priority Governor Daemon   │ (runs as service)
├──────────────────────────────┤
│ • Monitor active processes   │
│ • Track CPU/RAM usage        │
│ • Measure session activity   │
│ • Adjust nice/priority       │
│ • Update CGroup limits       │
└──────────────────────────────┘
         ↓ (syscalls)
┌──────────────────────────────┐
│  Kernel Scheduler            │
├──────────────────────────────┤
│ • CGroup enforcement         │
│ • Process priority           │
│ • CPU affinity               │
│ • Memory limits              │
└──────────────────────────────┘
```

**Behavior**:

| Scenario | Action |
|----------|--------|
| User typing in EmShell | Priority +50 (high), set to 1 CPU core, 2GB guarantee |
| Background tar/zip | nice +10, limited to 50% CPU, confined to slower cores |
| System service (SSH) | Standard priority, 10% CPU hard limit |
| Memory pressure | Kill non-essential background processes first |

**Implementation**:
- Monitor keyboard/mouse activity via `/proc/<pid>/stat`
- Use Linux CGroups (cgroup v2) for resource limits
- Nice level adjustments via `setpriority()` syscall

---

### 3.2 EmShell: Secure Bash-Like Shell

**Overview**: A terminal shell with strict typing, ANSI colors, piping, and redirection.

**Key Features**:
- **Strict Typing**: Variables declared with type (int, string, path)
- **Sandbox Mode**: Restrict built-in shells to avoid command injection
- **ANSI Colors**: Full 256-color support with theming
- **Compound Commands**: Pipes `|`, redirects `>`, `>>`, `2>`
- **History**: Encrypted history per user

**Early Example**:
```emshell
# Declare variables with types
int max_retries = 5
string output_file = "/tmp/backup.tar.gz"
path config_path = "/etc/emboar/config"

# Piping with color output
ls -la /home | grep "user" | head -10

# Redirection with error capture
command_with_errors 2>&1 > /var/log/output.log

# Control flow
if [[ $? -eq 0 ]]; then
    echo "Success!" --color=green
else
    echo "Failed!" --color=red
fi

# Functions with types
function backup_encrypted(string source, path dest) -> int {
    tar --create --file="$dest" "$source"
    if [[ $? -ne 0 ]]; then
        return 1
    fi
    encrypt "$dest" --key-vault
    return 0
}
```

**Shell Configuration**:
- Config: `/etc/emboar/emshell.conf` + `~/.emshellrc`
- Supports themes, aliases, functions
- Read-only history: `/home/<user>/.emshell_history` (immutable)

---

### 3.3 Privacy-First System

**MAC Address Randomization**:
```c
// Run at boot by init system
void randomize_mac_addresses() {
    for each_network_interface(iface) {
        new_mac = generate_random_mac();
        ip_link_set_address(iface, new_mac);
        log_action("MAC address randomized", iface);
    }
}
```

**RAM Wiping on Shutdown**:
```asm
; Assembly-level memory clearing during shutdown
shutdown_wipe_ram:
    mov rdi, 0x0000000000000000    ; start address
    mov rcx, physical_memory_limit ; size in bytes
    xor rax, rax                   ; zero register
    rep stosq                      ; fill with zeros
    ; TODO: Also wipe CPU caches (clflush)
```

**Zero Telemetry**:
- No DNS queries to external services (except user-initiated)
- No HTTP/HTTPS outbound connections (firewall default-deny)
- No logging of user activities beyond what's necessary for security audit

---

### 3.4 Documentation System

**Every command supports**:

```bash
$ man ls
# Outputs: Full technical manual (groff format)

$ doesthisdo ls
# Outputs: "Lists files and directories in a folder. 
#           Shows names, sizes, and permissions. 
#           Color codes help you spot files vs folders."

$ man --toc
# Outputs: Table of all available man pages by category
```

**Documentation Storage**:
- Manual pages: `/usr/share/man/man1/<cmd>.1` (groff format)
- ELI5 explanations: `/etc/emboar/docs/doesthisdo/<cmd>.txt`
- Command signatures: Auto-generated from `--help`

---

## 4. EmShell Low-Level Specification

### 4.1 Type System

```
Primitive Types:
  int         - 64-bit signed integer
  uint        - 64-bit unsigned integer
  float       - 64-bit IEEE 754 double
  string      - UTF-8 encoded text, immutable
  bool        - true / false
  path        - Filesystem path with validation
  bytes       - Raw binary data

Collections:
  array<T>    - Ordered collection of type T
  map<K, V>   - Key-value store

Example:
  int count = 42
  string name = "emboar"
  path config = "/etc/emboar/config"
  array<string> files = ["/tmp/a.txt", "/tmp/b.txt"]
```

### 4.2 Piping & Redirection

```emshell
# Pipe output to next command
ps aux | grep sshd | awk '{print $2}'

# Redirect stdout to file
echo "log entry" > /var/log/app.log

# Append to file
echo "another entry" >> /var/log/app.log

# Redirect stderr
command_with_error 2> /var/log/error.log

# Redirect both
command_mixed 2>&1 > /var/log/combined.log

# Input redirection
sort < /tmp/unsorted.txt > /tmp/sorted.txt
```

### 4.3 Color Support

```emshell
# ANSI 256-color codes
echo "Critical error!" --color=red --bold
echo "Success!" --color=green
echo "Info message" --color=cyan
echo "Muted text" --color=dark-gray

# Background colors
echo "Alert!" --bg=yellow --color=black
```

---

## 5. Package Manager (EBM)

### 5.1 Package Format (.emb)

```
emboar_package.emb (ZIP-like container):
  ├── metadata.json          # Package info, version, deps
  ├── manifest.sign          # RSA-4096 signature
  ├── payload/
  │   ├── bin/               # Executables
  │   ├── lib/               # Shared libraries
  │   ├── etc/               # Config files
  │   └── man/               # Manual pages
  └── scripts/
      ├── pre-install.sh     # Pre-install hook
      ├── post-install.sh    # Post-install hook
      └── pre-remove.sh      # Pre-remove hook
```

### 5.2 RSA-4096 Signature Verification

```
Installation flow:
1. Download package: curl-p https://repo.emboar.io/pkg.emb
2. Extract metadata.json + manifest.sign
3. Compute SHA-512 hash of payload
4. Verify signature: RSA_verify(public_key, hash, signature)
5. If valid, install; else reject with error
```

### 5.3 EBM Commands

```bash
# Install package
ebm install apache-emb --from=official-repo

# Install from file
ebm install ./custom-pkg.emb

# Remove package
ebm remove apache-emb

# List installed packages
ebm list

# Show package info
ebm info apache-emb

# Update all packages
ebm update

# Search for packages
ebm search "web server"

# Verify package signature
ebm verify-sig ./pkg.emb
```

---

## 6. Security Model

### 6.1 Zero-Trust Architecture

**Principle**: No implicit access, even for root. Every action requires re-authentication or capability proof.

```
┌─────────────────────────────────────────┐
│ User attempts: access /etc/emboar/vault │
├─────────────────────────────────────────┤
│ 1. Check file ACL (even if root)        │
│ 2. Request: "Allow access? [Y/n]"       │
│ 3. Authenticate: password re-entry      │
│ 4. Log action: /var/log/audit.json      │
│ 5. Grant/Deny with timestamp            │
└─────────────────────────────────────────┘
```

### 6.2 Immutable Audit Logging

**Purpose**: Write-once, read-only audit trail for forensics.

**Log Format** (JSON):
```json
{
  "timestamp": "2026-03-30T14:32:15Z",
  "event_type": "emsudo_execution",
  "user": "alice",
  "command": "rm -rf /etc/emboar/*",
  "exit_code": 1,
  "reason_denied": "Attempting deletion of system files",
  "session_id": "sess_12345"
}
```

**Storage**:
- Location: `/var/log/audit/` (protected, append-only)
- Signed with daily keys (rotation)
- Backed up to encrypted external storage on each reboot

### 6.3 Sudo Equivalent (EmSudo)

**Features**:
- Session timeout: 15 minutes (user-configurable)
- Granular command allowlist
- Automatic re-authentication for sensitive commands
- Full audit trail

**Configuration** (`/etc/emsudo/conf`):
```
# Allow alice to run specific commands
alice ALL=(root) /bin/ebm, /bin/firewall, /bin/encrypt

# Require password re-entry for encryption commands
alice ALL=(root) PASSWD: /bin/encrypt

# Log all emsudo usage
Defaults log

# Session timeout
Defaults timestamp_timeout=15
```

---

## 7. Cryptographic Standards

| Component | Algorithm | Details |
|-----------|-----------|---------|
| Password Hashing | Argon2id | m=256MB, t=3, p=4 |
| Full-Disk Encryption | AES-256-XTS | LUKS2 standard |
| File Encryption | AES-256-GCM | Authenticated encryption |
| Signing | RSA-4096 | PKCS#1 v2.1 (OAEP) |
| Hashing | SHA-512 | For integrity checks |
| MAC Address Generation | CSPRNG | Cryptographically secure random |
| Session Keys | ECDH-P256 | Key exchange |

---

## 8. Memory Safety Guarantees

**Rust modules (memory-safe)**:
- Kernel core
- IPC layer
- Process scheduler
- Cryptographic primitives
- Package manager
- EmShell parser

**C modules (audited, minimal unsafe)**:
- Hardware abstraction layer (CPU-specific)
- Device drivers (with bounds checking)
- Legacy library bindings

**Assembly modules (hardened)**:
- Bootloader
- Context switch routines
- Memory wipe (shutdown)

---

## 9. Implementation Phases

See [IMPLEMENTATION_ROADMAP.md](IMPLEMENTATION_ROADMAP.md) for detailed phase breakdown.

**Phase 1**: Bootloader + Kernel foundation  
**Phase 2**: Init system + Core services  
**Phase 3**: EmShell + Commands (1-40)  
**Phase 4**: Commands (41-80+) + Package manager  
**Phase 5**: Security hardening + Testing  

---

*Emboar OS Architecture v1.0*
