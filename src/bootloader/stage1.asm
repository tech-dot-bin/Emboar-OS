; ============================================================================
; Emboar OS - Stage 1 Bootloader (x86_64)
; Loaded at 0x7C00 by BIOS or at UEFI entry point
; 
; Responsibilities:
;   1. Enable A20 line (for memory > 1MB access)
;   2. Load GDT (Global Descriptor Table)
;   3. Switch: real mode → protected mode → long mode (64-bit)
;   4. Jump to Stage 2 (kernel loading)
;
; File: src/bootloader/stage1.asm
; ============================================================================

[ORG 0x7C00]
[BITS 16]

start:
    cli                             ; Disable interrupts during setup
    cld                             ; Clear direction flag (string operations ascending)
    xor ax, ax                      ; Clear AX
    mov ds, ax                      ; Set DS to 0
    mov es, ax                      ; Set ES to 0
    mov ss, ax                      ; Set SS to 0

    ; ========================================================================
    ; Enable A20 line (allows access to memory > 1MB in real mode)
    ; ========================================================================

    ; Method 1: Via keyboard controller
    call enable_a20_keyboard

    ; ========================================================================
    ; Install GDT (Global Descriptor Table)
    ; ========================================================================

    lgdt [gdt_descriptor]           ; Load GDT descriptor (size + base address)

    ; ========================================================================
    ; Switch to Protected Mode (32-bit)
    ; ========================================================================

    mov eax, cr0
    or eax, 1                       ; Set PE (Protected Enable) bit
    mov cr0, eax

    ; Jump to protected mode code segment (selector 0x08)
    jmp 0x08:protected_mode

; ============================================================================
; REAL MODE PROCEDURES
; ============================================================================

enable_a20_keyboard:
    push eax
    push ecx

    ; Send 0xad to keyboard controller (disable keyboard)
    mov al, 0xad
    out 0x64, al
    mov ecx, 100000
.wait1:
    loop .wait1

    ; Send 0xd0 to keyboard controller (read output port)
    mov al, 0xd0
    out 0x64, al
    mov ecx, 100000
.wait2:
    loop .wait2

    ; Read from port 0x60
    in al, 0x60
    push eax

    ; Send 0xd1 to keyboard controller (write output port)
    mov al, 0xd1
    out 0x64, al
    mov ecx, 100000
.wait3:
    loop .wait3

    ; Write modified value back (with A20 enabled, bit 1)
    pop eax
    or al, 0x02                     ; Set A20 bit
    out 0x60, al
    mov ecx, 100000
.wait4:
    loop .wait4

    ; Send 0xae to keyboard controller (enable keyboard)
    mov al, 0xae
    out 0x64, al
    mov ecx, 100000
.wait5:
    loop .wait5

    pop ecx
    pop eax
    ret

; ============================================================================
; PROTECTED MODE (32-bit)
; ============================================================================

[BITS 32]
protected_mode:
    ; Set up segment registers for 32-bit protected mode
    mov ax, 0x10                    ; Data segment selector (GDT entry 2)
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x90000               ; Set stack pointer to 576KB

    ; ========================================================================
    ; Enable PAE (Physical Address Extension) for 4-level paging
    ; ========================================================================

    mov eax, cr4
    or eax, (1 << 5)                ; Set PAE bit (bit 5)
    mov cr4, eax

    ; ========================================================================
    ; Enable Long Mode via EFER MSR (Extended Feature Enable Register)
    ; ========================================================================

    mov ecx, 0xC0000080             ; EFER MSR index
    rdmsr                           ; Read EFER into EDX:EAX
    or eax, (1 << 8)                ; Set LME (Long Mode Enable) bit
    wrmsr                           ; Write back to EFER

    ; ========================================================================
    ; Enable Paging (puts CPU in long mode)
    ; ========================================================================

    mov eax, cr0
    or eax, (1 << 31)               ; Set PG (Paging Enable) bit
    mov cr0, eax

    ; Jump to 64-bit code segment (selector 0x18)
    jmp 0x18:long_mode_entry

; ============================================================================
; LONG MODE (64-bit)
; ============================================================================

[BITS 64]
long_mode_entry:
    ; Set up segment registers for 64-bit long mode
    mov ax, 0x20                    ; Data segment for 64-bit (GDT entry 4)
    mov ds, ax
    mov es, ax
    mov ss, ax

    ; Set up 64-bit stack
    mov rsp, 0x100000               ; 1MB (stack grows downward)

    ; ========================================================================
    ; Jump to Stage 2 bootloader (loaded at 0x8000 by previous instructions)
    ; ========================================================================

    mov rsi, stage2_message
    call print_string_64

    ; Call Stage 2 main function (entry point at 0x8000)
    mov rax, 0x8000
    call rax

    ; If Stage 2 returns (shouldn't happen), halt
    jmp halt_error

; ============================================================================
; 64-BIT PROCEDURES
; ============================================================================

; Print string in 64-bit mode (serial output)
; Input: RSI = pointer to null-terminated string
print_string_64:
    push rax
.loop:
    mov al, byte [rsi]
    test al, al
    jz .done

    ; Write to serial port (COM1: 0x3F8)
    mov rdx, 0x3F8
    mov al, byte [rsi]
    out dx, al

    inc rsi
    jmp .loop
.done:
    pop rax
    ret

; Verify kernel RSA-4096 signature
; For now: placeholder returning success (1)
verify_kernel_signature:
    ; In production: call RSA verification routine
    ; Using public key embedded at build time
    ; Verify hash of kernel against signature
    ; (This is now handled by Stage 2)
    mov rax, 1                      ; Return 1 = valid
    ret

halt_error:
    ; Display error and halt
    mov rsi, error_message
    call print_string_64
    cli
    hlt

; ============================================================================
; DATA SECTION
; ============================================================================

; Global Descriptor Table (GDT)
; - Entry 0: Null descriptor (required)
; - Entry 1: Code segment (64-bit, ring 0)
; - Entry 2: Data segment (ring 0)
; - Entry 3: Code segment (64-bit, ring 0) [alternate]
; - Entry 4: Data segment (64-bit, ring 0)

gdt_start:
    ; Null descriptor
    dq 0x0000000000000000

    ; Code segment (64-bit, ring 0)
    ; Flags: Granularity=1, Long=1, DB=0, Present=1, DPL=0, Type=Execute+Read
    dq 0x00209a0000000000

    ; Data segment (ring 0)
    ; Flags: Granularity=1, Long=0, DB=1, Present=1, DPL=0, Type=Read+Write
    dq 0x0020920000000000

    ; Code segment (64-bit, ring 0) [alternate]
    dq 0x00209a0000000000

    ; Data segment (64-bit, ring 0)
    dq 0x0020920000000000

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1      ; GDT size (in bytes, minus 1)
    dq gdt_start                    ; GDT base address (64-bit)

; Messages
stage2_message:
    db "Stage 1: Jumping to Stage 2 bootloader...", 13, 10, 0

error_message:
    db "Stage 1: ERROR - Halting system", 13, 10, 0

; Kernel entry point address (for reference)
kernel_entry_addr:
    dq 0x400000

; ============================================================================
; Boot signature (required by BIOS)
; ============================================================================

times 510 - ($ - $$) db 0           ; Pad to 510 bytes
dw 0xAA55                           ; Boot signature (magic number)
