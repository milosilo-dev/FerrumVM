#pragma once
#include <stdint.h>

#define QUEUE_SIZE 16   // must be power of 2, <= QUEUE_NUM_MAX

// One entry in the descriptor table
typedef struct {
    uint64_t addr;    // guest physical address of buffer
    uint32_t len;     // length of buffer
    uint16_t flags;   // VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE
    uint16_t next;    // index of next descriptor (if NEXT flag set)
} __attribute__((packed)) VirtqDesc;

#define VIRTQ_DESC_F_NEXT  1   // descriptor chains to next
#define VIRTQ_DESC_F_WRITE 2   // device writes to this buffer (not guest)

// The available ring — guest writes here to give descriptors to device
// NOTE: no used_event field — VIRTIO_F_EVENT_IDX is not negotiated,
// and adding it changes the layout from 36 to 38 bytes, which misaligns
// the used ring (must be 4-byte aligned per the virtio spec).
typedef struct {
    uint16_t flags;
    uint16_t idx;               // next slot guest will write
    uint16_t ring[QUEUE_SIZE];  // descriptor indices
} __attribute__((packed)) VirtqAvail;

// One entry in the used ring
typedef struct {
    uint32_t id;   // descriptor index device used
    uint32_t len;  // bytes written by device
} __attribute__((packed)) VirtqUsedElem;

// The used ring — device writes here when done
// NOTE: no avail_event field — see comment above.
typedef struct {
    uint16_t flags;
    uint16_t idx;                    // next slot device will write
    VirtqUsedElem ring[QUEUE_SIZE];
} __attribute__((packed)) VirtqUsed;

// A complete virtqueue
typedef struct {
    VirtqDesc  desc [QUEUE_SIZE];
    VirtqAvail avail;
    VirtqUsed  used;
} Virtqueue;

static inline void virtio_mb(void) {
    __asm__ volatile("mfence" ::: "memory");
}