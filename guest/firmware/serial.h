#pragma once
#include <stdint.h>

#define COM1 0x3F8

static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile("outb %0, %1" :: "a"(val), "Nd"(port));
}

static inline uint8_t inb(uint16_t port) {
    uint8_t val;
    __asm__ volatile("inb %1, %0" : "=a"(val) : "Nd"(port));
    return val;
}

static inline void serial_init(void) {
    outb(COM1 + 1, 0x00);   // disable interrupts
    outb(COM1 + 3, 0x80);   // enable DLAB (set baud rate divisor)
    outb(COM1 + 0, 0x03);   // divisor low  = 3 → 38400 baud
    outb(COM1 + 1, 0x00);   // divisor high = 0
    outb(COM1 + 3, 0x03);   // 8 bits, no parity, one stop bit
    outb(COM1 + 2, 0xC7);   // enable FIFO, clear, 14-byte threshold
    outb(COM1 + 4, 0x0B);   // IRQs enabled, RTS/DSR set
}

static inline void serial_putc(char c) {
    // Spin until transmit buffer is empty (bit 5 of Line Status Register)
    while ((inb(COM1 + 5) & 0x20) == 0);
    outb(COM1, (uint8_t)c);
}

static inline void int_to_hex(unsigned int n, char *buffer) {
    static const char hex_chars[] = "0123456789ABCDEF";
    char temp[9];
    int i = 0;

    // Handle zero explicitly
    if (n == 0) {
        buffer[0] = '0';
        buffer[1] = '\0';
        return;
    }

    // Convert digits in reverse order
    while (n > 0) {
        temp[i++] = hex_chars[n & 0xF];
        n >>= 4;
    }

    // Reverse the string into the buffer
    buffer[i] = '\0';
    for (int j = 0; j < i; j++) {
        buffer[j] = temp[i - 1 - j];
    }
}

static inline void serial_putx(uint32_t x) {
    char s[11];  // "0x" + 8 hex digits + null terminator
    int_to_hex(x, s);

    char *p = s;
    while (*p) {
        serial_putc(*p++);
    }
}

static inline void serial_puts(const char *s) {
    while (*s) {
        if (*s == '\n') serial_putc('\r');  // CRLF for terminals
        serial_putc(*s++);
    }
}