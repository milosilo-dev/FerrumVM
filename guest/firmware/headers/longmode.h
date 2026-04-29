#pragma once

// Tells C "this function exists, it's defined in assembly"
#include <stdint.h>
extern void enter_long_mode(uint32_t pml4_addr);