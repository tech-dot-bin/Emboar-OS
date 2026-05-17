# Emboar OS - Boot & Installation Specification

## 1. Bootloader Stages

### 1.1 Stage 1: UEFI/BIOS Entry Point (512 bytes, x86_64 Assembly)

**File**: `src/bootloader/stage1.asm`

```asm
; STAGE 1: Initial bootloader entry (x86_64)
; Loaded at 0x7C00 by BIOS or at UEFI entry point
; Responsibilities:
;   1. Enable A20 line (for memory > 1MB access)
;   2. Load GDT (Global Descriptor Table)
;   3. Switch from real mode → protected mode → long mode (64-bit)
;   4. Jump to Stage 2

[ORG 0x7C00]
[BITS 16]

start:
    cli                         ; Disable interrupts
    cld                         ; Clear direction flag

    ; Enable A20 line (legacy compatibility)
    mov ax, 0x2401
    int 0x15

    ; Install GDT (Global Descriptor Table)
    lgdt [gdt_descriptor]

    ; Switch to protected mode
    mov eax, cr0
    or eax, 1                   ; Set PE (Protected Enable) bit
    mov cr0, eax
    jmp 0x08:protected_mode

[BITS 32]
protected_mode:
    mov ax, 0x10                ; Data segment selector
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x90000           ; Set stack pointer

    ; Switch to long mode (64-bit)
    mov eax, cr4
    or eax, (1 << 5)           ; Enable PAE (Physical Address Extension)
    mov cr4, eax

    mov ecx, 0xC0000080        ; EFER MSR
    rdmsr
    or eax, (1 << 8)           ; Set LME (Long Mode Enable)
    wrmsr

    mov eax, cr0
    or eax, (1 << 31)          ; Enable paging
    mov cr0, eax

    jmp 0x18:long_mode_entry

[BITS 64]
long_mode_entry:
    mov ax, 0x20                ; Data segment for 64-bit
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, 0x100000          ; Set 64-bit stack

    ; Load Stage 2 from disk
    ; (Simplified: assume kernel at sector 1)
    call load_stage2_from_disk

    ; Verify kernel signature (RSA-4096)
    lea rsi, [kernel_image]
    call verify_kernel_signature

    cmp rax, 1
    jne halt_error              ; Halt on signature failure

    ; Jump to kernel entry point
    jmp qword [kernel_image]

load_stage2_from_disk:
    ; Read kernel from disk into memory at 0x400000
    ; Using BIOS INT 0x13 or ATA commands (PIO mode)
    ; Simplified: assume 0x100000 bytes loaded
    ret

verify_kernel_signature:
    ; RSA-4096 verification of kernel
    ; Load kernel hash, verify against public key
    ; Return 1 if valid, 0 if invalid
    ret

halt_error:
    hlt

; GDT (Global Descriptor Table)
gdt_start:
    dq 0x0000000000000000       ; Null descriptor
    dq 0x00209a0000000000       ; Code segment (64-bit)
    dq 0x0020920000000000       ; Data segment
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1  ; GDT size
    dq gdt_start                ; GDT base address

kernel_image:
    dq 0x400000                 ; Address where kernel is loaded

times 510 - ($ - $$) db 0      ; Pad to 510 bytes
dw 0xAA55                       ; Boot signature
```

### 1.2 Stage 2: Kernel Loading (C + Assembly)

**File**: `src/bootloader/stage2.c`

```c
#include <stdint.h>
#include <string.h>

typedef struct {
    uint8_t signature[256];     // RSA-4096 signature
    uint32_t kernel_size;
    uint32_t kernel_checksum;
} kernel_header_t;

// Verify kernel signature using RSA-4096
int verify_kernel_rsa_signature(uint8_t *kernel, uint32_t size, 
                                uint8_t *signature, uint8_t *public_key) {
    // Compute SHA-512 hash of kernel
    uint8_t hash[64];
    sha512_compute(kernel, size, hash);

    // RSA-4096 verification (basic outline)
    // In production: use ring or similar cryptographic library
    // Here: simplified concept
    uint8_t decrypted[256];
    rsa4096_public_decrypt(signature, decrypted, public_key);

    // Compare hash with decrypted signature (first 64 bytes)
    if (memcmp(hash, decrypted, 64) == 0) {
        return 1;  // Valid
    }
    return 0;      // Invalid
}

// Load kernel from disk and verify
void load_kernel(void) {
    kernel_header_t *header = (kernel_header_t *)0x400000;

    // Check signature
    uint8_t *public_key = embedded_public_key;  // Embedded during build
    if (!verify_kernel_rsa_signature((uint8_t *)0x400000 + sizeof(kernel_header_t),
                                      header->kernel_size,
                                      header->signature,
                                      public_key)) {
        panic("Kernel signature verification failed!");
    }

    // Verify checksum
    uint32_t computed_checksum = crc32((uint8_t *)0x400000, header->kernel_size);
    if (computed_checksum != header->kernel_checksum) {
        panic("Kernel checksum mismatch!");
    }

    // Set up initial page tables
    setup_4level_paging();

    // Jump to kernel main
    typedef void (*kernel_entry_t)(void);
    kernel_entry_t kernel_main = (kernel_entry_t)(0x400000 + sizeof(kernel_header_t));
    kernel_main();
}

// Basic panic (halts system)
void panic(const char *msg) {
    // Write error message to serial port
    serial_write_string(msg);
    asm("hlt");
}

// CRC32 checksum
uint32_t crc32(uint8_t *data, uint32_t length) {
    uint32_t crc = 0xFFFFFFFF;
    for (uint32_t i = 0; i < length; i++) {
        crc ^= data[i];
        for (int j = 0; j < 8; j++) {
            crc = (crc >> 1) ^ ((crc & 1) ? 0xEDB88320 : 0);
        }
    }
    return crc ^ 0xFFFFFFFF;
}
```

---

## 2. Installation Routine

### 2.1 Pre-Installation: Live Boot

**File**: `src/install/emboar-setup.sh`

```bash
#!/bin/emshell

# Emboar OS Installation Setup Script
# This script runs in the live environment

set -e

echo "╔═══════════════════════════════╗"
echo "║  Emboar OS Installation Tool  ║"
echo "║  Version: 1.0 Alpha           ║"
echo "╚═══════════════════════════════╝"

# Step 1: Detect available disks
echo ""
echo "Available disks:"
lsblk --nodeps --output NAME,SIZE,TYPE

echo ""
echo "Select target disk (e.g., sda, sdb):"
read -r target_disk

TARGET="/dev/${target_disk}"
check_disk_exists "$TARGET" || exit 1

echo ""
echo "WARNING: All data on $TARGET will be erased!"
echo "Type 'yes' to continue:"
read -r confirm
[[ "$confirm" != "yes" ]] && exit 0

# Step 2: Partition disk (LVM/LUKS)
echo ""
echo "Setting up encryption and partitioning..."
setup_encrypted_volumes "$TARGET"

# Step 3: Create filesystems
echo "Creating filesystems..."
mkfs.ext4 /dev/mapper/emboar_vg-root
mkfs.ext4 /dev/mapper/emboar_vg-var

# Step 4: Create user
echo ""
echo "Setting up user account..."
echo "Enter username:"
read -r username

setup_user "$username"

# Step 5: Install bootloader
echo ""
echo "Installing bootloader..."
install_bootloader "$TARGET"

echo ""
echo "Installation complete!"
echo "System will reboot in 5 seconds..."
sleep 5
reboot
```

### 2.2 Disk Partitioning with LVM/LUKS

**File**: `src/install/partition.sh`

```bash
#!/bin/bash

setup_encrypted_volumes() {
    local disk=$1

    echo "Partitioning $disk..."

    # Clear MBR/GPT
    dd if=/dev/zero of="$disk" bs=1M count=1

    # Create partition table (GPT for UEFI)
    parted -s "$disk" mklabel gpt

    # Create partitions
    # Partition 1: EFI System (512 MB)
    parted -s "$disk" mkpart ESP fat32 1MiB 513MiB
    parted -s "$disk" set 1 esp on

    # Partition 2: Boot (256 MB, unencrypted)
    parted -s "$disk" mkpart boot ext4 513MiB 769MiB

    # Partition 3: Root + Var (Rest of disk, encrypted)
    parted -s "$disk" mkpart root ext4 769MiB 100%

    # Format EFI partition
    mkfs.fat -F 32 "${disk}1"

    # Prompt for encryption password
    echo ""
    echo "Enter encryption password for root partition:"
    echo "(Must be 16+ characters, saved to memory only)"
    read -rs encryption_password

    # Create LUKS2 encrypted container
    echo -n "$encryption_password" | cryptsetup luksFormat \
        --type luks2 \
        --cipher aes-xts-plain64 \
        --key-size 512 \
        --iter-time 5000 \
        --pbkdf pbkdf2 \
        "${disk}3" -

    # Open encrypted volume
    echo -n "$encryption_password" | cryptsetup open "${disk}3" emboar_crypt -

    # Set up LVM on encrypted volume
    pvcreate /dev/mapper/emboar_crypt
    vgcreate emboar_vg /dev/mapper/emboar_crypt

    # Create logical volumes
    lvcreate -L 30G -n root emboar_vg
    lvcreate -L 10G -n var emboar_vg
    lvcreate -L 5G -n home emboar_vg

    echo "Encryption and partitioning complete!"
    echo ""
    echo "Partition Summary:"
    lvdisplay
}
```

### 2.3 User Identity Setup

**File**: `src/install/user_setup.c`

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pwd.h>
#include <grp.h>
#include <argon2.h>

// Argon2id parameters (recommended by OWASP)
#define ARGON2_PASSWORD_HASH_TIME 3
#define ARGON2_PASSWORD_HASH_MEMORY (256 * 1024)  // 256 MB
#define ARGON2_PASSWORD_HASH_PARALLELISM 4
#define ARGON2_SALT_LENGTH 32
#define ARGON2_HASH_LENGTH 64

typedef struct {
    char username[256];
    uint8_t salt[ARGON2_SALT_LENGTH];
    uint8_t hash[ARGON2_HASH_LENGTH];
    uint64_t uid;
} emboar_user_t;

// Hash password using Argon2id
int hash_password_argon2id(const char *password, uint8_t *salt, uint8_t *hash) {
    // Generate random salt if not provided
    if (salt == NULL) {
        salt = malloc(ARGON2_SALT_LENGTH);
        if (getrandom(salt, ARGON2_SALT_LENGTH, 0) == -1) {
            perror("getrandom");
            return -1;
        }
    }

    // Hash password using Argon2id
    int ret = argon2id_hash_raw(
        ARGON2_PASSWORD_HASH_TIME,
        ARGON2_PASSWORD_HASH_MEMORY,
        ARGON2_PASSWORD_HASH_PARALLELISM,
        password,
        strlen(password),
        salt,
        ARGON2_SALT_LENGTH,
        hash,
        ARGON2_HASH_LENGTH
    );

    if (ret != ARGON2_OK) {
        fprintf(stderr, "Argon2id hashing failed: %s\n", argon2_error_message(ret));
        return -1;
    }

    return 0;
}

// Create user account
int create_user(const char *username, const char *password) {
    FILE *shadow_file;
    emboar_user_t user;
    uint8_t hash[ARGON2_HASH_LENGTH];
    uint8_t salt[ARGON2_SALT_LENGTH];

    strncpy(user.username, username, 255);

    // Hash password
    if (hash_password_argon2id(password, salt, hash) != 0) {
        return -1;
    }

    memcpy(user.salt, salt, ARGON2_SALT_LENGTH);
    memcpy(user.hash, hash, ARGON2_HASH_LENGTH);

    // Assign UID (next available after 1000)
    user.uid = get_next_uid();

    // Create home directory
    char home_dir[512];
    snprintf(home_dir, sizeof(home_dir), "/home/%s", username);
    mkdir(home_dir, 0700);
    chown(home_dir, user.uid, user.uid);

    // Write to shadow file (encrypted credentials)
    shadow_file = fopen("/etc/emboar/shadow", "a");
    if (shadow_file == NULL) {
        perror("fopen");
        return -1;
    }

    fprintf(shadow_file, "%s:%ld:%s\n", user.username, user.uid, 
            bytes_to_hex(hash, ARGON2_HASH_LENGTH));
    fclose(shadow_file);

    // Set permissions on shadow file (root only)
    chmod("/etc/emboar/shadow", 0600);

    printf("User '%s' created successfully (UID: %ld)\n", username, user.uid);
    return 0;
}

// Verify password against stored hash
int verify_password(const char *username, const char *password) {
    FILE *shadow_file = fopen("/etc/emboar/shadow", "r");
    if (shadow_file == NULL) {
        perror("fopen");
        return 0;
    }

    char line[1024];
    while (fgets(line, sizeof(line), shadow_file) != NULL) {
        char stored_username[256];
        char stored_hash_hex[256];
        sscanf(line, "%255[^:]:%*ld:%255s", stored_username, stored_hash_hex);

        if (strcmp(stored_username, username) == 0) {
            // Hash incoming password
            uint8_t incoming_hash[ARGON2_HASH_LENGTH];
            // Note: In production, retrieve salt from /etc/emboar/salt file
            uint8_t salt[ARGON2_SALT_LENGTH];  // Placeholder
            hash_password_argon2id(password, salt, incoming_hash);

            // Compare hashes (timing-safe comparison)
            uint8_t stored_hash[ARGON2_HASH_LENGTH];
            hex_to_bytes(stored_hash_hex, stored_hash, ARGON2_HASH_LENGTH);

            int match = constant_time_compare(incoming_hash, stored_hash, ARGON2_HASH_LENGTH);
            fclose(shadow_file);
            return match;
        }
    }

    fclose(shadow_file);
    return 0;  // User not found
}
```

### 2.4 Boot Integrity Setup

**File**: `src/install/integrity_check.sh`

```bash
#!/bin/bash

# Generate integrity database at installation time
setup_boot_integrity() {
    echo "Generating boot integrity manifest..."

    # Create directory if not exists
    mkdir -p /etc/emboar

    # Compute SHA-512 hashes of critical binaries
    {
        echo "# Boot Integrity Manifest - $(date -u)"
        echo ""
        
        for binary in /bin/emshell /bin/ls /bin/ps /sbin/init /sbin/emsudo; do
            if [[ -f "$binary" ]]; then
                sha512sum "$binary" | awk '{print $1 " " $2}'
            fi
        done
        
        echo ""
        echo "# System libraries"
        for lib in /lib/libx.so.1 /lib/libc.so.6; do
            if [[ -f "$lib" ]]; then
                sha512sum "$lib" | awk '{print $1 " " $2}'
            fi
        done
    } > /etc/emboar/boot_manifest.txt

    # Sign the manifest with private key (generated during install)
    openssl dgst -sha512 -sign /etc/emboar/private_key.pem \
        /etc/emboar/boot_manifest.txt \
        > /etc/emboar/boot_manifest.sig

    # Make immutable
    chmod 0400 /etc/emboar/boot_manifest.txt
    chmod 0400 /etc/emboar/boot_manifest.sig

    echo "Boot integrity manifest created and signed."
}

# Verify integrity on boot (called by init)
verify_boot_integrity() {
    echo "Verifying boot integrity..."

    # Verify signature
    openssl dgst -sha512 -verify /etc/emboar/public_key.pem \
        -signature /etc/emboar/boot_manifest.sig \
        /etc/emboar/boot_manifest.txt

    if [[ $? -ne 0 ]]; then
        echo "ERROR: Boot integrity verification failed!"
        echo "System may have been tampered with. Halting."
        sleep 5
        halt
    fi

    echo "Boot integrity verified. System safe to proceed."
}
```

### 2.5 Security Integration (EmSudo Setup)

**File**: `src/install/emsudo_setup.sh`

```bash
#!/bin/bash

setup_emsudo() {
    echo "Configuring EmSudo (elevated privilege system)..."

    mkdir -p /etc/emsudo

    # Create default emsudo configuration
    cat > /etc/emsudo/conf << 'EOF'
# EmSudo Configuration
# Format: user HOST=(runuser) PASSWD: command

# Allow users in group 'wheel' to run any command
%wheel ALL=(ALL) ALL

# Specific command permissions
alice ALL=(root) /bin/ebm, /bin/firewall
bob ALL=(root) PASSWD: /bin/encrypt, /bin/decrypt

# Session configuration
# Default session timeout: 15 minutes (in minutes)
Defaults timestamp_timeout=15

# Require password for sensitive operations
Defaults use_pty

# Enable logging
Defaults log
Defaults logfile="/var/log/emsudo.log"
EOF

    chmod 0440 /etc/emsudo/conf
    chown root:root /etc/emsudo/conf

    # Create log directory
    mkdir -p /var/log
    touch /var/log/emsudo.log
    chmod 0600 /var/log/emsudo.log

    echo "EmSudo configuration complete."
}
```

---

## 3. Boot Flow Sequence

```
1. Power on → UEFI/BIOS Firmware
     ↓
2. Load Stage 1 Bootloader (512 bytes)
     ↓
3. Initialize CPU (A20, GDT, Protected Mode, Long Mode)
     ↓
4. Load Stage 2 from disk
     ↓
5. Verify kernel signature (RSA-4096)
     ↓
6. Set up initial page tables
     ↓
7. Jump to Kernel Entry Point
     ↓
8. Kernel initializes:
    - CPU exceptions & interrupts
    - Virtual memory (paging)
    - Early device drivers
     ↓
9. Mount root filesystem (decrypt LUKS if needed)
     ↓
10. Launch init system (PID 1)
     ↓
11. Init verifies boot integrity
     ↓
12. Init launches services (filesystem, network, logging)
     ↓
13. Init spawns EmShell for user
     ↓
14. System ready for user input
```

---

*Emboar OS Boot & Installation v1.0*
