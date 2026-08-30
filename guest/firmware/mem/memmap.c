#include "memmap.h"
#include "../headers/serial.h"
#include "../headers/memmap.h"
#include <stdint.h>

MemMapEntry memmap[MEMMAP_MAX_ENTRIES];

void init_memmap() {
    MemMapHeader* header = (MemMapHeader *)MEMMAP_ADDR;

    if (header->mgk_num == MEMMAP_MGK_NUM) {
        uint32_t length = header->length;
        memmap_length = length;
        serial_putx(length); serial_puts("\n");
        for (uint32_t i = 0; i < length && i < MEMMAP_MAX_ENTRIES; i++) {
            MemMapEntry* entry = memmap_entry(i);
            if (entry)
                memmap[i] = *entry;
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

        // Runtime services regions must carry the runtime attribute so the
        // OS keeps them mapped while setting up EFI virtual mode; otherwise
        // Linux's should_map_region() skips them and later faults reading
        // the map/runtime tables through the EFI page tables.
        if (entry->type == 5 /* EfiRuntimeServicesCode */
            || entry->type == 6 /* EfiRuntimeServicesData */) {
            buf[out].Attribute = EFI_MEMORY_WB | EFI_MEMORY_RUNTIME;
        } else {
            buf[out].Attribute = EFI_MEMORY_WB;
        }

        out++;
    }

    return out;
}