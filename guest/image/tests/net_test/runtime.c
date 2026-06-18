#include <stddef.h>

void* memcpy(void* dest, const void* src, size_t n)
{
    unsigned char* d = dest;
    const unsigned char* s = src;

    for (size_t i = 0; i < n; i++)
        d[i] = s[i];

    return dest;
}

void* memset(void* dest, int c, size_t n)
{
    unsigned char* d = dest;

    for (size_t i = 0; i < n; i++)
        d[i] = (unsigned char)c;

    return dest;
}

int memcmp(const void* a, const void* b, size_t n)
{
    const unsigned char* x = a;
    const unsigned char* y = b;

    for (size_t i = 0; i < n; i++) {
        if (x[i] != y[i])
            return x[i] - y[i];
    }

    return 0;
}