#include <stdint.h>
#include "serial.h"

#define IDT_ENTRIES 32
#define ISR_LIST \
    X(0)  X(1)  X(2)  X(3)  X(4)  X(5)  X(6)  X(7)  \
    X(8)  X(9)  X(10) X(11) X(12) X(13) X(14) X(15)  \
    X(16) X(17) X(18) X(19) X(20) X(21) X(22) X(23)  \
    X(24) X(25) X(26) X(27) X(28) X(29) X(30) X(31)

#define X(n) extern void isr##n(void);
ISR_LIST
#undef X

typedef struct {
    uint16_t offset_low;    // handler address [15:0]
    uint16_t selector;      // code segment selector (0x18 — your 64-bit CS)
    uint8_t  ist;           // interrupt stack table, 0 = none
    uint8_t  type_attr;     // 0x8E = present, ring-0, interrupt gate
    uint16_t offset_mid;    // handler address [31:16]
    uint32_t offset_high;   // handler address [63:32]
    uint32_t reserved;      // must be zero
} __attribute__((packed)) IDTEntry;

typedef struct {
    uint16_t size;
    uint64_t base;
} __attribute__((packed)) IDTPointer;

static IDTEntry idt[IDT_ENTRIES];
static IDTPointer idtp;

static inline void idt_set_entry(int n, uint64_t handler) {
    idt[n].offset_low  = handler & 0xFFFF;
    idt[n].selector    = 0x18;        // your 64-bit code segment
    idt[n].ist         = 0;
    idt[n].type_attr   = 0x8E;        // present, ring-0, interrupt gate
    idt[n].offset_mid  = (handler >> 16) & 0xFFFF;
    idt[n].offset_high = (handler >> 32) & 0xFFFFFFFF;
    idt[n].reserved    = 0;
}

static inline void idt_init(void) {
    idtp.size = sizeof(idt) - 1;
    idtp.base = (uint64_t)idt;

    // install a handler for each CPU exception
    #define X(n) idt_set_entry(n, (uint64_t)isr##n);
    ISR_LIST
    #undef X

    __asm__ volatile("lidt %0" :: "m"(idtp));
}

void exception_handler(uint64_t exc_num, uint64_t err_code) {
    serial_puts("EXCEPTION: ");
    serial_putx(exc_num);
    serial_puts(" ERR: ");
    serial_putx(err_code);
    serial_puts("\n");
    for(;;) __asm__("hlt");
}