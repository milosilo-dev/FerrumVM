#pragma once
#include <stdint.h>

typedef struct {
    uint8_t mac[6];
    uint16_t status;
    uint16_t max_virtqueue_pairs;
    uint16_t mtu;
    uint32_t speed;
    uint8_t duplex;
    uint8_t rss_max_key_size;
    uint16_t rss_max_indirection_table_length;
    uint32_t supported_hash_types;
} VirtioNetConfig;

static VirtioNetConfig virtio_net_config = {};

typedef struct {
    uint8_t flags; // Bit 0: Needs checksum; Bit 1: Received packet has valid data; Bit 2: If VIRTIO_NET_F_RSC_EXT was negotiated
    uint8_t segmentation_offload; // 0:None 1:TCPv4 3:UDP 4:TCPv6 0x80:ECN
    uint16_t desc_length; // Size of desc to be used during segmentation.
    uint16_t segment_length; // Maximum segment size (not including desc).
    uint16_t checksum_start; // The position to begin calculating the checksum.
    uint16_t checksum_offset; // The position after ChecksumStart to store the checksum.
    uint16_t buffer_count; // Used when merging buffers.
} VirtioPacketDesc;