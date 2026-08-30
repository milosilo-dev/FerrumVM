#pragma once
#include <stdint.h>
#include "memory_layout.h"

// Low-level access to the host-provided memory map.
//
// The host injects the map into guest physical memory at MEMMAP_ADDR as:
//   header : { u32 mgk_num; u32 length; }            (8 bytes)
//   entries: array of 'length' packed MemMapEntry      (20 bytes each)
//
// These helpers are the single source of truth for walking that map. Both the
// paging code and the UEFI memory code use them so the layout/offsets are not
// reinvented in several places.

#define MEMMAP_MGK_NUM     0xFE02FE02
#define MEMMAP_MAX_ENTRIES 32
#define MEMMAP_ENTRY_SIZE  20

typedef struct {
    uint64_t start;
    uint64_t end;
    uint32_t type;
} __attribute__((packed)) MemMapEntry;

typedef struct {
    uint32_t mgk_num;
    uint32_t length;
} __attribute__((packed)) MemMapHeader;

// EFI memory type (only the ones the firmware cares about).
#define EFI_CONVENTIONAL_MEMORY   7

// Highest 'end' address of a ConventionalMemory region; 0 if the map is absent
// or empty.
static inline uint64_t memmap_ram_top(void) {
    MemMapHeader *hdr = (MemMapHeader *)MEMMAP_ADDR;
    if (hdr->mgk_num != MEMMAP_MGK_NUM || hdr->length == 0)
        return 0;

    uint32_t n = hdr->length > MEMMAP_MAX_ENTRIES ? MEMMAP_MAX_ENTRIES : hdr->length;
    uint64_t top = 0;
    for (uint32_t i = 0; i < n; i++) {
        MemMapEntry *e = (MemMapEntry *)(MEMMAP_ADDR + 8 + (uint64_t)i * MEMMAP_ENTRY_SIZE);
        if (e->type == EFI_CONVENTIONAL_MEMORY && e->end > top)
            top = e->end;
    }
    return top;
}

// Pointer to the i-th entry of the current host memory map, or NULL if the map
// is absent or i is out of range.
static inline MemMapEntry *memmap_entry(uint32_t i) {
    MemMapHeader *hdr = (MemMapHeader *)MEMMAP_ADDR;
    if (hdr->mgk_num != MEMMAP_MGK_NUM || i >= hdr->length || i >= MEMMAP_MAX_ENTRIES)
        return NULL;
    return (MemMapEntry *)(MEMMAP_ADDR + 8 + (uint64_t)i * MEMMAP_ENTRY_SIZE);
}
