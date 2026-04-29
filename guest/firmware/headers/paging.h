// paging.h
#pragma once

#include <stdint.h>
#define PAGE_PRESENT  (1 << 0)
#define PAGE_WRITE    (1 << 1)
#define PAGE_HUGE     (1 << 7)  // 2MB pages in PD entries

// Each table is 512 entries of 8 bytes = 4096 bytes, must be page aligned
static uint64_t pml4[512] __attribute__((aligned(4096)));
static uint64_t pdpt[512] __attribute__((aligned(4096)));
static uint64_t pd  [512] __attribute__((aligned(4096)));

static inline void paging_init(void) {
    // PML4[0] → PDPT
    pml4[0] = (uint64_t)pdpt | PAGE_PRESENT | PAGE_WRITE;

    // PDPT[0] → PD
    pdpt[0] = (uint64_t)pd | PAGE_PRESENT | PAGE_WRITE;

    // PD[0..511] → identity map first 1GB in 2MB chunks
    for (int i = 0; i < 512; i++) {
        pd[i] = ((uint32_t)i * 0x200000) | PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE;
    }
}