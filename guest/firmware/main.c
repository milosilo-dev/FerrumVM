#include "headers/paging.h"
#include "headers/gdt.h"
#include "headers/serial.h"
#include "headers/longmode.h"

#include "rng.c"
#include "counter.c"
#include "blk.c"

void c_main_32(void) {
    serial_init();
    virtio_rng_init();
    virtio_cnt_init();
    virtio_blk_init();

    uint8_t rnd_buf[16] = {0};
    uint32_t written = virtio_rng_read(rnd_buf, 16);

    serial_puts("bytes written: "); serial_putx(written); serial_puts("\n");
    serial_puts("random bytes: ");
    for (int i = 0; i < 16; i++) {
        serial_putx(rnd_buf[i]);
        serial_putc(' ');
    }
    serial_puts("\n");

    uint32_t increament = virtio_cnt(0x20);
    serial_puts("counter: ");
    serial_putx(increament);
    serial_puts("\n");

    uint8_t sector[512];
    uint32_t status = virtio_blk_read(0, 512, sector);
    serial_puts("status: "); serial_putx(status); serial_puts("\n");
    serial_puts("MBR sig: ");
    serial_putx(sector[510]); serial_putc(' ');
    serial_putx(sector[511]); serial_puts("\n");

    // Set up page tables
    paging_init();

    // Load 64-bit GDT
    GDTPointer32 gdtp = {
        .size = sizeof(gdt64) - 1,
        .base = (uint32_t)gdt64
    };
    __asm__ volatile("lgdt %0" :: "m"(gdtp));

    // Switch — jumps to c_main_64, never returns
    enter_long_mode((uint32_t)pml4);
}