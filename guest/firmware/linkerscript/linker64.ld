OUTPUT_FORMAT(elf64-x86-64)
ENTRY(c_main_64)

SECTIONS {
    . = 0x100000;   /* load it at 1MB, well above your firmware */

    .text : {
        *(.text*)
        guest/firmware/build/idt_handlers.o(.text*)
    }
    .rodata : { *(.rodata*) }
    .data   : { *(.data*)   }
    .bss    : {
        *(.bss*)
        *(COMMON)
    }

    /DISCARD/ : { *(.eh_frame*) *(.note*) *(.comment*) }
}