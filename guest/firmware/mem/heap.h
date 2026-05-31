#pragma once
#include <stdint.h>
#include <stddef.h>

extern uint8_t* heap_ptr;
extern uint64_t heap_end;

void init_heap(uint64_t p_start, uint64_t p_end);
void free(void* ptr);
void* malloc(uint64_t size);
int memcpy(void* dst, const void* src, uint64_t len);
int memset(void* buf, uint8_t val, uint64_t size);
