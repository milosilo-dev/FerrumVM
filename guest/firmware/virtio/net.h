#pragma once
#include <stdint.h>

typedef struct {
    uint8_t  mac[6];
    uint16_t status;
} VirtioNetConfig;

typedef struct __attribute__((packed)) {
    uint8_t  flags;
    uint8_t  gso_type;
    uint16_t hdr_len;
    uint16_t gso_size;
    uint16_t csum_start;
    uint16_t csum_offset;
} VirtioNetHdr;

void virtio_net_init(void);
uint8_t virtio_net_send(const uint8_t* buf, uint16_t length);
uint16_t virtio_net_poll_receive(uint8_t* buf, uint16_t buf_cap);
void virtio_net_dump(void);