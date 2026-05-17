/*
 * Emboar OS Kernel
 * 
 * Entry point: kernel_main()
 * Called from bootloader at address 0x400000 (long mode, paging enabled)
 * 
 * File: src/kernel/src/main.rs
 */

#![no_std]
#![no_main]
#![allow(dead_code)]

use core::panic::PanicInfo;
use core::fmt::Write;
use core::arch::asm;

/* Bootloader header signature at entry point */
const KERNEL_SIGNATURE: u32 = 0x4E524B45; /* "EKRN" in little-endian */
const KERNEL_VERSION: u32 = 0x00000001;

/* =========================================================================
 * SERIAL PORT I/O (For debugging)
 * ======================================================================= */

const SERIAL_PORT: u16 = 0x3F8; /* COM1 base address */

pub struct SerialPort;

impl SerialPort {
    fn is_transmit_empty() -> bool {
        true  // Simplified for stable Rust
    }

    fn send_byte(byte: u8) {
        while !Self::is_transmit_empty() {
            unsafe {
                // Pause instruction using inline assembly
                asm!("pause");
            }
        }
        
        unsafe {
            // Write to serial port using inline assembly
            asm!(
                "out dx, al",
                in("dx") SERIAL_PORT,
                in("al") byte,
            );
        }
    }

    pub fn send_str(s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                Self::send_byte(b'\r');
                Self::send_byte(b'\n');
            } else {
                Self::send_byte(byte);
            }
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        SerialPort::send_str(s);
        Ok(())
    }
}

macro_rules! println {
    () => {
        SerialPort::send_str("\r\n");
    };
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = writeln!(&mut SerialPort, $($arg)*);
        }
    };
}

/* =========================================================================
 * CPU SETUP (GDT, IDT, Interrupts)
 * ======================================================================= */

/**
 * Global Descriptor Table Descriptor
 * 
 * Structure:
 * - gdt_entries: Array of GDT entries (8 bytes each)
 *   Entry 0: Null descriptor (required, must be 0)
 *   Entry 1: Code segment (kernel 64-bit)
 *   Entry 2: Data segment (kernel)
 *   Entry 3: TSS (Task State Segment, required for interrupts)
 * 
 * Each GDT entry consists of:
 *   Bytes 0-1:   Limit (low 16 bits)
 *   Bytes 2-4:   Base (24 bits)
 *   Byte 5:      Access byte
 *   Byte 6:      Limit high + flags
 *   Byte 7:      Base high
 */
#[repr(C, packed)]
struct GDTDescriptor {
    limit: u16,    /* Limit in bytes - 1 */
    base: u64,     /* Base address of GDT */
}

#[repr(C)]
struct GDTEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    limit_high_flags: u8,
    base_high: u8,
}

impl GDTEntry {
    const fn null() -> Self {
        GDTEntry {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            limit_high_flags: 0,
            base_high: 0,
        }
    }

    const fn code_segment() -> Self {
        GDTEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x9A,      /* Code, ring 0, executable, readable */
            limit_high_flags: 0xAF, /* 4KB granularity, 64-bit mode */
            base_high: 0,
        }
    }

    const fn data_segment() -> Self {
        GDTEntry {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x92,      /* Data, ring 0, writable */
            limit_high_flags: 0xAF, /* 4KB granularity */
            base_high: 0,
        }
    }
}

/**
 * Static GDT (loaded at kernel startup)
 * 
 * Structure:
 * - Entry 0: Null (required by x86_64 spec)
 * - Entry 1: Code segment (kernel mode 64-bit)
 * - Entry 2: Data segment (kernel mode)
 */
static GDT: [GDTEntry; 3] = [
    GDTEntry::null(),
    GDTEntry::code_segment(),
    GDTEntry::data_segment(),
];

fn load_gdt() {
    let gdt_ptr = GDTDescriptor {
        limit: (core::mem::size_of_val(&GDT) - 1) as u16,
        base: &GDT as *const _ as u64,
    };

    unsafe {
        /* Load GDT using lgdt instruction */
        asm!(
            "lgdt [{0}]",
            in(reg) &gdt_ptr,
        );

        /* Reload segment registers with new selectors from GDT */
        asm!(
            "mov ax, 0x10",       /* Data segment selector */
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
        );
    }
}

/* =========================================================================
 * INTERRUPT DESCRIPTOR TABLE (IDT)
 * ======================================================================= */

/**
 * Interrupt Gate
 * x86_64 format:
 *   Bytes 0-1:   Handler address (bits 0-15)
 *   Bytes 2-3:   Code segment selector
 *   Byte 4:      IST (Interrupt Stack Table, zero for now)
 *   Byte 5:      Type and attributes (0x8E = interrupt gate, ring 0)
 *   Bytes 6-7:   Handler address (bits 16-31)
 *   Bytes 8-11:  Handler address (bits 32-63)
 *   Bytes 12-15: Reserved (zero)
 */
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IDTEntry {
    handler_low: u16,
    code_segment: u16,
    ist_and_attributes: u8,
    type_and_flags: u8,
    handler_mid: u16,
    handler_high: u32,
    reserved: u32,
}

impl IDTEntry {
    fn new(handler: u64, _ist: u8) -> Self {
        IDTEntry {
            handler_low: handler as u16,
            code_segment: 0x08,                /* Code segment from GDT entry 1 */
            ist_and_attributes: 0,             /* No IST for now */
            type_and_flags: 0x8E,              /* Interrupt gate, ring 0 */
            handler_mid: (handler >> 16) as u16,
            handler_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    const fn null() -> Self {
        IDTEntry {
            handler_low: 0,
            code_segment: 0,
            ist_and_attributes: 0,
            type_and_flags: 0,
            handler_mid: 0,
            handler_high: 0,
            reserved: 0,
        }
    }
}

/**
 * Default interrupt handlers
 * 
 * In production, these would properly save/restore registers,
 * handle specific exceptions, and dispatch to appropriate handlers.
 */

extern "C" fn exception_handler_divide_by_zero() {
    println!("EXCEPTION: Divide by zero");
    loop {}
}

extern "C" fn exception_handler_debug() {
    println!("EXCEPTION: Debug");
    loop {}
}

extern "C" fn exception_handler_nmi() {
    println!("EXCEPTION: Non-maskable interrupt");
    loop {}
}

extern "C" fn exception_handler_breakpoint() {
    println!("EXCEPTION: Breakpoint");
}

extern "C" fn exception_handler_overflow() {
    println!("EXCEPTION: Overflow");
    loop {}
}

extern "C" fn exception_handler_invalid_opcode() {
    println!("EXCEPTION: Invalid opcode");
    loop {}
}

extern "C" fn exception_handler_general_protection_fault() {
    println!("EXCEPTION: General Protection Fault");
    loop {}
}

extern "C" fn exception_handler_page_fault() {
    println!("EXCEPTION: Page Fault");
    loop {}
}

/**
 * IDT with exception handlers registered
 * 
 * First 32 entries are CPU exceptions:
 * 0: Divide by zero
 * 1: Debug
 * 2: NMI
 * 3: Breakpoint
 * 4: Overflow
 * 5: Bound range exceeded
 * 6: Invalid opcode
 * ...
 * 14: Page fault
 * ...
 * Remaining: User-defined or external interrupts
 */
static mut IDT: [IDTEntry; 256] = [IDTEntry::null(); 256];

fn load_idt() {
    let idt_ptr = unsafe {
        GDTDescriptor {
            limit: (core::mem::size_of::<[IDTEntry; 256]>() - 1) as u16,
            base: &raw const IDT as u64,
        }
    };

    unsafe {
        /* Load IDT using lidt instruction */
        asm!(
            "lidt [{0}]",
            in(reg) &idt_ptr,
        );
    }
}

/* =========================================================================
 * MEMORY MANAGEMENT
 * ======================================================================= */

/**
 * Page Table setup (simplified)
 * 
 * For detailed implementation, see ARCHITECTURE.md Section 1.3
 * 
 * x86_64 uses 4-level paging:
 * - PML4 (Page Map Level 4, 64 entries needed for 0xFFFF_FFFF_FFFF_FFFF)
 * - PDPT (Page Directory Pointer Table)
 * - PD (Page Directory)
 * - PT (Page Table)
 * 
 * Each table entry is 8 bytes, containing:
 * - Bits 0-11: Flags (P, RW, US, PWT, PCD, A, D, PS, G, etc.)
 * - Bits 12-51: Physical address of next table or page
 * - Bits 52-62: Software-available bits
 * - Bit 63: Execution Disable (XD)
 */

#[repr(C, align(4096))]
struct PageTable {
    entries: [u64; 512], /* 512 entries × 8 bytes = 4096 bytes */
}

impl PageTable {
    const fn new() -> Self {
        PageTable {
            entries: [0; 512],
        }
    }
}

/**
 * Flags for page table entries
 */
const PT_FLAG_PRESENT: u64 = 1 << 0;
const PT_FLAG_WRITE: u64 = 1 << 1;
const PT_FLAG_USER: u64 = 1 << 2;
const PT_FLAG_WRITETHROUGH: u64 = 1 << 3;
const PT_FLAG_CACHE_DISABLE: u64 = 1 << 4;
const PT_FLAG_ACCESSED: u64 = 1 << 5;
const PT_FLAG_DIRTY: u64 = 1 << 6;
const PT_FLAG_LARGE: u64 = 1 << 7;      /* For 2MB/1GB pages in PD/PDPT */
const PT_FLAG_GLOBAL: u64 = 1 << 8;
const PT_FLAG_XD: u64 = 1u64 << 63;

/* =========================================================================
 * KERNEL MAIN
 * ======================================================================= */

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    /* Disable interrupts during initialization */
    unsafe {
        asm!("cli");
    }

    println!("========================================");
    println!("Emboar OS Kernel Initializing");
    println!("========================================");

    /* Step 1: Load GDT */
    println!("Loading GDT...");
    load_gdt();
    println!("GDT loaded");

    /* Step 2: Initialize and Load IDT */
    println!("Initializing IDT...");
    unsafe {
        IDT[0] = IDTEntry::new(exception_handler_divide_by_zero as *const () as u64, 0);
        IDT[1] = IDTEntry::new(exception_handler_debug as *const () as u64, 0);
        IDT[2] = IDTEntry::new(exception_handler_nmi as *const () as u64, 0);
        IDT[3] = IDTEntry::new(exception_handler_breakpoint as *const () as u64, 0);
        IDT[4] = IDTEntry::new(exception_handler_overflow as *const () as u64, 0);
        IDT[6] = IDTEntry::new(exception_handler_invalid_opcode as *const () as u64, 0);
        IDT[13] = IDTEntry::new(exception_handler_general_protection_fault as *const () as u64, 0);
        IDT[14] = IDTEntry::new(exception_handler_page_fault as *const () as u64, 0);
    }
    println!("Loading IDT...");
    load_idt();
    println!("IDT loaded");

    /* Step 3: Enable interrupts */
    println!("Enabling interrupts...");
    unsafe {
        asm!("sti");
    }

    println!("Kernel initialized successfully");
    println!("");
    println!("System ready. Waiting for input...");

    /* Kernel main loop (placeholder) */
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

/* =========================================================================
 * PANIC HANDLER
 * ======================================================================= */

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Use SerialPort::send_str directly instead of println! macro
    SerialPort::send_str("KERNEL PANIC!\r\n");
    
    if let Some(location) = info.location() {
        SerialPort::send_str("Location: ");
        SerialPort::send_str(location.file());
        SerialPort::send_str(":");
        // For line number, we'd need a separate function since we can't use format!
        // For now, just indicate there's a location
        SerialPort::send_str(" (see file)\r\n");
    }
    
    loop {
        unsafe {
            asm!("hlt");
        }
    }
}

/* =========================================================================
 * LINKER SYMBOLS (from linker script)
 * ======================================================================= */

extern "C" {
    static _start: u8;    /* Start of kernel code */
    static _end: u8;      /* End of kernel data */
}

/* =========================================================================
 * LANGUAGE RUNTIME (minimal)
 * ======================================================================= */

#[no_mangle]
extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe {
            *s.add(i) = c as u8;
        }
    }
    s
}

#[no_mangle]
extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe {
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}