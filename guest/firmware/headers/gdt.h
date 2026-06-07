#include <stdint.h>

static uint64_t gdt64[] __attribute__((aligned(8))) = {
    0x0000000000000000,  // 0x00 null
    0x0000000000000000,  // 0x08 null
    0x0000000000000000,  // 0x10 null

    0x00AF9A000000FFFF,  // 0x18 kernel code
    0x00CF92000000FFFF,  // 0x20 kernel data

    0x00AFFA000000FFFF,  // 0x28 user code (OK if DPL=3 inside descriptor)
    0x00CFF2000000FFFF,  // 0x30 user data (OK if DPL=3 inside descriptor)
};

typedef struct {
    uint16_t size;
    uint32_t base;
} __attribute__((packed)) GDTPointer32;

typedef struct {
    uint16_t size;
    uint64_t base;
} __attribute__((packed)) GDTPointer64;