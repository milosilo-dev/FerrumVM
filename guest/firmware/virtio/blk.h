#pragma once
#include <stdint.h>

void virtio_blk_init(void);
uint8_t virtio_blk_read(uint64_t sector, uint32_t length, uint8_t* buf);
uint8_t virtio_blk_write(uint64_t sector, uint32_t length, uint8_t* buf);
void virtio_blk_dump(void);
