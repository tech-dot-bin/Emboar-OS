# Emboar OS - Core Features

## 1. Priority Governor

### 1.1 Resource Allocation Strategy

The Priority Governor dynamically manages CPU and RAM allocation based on system activity and user input. This ensures responsive terminal sessions while preventing background tasks from hogging resources.

### 1.2 Architecture

```
┌─────────────────────────────────────────────┐
│     Priority Governor Daemon (user-space)   │
├─────────────────────────────────────────────┤
│ • Monitor active processes                  │
│ • Detect terminal input (keyboard/mouse)    │
│ • Measure CPU/memory per process            │
│ • Adjust CGroup limits dynamically          │
│ • Update nice values                        │
└─────────────────────────────────────────────┘
         ↓
     (IPC calls via /dev/cgroup_daemon)
         ↓
┌─────────────────────────────────────────────┐
│  Kernel CGroup Subsystem (enforcement)      │
├─────────────────────────────────────────────┤
│ • CPU.max enforcement                       │
│ • Memory limits (OOM killer)                │
│ • I/O throttling                            │
│ • Process priority (scheduler hints)        │
└─────────────────────────────────────────────┘
```

### 1.3 Priority Classes

| Class | CPU Limit | RAM Limit | Example |
|-------|-----------|-----------|---------|
| **Interactive** | 100% (1-2 cores) | 2GB | Terminal, text editor |
| **Standard** | 50% | 1GB | Network services |
| **Background** | 25% | 500MB | Cron jobs, backups |
| **System** | 10% | 200MB | Logging daemon |

### 1.4 Implementation (Rust)

```rust
// src/services/priority_governor/main.rs

use std::fs;
use std::time::SystemTime;
use procfs::process::all_processes;

const INTERACTIVE_THRESHOLD: i32 = 100;  // ms since last input
const KBD_INPUT_PATH: &str = "/proc/input/mice";

pub struct PriorityGovernor {
    last_input_time: SystemTime,
    process_priorities: HashMap<u32, ProcessPriority>,
}

#[derive(Clone, Debug)]
pub struct ProcessPriority {
    pid: u32,
    class: PriorityClass,
    nice: i32,
    last_update: SystemTime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PriorityClass {
    Interactive,
    Standard,
    Background,
    System,
}

impl PriorityGovernor {
    pub fn new() -> Self {
        Self {
            last_input_time: SystemTime::now(),
            process_priorities: HashMap::new(),
        }
    }

    /// Main loop: runs every 100ms
    pub fn update_priorities(&mut self) {
        // Check for user input
        let elapsed = self.last_input_time
            .elapsed()
            .unwrap_or_default()
            .as_millis();

        for process in all_processes().unwrap() {
            match process {
                Ok(proc) => {
                    let pid = proc.pid;
                    let priority_class = self.determine_priority(&proc, elapsed);

                    if priority_class != self.get_priority_class(pid) {
                        self.apply_priority(pid, &priority_class);
                        self.process_priorities.insert(
                            pid as u32,
                            ProcessPriority {
                                pid: pid as u32,
                                class: priority_class,
                                nice: self.class_to_nice(&priority_class),
                                last_update: SystemTime::now(),
                            },
                        );
                    }
                }
                Err(_) => {}
            }
        }
    }

    fn determine_priority(&self, proc: &Process, input_elapsed_ms: u128) -> PriorityClass {
        // If terminal is active, boost foreground process
        if input_elapsed_ms < INTERACTIVE_THRESHOLD {
            if let Ok(stat) = proc.stat {
                if stat.flags.contains(&procfs::process::StatFlags::VforkDone) {
                    return PriorityClass::Interactive;
                }
            }
        }

        // Detect service processes
        if let Ok(cmdline) = proc.cmdline {
            if cmdline.iter().any(|c| c.contains("sshd")
                || c.contains("logging")
                || c.contains("init")) {
                return PriorityClass::System;
            }
        }

        // Check CPU/memory usage
        if let Ok(stat) = proc.stat {
            if stat.utime + stat.stime > 1000 {
                return PriorityClass::Background;
            }
        }

        PriorityClass::Standard
    }

    fn apply_priority(&self, pid: u32, class: &PriorityClass) {
        let nice = self.class_to_nice(class);
        let cpu_quota = self.class_to_cpu_quota(class);
        let memory_limit = self.class_to_memory_limit(class);

        // Set nice value
        unsafe {
            libc::setpriority(0, pid, nice);
        }

        // Update CGroup (cgroup v2)
        let cgroup_path = format!("/sys/fs/cgroup/agent_{}", pid);
        if let Ok(_) = fs::create_dir_all(&cgroup_path) {
            fs::write(
                format!("{}/cpu.max", cgroup_path),
                format!("{} 100000", cpu_quota),
            ).ok();

            fs::write(
                format!("{}/memory.max", cgroup_path),
                memory_limit.to_string(),
            ).ok();
        }
    }

    fn class_to_nice(&self, class: &PriorityClass) -> i32 {
        match class {
            PriorityClass::Interactive => -10,
            PriorityClass::Standard => 0,
            PriorityClass::Background => 10,
            PriorityClass::System => 5,
        }
    }

    fn class_to_cpu_quota(&self, class: &PriorityClass) -> u64 {
        match class {
            PriorityClass::Interactive => 100000,  // 100%
            PriorityClass::Standard => 50000,      // 50%
            PriorityClass::Background => 25000,    // 25%
            PriorityClass::System => 10000,        // 10%
        }
    }

    fn class_to_memory_limit(&self, class: &PriorityClass) -> u64 {
        match class {
            PriorityClass::Interactive => 2 * 1024 * 1024 * 1024,    // 2GB
            PriorityClass::Standard => 1 * 1024 * 1024 * 1024,       // 1GB
            PriorityClass::Background => 500 * 1024 * 1024,          // 500MB
            PriorityClass::System => 200 * 1024 * 1024,              // 200MB
        }
    }

    fn get_priority_class(&self, pid: u32) -> PriorityClass {
        self.process_priorities
            .get(&pid)
            .map(|p| p.class.clone())
            .unwrap_or(PriorityClass::Standard)
    }
}

// Main loop
fn main() {
    let mut governor = PriorityGovernor::new();

    loop {
        governor.update_priorities();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
```

---

## 2. EmShell: Secure Bash-Like Shell

### 2.1 Design Goals

1. **Strict Typing**: Prevent type coercion vulnerabilities
2. **ANSI Colors**: Full 256-color support for user feedback
3. **Piping & Redirection**: Unix philosophy pipes and redirects
4. **Session Safety**: Encrypted history, timeout protection
5. **Command Sandboxing**: Built-in commands can't break out

### 2.2 EmShell Architecture

```
User Input
    ↓
Lexer (tokenize)
    ↓
Parser (build AST with type checking)
    ↓
Type Checker (verify types, emit errors if mismatch)
    ↓
Optimizer (constant folding, dead code elimination)
    ↓
Interpreter (execute AST, manage state)
    ↓
Output (with ANSI colors)
```

### 2.3 Example EmShell Programs

```emshell
# Declare typed variables
int count = 5
string name = "alice"
path config = "/etc/emboar/config"
array<string> files = ["a.txt", "b.txt"]

# Pipe with colors
ps aux | grep sshd | head -5

# Conditional with type check
if [[ count > 0 ]]; then
    echo "Count is positive" --color=green
else
    echo "Count is zero or negative" --color=red
fi

# Function with return type
function backup(path source, path dest) -> int {
    if [[ -d "$source" ]]; then
        tar --create --file="$dest/backup.tar" "$source"
        return 0
    fi
    return 1
}

# Loop
for file in array; do
    echo "Processing: $file"
    hashcheck "$file" > "$file.sha512"
done

# Error handling
result = encrypt "secret.txt" --key-vault
if [[ $? -ne 0 ]]; then
    echo "Encryption failed!" --color=red --bold
    exit 1
fi
```

### 2.4 Type System Details

```
Primitive types:
  int, uint, float, bool, string, path, bytes

Composite types:
  array<T>        - Ordered list
  map<K, V>       - Key-value map
  (T1, T2, ...)   - Tuple

Type coercion rules:
  - string → int: only if string matches /^-?[0-9]+$/
  - int → string: always allowed
  - path → string: always allowed
  - string → path: validate path doesn't escape root

Type safety:
  - No implicit coercions
  - All operations type-checked at parse time
  - Runtime type checks on external input
```

---

## 3. Privacy-First System

### 3.1 MAC Address Randomization

**Boot-time process**:
```bash
#!/bin/emshell

# Called by init at boot
randomize_mac_addresses:
    for interface in /sys/class/net/*; do
        iface_name=$(basename "$interface")
        
        # Generate random MAC (keeping OUI for driver compat)
        random_mac=$(generate_random_mac)
        
        # Apply MAC
        ip link set "$iface_name" address "$random_mac"
        
        # Log action (audit)
        audit_log "MAC address randomized: $iface_name → $random_mac"
    done
```

**Features**:
- Randomization at each boot
- Audit trail of old MAC for forensics
- Keeps vendor OUI in first 3 bytes (compatibility)
- No persistent MAC storage

### 3.2 RAM Wiping on Shutdown

**Assembly routine** (x86_64):
```asm
; src/kernel/mem_wipe.asm

; void wipe_memory_regions(void *start, size_t size)
; rdi = start address
; rsi = size in bytes
wipe_memory:
    push rbp
    mov rbp, rsp

    mov rcx, rsi            ; size in bytes
    xor rax, rax            ; zero register
    mov rdi, rdi            ; start address already in rdi

    ; Fill with zeros
    shr rcx, 3              ; convert bytes to 8-byte chunks (qwords)
    rep stosq               ; fill memory with zeros

    ; Flush CPU caches
    mov rax, rcx
    call clflush_all

    pop rbp
    ret

; Flush all kernel memory from CPU caches
clflush_all:
    mov rcx, 0
.loop:
    clflush [rcx]
    add rcx, 64             ; CPU cache line size
    cmp rcx, 0x100000000    ; check if reached top of addressable memory
    jl .loop
    ret
```

**Called during shutdown**:
```c
// src/kernel/shutdown.c
void shutdown_sequence(void) {
    // Kill all user processes
    kill_all_processes();

    // Unmount filesystems
    unmount_filesystems();

    // Wipe sensitive kernel memory
    wipe_memory_regions((void *)0x0, 0x80000000);  // Lower 2GB

    // Trigger ACPI shutdown
    acpi_shutdown();
}
```

### 3.3 Zero Telemetry Enforcement

**Firewall rules** (NFTables):
```nftables
table inet emboar_telemetry_block {
    chain outbound {
        type filter hook output priority 0;
        policy drop;

        # Allow SSH (port 22)
        tcp dport 22 accept

        # Allow DNS (port 53) only through system resolver
        udp dport 53 accept

        # Allow HTTP/HTTPS (user-initiated only)
        tcp dport { 80, 443 } accept

        # Reject everything else
        reject with icmpx type admin-prohibited
    }

    chain input {
        type filter hook input priority 0;
        policy drop;

        # Allow SSH inbound
        tcp dport 22 accept

        # Accept ICMP (ping)
        icmp type echo-request accept
    }
}
```

**Kernel check**: Patch in kernel scheduler prevents spawning processes that communicate to known telemetry servers (e.g., google analytics, mixpanel).

---

## 4. Documentation System

### 4.1 Man Page Format

**File**: `/usr/share/man/man1/ls.1` (groff format)

```groff
.TH ls 1 "March 2026" "Emboar OS" "User Commands"
.SH NAME
ls \- list directory contents
.SH SYNOPSIS
.B ls
[\fIOPTIONS\fR] [\fIPATH\fR]
.SH DESCRIPTION
The
.B ls
utility lists files and directories, displaying their names, sizes, permissions,
and modification times. Files are color-coded for quick visual parsing.
.SH OPTIONS
.TP
.B -l
Long format with detailed permissions and ownership.
.TP
.B -a
Show hidden files (starting with .).
.TP
.B -h
Human-readable file sizes (KB, MB, GB).
.SH EXAMPLES
.BR "ls -la /home"
Display all files in detailed format.
.BR "ls -lh /var/log"
Show sizes in human-readable format.
.SH EXIT STATUS
.TP 5
.B 0
Success.
.TP
.B 1
Directory not found.
.TP
.B 2
Permission denied.
.SH SEE ALSO
.BR find (1),
.BR cp (1)
```

### 4.2 ELI5 Documentation

**File**: `/etc/emboar/docs/doesthisdo/ls.txt`

```
[command] ls
[category] File Operations
[summary]
Lists all files and folders in a directory, showing their names, sizes,
and who can access them. Color-coded for quick recognition: blue for
folders, green for programs, white for regular files.

[related-commands]
- find: search for files in a directory tree
- ls-long: similar to 'ls -l'
```

### 4.3 Dynamic Help System

```rust
// src/emshell/docs.rs

pub struct DocSystem {
    man_pages: HashMap<String, ManPage>,
    eli5_docs: HashMap<String, String>,
}

impl DocSystem {
    pub fn show_man(cmd: &str) -> Result<String> {
        // Check if man page exists
        let man_path = format!("/usr/share/man/man1/{}.1", cmd);
        if !Path::new(&man_path).exists() {
            return Err(format!("No manual entry for '{}'", cmd));
        }

        // Format with groff
        let output = std::process::Command::new("groff")
            .args(&["-T", "utf8", "-man", &man_path])
            .output()?;

        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn show_eli5(cmd: &str) -> Result<String> {
        let eli5_path = format!("/etc/emboar/docs/doesthisdo/{}.txt", cmd);
        fs::read_to_string(&eli5_path)
            .map_err(|_| format!("No ELI5 for '{}'", cmd))
    }

    pub fn show_examples(cmd: &str) -> Result<String> {
        // Extract EXAMPLES section from man page
        let man_text = Self::show_man(cmd)?;
        let start = man_text.find("EXAMPLES").ok_or("No examples found")?;
        let end = man_text[start..].find("\n\n").unwrap_or(man_text.len() - start);
        Ok(man_text[start..start + end].to_string())
    }
}
```

---

*Emboar OS Core Features v1.0*
