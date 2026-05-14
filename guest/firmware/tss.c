#include <stdint.h>
#include "mem/heap.c"

typedef struct {
    uint32_t reserved0;
    uint64_t rsp0;      // stack for ring 0
    uint64_t rsp1;
    uint64_t rsp2;
    uint64_t reserved1;
    uint64_t ist1;      // your double fault stack goes here
    uint64_t ist2;
    uint64_t ist3;
    uint64_t ist4;
    uint64_t ist5;
    uint64_t ist6;
    uint64_t ist7;
    uint64_t reserved2;
    uint16_t reserved3;
    uint16_t iopb_offset;
} __attribute__((packed)) TSS;

static TSS tss;
static uint8_t kernel_stack[4096];
static uint8_t double_fault_stack[4096];

void tss_init() {
    memset(&tss, 0, sizeof(TSS));
    tss.rsp0 = (uint64_t)kernel_stack + sizeof(kernel_stack);
    tss.ist1 = (uint64_t)double_fault_stack + sizeof(double_fault_stack);
    tss.iopb_offset = sizeof(TSS);
}

void gdt_set_tss(uint64_t* gdt, int selector_index) {
    uint64_t base = (uint64_t)&tss;
    uint64_t limit = sizeof(TSS) - 1;

    // low 8 bytes
    gdt[selector_index] =
        (limit & 0xFFFF) |
        ((base & 0xFFFFFF) << 16) |
        ((uint64_t)0x89 << 40) |   // present, type=TSS available
        (((limit >> 16) & 0xF) << 48) |
        (((base >> 24) & 0xFF) << 56);

    // high 8 bytes — upper 32 bits of base
    gdt[selector_index + 1] = (base >> 32) & 0xFFFFFFFF;
}

void tss_enable(uint32_t tss_selector) {
    // selector = byte offset of TSS entry in GDT
    // e.g. if TSS is at index 3: selector = 3 * 8 = 0x18
    __asm__ volatile("ltr %0" :: "r"((uint16_t)tss_selector));
}