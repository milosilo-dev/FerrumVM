#pragma once
#include <stdint.h>

static uint8_t* heap_ptr;
static uint64_t heap_end = 0x00300000;
static int initilized = 0;

void init_heap(uint64_t p_start, uint64_t p_end) {
    heap_ptr = (uint8_t*)p_start;
    heap_end = p_end;
    initilized = 1;
}

void* malloc(uint64_t size) {
    size = (size + 7) & ~7;

    if (((uint64_t)heap_ptr + size > heap_end) || !initilized) {
        return (void*)0;
    }

    void* ptr = heap_ptr;
    heap_ptr += size;
    return ptr;
}

int memcpy(void* dst, void* src, uint64_t len){
    uint8_t* d = (uint8_t*)dst;
    const uint8_t* s = (const uint8_t*)src;
    for (uint64_t i = 0; i < len; i++)
        d[i] = s[i];
}

int memset(void* buf, uint8_t val, uint64_t size){
    uint8_t* d = (uint8_t*)buf;
    for (uint64_t i = 0; i < size; i++)
        d[i] = val;
}