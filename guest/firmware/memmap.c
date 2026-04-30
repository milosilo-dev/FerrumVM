#include <stdint.h>

#define MEMMAP_MAX_ENTRIES 16
#define MEMMAP_MGK_NUM 0xFE02FE02

typedef struct {
    uint64_t base;
    uint64_t length;
    uint32_t type;
} __attribute__((packed)) MemMapEntry;

typedef struct {
    uint32_t mgk_num;
    uint32_t length;
} __attribute__((packed)) MemMapHeader;

MemMapEntry memmap[MEMMAP_MAX_ENTRIES];

void init_memmap() {
    MemMapHeader* header = (MemMapHeader *)0x7000;
    serial_puts("mem_map: Mgk num = ");
    serial_putx(header->mgk_num); serial_puts("\n");

    if (header->mgk_num == MEMMAP_MGK_NUM) {
        uint32_t length = header->length;
        serial_puts("mem_map: length = ");
        serial_putx(length); serial_puts("\n");
        for (int i = 0; i < length; i++) {
            if (i - 1 < MEMMAP_MAX_ENTRIES) {
                MemMapEntry* entry = (MemMapEntry *)((uint8_t *)0x7000 + 8 + i * 20);
                serial_puts("mem_map: Entry ");
                serial_putx(i); serial_puts("\n");

                memmap[i] = *entry;
            }
        }
    }
}