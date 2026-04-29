#include <stdint.h>

static uint64_t gdt64[] __attribute__((aligned(8))) = {
    0x0000000000000000,  // 0x00 null
    0x0000000000000000,  // 0x08 null (was 32-bit code)
    0x0000000000000000,  // 0x10 null (was 32-bit data)
    0x00AF9A000000FFFF,  // 0x18 64-bit code ring-0  (L=1, D=0)
    0x00CF92000000FFFF,  // 0x20 64-bit data ring-0
};

typedef struct {
    uint16_t size;
    uint32_t base;
} __attribute__((packed)) GDTPointer32;

typedef struct {
    uint16_t size;
    uint64_t base;
} __attribute__((packed)) GDTPointer64;