#pragma once

#include <stdint.h>
#include "../mem/heap.c"
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

#define MEMMAP_ADDR 0x7000ULL
#define MEMMAP_MGK_NUM 0xFE02FE02

typedef struct {
    uint64_t start;
    uint64_t end;
    uint32_t type;
} __attribute__((packed)) MemMapEntry;

typedef struct {
    uint32_t mgk_num;
    uint32_t length;
} __attribute__((packed)) MemMapHeader;

static uint64_t ram_size_bytes(void) {
    MemMapHeader* hdr = (MemMapHeader*)MEMMAP_ADDR;
    if (hdr->mgk_num != MEMMAP_MGK_NUM || hdr->length == 0) {
        return 0;
    }

    uint64_t top = 0;
    for (uint32_t i = 0; i < hdr->length; i++) {
        MemMapEntry* e = (MemMapEntry*)(MEMMAP_ADDR + 8 + i * sizeof(MemMapEntry));
        if (e->type == 7 /* ConventionalMemory */ && e->end > top) {
            top = e->end;
        }
    }
    return top;
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

    // identity-map the RAM below the MMIO region so the bootloader can use all
    // of it before installing its own paging. The amount of RAM comes from the
    // host-provided memory map rather than a hardcoded constant, so changing the
    // guest's RAM size in the host config needs no firmware edit.
    // One PDPT entry + one 512-entry PD page of 2 MiB huge pages per 1 GiB.
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

    // Map the virtio MMIO region at the top of the canonical address space,
    // far above any RAM so RAM can grow arbitrarily large without ever
    // colliding with the MMIO devices.
    //
    // The firmware dereferences MMIO at the canonical VIRTUAL base
    //   0xFFFF_FFFF_FFE0_0000   (bits 63:48 = 0xFFFF, canonical high half),
    // which is redirected here to the guest-PHYSICAL frame 0x400000000 (16 GiB)
    // (what the host registers / KVM reports for the MMIO bus cycle). The
    // physical frame is kept well below MAXPHYADDR (the 48-bit frame 0xFFFF_FFE0_0000
    // faulted with #PF/RSVD) while still sitting above any RAM of practical size.
    //
    //   0xFFFF_FFFF_FFE0_0000 splits as:
    //     PML4[511]  -> top 512 GiB   (0xFF80_0000_0000 ... 0x1_0000_0000_0000)
    //     PDPT2[511] -> top 1 GiB     (0xFFC0_0000_0000  ... 0x1_0000_0000_0000)
    //     PD9[511]   -> final 2 MiB huge page @ 0x400000000 (physical)
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
