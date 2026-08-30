#pragma once
#include <stdint.h>
#include <stdbool.h>

#define COM1 0x3F8
#define COM2 0x2F8

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

static void __attribute__((noinline)) serial_putx(uint32_t x) {
    char s[11];  // "0x" + 8 hex digits + null terminator
    int_to_hex(x, s);

    char *p = s;
    while (*p) {
        serial_putc(*p++);
    }
}

static void __attribute__((noinline)) serial_puts(const char *s) {
    while (*s) {
        if (*s == '\n') serial_putc('\r');  // CRLF for terminals
        serial_putc(*s++);
    }
}

static uint32_t __attribute__((noinline))
serial_input(char *buf, uint32_t length) {
    uint32_t ret_len = 0;

    for (uint32_t i = 0; i < length; i++) {
        if ((inb(COM1 + 5) & 0x01) == 0) {
            break;
        }

        buf[i] = inb(COM1);
        ret_len++;
    }

    return ret_len;
}

static bool __attribute__((noinline)) serial_isdata() {
    return (inb(COM1 + 5) & 0x01) != 0;
}

static inline void serial2_init(void) {
    outb(COM2 + 1, 0x00);   // disable interrupts
    outb(COM2 + 3, 0x80);   // enable DLAB (set baud rate divisor)
    outb(COM2 + 0, 0x03);   // divisor low  = 3 → 38400 baud
    outb(COM2 + 1, 0x00);   // divisor high = 0
    outb(COM2 + 3, 0x03);   // 8 bits, no parity, one stop bit
    outb(COM2 + 2, 0xC7);   // enable FIFO, clear, 14-byte threshold
    outb(COM2 + 4, 0x0B);   // IRQs enabled, RTS/DSR set
}

static inline void serial2_putc(char c) {
    // Spin until transmit buffer is empty (bit 5 of Line Status Register)
    while ((inb(COM2 + 5) & 0x20) == 0);
    outb(COM2, (uint8_t)c);
}

static void __attribute__((noinline)) serial2_putx(uint32_t x) {
    char s[11];  // "0x" + 8 hex digits + null terminator
    int_to_hex(x, s);

    char *p = s;
    while (*p) {
        serial2_putc(*p++);
    }
}

static void __attribute__((noinline)) serial2_puts(const char *s) {
    while (*s) {
        if (*s == '\n') serial2_putc('\r');  // CRLF for terminals
        serial2_putc(*s++);
    }
}

// ---------------------------------------------------------------------------
// Logging layer.
//
// Firmware diagnostics go to COM2 (the log file). There are two tiers:
//   - log_* / logf(): meaningful messages (errors, progress, results).
//   - trace_* / tracef(): fine-grained per-call noise.
//
// g_log_enabled controls log_* at runtime. trace_* additionally disappears
// entirely when LOG_ENABLED is 0, so the noisy tracing can be compiled out.
// ---------------------------------------------------------------------------

#define LOG_ENABLED 1
static int g_log_enabled = 1;

static void __attribute__((noinline)) log_putc(char c) {
    serial2_putc(c);
}

static void __attribute__((noinline)) log_puts(const char *s) {
    serial2_puts(s);
}

static void __attribute__((noinline)) log_putx(uint32_t x) {
    serial2_putx(x);
}

#if LOG_ENABLED
// Fine-grained trace tier. gated at runtime so it can be flipped without
// recompiling, but the whole call disappears when LOG_ENABLED is 0.
static void __attribute__((noinline)) trace_puts(const char *s) {
    if (g_log_enabled) serial2_puts(s);
}
static void __attribute__((noinline)) trace_putx(uint32_t x) {
    if (g_log_enabled) serial2_putx(x);
}
static void __attribute__((noinline)) trace_putc(char c) {
    if (g_log_enabled) serial2_putc(c);
}
static void __attribute__((noinline)) tracef(const char *fmt, ...) {
    if (!g_log_enabled)
        return;
    __builtin_va_list ap;
    __builtin_va_start(ap, fmt);
    for (const char *p = fmt; *p; p++) {
        if (*p != '%') { serial2_putc(*p); continue; }
        switch (*++p) {
            case 's': serial2_puts(__builtin_va_arg(ap, const char *)); break;
            case 'x': serial2_putx(__builtin_va_arg(ap, uint32_t)); break;
            case 'p': {
                uint64_t v = __builtin_va_arg(ap, uint64_t);
                serial2_putx((uint32_t)(v >> 32)); serial2_putx((uint32_t)v);
                break;
            }
            case 'u': {
                unsigned v = __builtin_va_arg(ap, unsigned);
                char b[12]; unsigned n = 0;
                do { b[n++] = (char)('0' + v % 10); v /= 10; } while (v);
                while (n) serial2_putc(b[--n]);
                break;
            }
            case 'c': serial2_putc((char)__builtin_va_arg(ap, int)); break;
            case '%': serial2_putc('%'); break;
            default: serial2_putc('%'); serial2_putc(*p); break;
        }
    }
    __builtin_va_end(ap);
}
#else
static inline void trace_puts(const char *s) { (void)s; }
static inline void trace_putx(uint32_t x) { (void)x; }
static inline void trace_putc(char c) { (void)c; }
static inline void tracef(const char *fmt, ...) { (void)fmt; }
#endif

// Minimal freestanding printf-style logger: %s %x %p %u %c %% (writes to COM2).
static void __attribute__((noinline)) logf(const char *fmt, ...) {
    if (!g_log_enabled)
        return;

    __builtin_va_list ap;
    __builtin_va_start(ap, fmt);

    for (const char *p = fmt; *p; p++) {
        if (*p != '%') {
            log_putc(*p);
            continue;
        }
        switch (*++p) {
            case 's': {
                const char *s = __builtin_va_arg(ap, const char *);
                if (s) log_puts(s);
                break;
            }
            case 'x': {
                uint32_t v = __builtin_va_arg(ap, uint32_t);
                serial2_putx(v);
                break;
            }
            case 'p': {
                uint64_t v = __builtin_va_arg(ap, uint64_t);
                serial2_putx((uint32_t)(v >> 32));
                serial2_putx((uint32_t)v);
                break;
            }
            case 'u': {
                unsigned v = __builtin_va_arg(ap, unsigned);
                char buf[12];
                unsigned n = 0;
                do { buf[n++] = (char)('0' + v % 10); v /= 10; } while (v);
                while (n) log_putc(buf[--n]);
                break;
            }
            case 'c': {
                int c = __builtin_va_arg(ap, int);
                log_putc((char)c);
                break;
            }
            case '%':
                log_putc('%');
                break;
            default:
                log_putc('%');
                log_putc(*p);
                break;
        }
    }

    __builtin_va_end(ap);
}