/* 
 * Kernel Linker Script for x86_64
 * 
 * Defines memory layout for kernel sections:
 *   - .text: Code segment (read-only, executable)
 *   - .rodata: Read-only data
 *   - .data: Initialized data
 *   - .bss: Uninitialized data (zeroed at startup)
 * 
 * File: src/kernel/kernel.ld
 */

ENTRY(kernel_main)

SECTIONS
{
    . = 0x400000;       /* Kernel load address (4 MB) */

    /* Code section */
    .text : {
        *(.text*)
        *(.rodata*)
    }

    /* Initialized data */
    .data : {
        *(.data*)
    }

    /* Uninitialized data (BSS section) */
    .bss : {
        *(.bss*)
        *(COMMON)
    }

    /* Debugging symbols (remove for production) */
    .debug_info 0 : { *(.debug_info) }
    .debug_abbrev 0 : { *(.debug_abbrev) }
    .debug_line 0 : { *(.debug_line) }
    .debug_frame 0 : { *(.debug_frame) }
    .debug_str 0 : { *(.debug_str) }
    .debug_loc 0 : { *(.debug_loc) }

    /DISCARD/ : {
        *(.note.GNU-stack)
        *(.gnu_debuglink)
    }
}
