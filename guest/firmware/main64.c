#include "headers/serial.h"
#include "virtio/blk.c"
#include "headers/idt.h"
#include "disk/load_part_table.c"
#include "memmap.c"

void c_main_64(void) {
    serial_puts("=-- Long mode --=\n");
    idt_init();
    init_memmap();
    virtio_blk_init();

    SectorRange sec_range;
    load_part_table(&sec_range);

    // spin forever
    while (1) {
        __asm__ volatile("hlt");
    }
}