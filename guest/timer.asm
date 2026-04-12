org 0x1000

cli

; Load IDT (minimal — required for IOAPIC mode)
xor ax, ax
mov ds, ax

; Point IDT entry 33 (IRQ1 = 32 + 1) to handler
mov word [33 * 8], timer_handler
mov word [33 * 8 + 2], 0x0000
mov word [33 * 8 + 4], 0x8E00
mov word [33 * 8 + 6], 0x0000

sti

main:
    hlt
    jmp main

timer_handler:
    mov dx, 0x3F8
    mov al, 'X'
    out dx, al

    mov al, 0x20
    out 0x20, al      ; EOI PIC compatibility (safe in KVM)

    iret