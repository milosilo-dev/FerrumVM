org 0x1000

mov dx, 0x3F8

loop:
    mov al, 'X'
    out dx, al

    ; crude delay
    mov cx, 0xFFFF
delay:
    loop delay

    jmp loop