#include "heap.h"

// ── free-list allocator ────────────────────────────────────────────
// Each allocated block has a header (struct heap_hdr) placed before
// the user pointer.  Free blocks live on a singly-linked list using
// the same header.  Bump allocation is used as a fallback when no
// free block is large enough.

struct heap_hdr {
    struct heap_hdr* next;   // free-list link (overwritten when allocated)
    uint64_t         size;   // total block size incl. header (8-byte aligned)
};

static struct heap_hdr* free_list;
uint8_t*         heap_ptr;
uint64_t         heap_end;
static int              initilized;

void init_heap(uint64_t p_start, uint64_t p_end) {
    free_list  = NULL;
    heap_ptr   = (uint8_t*)(uintptr_t)p_start;
    heap_end   = p_end;
    initilized = 1;
}

void free(void* ptr) {
    if (!ptr || !initilized) return;
    struct heap_hdr* h = (struct heap_hdr*)ptr - 1;
    // insert at head of free list
    h->next    = free_list;
    free_list  = h;
}

void* malloc(uint64_t size) {
    if (!initilized) return NULL;
    // round up and add header
    uint64_t needed = sizeof(struct heap_hdr) + size;
    needed = (needed + 7) & ~7;

    // search free list for a suitable block
    struct heap_hdr** prev = &free_list;
    while (*prev) {
        struct heap_hdr* h = *prev;
        if (h->size >= needed) {
            *prev = h->next;          // unlink
            return (void*)(h + 1);    // return user pointer
        }
        prev = &h->next;
    }

    // bump-allocate a new block
    if ((uint64_t)(uintptr_t)heap_ptr + needed > heap_end)
        return NULL;
    struct heap_hdr* h = (struct heap_hdr*)heap_ptr;
    h->size = needed;
    heap_ptr += needed;
    return (void*)(h + 1);
}

int memcpy(void* dst, const void* src, uint64_t len){
    uint8_t* d = (uint8_t*)dst;
    const uint8_t* s = (const uint8_t*)src;
    for (uint64_t i = 0; i < len; i++)
        d[i] = s[i];
    return 0;
}

int memset(void* buf, uint8_t val, uint64_t size){
    uint8_t* d = (uint8_t*)buf;
    for (uint64_t i = 0; i < size; i++)
        d[i] = val;
    return 0;
}