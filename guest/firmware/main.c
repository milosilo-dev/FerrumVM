#include "serial.h"
#include "rng.c"

void c_main(void) {
    serial_init();
    virtio_rng_init();

    uint8_t buf[16] = {0};
    uint32_t written = virtio_rng_read(buf, 16);

    serial_puts("bytes written: "); serial_putx(written); serial_puts("\n");
    serial_puts("random bytes: ");
    for (int i = 0; i < 16; i++) {
        serial_putx(buf[i]);
        serial_putc(' ');
    }
    serial_puts("\n");

    // spin forever
    while (1) {
        __asm__ volatile("hlt");
    }
}