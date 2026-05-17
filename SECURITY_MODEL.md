# Emboar OS - Security Model

## 1. Zero-Trust Architecture

### 1.1 Principle: Never Implicit Access

Even privileged users require re-authentication and proof of authorization for sensitive operations.

### 1.2 Permission Levels

```
┌─────────────────────────────────┐
│      Requesting User            │
├─────────────────────────────────┤
│ 1. Check local ACL              │
│    - File permissions           │
│    - Group membership           │
├─────────────────────────────────┤
│ 2. Evaluate resource class      │
│    - user_home: Own directory   │
│    - system_config: root only   │
│    - vault: re-auth required    │
│    - audit_log: immutable       │
├─────────────────────────────────┤
│ 3. Re-authenticate if needed    │
│    - Prompt password            │
│    - TOTP/MFA if configured     │
│    - Create session token       │
├─────────────────────────────────┤
│ 4. Record in audit log          │
│    - User, timestamp, action    │
│    - Granted or denied          │
├─────────────────────────────────┤
│ 5. Grant/Deny access           │
└─────────────────────────────────┘
```

### 1.3 Resource Classes

| Resource | Default Owner | Re-auth? | Audit Log |
|----------|---------------|----------|-----------|
| `/home/<user>` | user | - | - |
| `/etc/emboar/config` | root | Yes | Yes |
| `/var/lib/vault/` | root | Yes | Yes |
| `/var/log/audit/` | root | - | Yes (immutable) |
| `/tmp/` | user | - | - |
| Cryptographic keys | vault | Yes | Yes |
| Shadow file | root | - | Yes |

### 1.4 Implementation

```rust
// src/security/zero_trust.rs

pub mod access_control {
    use std::fs::metadata;
    use std::os::unix::fs::PermissionsExt;

    pub struct AccessRequest {
        user: u32,
        resource: String,
        action: Action,
    }

    pub enum Action {
        Read,
        Write,
        Execute,
        Delete,
    }

    pub fn check_access(req: &AccessRequest) -> Result<bool, String> {
        // Step 1: Check ACL
        let resource_metadata = metadata(&req.resource)?;
        let current_uid = get_current_uid();

        if current_uid != 0 && !has_file_permission(&resource_metadata, req.user, &req.action) {
            return Err("Permission denied".to_string());
        }

        // Step 2: Determine if re-auth needed
        let resource_class = classify_resource(&req.resource);
        if needs_reauth(&resource_class) {
            // Step 3: Re-authenticate
            if !reauthenticate(current_uid)? {
                audit_log(&format!("Access denied: {}", req.resource));
                return Ok(false);
            }
        }

        // Step 4: Log action
        audit_log(&format!("Access granted: {} action={:?}", req.resource, req.action));

        Ok(true)
    }

    fn classify_resource(path: &str) -> ResourceClass {
        match path {
            p if p.starts_with("/home/") => ResourceClass::UserHome,
            p if p.starts_with("/etc/emboar") => ResourceClass::SystemConfig,
            p if p.starts_with("/var/lib/vault") => ResourceClass::Vault,
            p if p.starts_with("/var/log/audit") => ResourceClass::AuditLog,
            _ => ResourceClass::Normal,
        }
    }

    fn needs_reauth(class: &ResourceClass) -> bool {
        matches!(class, ResourceClass::SystemConfig | ResourceClass::Vault)
    }

    fn reauthenticate(uid: u32) -> Result<bool, String> {
        // Prompt user for password
        println!("This action requires authentication.");
        println!("Password: ");

        let password = read_password_silent()?;

        // Verify against Argon2id hash
        verify_user_password(uid, &password)
    }

    enum ResourceClass {
        UserHome,
        SystemConfig,
        Vault,
        AuditLog,
        Normal,
    }
}
```

---

## 2. Immutable Audit Logging

### 2.1 Write-Once, Read-Only Design

Audit logs are **append-only** and **cryptographically signed** to prevent tampering.

### 2.2 Log Format (JSON)

```json
{
    "timestamp": "2026-03-30T14:32:15.123Z",
    "event_type": "emsudo_execution",
    "user": "alice",
    "user_uid": 1001,
    "command": "encrypt /data/secrets.txt --key-vault",
    "exit_code": 0,
    "result": "success",
    "session_id": "sess_abc123def456",
    "session_token": "tok_xyz789",
    "duration_ms": 245,
    "resource_accessed": ["/data/secrets.txt"],
    "signature": "RSA-4096 signature of entire log entry"
}
```

### 2.3 Audit Log Storage

```
/var/log/audit/
├── 2026-03-30.log          # Daily rotation
├── 2026-03-30.sig          # RSA-4096 signature
├── 2026-03-29.log
├── 2026-03-29.sig
└── manifest.json           # Contains all log file hashes
```

**Manifest** (`manifest.json`):
```json
{
    "generated": "2026-03-30T23:59:59Z",
    "logs": [
        {
            "filename": "2026-03-30.log",
            "sha512": "abc123...",
            "size_bytes": 102400,
            "entry_count": 456
        },
        {
            "filename": "2026-03-29.log",
            "sha512": "def456...",
            "size_bytes": 98304,
            "entry_count": 423
        }
    ],
    "total_entries": 879,
    "signature": "RSA-4096 signature of manifest"
}
```

### 2.4 Implementation (Rust)

```rust
// src/security/audit_logging.rs

use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

pub struct AuditLogger {
    log_path: String,
    private_key: Vec<u8>,  // Loaded from vault at startup
}

impl AuditLogger {
    pub fn log_event(&self, event: &AuditEvent) -> Result<(), String> {
        let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let entry = json!({
            "timestamp": timestamp,
            "event_type": event.event_type,
            "user": event.user,
            "user_uid": event.user_uid,
            "command": event.command,
            "exit_code": event.exit_code,
            "result": if event.exit_code == 0 { "success" } else { "failure" },
            "session_id": event.session_id,
            "duration_ms": event.duration_ms,
            "resource_accessed": event.resources,
        });

        // Sign the entry
        let entry_str = entry.to_string();
        let signature = rsa_sign(&self.private_key, entry_str.as_bytes())?;

        let signed_entry = json!({
            "entry": entry,
            "signature": signature,
        });

        // Append to log file (atomic write)
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(|e| e.to_string())?;

        writeln!(file, "{}", signed_entry.to_string())
            .map_err(|e| e.to_string())?;

        // Make file immutable (on supported filesystems)
        set_file_immutable(&self.log_path)?;

        Ok(())
    }

    pub fn verify_logs(&self, public_key: &[u8]) -> Result<bool, String> {
        // Verify all entries in log file
        let contents = std::fs::read_to_string(&self.log_path)
            .map_err(|e| e.to_string())?;

        for line in contents.lines() {
            let entry: Value = serde_json::from_str(line)
                .map_err(|e| e.to_string())?;

            let signature = entry["signature"].as_str()
                .ok_or("Missing signature")?;
            let entry_data = entry["entry"].to_string();

            // Verify RSA signature
            if !rsa_verify(public_key, entry_data.as_bytes(), signature)? {
                return Ok(false);  // Tampering detected
            }
        }

        Ok(true)  // All entries verified
    }
}

pub struct AuditEvent {
    pub event_type: String,
    pub user: String,
    pub user_uid: u32,
    pub command: String,
    pub exit_code: i32,
    pub session_id: String,
    pub duration_ms: u64,
    pub resources: Vec<String>,
}
```

---

## 3. Sudo Equivalent (EmSudo)

### 3.1 Configuration Format

**File**: `/etc/emsudo/conf`

```
# EmSudo configuration
# Format: user HOSTS=(runuser) [PASSWD:|NOPASSWD:] commands

# All users in wheel group can run any command with password
%wheel ALL=(ALL) ALL

# Allow alice to run specific commands without password
alice ALL=(root) NOPASSWD: /bin/firewall, /bin/netstat

# Require password for sensitive operations
alice ALL=(root) PASSWD: /bin/encrypt, /bin/decrypt, /bin/vault

# Allow bob to manage packages
bob ALL=(root) /bin/ebm

# Session timeout (minutes)
Defaults timestamp_timeout=15

# Force password re-entry after timeout
Defaults timestamp_type=global

# Log to syslog
Defaults syslog=auth

# Log all commands
Defaults log_all
```

### 3.2 Session Management

```rust
// src/security/emsudo.rs

pub struct EmSudoSession {
    user: u32,
    command: String,
    timestamp: SystemTime,
    session_token: String,
}

impl EmSudoSession {
    const SESSION_TIMEOUT: Duration = Duration::from_secs(15 * 60);  // 15 minutes

    pub fn create(user: u32, command: &str) -> Result<Self, String> {
        // Verify user is allowed to run command
        let config = load_emsudo_config()?;
        if !config.is_allowed(user, command)? {
            return Err("Permission denied".to_string());
        }

        // Check if re-authentication needed
        if config.requires_password(user, command)? {
            if !reauthenticate(user)? {
                return Err("Authentication failed".to_string());
            }
        }

        // Generate session token
        let session_token = generate_secure_token();

        Ok(EmSudoSession {
            user,
            command: command.to_string(),
            timestamp: SystemTime::now(),
            session_token,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.timestamp.elapsed()
            .map(|elapsed| elapsed < Self::SESSION_TIMEOUT)
            .unwrap_or(false)
    }

    pub fn execute(&self) -> Result<i32, String> {
        if !self.is_valid() {
            return Err("Session expired".to_string());
        }

        // Execute command with elevated privileges
        let exit_code = execute_with_privileges(&self.command)?;

        // Log execution
        audit_log(&AuditEvent {
            event_type: "emsudo_execution".to_string(),
            user: get_username(self.user)?,
            user_uid: self.user,
            command: self.command.clone(),
            exit_code,
            session_id: self.session_token.clone(),
            duration_ms: 0,
            resources: vec![],
        })?;

        Ok(exit_code)
    }
}
```

---

## 4. Cryptographic Standards

| Component | Algorithm | Parameters | Justification |
|-----------|-----------|------------|-----------------|
| **Password Hashing** | Argon2id | m=256MB, t=3, p=4 | Memory-hard, GPU-resistant |
| **Full-Disk Encryption** | AES-256-XTS | LUKS2 | Industry standard for disk encryption |
| **File Encryption** | AES-256-GCM | 96-bit IV, 128-bit tag | Authenticated encryption |
| **Digital Signing** | RSA-4096 | PKCS#1 v2.1 OAEP | High assurance, long-term security |
| **Hashing** | SHA-512 | - | 512-bit output, collision-resistant |
| **Key Exchange** | ECDH | P-256 curve | Efficient, forward secrecy |
| **MAC Generation** | CSPRNG + filtering | 48-bit randomization | Sufficient entropy for MAC space |

---

## 5. Secure Deletion

### 5.1 Shred Implementation

Files are overwritten multiple times before deletion:

```
Pass 1: All 0s (0x00 × N)
Pass 2: All 1s (0xFF × N)
Pass 3: Random data (cryptographically random)
```

**Available with `shred --passes` option**:
- `3` passes (default, adequate for modern drives)
- `7` passes (DOD 5220.22-M standard, paranoid)
- `35` passes (Gutmann method, extreme)

### 5.2 Wiping Before Shutdown

At system shutdown:
1. Identify encryption keys in RAM
2. Allocate large buffer (equal to available RAM)
3. Fill with `/dev/urandom`
4. Force flush to physically do it (mlockall)
5. Clear and deallocate
6. Call ACPI shutdown

**Code** (Assembly + C):

```c
// src/kernel/secure_shutdown.c

void secure_shutdown(void) {
    // Get system memory size
    long sys_memory = sysconf(_SC_PHYS_PAGES) * sysconf(_SC_PAGE_SIZE);

    // Allocate buffer
    void *wipe_buffer = malloc(sys_memory);
    if (wipe_buffer == NULL) {
        // Fallback: use in-place wiping
        goto fallback_wipe;
    }

    // Lock pages in memory (prevent swap)
    mlockall(MCL_CURRENT | MCL_FUTURE);

    // Fill with random data
    int urandom = open("/dev/urandom", O_RDONLY);
    ssize_t bytes_read = read(urandom, wipe_buffer, sys_memory);
    close(urandom);

    // Explicit flush (barrier)
    asm volatile("mfence");

    // Zero and free
    memset(wipe_buffer, 0, sys_memory);
    free(wipe_buffer);

    // ACPI shutdown
    acpi_shutdown();

    return;

fallback_wipe:
    // In-place wipe (no allocation)
    wipe_memory_inplace();
    acpi_shutdown();
}
```

---

*Emboar OS Security Model v1.0*
