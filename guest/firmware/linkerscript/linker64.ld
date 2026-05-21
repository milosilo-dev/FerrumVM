OUTPUT_FORMAT(elf64-x86-64)
ENTRY(c_main_64)

SECTIONS {
    . = 0x100000;   /* load it at 1MB, well above your firmware */

    .text : {
        */entry64.o(.text*)
        *(.text*)
        */idt_handlers.o(.text*)
    }
    .rodata : { *(.rodata*) }
    .data   : { *(.data*)   }
    .bss    : {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        __bss_end = .;
    }

    /DISCARD/ : { *(.eh_frame*) *(.note*) *(.comment*) }
}