#include "headers/paging.h"
#include "headers/gdt.h"
#include "headers/serial.h"
#include "headers/longmode.h"

void c_main_32(void) {
    serial_init();

    paging_init();
    GDTPointer32 gdtp = {
        .size = sizeof(gdt64) - 1,
        .base = (uint32_t)gdt64
    };
    __asm__ volatile("lgdt %0" :: "m"(gdtp));

    enter_long_mode((uint32_t)pml4);
}