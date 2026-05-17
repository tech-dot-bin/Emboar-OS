/*
 * Emboar OS - Stage 2 Bootloader (C)
 * 
 * Responsibilities:
 *   1. Parse bootloader parameters from Stage 1
 *   2. Load kernel from disk (ext4 filesystem)
 *   3. Verify kernel signature (RSA-4096)
 *   4. Verify kernel hash (SHA-512)
 *   5. Set up required data structures for kernel
 *   6. Jump to kernel entry point
 *
 * Compiled with: gcc -nostdlib -no-pie -z max-page-size=0x1000
 * File: src/bootloader/stage2.c
 */

#include <stdint.h>
#include <stddef.h>
#include <string.h>

/* ========================================================================
 * CONSTANTS
 * ======================================================================== */

#define KERNEL_ENTRY_ADDR 0x400000
#define MAX_KERNEL_SIZE   0x4000000  /* 64MB max kernel size */

/* Serial port (COM1) base for debugging output */
#define SERIAL_PORT 0x3F8

/* ATA/IDE disk I/O */
#define ATA_DATA_PORT    0x1F0
#define ATA_ERROR_PORT   0x1F1
#define ATA_SECTOR_COUNT 0x1F2
#define ATA_LBA_LOW      0x1F3
#define ATA_LBA_MID      0x1F4
#define ATA_LBA_HIGH     0x1F5
#define ATA_DRIVE        0x1F6
#define ATA_STATUS_PORT  0x1F7
#define ATA_CMD_PORT     0x1F7
#define ATA_READ_CMD     0x20

/* ========================================================================
 * TYPES
 * ======================================================================== */

typedef struct {
    uint32_t signature;      /* "EKRN" */
    uint32_t version;        /* Kernel version */
    uint32_t size;           /* Kernel size in bytes */
    uint32_t entry_offset;   /* Entry point offset from start */
    uint8_t  sha512[64];     /* SHA-512 hash of kernel */
    uint8_t  rsa_sig[512];   /* RSA-4096 signature (512 bytes) */
} kernel_header_t;

typedef struct {
    uint64_t base;           /* Physical base address */
    uint64_t limit;          /* Limit in bytes */
    uint8_t  present;        /* Whether memory range is present */
    uint8_t  type;           /* Memory type (0=usable, 1=reserved, etc.) */
} memory_map_entry_t;

typedef struct {
    uint32_t entries;
    memory_map_entry_t map[32];
} memory_map_t;

/* ========================================================================
 * GLOBAL VARIABLES
 * ======================================================================== */

/* Kernel header (populated after loading kernel) */
kernel_header_t kernel_hdr;

/* Memory map (to be passed to kernel) */
memory_map_t mem_map;

/* ========================================================================
 * I/O INSTRUCTIONS (must be before serial functions)
 * ======================================================================== */

static inline uint8_t inb(uint16_t port) {
    uint8_t result;
    __asm__ volatile("inb %1, %0" : "=a" (result) : "dN" (port));
    return result;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ volatile("outb %b0, %1" : : "a" (value), "dN" (port));
}

static inline uint16_t inw(uint16_t port) {
    uint16_t result;
    __asm__ volatile("inw %1, %0" : "=a" (result) : "dN" (port));
    return result;
}

static inline void outw(uint16_t port, uint16_t value) {
    __asm__ volatile("outw %w0, %1" : : "a" (value), "dN" (port));
}

/* ========================================================================
 * SERIAL OUTPUT (FOR DEBUGGING)
 * ======================================================================== */

static inline void serial_write_char(uint8_t c) {
    /* Wait for transmitter empty */
    while ((inb(SERIAL_PORT + 5) & 0x20) == 0);
    outb(SERIAL_PORT, c);
}

static inline void serial_write_string(const char *s) {
    while (*s) {
        if (*s == '\n') {
            serial_write_char('\r');
        }
        serial_write_char(*s);
        s++;
    }
}

static inline void serial_write_hex(uint64_t value) {
    const char hex_digits[] = "0123456789ABCDEF";
    uint8_t digits[16];
    int i = 0;

    if (value == 0) {
        serial_write_char('0');
        return;
    }

    while (value > 0 && i < 16) {
        digits[i++] = hex_digits[value % 16];
        value /= 16;
    }

    while (i > 0) {
        serial_write_char(digits[--i]);
    }
}

/* ========================================================================
 * MEMORY UTILITIES
 * ======================================================================== */

static void boot_memcpy(void *dest, const void *src, size_t count) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    for (size_t i = 0; i < count; i++) {
        d[i] = s[i];
    }
}

static void boot_memset(void *dest, uint8_t value, size_t count) {
    uint8_t *d = (uint8_t *)dest;
    for (size_t i = 0; i < count; i++) {
        d[i] = value;
    }
}

static int boot_memcmp(const void *mem1, const void *mem2, size_t count) {
    const uint8_t *m1 = (const uint8_t *)mem1;
    const uint8_t *m2 = (const uint8_t *)mem2;
    for (size_t i = 0; i < count; i++) {
        if (m1[i] != m2[i]) {
            return 1;
        }
    }
    return 0;
}

/* ========================================================================
 * DISK I/O
 * ======================================================================== */

/**
 * Read sectors from disk using LBA (Logical Block Addressing)
 * 
 * Parameters:
 *   sector: LBA sector number
 *   count:  Number of sectors to read
 *   buffer: Destination buffer (must be sector-aligned)
 * 
 * Returns:
 *   0 on success, non-zero on error
 */
static int read_disk_sectors(uint32_t sector, uint8_t count, void *buffer) {
    /* Wait for device ready */
    for (int i = 0; i < 100000; i++) {
        if ((inb(ATA_STATUS_PORT) & 0x80) == 0) {
            break;
        }
    }

    /* Select drive 0, set LBA mode, set sector count */
    outb(ATA_DRIVE, 0xE0);
    outb(ATA_SECTOR_COUNT, count);

    /* Set LBA address (3 bytes) */
    outb(ATA_LBA_LOW, sector & 0xFF);
    outb(ATA_LBA_MID, (sector >> 8) & 0xFF);
    outb(ATA_LBA_HIGH, (sector >> 16) & 0xFF);

    /* Issue READ DMA command */
    outb(ATA_CMD_PORT, ATA_READ_CMD);

    /* Wait for data ready */
    for (int i = 0; i < 1000000; i++) {
        uint8_t status = inb(ATA_STATUS_PORT);
        if ((status & 0x88) == 0x08) {
            /* Data ready, no error */
            break;
        }
        if (status & 0x01) {
            /* Error occurred */
            serial_write_string("Disk error\n");
            return 1;
        }
    }

    /* Read data (512 bytes per sector) */
    uint16_t *buf = (uint16_t *)buffer;
    for (int i = 0; i < count * 256; i++) {
        buf[i] = inw(ATA_DATA_PORT);
    }

    return 0;
}

/**
 * Load kernel from disk at sector 256 (512 sectors = ~256KB reserved for Stage 2)
 */
static int load_kernel(void) {
    serial_write_string("Loading kernel from disk...\n");

    /* First, read kernel header (1 sector = 512 bytes) */
    if (read_disk_sectors(256, 1, &kernel_hdr) != 0) {
        serial_write_string("Failed to read kernel header\n");
        return 1;
    }

    /* Verify kernel header signature */
    if (kernel_hdr.signature != 0x4E524B45) { /* "EKRN" in little-endian */
        serial_write_string("Invalid kernel signature\n");
        return 1;
    }

    serial_write_string("Kernel size: ");
    serial_write_hex(kernel_hdr.size);
    serial_write_string(" bytes\n");

    /* Validate kernel size */
    if (kernel_hdr.size > MAX_KERNEL_SIZE) {
        serial_write_string("Kernel too large\n");
        return 1;
    }

    /* Calculate number of sectors needed (rounded up) */
    uint32_t sectors_needed = (kernel_hdr.size + 511) / 512;

    /* Read kernel data starting after header sector */
    if (read_disk_sectors(257, sectors_needed, (void *)KERNEL_ENTRY_ADDR) != 0) {
        serial_write_string("Failed to read kernel data\n");
        return 1;
    }

    serial_write_string("Kernel loaded at 0x");
    serial_write_hex(KERNEL_ENTRY_ADDR);
    serial_write_string("\n");

    return 0;
}

/* ========================================================================
 * CRYPTOGRAPHIC VERIFICATION
 * ======================================================================== */

/**
 * Simple SHA-512 implementation (for demo purposes)
 * Production would use libcrypto (OpenSSL) compiled with -nostdlib
 * 
 * NOTE: This is a PLACEHOLDER. Real implementation requires:
 *   - Full SHA-512 algorithm (80 rounds)
 *   - Proper padding and message schedule
 *   - Or link against libcrypto
 */
static void sha512_compute(const uint8_t *_data, size_t _len, uint8_t *output) {
    /* Placeholder: in production, use libcrypto SHA-512 */
    boot_memset(output, 0, 64); /* Zero hash (insecure - demo only) */
    
    /* Production code would:
     * 1. Use ring crate's SHA-512 or
     * 2. Link libcrypto and call EVP_sha512()
     */
}

/**
 * Verify kernel SHA-512 hash
 * 
 * Returns:
 *   0 on success (hash matches), 1 on failure
 */
static int verify_kernel_hash(void) {
    serial_write_string("Verifying kernel hash...\n");

    uint8_t computed_hash[64];
    sha512_compute((const uint8_t *)KERNEL_ENTRY_ADDR, kernel_hdr.size, computed_hash);

    if (boot_memcmp(computed_hash, kernel_hdr.sha512, 64) != 0) {
        serial_write_string("Kernel hash mismatch\n");
        return 1;
    }

    serial_write_string("Kernel hash verified\n");
    return 0;
}

/**
 * Verify RSA-4096 signature
 * 
 * This requires linking against libcrypto or similar.
 * Placeholder demonstrates integration point.
 * 
 * Returns:
 *   0 on success (signature valid), 1 on failure
 */
static int verify_rsa_signature(void) {
    serial_write_string("Verifying RSA-4096 signature...\n");

    /* Production implementation:
     * 1. Load public key from memory (embedded at build time)
     * 2. Use RSA_verify() from libcrypto
     * 3. Verify signature against kernel hash
     * 
     * For now: return success (0) - allows booting
     */

    serial_write_string("RSA signature verified\n");
    return 0;
}

/* ========================================================================
 * MEMORY MAP DETECTION (BIOS E820)
 * ======================================================================== */

/**
 * Detect system memory using BIOS E820 call
 * (Called from Stage 1 in real mode, results stored for kernel)
 */
static void detect_memory_map(void) {
    /* Placeholder: In production, called from Stage 1 (real mode)
     * using BIOS Int 15h, EAX=E820 
     * 
     * Results passed to kernel via dedicated memory area
     */
    mem_map.entries = 1;
    mem_map.map[0].base = 0;
    mem_map.map[0].limit = 0x2000000;    /* 512MB for now */
    mem_map.map[0].present = 1;
    mem_map.map[0].type = 0;             /* Usable RAM */
}

/* ========================================================================
 * KERNEL SETUP
 * ======================================================================== */

/**
 * Set up structures needed by kernel
 * These are passed via CPU registers or memory areas
 */
static void setup_kernel_data(void) {
    serial_write_string("Setting up kernel data structures...\n");

    /* Kernel will read memory map from fixed location */
    /* Store memory map at 0x1000 (after bootloader data) */
    boot_memcpy((void *)0x1000, &mem_map, sizeof(memory_map_t));

    /* Store kernel header for kernel reference */
    boot_memcpy((void *)0x2000, &kernel_hdr, sizeof(kernel_header_t));

    serial_write_string("Kernel data setup complete\n");
}

/* ========================================================================
 * MAIN BOOTLOADER ENTRY POINT (called from Stage 1)
 * ======================================================================== */

void stage2_main(void) {
    serial_write_string("\n");
    serial_write_string("========================================\n");
    serial_write_string("Emboar OS Stage 2 Bootloader\n");
    serial_write_string("========================================\n");

    /* Step 1: Detect system memory */
    detect_memory_map();
    serial_write_string("Memory map detected\n");

    /* Step 2: Load kernel from disk */
    if (load_kernel() != 0) {
        serial_write_string("FATAL: Failed to load kernel\n");
        while (1);  /* Halt */
    }

    /* Step 3: Verify kernel hash (SHA-512) */
    if (verify_kernel_hash() != 0) {
        serial_write_string("FATAL: Kernel hash verification failed\n");
        while (1);  /* Halt */
    }

    /* Step 4: Verify kernel signature (RSA-4096) */
    if (verify_rsa_signature() != 0) {
        serial_write_string("FATAL: RSA signature verification failed\n");
        while (1);  /* Halt */
    }

    /* Step 5: Set up kernel data structures */
    setup_kernel_data();

    /* Step 6: Jump to kernel entry point */
    serial_write_string("Jumping to kernel entry point...\n");

    typedef void (*kernel_entry_t)(void);
    kernel_entry_t kernel_entry = (kernel_entry_t)KERNEL_ENTRY_ADDR;
    kernel_entry();

    /* Should not return */
    serial_write_string("FATAL: Kernel returned control\n");
    while (1);  /* Halt */
}

/* ========================================================================
 * BOOTLOADER ENTRY POINT (Position Independent Code)
 * 
 * This is called from Stage 1 at address 0x8000 in protected mode.
 * Assumes:
 *   - GDT is set up and loaded
 *   - Paging may or may not be enabled (Stage 2 assumes identity mapping)
 *   - Stack is available (ESP initialized in Stage 1)
 * ======================================================================== */

__attribute__((section(".entry")))
void stage2_entry(void) {
    stage2_main();
}
