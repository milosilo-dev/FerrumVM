#pragma once
#include <stdint.h>

typedef struct {
    uint64_t capacity;
    uint32_t size_max;
    uint32_t seg_max;
    uint32_t blk_size;
} VirtioBlkConfig;

static VirtioBlkConfig virtio_blk_config = {0, 0, 0, 0};

void virtio_blk_init(void);
uint8_t virtio_blk_read(uint64_t sector, uint32_t length, uint8_t* buf);
uint8_t virtio_blk_write(uint64_t sector, uint32_t length, uint8_t* buf);
void virtio_blk_dump(void);
