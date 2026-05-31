#pragma once
#include <stdint.h>

void tss_init(void);
void gdt_set_tss(uint64_t* gdt, int selector_index);
void tss_enable(uint32_t tss_selector);
