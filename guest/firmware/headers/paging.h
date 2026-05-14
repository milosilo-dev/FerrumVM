#pragma once

#include <stdint.h>
#include "../mem/heap.c"
#define PAGE_PRESENT  (1 << 0)
#define PAGE_WRITE    (1 << 1)
#define PAGE_HUGE     (1 << 7)  // 2MB pages in PD entries

#define PML4_ADDR 0x60000
#define PDPT_ADDR 0x61000
#define PD_ADDR   0x62000

static inline void paging_init(void) {
    uint64_t *pml4 = (uint64_t*)0x60000;
    uint64_t *pdpt = (uint64_t*)0x61000;
    uint64_t *pd   = (uint64_t*)0x62000;

    // Zero all three tables manually
    for (int i = 0; i < 512; i++) pml4[i] = 0;
    for (int i = 0; i < 512; i++) pdpt[i] = 0;
    for (int i = 0; i < 512; i++) pd[i]   = 0;

    pml4[0] = 0x61000 | PAGE_PRESENT | PAGE_WRITE;
    pdpt[0] = 0x62000 | PAGE_PRESENT | PAGE_WRITE;

    for (int i = 0; i < 512; i++)
        pd[i] = ((uint64_t)i * 0x200000ULL) | PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE;
}