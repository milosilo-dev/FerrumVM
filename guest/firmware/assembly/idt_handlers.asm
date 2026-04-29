BITS 64

extern exception_handler

; No error code — push a dummy 0
%macro ISR_NOERR 1
global isr%1

isr%1:
    push 0          ; dummy error code
    push %1         ; exception number
    jmp isr_common
%endmacro

; Has error code — CPU already pushed it
%macro ISR_ERR 1
global isr%1

isr%1:
    push %1         ; exception number
    jmp isr_common
%endmacro

ISR_NOERR 0   ; #DE divide by zero
ISR_NOERR 1   ; #DB debug
ISR_NOERR 2   ; NMI
ISR_NOERR 3   ; #BP breakpoint
ISR_NOERR 4   ; #OF overflow
ISR_NOERR 5   ; #BR bound range
ISR_NOERR 6   ; #UD invalid opcode
ISR_NOERR 7   ; #NM device not available
ISR_ERR   8   ; #DF double fault
ISR_NOERR 9   ; coprocessor overrun (obsolete)
ISR_ERR   10  ; #TS invalid TSS
ISR_ERR   11  ; #NP segment not present
ISR_ERR   12  ; #SS stack fault
ISR_ERR   13  ; #GP general protection
ISR_ERR   14  ; #PF page fault
ISR_NOERR 15  ; reserved
ISR_NOERR 16  ; #MF x87 fault
ISR_ERR   17  ; #AC alignment check
ISR_NOERR 18  ; #MC machine check
ISR_NOERR 19  ; #XM SIMD fault
; 20-31 reserved
%assign i 20
%rep 12
ISR_NOERR i
%assign i i+1
%endrep

isr_common:
    ; stack now: error_code, exception_num, rip, cs, rflags, rsp, ss
    ; call a C handler
    mov rdi, [rsp]      ; exception number (first arg)
    mov rsi, [rsp + 8]    ; error code (second arg)
    call exception_handler
    hlt