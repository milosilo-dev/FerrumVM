OUTPUT_FORMAT(elf32-i386)
ENTRY(_start)

SECTIONS {
    . = 0x7E00;

    .text : {
        guest/firmware/build/entry.o(.text*)   /* entry stub goes first */
        *(.text*)
    }

    .rodata : { *(.rodata*) }
    .data   : { *(.data*)   }

    .bss : {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        __bss_end = .;
    }

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment*)
    }
}