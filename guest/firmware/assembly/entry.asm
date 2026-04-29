bits 16

global _start
_start:
    cli

    ; Set up segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00

    ; Enable A20 via port 0x92 (fast method)
    in  al, 0x92
    or  al, 0x02
    and al, 0xFE        ; don't reset the machine (bit 0)
    out 0x92, al

    ; Load GDT and enter protected mode
    lgdt [gdt_ptr]

    mov eax, cr0
    or  eax, 1
    mov cr0, eax

    ; Far jump to flush the pipeline and enter 32-bit CS
    jmp 0x08:protected_entry

BITS 32
protected_entry:
    ; Reload data segment registers
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    ; Set up a proper 32-bit stack
    mov esp, 0x9FC00

    ; Call into C - never returns
    extern c_main_32
    call c_main_32
    hlt

; ── Minimal flat GDT (null / code / data) ──────────────────────────────────
align 8
gdt_start:
    dq 0x0000000000000000   ; 0x00 null
    dq 0x00CF9A000000FFFF   ; 0x08 code  ring-0, 32-bit, 4 GB
    dq 0x00CF92000000FFFF   ; 0x10 data  ring-0, 32-bit, 4 GB
gdt_end:

gdt_ptr:
    dw gdt_end - gdt_start - 1
    dd gdt_start

global enter_long_mode
enter_long_mode:
    ; Set CR3
    mov eax, [esp + 4]
    mov cr3, eax

    ; Enable PAE
    mov eax, cr4
    or  eax, (1 << 5)
    mov cr4, eax

    ; Read EFER
    mov ecx, 0xC0000080
    rdmsr
    ; Set long mode enable bit
    or eax, (1 << 8)
    wrmsr

    ; Enable paging
    mov eax, cr0
    or  eax, (1 << 31)
    mov cr0, eax

    mov al, '6'
    out dx, al

    jmp 0x18:long_mode_entry

BITS 64
long_mode_entry:
    ; Reload data segments with 64-bit data selector
    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    ; Call 64-bit C entry — never returns
    mov rax, 0x100000   ; address of main64.bin
    call rax            ; call c_main_64
    hlt