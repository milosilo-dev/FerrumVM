#include <stdint.h>

#include "headers/serial.h"
#include "headers/gdt.h"
#include "headers/idt.h"
#include "headers/halt.h"

#include "tss.h"
#include "mem/heap.h"
#include "mem/memmap.h"
#include "mem/stack.h"
#include "virtio/blk.h"
#include "virtio/net.h"
#include "disk/esp.h"
#include "disk/fat32.h"
#include "disk/boot.h"
#include "disk/format_PE.h"
#include "efi/uefi.h"

extern uint8_t __bss_start[];
extern uint8_t __bss_end[];

static void zero_bss(void) {
    for (uint8_t* p = __bss_start; p < __bss_end; p++)
        *p = 0;
}

static void init_tss_gdt(void) {
    tss_init();
    gdt_set_tss(gdt64, 5);
    GDTPointer64 gdtp = {
        .size = sizeof(gdt64) - 1,
        .base = (uint64_t)gdt64
    };
    __asm__ volatile("lgdt %0" :: "m"(gdtp));
    tss_enable(5 * 8);
}

void c_main_64(void) {
    zero_bss();

    init_tss_gdt();
    idt_init();
    init_memmap();
    init_heap(0x3000000, 0x4000000);

    virtio_blk_init();
    virtio_net_init();

    uint8_t net_buf[100];
    uint16_t len = virtio_net_poll_receive(net_buf, 100);
    serial_puts("Net result: [");
    for (int i = 0; i < len; i++){
        serial_puts("0x");
        serial_putx(net_buf[i]);
        serial_puts(", ");
    }
    serial_puts("]\n");

    uint8_t* file_buf = (uint8_t*)0x1000000;
    int status = load_efi_application(file_buf);
    if (status != 0) {
        serial_puts("Failed to load EFI application\n");
        hang();
    }

    format_pe(file_buf);
    hang();
}

#include "mem/heap.c"
#include "mem/memmap.c"
#include "mem/stack.c"
#include "tss.c"
#include "virtio/blk.c"
#include "virtio/net.c"
#include "efi/blockio.c"
#include "efi/dev_path.c"
#include "efi/sfsp.c"
#include "disk/esp.c"
#include "disk/fat32.c"
#include "efi/uefi.c"
#include "disk/format_PE.c"
#include "disk/boot.c"
