#include "headers/serial.h"
#include "headers/virtio_mmio.h"
#include "headers/virtqueue.h"
#include <stdint.h>
#include <stdbool.h>

static Virtqueue blk_queue __attribute__((aligned(4096)));
static uint16_t  blk_next_desc = 0;
static uint16_t  blk_avail_idx = 0;
static uint16_t blk_last_used = 0;

void virtio_blk_init(void){
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_STATUS, 0);

    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

    uint32_t features = mmio_read(VIRTIO_BLK_BASE, VIRTIO_MMIO_DEVICE_FEATURES);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_DRVR_FEATURES, features);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_SEL, 0);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE);

    // Pointers to the memory holding the respective parts of the queue
    uint32_t desc_addr  = (uint32_t)&blk_queue.desc;
    uint32_t avail_addr = (uint32_t)&blk_queue.avail;
    uint32_t used_addr = (uint32_t)&blk_queue.used;

    // Fill the locations at the pointers with the correct values
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,    desc_addr);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,   0);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,  avail_addr);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH, 0);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,  used_addr);
    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH, 0);

    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_READY, 1);

    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE
        | VIRTIO_STATUS_DRIVER
        | VIRTIO_STATUS_FEATURES_OK
        | VIRTIO_STATUS_DRIVER_OK);

    serial_puts("virtio-blk: init done\n");
}

typedef struct {
    uint32_t rqst_type;
    uint32_t ignore;
    uint64_t sector;
} VirtioBlockRequest;

uint8_t virtio_blk_read(uint64_t sector, uint32_t length, uint8_t* buf) {
    static VirtioBlockRequest request;
    request.rqst_type = 0;
    request.sector = sector;

    uint16_t d = blk_next_desc % QUEUE_SIZE;
    uint16_t d2 = (blk_next_desc + 1) % QUEUE_SIZE;
    uint16_t d3 = (blk_next_desc + 2) % QUEUE_SIZE;
    blk_next_desc = (blk_next_desc + 3) % QUEUE_SIZE;

    blk_queue.desc[d].addr      = (uint32_t)&request;
    blk_queue.desc[d].len       = 16;
    blk_queue.desc[d].flags     = VIRTQ_DESC_F_NEXT;
    blk_queue.desc[d].next      = d2;

    blk_queue.desc[d2].addr     = (uint32_t)buf;
    blk_queue.desc[d2].len      = length;
    blk_queue.desc[d2].flags    = VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT;
    blk_queue.desc[d2].next     = d3;

    uint8_t status = 0xFF;

    blk_queue.desc[d3].addr     = (uint32_t)&status;
    blk_queue.desc[d3].len      = 1;
    blk_queue.desc[d3].flags    = VIRTQ_DESC_F_WRITE;
    blk_queue.desc[d3].next     = 0;

    blk_queue.avail.ring[blk_avail_idx % QUEUE_SIZE] = d;
    blk_avail_idx++;
    __asm__ volatile("" ::: "memory");
    blk_queue.avail.idx = blk_avail_idx;

    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    uint32_t ready = mmio_read(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_READY);
    if (ready != 1) {
        serial_puts("virtio-blk: queue not ready!\n");
    }

    while (blk_queue.used.idx == blk_last_used) {
        __asm__ volatile("pause");
    }

    uint32_t written = blk_queue.used.ring[blk_last_used % QUEUE_SIZE].len;
    blk_last_used++;

    return status;
}

uint8_t virtio_blk_write(uint64_t sector, uint32_t length, uint8_t* buf) {
    static VirtioBlockRequest request;
    request.rqst_type = 1;
    request.sector = sector;

    uint16_t d = blk_next_desc % QUEUE_SIZE;
    uint16_t d2 = (blk_next_desc + 1) % QUEUE_SIZE;
    uint16_t d3 = (blk_next_desc + 2) % QUEUE_SIZE;
    blk_next_desc = (blk_next_desc + 3) % QUEUE_SIZE;

    blk_queue.desc[d].addr      = (uint32_t)&request;
    blk_queue.desc[d].len       = 16;
    blk_queue.desc[d].flags     = VIRTQ_DESC_F_NEXT;
    blk_queue.desc[d].next      = d2;

    blk_queue.desc[d2].addr     = (uint32_t)buf;
    blk_queue.desc[d2].len      = length;
    blk_queue.desc[d2].flags    = VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT;
    blk_queue.desc[d2].next     = d3;

    uint8_t status = 0xFF;

    blk_queue.desc[d3].addr     = (uint32_t)&status;
    blk_queue.desc[d3].len      = 1;
    blk_queue.desc[d3].flags    = VIRTQ_DESC_F_WRITE;
    blk_queue.desc[d3].next     = 0;

    blk_queue.avail.ring[blk_avail_idx % QUEUE_SIZE] = d;
    blk_avail_idx++;
    __asm__ volatile("" ::: "memory");
    blk_queue.avail.idx = blk_avail_idx;

    mmio_write(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    uint32_t ready = mmio_read(VIRTIO_BLK_BASE, VIRTIO_MMIO_QUEUE_READY);
    if (ready != 1) {
        serial_puts("virtio-blk: queue not ready!\n");
    }

    while (blk_queue.used.idx == blk_last_used) {
        __asm__ volatile("pause");
    }

    uint32_t written = blk_queue.used.ring[blk_last_used % QUEUE_SIZE].len;
    blk_last_used++;

    return status;
}