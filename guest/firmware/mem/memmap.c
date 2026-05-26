#include <stdint.h>
#include "../headers/uefi/uefi.h"

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
    serial_puts("mem_map: Mgk num = ");
    serial_putx(header->mgk_num); serial_puts("\n");

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

    //serial_puts("mem_map: length = ");
    //serial_putx(max_entries);
    //serial_puts("\n");

    for (uint32_t i = 0; i < max_entries; i++) {
        MemMapEntry* entry =
            (MemMapEntry*)((uint64_t)memmap + i * sizeof(MemMapEntry));

        // Align to EFI page boundaries
        uint64_t start = (entry->start + 0xFFFULL) & ~0xFFFULL;
        uint64_t end   = entry->end & ~0xFFFULL;

        // Region vanished after alignment
        if (end <= start) {
            continue;
        }

        uint64_t pages = (end - start) / 4096ULL;

        buf[out].Type = entry->type;
        buf[out].Pad = 0;
        buf[out].PhysicalStart = start;

        // Usually 0 before SetVirtualAddressMap()
        buf[out].VirtualStart = 0;

        buf[out].NumberOfPages = pages;

        // You probably want WB cacheable memory
        buf[out].Attribute = EFI_MEMORY_WB;

        /*
        serial_puts("mem_map: Entry ");
        serial_putx(out);

        serial_puts(" physicalStart= ");
        serial_putx(buf[out].PhysicalStart);

        serial_puts(" pages= ");
        serial_putx(buf[out].NumberOfPages);

        serial_puts(" type= ");
        serial_putx(buf[out].Type);

        serial_puts("\n");
        */

        out++;
    }

    return out;
}