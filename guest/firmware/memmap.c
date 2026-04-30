#include <stdint.h>
#define MEMMAP_MAX_ENTRIES 16

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
    serial_putx(header->mgk_num);
    serial_puts("\n");
}