#pragma once

#include <stdint.h>
#include "../mem/heap.c"
#define PAGE_PRESENT    (1 << 0)
#define PAGE_WRITE      (1 << 1)
#define PAGE_HUGE       (1 << 7)  // 2MB pages in PD entries
#define PAGE_USER       (1 << 2)

#define PML4_ADDR 0x60000
#define PDPT_ADDR 0x61000
#define PD_ADDR   0x62000
#define PD2_ADDR  0x63000

static inline void paging_init(void) {
    uint64_t *pml4 = (uint64_t*)PML4_ADDR;
    uint64_t *pdpt  = (uint64_t*)PDPT_ADDR;
    uint64_t *pd    = (uint64_t*)PD_ADDR;
    uint64_t *pd2   = (uint64_t*)PD2_ADDR;

    for (int i = 0; i < 512; i++) {
        pml4[i] = 0;
        pdpt[i] = 0;
        pd[i]   = 0;
        pd2[i]  = 0;
    }

    // correct pointer masking (IMPORTANT)
    pml4[0] = ((uint64_t)pdpt & 0x000FFFFFFFFFF000ULL)
             | PAGE_PRESENT | PAGE_WRITE;

    pdpt[0] = ((uint64_t)pd & 0x000FFFFFFFFFF000ULL)
            | PAGE_PRESENT | PAGE_WRITE;

    // identity map ONLY first 1GB safely (not all 512 entries blindly)
    for (int i = 0; i < 512; i++) {
        uint64_t addr = (uint64_t)i * 0x200000ULL;

        pd[i] = (addr & 0x000FFFFFFFFFF000ULL)
              | PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE | PAGE_USER;
    }

    // Map the virtio MMIO region at 0xFFF00000 (above 1GB RAM, near 4GB).
    // PDPT[3] covers 3GB-4GB; a 2MB huge page at 0xFFE00000 spans 0xFFF00000.
    pdpt[3] = ((uint64_t)pd2 & 0x000FFFFFFFFFF000ULL)
            | PAGE_PRESENT | PAGE_WRITE;

    uint64_t mmio_huge = 0xFFE00000ULL;
    pd2[511] = (mmio_huge & 0x000FFFFFFFFFF000ULL)
             | PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE | PAGE_USER;

    // flush TLB safety (important in VM setups)
    uint64_t cr3 = ((uint64_t)pml4 & 0x000FFFFFFFFFF000ULL);
    asm volatile("mov %0, %%cr3" :: "r"(cr3));
}