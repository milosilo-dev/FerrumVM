#pragma once

#include <stdint.h>
#include "../mem/heap.c"
#include "memmap.h"
#define PAGE_PRESENT    (1 << 0)
#define PAGE_WRITE      (1 << 1)
#define PAGE_HUGE       (1 << 7)  // 2MB pages in PD entries
#define PAGE_USER       (1 << 2)

#define PML4_ADDR   0x60000

// Regular Memory
#define PDPT_ADDR   0x61000
#define PD0_ADDR    0x62000     // RAM 0-1 GiB
#define PD1_ADDR    0x64000     // RAM 1-2 GiB
#define PD2_ADDR    0x67000     // RAM 2-3 GiB
#define PD3_ADDR    0x68000     // RAM 3-4 GiB
#define PD4_ADDR    0x69000     // RAM 4-5 GiB
#define PD5_ADDR    0x6A000     // RAM 5-6 GiB
#define PD6_ADDR    0x6B000     // RAM 6-7 GiB
#define PD7_ADDR    0x6C000     // RAM 7-8 GiB

// MMIO Mappings
#define PDPT2_ADDR  0x65000     // high MMIO pml4[511]
#define PD9_ADDR    0x66000     // MMIO 2 MiB huge page

#define GB (0x40000000ULL)
#define HUGE_PAGE (0x200000ULL)

#define MAX_RAM_GB 8

static uint64_t ram_size_bytes(void) {
    return memmap_ram_top();
}

static void paging_init(void) {
    uint64_t *pml4 = (uint64_t*)PML4_ADDR;
    uint64_t *pdpt  = (uint64_t*)PDPT_ADDR;
    uint64_t *pd0   = (uint64_t*)PD0_ADDR;
    uint64_t *pd1   = (uint64_t*)PD1_ADDR;
    uint64_t *pd2   = (uint64_t*)PD2_ADDR;
    uint64_t *pd3   = (uint64_t*)PD3_ADDR;
    uint64_t *pd4   = (uint64_t*)PD4_ADDR;
    uint64_t *pd5   = (uint64_t*)PD5_ADDR;
    uint64_t *pd6   = (uint64_t*)PD6_ADDR;
    uint64_t *pd7   = (uint64_t*)PD7_ADDR;
    uint64_t *pdpt2 = (uint64_t*)PDPT2_ADDR;
    uint64_t *pd9   = (uint64_t*)PD9_ADDR;


    for (int i = 0; i < 512; i++) {
        pml4[i] = 0;
        pdpt[i] = 0;
        pd0[i]  = 0;
        pd1[i]  = 0;
        pd2[i]  = 0;
        pd3[i]  = 0;
        pd4[i]  = 0;
        pd5[i]  = 0;
        pd6[i]  = 0;
        pd7[i]  = 0;
        pdpt2[i] = 0;
        pd9[i]  = 0;
    }

    // correct pointer masking (IMPORTANT)
    pml4[0] = ((uint64_t)pdpt & 0x000FFFFFFFFFF000ULL)
             | PAGE_PRESENT | PAGE_WRITE;

    uint64_t ram_bytes = ram_size_bytes();
    uint32_t ram_gb = (uint32_t)((ram_bytes + GB - 1) / GB); // round up to cover all RAM
    if (ram_gb == 0) {
        ram_gb = 1; // fallback: always identity-map at least the first GiB
    }
    if (ram_gb > MAX_RAM_GB) {
        ram_gb = MAX_RAM_GB;
    }

    const uint64_t ram_pd[MAX_RAM_GB] = {
        PD0_ADDR, PD1_ADDR, PD2_ADDR, PD3_ADDR, PD4_ADDR, PD5_ADDR, PD6_ADDR, PD7_ADDR
    };

    for (uint32_t gb = 0; gb < ram_gb; gb++) {
        pdpt[gb] = (ram_pd[gb] & 0x000FFFFFFFFFF000ULL)
                 | PAGE_PRESENT | PAGE_WRITE;

        uint64_t *pdgb = (uint64_t*)ram_pd[gb];
        for (int i = 0; i < 512; i++) {
            uint64_t addr = (uint64_t)gb * GB + (uint64_t)i * HUGE_PAGE;

            pdgb[i] = (addr & 0x000FFFFFFFFFF000ULL)
                    | PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE | PAGE_USER;
        }
    }

    pml4[511] = ((uint64_t)pdpt2 & 0x000FFFFFFFFFF000ULL)
              | PAGE_PRESENT | PAGE_WRITE;

    pdpt2[511] = ((uint64_t)pd9 & 0x000FFFFFFFFFF000ULL)
               | PAGE_PRESENT | PAGE_WRITE;

    uint64_t mmio_huge = 0x400000000ULL; // 16 GiB, above any RAM of practical size
    pd9[511] = (mmio_huge & 0x000FFFFFFFFFF000ULL)
             | PAGE_PRESENT | PAGE_WRITE | PAGE_HUGE | PAGE_USER;

    // flush TLB safety (important in VM setups)
    uint64_t cr3 = ((uint64_t)pml4 & 0x000FFFFFFFFFF000ULL);
    asm volatile("mov %0, %%cr3" :: "r"(cr3));
}
