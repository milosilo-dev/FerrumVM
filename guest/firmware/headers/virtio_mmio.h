// virtio_mmio.h
#pragma once

#include <stdint.h>
#define VIRTIO_RNG_BASE     0x10001000
#define VIRTIO_CNT_BASE     0x10002000
#define VIRTIO_BLK_BASE     0x10003000

// MMIO register offsets (all 32-bit reads/writes)
#define VIRTIO_MMIO_MAGIC           0x000  // must read 0x74726976 ("virt")
#define VIRTIO_MMIO_VERSION         0x004  // must be 2
#define VIRTIO_MMIO_DEVICE_ID       0x008  // 2 = block device
#define VIRTIO_MMIO_VENDOR_ID       0x00C
#define VIRTIO_MMIO_DEVICE_FEATURES 0x010
#define VIRTIO_MMIO_DRVR_FEATURES   0x020
#define VIRTIO_MMIO_QUEUE_SEL       0x030  // write queue index to select it
#define VIRTIO_MMIO_QUEUE_NUM_MAX   0x034  // read max descriptors supported
#define VIRTIO_MMIO_QUEUE_NUM       0x038  // write how many descriptors you want
#define VIRTIO_MMIO_QUEUE_READY     0x044  // write 1 when queue is set up
#define VIRTIO_MMIO_QUEUE_NOTIFY    0x050  // write queue index to kick device
#define VIRTIO_MMIO_INT_STATUS      0x060  // read to check for interrupt
#define VIRTIO_MMIO_INT_ACK         0x064  // write to acknowledge interrupt
#define VIRTIO_MMIO_STATUS          0x070  // device status register
#define VIRTIO_MMIO_QUEUE_DESC_LOW  0x080  // descriptor table address low 32
#define VIRTIO_MMIO_QUEUE_DESC_HIGH 0x084  // descriptor table address high 32
#define VIRTIO_MMIO_QUEUE_DRIVER_LOW  0x090 // available ring address low 32
#define VIRTIO_MMIO_QUEUE_DRIVER_HIGH 0x094
#define VIRTIO_MMIO_QUEUE_DEVICE_LOW  0x0A0 // used ring address low 32
#define VIRTIO_MMIO_QUEUE_DEVICE_HIGH 0x0A4

// Status register bits (OR these together as you init)
#define VIRTIO_STATUS_ACKNOWLEDGE   1
#define VIRTIO_STATUS_DRIVER        2
#define VIRTIO_STATUS_DRIVER_OK     4
#define VIRTIO_STATUS_FEATURES_OK   8
#define VIRTIO_STATUS_FAILED        128

// Block device request type
#define VIRTIO_BLK_T_IN  0   // read from disk
#define VIRTIO_BLK_T_OUT 1   // write to disk

static inline void mmio_write(uint32_t base, uint32_t offset, uint32_t val) {
    *((volatile uint32_t *)(base + offset)) = val;
}

static inline uint32_t mmio_read(uint32_t base, uint32_t offset) {
    return *((volatile uint32_t *)(base + offset));
}