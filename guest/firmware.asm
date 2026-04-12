sti
mov dx, 0x03F8      ; COM1 serial port
mov al, 4           ; set al to 10
add al, bl          ; AL = AL + BL
add al, 0x30        ; convert to ASCII digit
out dx, al          ; send to serial

mov al, 0x0A        ; newline ('\n')
out dx, al          ; send newline

stop:
mov dx, 0xFFFF
mov al, 1
out dx, al          ; trigger hlt port
jmp stop            ; prevent random memory execution