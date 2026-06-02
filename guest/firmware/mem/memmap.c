#include "memmap.h"
#include "../headers/serial.h"
#include <stdint.h>

#define MEMMAP_MAX_ENTRIES 16
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

MemMapEntry memmap[MEMMAP_MAX_ENTRIES];
static uint32_t memmap_length;

void init_memmap() {
    MemMapHeader* header = (MemMapHeader *)0x7000;

    if (header->mgk_num == MEMMAP_MGK_NUM) {
        uint32_t length = header->length;
        memmap_length = length;
        serial_putx(length); serial_puts("\n");
        for (int i = 0; i < length; i++) {
            if (i - 1 < MEMMAP_MAX_ENTRIES) {
                MemMapEntry* entry = (MemMapEntry *)((uint8_t *)0x7000 + 8 + i * 20);
                memmap[i] = *entry;
            }
        }
    }
}

uint32_t memmap_to_uefi(EFI_MEMORY_DESCRIPTOR* buf, uint32_t length) {
    uint32_t max_entries = length / sizeof(EFI_MEMORY_DESCRIPTOR);
    uint32_t out = 0;

    if (memmap_length < max_entries) {
        max_entries = memmap_length;
    }

    for (uint32_t i = 0; i < max_entries; i++) {
        MemMapEntry* entry =
            (MemMapEntry*)((uint64_t)memmap + i * sizeof(MemMapEntry));

        uint64_t start = (entry->start + 0xFFFULL) & ~0xFFFULL;
        uint64_t end   = entry->end & ~0xFFFULL;

        if (end <= start) {
            continue;
        }

        uint64_t pages = (end - start) / 4096ULL;

        buf[out].Type = entry->type;
        buf[out].Pad = 0;
        buf[out].PhysicalStart = start;
        buf[out].VirtualStart = 0;

        buf[out].NumberOfPages = pages;

        buf[out].Attribute = EFI_MEMORY_WB;

        out++;
    }

    return out;
}