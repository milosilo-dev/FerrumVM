#include "stack.h"
#include "../headers/serial.h"
#include <stdint.h>

uint64_t get_sp() {
    uint64_t sp;
    __asm__ volatile ("mov %%rsp, %0" : "=r"(sp));
    return sp;
}

void print_stack_usage() {
    uint64_t sp = get_sp();
    uint64_t used = 0x400000 - sp;
    serial_puts("stack: ");
    serial_putx(used);
    serial_puts(" Bytes used\n");
}