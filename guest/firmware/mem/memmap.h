#pragma once
#include <stdint.h>
#include "../headers/uefi/uefi.h"

void init_memmap(void);
uint32_t memmap_to_uefi(EFI_MEMORY_DESCRIPTOR* buf, uint32_t length);
